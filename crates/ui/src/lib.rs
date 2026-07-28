use rdev::{grab, Event, EventType, Key};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, fs};
use std::time::Instant;
use axum::routing::{get, post};
use axum::{http::{header, StatusCode, Uri}, response::{IntoResponse, Response}, Json, Router};
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use rust_embed::RustEmbed;
use tokio::sync::mpsc;
use notify_rust::{Notification, Timeout};
use talos_core::TalosConfig;
use app_dirs2::{AppDataType, AppInfo, get_app_root};

const ICON_ENABLED_BYTES: &[u8] = include_bytes!("..\\..\\..\\assets\\Icon.png");
const ICON_DISABLED_BYTES: &[u8] = include_bytes!("..\\..\\..\\assets\\Icon_Disabled.png");
const APP_INFO: AppInfo = AppInfo { name: "Talos", author: "NMCreator" };

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerStatus {
    uptime: u64,
    status: i32,
}

#[derive(RustEmbed)]
#[folder = "web_frontend/dist/"]
struct Assets;

#[derive(Clone)]
pub struct AppState {
    start_time: Instant,
    auth_state: talos_auth::AuthState,
    bus_tx: tokio::sync::broadcast::Sender<talos_core::SystemEvent>,
    config: Arc<std::sync::RwLock<TalosConfig>>,
}

impl axum::extract::FromRef<AppState> for talos_auth::AuthState {
    fn from_ref(app_state: &AppState) -> talos_auth::AuthState {
        app_state.auth_state.clone()
    }
}

pub async fn get_server_status(State(state): State<AppState>) -> Json<ServerStatus> {
    Json(ServerStatus {
        uptime: state.start_time.elapsed().as_secs(),
        status: 200,
    })
}

pub async fn handle_ws_talosbus(mut socket: WebSocket, bus_tx: tokio::sync::broadcast::Sender<talos_core::SystemEvent>) {
    let mut bus_rx = bus_tx.subscribe();
    loop {
        tokio::select! {
            Ok(msg) = bus_rx.recv() => {
                if let Ok(json) = serde_json::to_string(&msg)
                    && socket.send(Message::Text(json.into())).await.is_err() {
                        eprintln!("Error sending message");
                        break;
                    }
            }
            msg_result = socket.recv() => {
                let msg = match msg_result {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => {
                        eprintln!("Error in websocket: {}", e);
                        break;
                    }
                    None => {
                        println!("Client disconnected");
                        break;
                    }
                };
                match msg {
                    Message::Text(text) => {
                        println!("Received from client: {}", text);
                    }
                    Message::Binary(bytes) => {
                        println!("Received from client: {:?}", bytes);
                    }
                    Message::Close(_) => {
                        println!("Closing websocket connection");
                        break;
                    }
                    _ => {
                        println!("Received unexpected message: {:?}", msg);
                    }
                }
            }
        }
    }
}

pub async fn get_talosbus_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_talosbus(socket, state.bus_tx))
}

async fn static_handler(uri: Uri) -> Response {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if let Some(index) = Assets::get("index.html") {
                ([(header::CONTENT_TYPE, "text/html")], index.data).into_response()
            } else {
                StatusCode::NOT_FOUND.into_response()
            }
        }
    }
}

pub async fn get_config(State(state): State<AppState>) -> Json<TalosConfig> {
    Json(state.config.read().unwrap().clone())
}

pub async fn update_config(State(state): State<AppState>, Json(new_config): Json<TalosConfig>) -> StatusCode {
    let config_dir = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    new_config.save(&config_dir.join("config.json"));
    if let Ok(mut live_config) = state.config.write() {
        *live_config = new_config;
    }
    StatusCode::OK
}

pub fn get_icon_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let icon_enabled_path = env::temp_dir().join("icon.png");
    let icon_disabled_path = env::temp_dir().join("icon_disabled.png");
    let _ = fs::write(&icon_enabled_path, ICON_ENABLED_BYTES);
    let _ = fs::write(&icon_disabled_path, ICON_DISABLED_BYTES);
    (icon_enabled_path, icon_disabled_path)
}

pub async fn client_backend(stt_disabled: Arc<AtomicBool>, _ui_rx: mpsc::UnboundedReceiver<String>, _config: Arc<std::sync::RwLock<TalosConfig>>) {
    let (icon_enabled_path, icon_disabled_path) = get_icon_paths();
    let (tx, _rx) = mpsc::unbounded_channel();
    let alt_held = Arc::new(AtomicBool::new(false));
    let alt_held_clone = alt_held.clone();
    let callback = move |event: Event| -> Option<Event> {
        match event.event_type {
            EventType::KeyPress(Key::Alt) => {
                alt_held_clone.store(true, Ordering::SeqCst);
                Some(event)
            }
            EventType::KeyRelease(Key::Alt) => {
                alt_held_clone.store(false, Ordering::SeqCst);
                Some(event)
            }
            EventType::KeyPress(Key::KeyM) => {
                if alt_held_clone.load(Ordering::SeqCst) {
                    let _ = tx.send(());
                    if stt_disabled.load(Ordering::Relaxed) {
                        Notification::new()
                            .summary("Microphone Unmuted")
                            .body("Voice Control and STT Available. (Alt+M to Disable)")
                            .icon(icon_enabled_path.to_str().unwrap_or(""))
                            .timeout(Timeout::Milliseconds(6000))
                            .show().ok();
                        stt_disabled.store(false, Ordering::Relaxed);
                    } else {
                        Notification::new()
                            .summary("Microphone Muted")
                            .body("Until Alt+M is pressed again Voice Control will be unavailable.")
                            .icon(icon_disabled_path.to_str().unwrap_or(""))
                            .timeout(Timeout::Milliseconds(6000))
                            .show().unwrap();
                        stt_disabled.store(true, Ordering::Relaxed);
                    }
                    return None;
                }
                Some(event)
            }
            _ => Some(event),
        }
    };
    if let Err(error) = grab(callback) {
        eprintln!("Failed to grab keyboard: {:?}", error);
    }
}



pub async fn server_dashboard(bus_tx: tokio::sync::broadcast::Sender<talos_core::SystemEvent>, config: Arc<std::sync::RwLock<TalosConfig>>) {
    let state = AppState {
        start_time: Instant::now(),
        auth_state: talos_auth::AuthState::new(),
        bus_tx,
        config,
    };
    let app = Router::new()
        .route("/api/status", get(get_server_status))
        .route("/api/talosbus", get(get_talosbus_ws))
        .route("/api/2fa/signup", post(talos_auth::totp_setup_handler))
        .route("/api/2fa/verify", post(talos_auth::totp_verify_handler))
        .route("/api/2fa/login", post(talos_auth::totp_login_handler))
        .route("/api/password/login", post(talos_auth::password_login_handler))
        .route("/api/config", get(get_config).post(update_config))
        .fallback(static_handler)
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8080)).await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}