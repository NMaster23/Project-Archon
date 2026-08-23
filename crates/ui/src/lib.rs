use rdev::{grab, Event, EventType, Key};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{env, fs};
use std::collections::HashMap;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use axum::routing::{get, post};
use axum::{http::{header, StatusCode, Uri}, response::{IntoResponse, Response}, Json, Router, Extension};
use axum::extract::{Multipart, Query, State};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use rust_embed::RustEmbed;
use tokio::sync::mpsc;
use notify_rust::{Notification, Timeout};
use app_dirs2::{AppDataType, AppInfo, get_app_root};
use serde_json::{json, Value};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::io::AsyncBufReadExt;
use tokio::time;
use tower_http::classify::GrpcFailureClass::Status;
use talos_core::{ClientConfig, ServerConfig, UserPreferences};
use talos_core::ConfigFile;
use turso::Builder;
use talos_auth::verify_session_token;
use std::time::{SystemTime, UNIX_EPOCH};

const ICON_ENABLED_BYTES: &[u8] = include_bytes!("../../../assets/Icon.png");
const ICON_DISABLED_BYTES: &[u8] = include_bytes!("../../../assets/Icon_Disabled.png");
const APP_INFO: AppInfo = AppInfo { name: "Talos", author: "NMCreator" };

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ServerStatus {
    uptime: u64,
    status: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PermissionPayload {
    pub access_level: i32,
}

#[derive(RustEmbed)]
#[folder = "web_frontend/dist/"]
struct Assets;

#[derive(Clone)]
pub struct AppState {
    start_time: Instant,
    auth_state: talos_auth::AuthState,
    bus_tx: tokio::sync::broadcast::Sender<talos_core::SystemEvent>,
    config: Arc<std::sync::RwLock<ServerConfig>>,
    db_conn: turso::Connection,
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
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let token = match params.get("token") {
        Some(token) => token,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if verify_session_token(token).await.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
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

pub async fn get_server_config(State(state): State<AppState>) -> Response {
    let config = match state.config.read() {
        Ok(config) => config.clone(),
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Config fetch error").into_response();
        }
    };
    Json(config).into_response()
}

pub async fn get_user_preferences(Extension(email): Extension<String>) -> Response {
    let config_dir = match get_app_root(AppDataType::UserConfig, &APP_INFO) {
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Could not find path").into_response();
        }
    };
    let preferences_path = config_dir.join(format!("{}_prefs.json", email));
    let preferences = UserPreferences::load(&preferences_path, "{}");
    Json(preferences).into_response()
}

pub async fn update_user_prefs(Extension(email): Extension<String>, Json(new_prefs): Json<UserPreferences>) -> Response {
    let config_dir = match get_app_root(AppDataType::UserConfig, &APP_INFO) {
        Ok(dir) => dir,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let preferences_path = config_dir.join(format!("{}_prefs.json", email));
    new_prefs.save(&preferences_path);
    StatusCode::OK.into_response()
}

pub async fn update_server_config(State(state): State<AppState>, Json(new_config): Json<ServerConfig>) -> Response {
    let config_dir = match get_app_root(AppDataType::UserConfig, &APP_INFO) {
        Ok(dir) => dir.join("config"),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    new_config.save(&config_dir.join("server_config.json"));
    match state.config.write() {
        Ok(mut config) => {
            *config = new_config;
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }
    StatusCode::OK.into_response()
}

pub fn get_icon_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let icon_enabled_path = env::temp_dir().join("icon.png");
    let icon_disabled_path = env::temp_dir().join("icon_disabled.png");
    let _ = fs::write(&icon_enabled_path, ICON_ENABLED_BYTES);
    let _ = fs::write(&icon_disabled_path, ICON_DISABLED_BYTES);
    (icon_enabled_path, icon_disabled_path)
}

pub async fn client_backend(stt_disabled: Arc<AtomicBool>, _ui_rx: mpsc::UnboundedReceiver<String>, _config: Arc<std::sync::RwLock<ClientConfig>>) {
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
                            .show().ok();
                        stt_disabled.store(true, Ordering::Relaxed);
                    }
                    return None;
                }
                Some(event)
            }
            _ => Some(event),
        }
    };
    tokio::task::spawn_blocking(move || {
        if let Err(error) = grab(callback) {
            eprintln!("Failed to grab keyboard: {:?}", error);
        }
    });
}

