use crossterm::{
    ExecutableCommand,
    event::{self, Event},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tui_prompts::{Prompt, State, Status, TextPrompt, TextRenderStyle, TextState};
use totp_rs::{Algorithm, TOTP, Secret};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use axum::extract::State as AxumState;
use axum::Json;use axum::http::StatusCode;
#[derive(Clone)]
pub struct AuthState {
    pub start_time: Instant,
    pub pending_totp: Arc<RwLock<HashMap<String, String>>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            pending_totp: Arc::new(RwLock::new(HashMap::new()))
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SignUpRequest {
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyRequest {
    pub email: String,
    pub code: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub data: String,
}

#[derive(Serialize, Deserialize)]
pub struct SetupResponse {
    pub qr_code_base64: String,
    pub secret_key: String,
}

pub struct App<'a> {
    apikey_state: TextState<'a>,
}

impl<'a> App<'a> {
    fn draw_ui(&mut self, frame: &mut Frame) {
        TextPrompt::from("Gemini API Key")
            .with_render_style(TextRenderStyle::Password)
            .draw(frame, frame.area(), &mut self.apikey_state);
    }
}

pub async fn auth(path: &PathBuf) {
    let mut stdout = std::io::stdout();
    enable_raw_mode().unwrap();
    stdout.execute(EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App {
        apikey_state: TextState::new(),
    };
    let api_key = loop {
        terminal.draw(|f| app.draw_ui(f)).unwrap();
        if let Event::Key(key) = event::read().unwrap() {
            app.apikey_state.handle_key_event(key);

            if app.apikey_state.status() == Status::Done {
                break app.apikey_state.value().to_string();
            } else if app.apikey_state.status() == Status::Aborted {
                break String::new();
            }
        }
    };
    disable_raw_mode().unwrap();
    std::io::stdout().execute(LeaveAlternateScreen).unwrap();
    if !api_key.is_empty() {
        let auth_data = AuthData {
            data: api_key.trim().to_string(),
        };
        let json = serde_json::to_string(&auth_data).unwrap();
        fs::write(path.join("user_api.info"), json).unwrap();
    }
}

pub async fn get_auth(path: &PathBuf) -> AuthData {
    let json = fs::read_to_string(path.join("user_api.info")).unwrap();
    serde_json::from_str(&json).unwrap()
}

pub async fn totp_setup(email: &str) -> SetupResponse {
    let secret = Secret::generate_secret();
    let totp = TOTP::new(
        Algorithm::SHA256,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        Some("Project Archon".to_string()),
        email.to_string(),
    ).unwrap();
    let qr_base64 = totp.get_qr_base64().unwrap();
    let secret_str = secret.to_encoded().to_string();
    SetupResponse {
        qr_code_base64: qr_base64,
        secret_key: secret_str,
    }
}

pub async fn totp_verify(temp_secret: &str, code: &str, email: &str) -> bool {
    let totp = TOTP::new(
        Algorithm::SHA256,
        6,
        1,
        30,
        Secret::Encoded(temp_secret.to_string()).to_bytes().unwrap(),
        Some("Project Archon".to_string()),
        email.to_string(),
    ).unwrap();
    totp.check_current(code).unwrap_or(false)
}

pub async fn totp_setup_handler(
    AxumState(state): AxumState<AuthState>,
    Json(payload): Json<SignUpRequest>,
) -> Json<SetupResponse> {
    let response = totp_setup(&payload.email).await;
    if let Ok(mut pending) = state.pending_totp.write() {
        pending.insert(payload.email, response.secret_key.clone());
    }
    Json(response)
}

pub async fn totp_verify_handler(
    AxumState(state): AxumState<AuthState>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<bool>, StatusCode> {
    let secret_opt = {
        let pending = state.pending_totp.read().unwrap();
        pending.get(&payload.email).cloned()
    };
    
    if let Some(secret) = secret_opt {
        let is_valid = totp_verify(&secret, &payload.code, &payload.email).await;
        if is_valid {
            if let Ok(mut pending) = state.pending_totp.write() {
                pending.remove(&payload.email);
            }
            Ok(Json(true))
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}