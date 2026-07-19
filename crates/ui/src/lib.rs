use rdev::{grab, Event, EventType, Key};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, fs, thread};
use std::time::{Duration, Instant};
use axum::routing::{get, post};
use axum::{http::{header, StatusCode, Uri}, response::{IntoResponse, Response}, Json, Router};
use axum::extract::{Query, State};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use rust_embed::RustEmbed;
use tokio::sync::mpsc;
use notify_rust::{Notification, Timeout};

const ICON_ENABLED_BYTES: &[u8] = include_bytes!("..\\..\\..\\assets\\Icon.png");
const ICON_DISABLED_BYTES: &[u8] = include_bytes!("..\\..\\..\\assets\\Icon_Disabled.png");

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerStatus {
    uptime: u64,
    status: i32,
}

#[derive(RustEmbed)]
#[folder = "web_frontend/dist/"]
struct Assets;

#[derive(Clone)]
struct AppState {
    start_time: Instant,
    auth_state: talos_auth::AuthState,
    bus_tx: tokio::sync::broadcast::Sender<talos_core::SystemEvent>,
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
                if let Ok(json) = serde_json::to_string(&msg) {
                    if socket.send(Message::Text(json)).await.is_err() {
                        eprintln!("Error sending message");
                        break;
                    }
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

const DEFAULT_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    frames: &'static [&'static str],
    index: usize,
    last_tick: Instant,
    tick_rate: Duration,
}

impl Spinner {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            frames: DEFAULT_FRAMES,
            index: 0,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }
    pub fn with_custom_frames(tick_rate_ms: u64, frames: &'static [&'static str]) -> Self {
        Self {
            frames,
            index: 0,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }
    pub fn update(&mut self) {
        if self.last_tick.elapsed() >= self.tick_rate {
            self.index = (self.index + 1) % self.frames.len();
            self.last_tick = Instant::now();
        }
    }
    pub fn frame(&self) -> &str {
        self.frames[self.index]
    }
    pub fn reset(&mut self) {
        self.index = 0;
        self.last_tick = Instant::now();
    }
}

struct FadeUI {
    start_time: Instant,
}

impl FadeUI {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

impl eframe::App for FadeUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let duration = 2.0;
        let alpha_factor = (1.0 - (elapsed / duration)).clamp(0.0, 1.0);

        if alpha_factor <= 0.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let bg_alpha = (150.0 * alpha_factor) as u8;
        let frame = egui::Frame::none().fill(egui::Color32::from_black_alpha(bg_alpha));
        
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let text_alpha = (255.0 * alpha_factor) as u8;
                ui.colored_label(
                    egui::Color32::from_white_alpha(text_alpha),
                    egui::RichText::new("Alt + M Pressed!").size(32.0)
                );
            });
        });

        ctx.request_repaint();
    }
}

pub async fn client_backend(stt_disabled: Arc<AtomicBool>, ui_rx: tokio::sync::mpsc::UnboundedReceiver<String>) {
    let icon_enabled_path = env::temp_dir().join("icon.png");
    let icon_disabled_path = env::temp_dir().join("icon_disabled.png");
    fs::write(&icon_enabled_path, ICON_ENABLED_BYTES).unwrap();
    fs::write(&icon_disabled_path, ICON_DISABLED_BYTES).unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
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



pub async fn server_dashboard(bus_tx: tokio::sync::broadcast::Sender<talos_core::SystemEvent>) {
    let state = AppState {
        start_time: Instant::now(),
        auth_state: talos_auth::AuthState::new(),
        bus_tx,
    };
    let app = Router::new()
        .route("/api/status", get(get_server_status))
        .route("/api/talosbus", get(get_talosbus_ws))
        .route("/api/2fa/signup", post(talos_auth::totp_setup_handler))
        .route("/api/2fa/verify", post(talos_auth::totp_verify_handler))
        .route("/api/2fa/login", post(talos_auth::totp_login_handler))
        .route("/api/password/login", post(talos_auth::password_login_handler))
        .fallback(static_handler)
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8080)).await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}