pub async fn install_cloudflare() {
    #[cfg(target_os = "windows")]
    let (shell, args) = (
        "powershell",
        [
            "-Command",
            r#"Invoke-WebRequest "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe" -OutFile "cloudflared.exe"; Start-Process -FilePath ".\cloudflared.exe" -ArgumentList "service install" -Verb RunAs -Wait"#,
        ],
    );

    #[cfg(target_os = "macos")]
    let (shell, args) = (
        "sh",
        [
            "-c",
            r#"curl -L "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-darwin-$(uname -m | sed 's/x86_64/amd64/').tgz" | tar -xz && sudo mv cloudflared /usr/local/bin/"#
        ],
    );

    #[cfg(target_os = "linux")]
    let (shell, args) = (
        "sh",
        [
            "-c",
            r#"curl -L "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64" -o cloudflared && chmod +x cloudflared && sudo mv cloudflared /usr/local/bin"#
        ],
    );

    let status = Command::new(shell)
        .args(args)
        .status()
        .await
        .expect("Failed to start cloudflared installer");
    if status.success() {
        println!("Installed cloudflared successfully");
    } else {
        eprintln!("Installation failed with status: {}", status);
    }
}

pub async fn setup_cloudflare() {
    let bin_name = if std::path::Path::new("cloudflared.exe").exists() {
        ".\\cloudflared.exe"
    } else {
        "cloudflared"
    };
    match Command::new(bin_name).arg("--version").output().await {
        Ok(output) => {
            println!("Cloudflared installed and version is {}", String::from_utf8_lossy(&output.stdout));
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                println!("Failed to find cloudflared");
                install_cloudflare().await;
            } else {
                println!("Unknown error: {:?}", e);
            }
        }
    }
}

pub async fn spawn_cloudflare(port: u16, token: Option<String>) -> Result<String, String> {
    setup_cloudflare().await;
    let bin_name = if std::path::Path::new("cloudflared.exe").exists() {
        ".\\cloudflared.exe"
    } else {
        "cloudflared"
    };
    if let Some(t) = token {
        let _child = Command::new(bin_name)
            .args(["tunnel", "run", "--token"])
            .arg(&t)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn cloudflare (User Domain)");
        Ok("Using User Domain".to_string())
    } else {
        let mut child = Command::new(bin_name)
            .args(["tunnel", "--url"])
            .arg(format!("http://127.0.0.1:{}", port))
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn cloudflare (Auto Domain)");
        let stderr_handle = child.stderr.take().expect("Failed to take cloudflare (Auto Domain)");
        let mut reader = BufReader::new(stderr_handle).lines();
        while let Some(line) = reader.next_line().await.expect("failed to read line") {
            if line.contains("trycloudflare.com") && let Some(start_index) = line.find("https://") {
                let substring = &line[start_index..];
                if let Some(index) = substring.find(|c: char| c.is_whitespace() || c == '|') {
                    let final_url = &substring[0..index];
                    return Ok(final_url.to_string());
                }
            }
        }
        Err("Process closed without giving a URL".to_string())
    }
}

pub async fn get_plugin_config(axum::extract::Path(plugin_id): axum::extract::Path<String>, State(state): State<AppState>) -> Response {
    let conn = state.db_conn;
    let mut rows = conn.query("SELECT key, value FROM plugins WHERE plugin_id = ?", (plugin_id,)).await.expect("query failed");
    let mut settings = serde_json::Map::new();
    while let Ok(Some(row)) = rows.next().await {
        let key: String = row.get(0).expect("Failed to get key");
        let raw_val: String = row.get(1).expect("Failed to get value");
        let parsed_val = serde_json::from_str(&raw_val).expect("Failed to parse json");
        settings.insert(key, parsed_val);
    }
    Json(serde_json::Value::Object(settings)).into_response()
}

