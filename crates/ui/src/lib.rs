use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use axum::routing::get;
use axum::{http::{header, StatusCode, Uri}, response::{IntoResponse, Response}, Json, Router};
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use rust_embed::RustEmbed;

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
}

pub async fn get_server_status(State(state): State<AppState>) -> Json<ServerStatus> {
    Json(ServerStatus {
        uptime: state.start_time.elapsed().as_secs(),
        status: 200,
    })
}

pub async fn handle_ws_talosbus(mut socket: WebSocket) {
    while let Some(msg_result) = socket.recv().await {
        let msg = match msg_result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Error in websocket: {}", e);
                break;
            }
        };
        match msg {
            Message::Text(text) => {
                println!("{}", text);
                if let Err(e) = socket.send(Message::Text(format!("Echo: {}", text))).await {
                    eprintln!("Failed to send message: {}", e);
                    break;
                }
            }
            Message::Binary(bytes) => {
                println!("Bytes: {:?}", bytes);
            }
            Message::Close(_) => {
                println!("Closing websocket connection");
                break;
            }
            _ => {
                println!("Unknown Error");
            }
        }
    }
}

pub async fn get_talosbus_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_ws_talosbus(socket))
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


pub async fn dashboard(
    stt_enabled: Arc<AtomicBool>,
    mut rx_out: tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    let mut chat_history: Vec<String> = Vec::new();
    let mut spinner = Spinner::new(100);
    let mut processing = false;
    loop { spinner.update();
        terminal
            .draw(|f| {
                let stt_status_text = if stt_enabled.load(Ordering::Relaxed) {
                    "STT Status: Muted (Press 'm' to unmute)"
                } else {
                    "STT Status: Unmuted (Press 'm' to mute)"
                };
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(3)])
                    .split(f.area());
                let display_status_text = if processing { format!("{} {}", spinner.frame(), stt_status_text) } else { stt_status_text.to_string() };
                let stt_show_text = Paragraph::new(display_status_text)
                    .style(if stt_enabled.load(Ordering::Relaxed) {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Talos Dashboard"),
                    );
                f.render_widget(stt_show_text, chunks[0]);

                let chat_text = chat_history.join("\n");
                let paragraph = Paragraph::new(chat_text)
                    .block(Block::default().borders(Borders::ALL).title("Chat"))
                    .wrap(Wrap { trim: true });
                f.render_widget(paragraph, chunks[1]);
            })
            .unwrap();
        if event::poll(Duration::from_millis(16)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('m') => {
                            let current_state = stt_enabled.load(Ordering::Relaxed);
                            stt_enabled.store(!current_state, Ordering::Relaxed);
                        }
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
            }
        }
        while let Ok(message) = rx_out.try_recv() {
            match message.as_str() {
                "__PROCESSING_START__" => {
                    processing = true;
                    spinner.reset();
                    break;
                }
                "__PROCESSING_END__" => {
                    processing = false;
                    break;
                }
                other => chat_history.push(other.to_string()),
            }
        }
        thread::sleep(Duration::from_millis(16));
    }
    disable_raw_mode().unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
}

pub async fn server_dashboard() {
    let state = AppState {
        start_time: Instant::now(),
    };
    let app = Router::new()
        .route("/api/status", get(get_server_status))
        .route("/api/talosbus", get(get_talosbus_ws))
        .fallback(static_handler)
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8080)).await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}