pub async fn update_plugin_config(axum::extract::Path(plugin_id): axum::extract::Path<String>, State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> Response {
    let conn = state.db_conn;
    if let Some(settings) = payload.as_object() {
        for (key, parsed_val) in settings.iter() {
            let value_str = parsed_val.to_string();
            let _ = conn.execute("INSERT OR REPLACE INTO plugins (plugin_id, key, value) VALUES (?, ?, ?)", (plugin_id.clone(), key.clone(), value_str)).await.expect("Could not insert plugin");
        }
        return StatusCode::OK.into_response();
    } else {
        return (StatusCode::BAD_REQUEST, "Payload must be JSON").into_response();
    }
}

pub async fn update_plugin_permissions(axum::extract::Path(plugin_id): axum::extract::Path<String>, State(state): State<AppState>, Json(payload): Json<PermissionPayload>) -> Response {
    let conn = state.db_conn;
    conn.execute(
        "INSERT OR REPLACE INTO plugin_permissions (plugin_id, access_level) VALUES (?, ?)",
        (plugin_id, payload.access_level)
    ).await.expect("Could not update plugin permissions");
    return StatusCode::OK.into_response();
}

pub async fn install_plugin(mut multipart: Multipart) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    while let Ok(Some(mut field)) = multipart.next_field().await {
        if field.name() == Some("plugin_binary") {
            let formatted_filename = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| format!("{}_.wasm", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis())).to_string();
            let safe_filename = std::path::Path::new(&formatted_filename)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
                .unwrap_or_else(|| format!("{}_.wasm", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()));
            let file_path = get_app_root(AppDataType::UserConfig, &APP_INFO)
                .expect("Failed to get WASM Directory")
                .join("Plugins")
                .join(&safe_filename);
            let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })))
            })?;
            while let Ok(Some(chunk)) = field.chunk().await {
                file.write_all(&chunk).await.map_err(|e| {
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })))
                })?;
            }
            return Ok(Json(json!({
                "success": true,
                "message": "Plugin saved successfully",
                "filename": safe_filename
            })));
        }
    }
    Err((
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "Missing plugin_binary field" }))
    ))
}

pub async fn server_dashboard(bus_tx: tokio::sync::broadcast::Sender<talos_core::SystemEvent>, config: Arc<std::sync::RwLock<ServerConfig>>) {
    let app_root = get_app_root(AppDataType::UserConfig, &APP_INFO).expect("Failed to get user config");
    let plugin_dir = app_root.join("Plugins");
    std::fs::create_dir_all(&plugin_dir).expect("Failed to create plugin directory");
    let plugin_db = plugin_dir.join("plugins.db");
    let db = Builder::new_local(plugin_db.to_str().expect("Failed to convert db path to str")).build().await.expect("Failed to create/access plugin database");
    let conn = db.connect().expect("Failed to connect to database");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugins (
        plugin_id TEXT,
        key TEXT,
        value TEXT,
        vector F32_BLOB(384),
        PRIMARY KEY (plugin_id, key)
        )",
        (),
    ).await.expect("Could not create plugin database");
    let state = AppState {
        start_time: Instant::now(),
        auth_state: talos_auth::AuthState::new(),
        bus_tx,
        config: config.clone(),
        db_conn: conn,
    };
    let config = state.config.clone();
    let cloudflare_token = config.read().expect("Cloudflare config error").cloudflare_token.clone();
    let private_router = Router::new()
        .route("/api/config", get(get_server_config).post(update_server_config))
        .route("/api/user/prefs", get(get_user_preferences).post(update_user_prefs))
        .route("/api/plugins/{plugin_id}/permissions", post(update_plugin_permissions))
        .route("/api/plugins/{plugin_id}/config", get(get_plugin_config).post(update_plugin_config))
        .route("/api/plugins/install", post(install_plugin))
        .route_layer(axum::middleware::from_fn(talos_auth::axum_auth))
        .with_state(state.clone());
    let public_router = Router::new()
        .route("/api/talosbus", get(get_talosbus_ws))
        .route("/api/status", get(get_server_status))
        .route("/api/2fa/signup", post(talos_auth::totp_setup_handler))
        .route("/api/2fa/verify", post(talos_auth::totp_verify_handler))
        .route("/api/2fa/login", post(talos_auth::totp_login_handler))
        .route("/api/password/login", post(talos_auth::password_login_handler))
        .with_state(state.clone());
    let app = Router::new()
        .merge(private_router)
        .merge(public_router)
        .fallback(static_handler)
        .with_state(state);
    tokio::spawn(async move {
        match spawn_cloudflare(8080, cloudflare_token).await {
            Ok(url) => {
                println!("Dashboard online at: {}", url);
            }
            Err(error) => {
                eprintln!("Failed to spawn cloudflare: {:?}", error);
            }
        }
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8080)).await.expect("Failed to bind");
    println!("Listening on {}", listener.local_addr().expect("Could not get local address"));
    axum::serve(listener, app).await.expect("Error starting server dashboard");
}