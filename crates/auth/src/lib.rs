use directories::ProjectDirs;
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
use cocoon::Cocoon;
use rand::{rngs::OsRng, RngCore};
use totp_rs::qrcodegen_image::image::EncodableLayout;

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

pub async fn auth(input: &str, case: i32) {
    let proj_dirs = ProjectDirs::from("com", "NMaster23", "Talos").expect("Could not find project directories");
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir).unwrap();
    let entry = keyring::Entry::new("Talos", "encryption_key").unwrap();
    let password: Vec<u8> = match entry.get_password() {
        Ok(hex_str) => hex::decode(hex_str).unwrap(),
        Err(_) => {
            let mut new_key = [0u8; 32];
            OsRng.fill_bytes(&mut new_key);
            entry.set_password(&hex::encode(new_key)).unwrap();
            new_key.to_vec()
        }
    };
    let mut cocoon = Cocoon::new(&password);
    let auth_data = AuthData {
        data: input.to_string(),
    };
    let json = serde_json::to_string(&auth_data).unwrap();
    let encrypted: Vec<u8> = cocoon.wrap(json.as_bytes()).unwrap();
    if case == 1 {
        fs::write(config_dir.join("totp_code.info"), encrypted).unwrap();
    } else if case == 2 {
        fs::write(config_dir.join("user_api.info"), encrypted).unwrap();
    }
}

pub async fn get_auth(case: i32) -> Option<AuthData> {
    let proj_dirs = ProjectDirs::from("com", "NMaster23", "Talos")?;
    let config_dir = proj_dirs.config_dir();
    let file_path = match case {
        1 => config_dir.join("totp_code.info"),
        2 => config_dir.join("user_api.info"),
        _ => return None,
    };
    let encrypted = fs::read(file_path).ok()?;
    let entry = keyring::Entry::new("Talos", "encryption_key").ok()?;
    let hex_str = entry.get_password().ok()?;
    let password = hex::decode(hex_str).ok()?;
    let mut cocoon = Cocoon::new(&password);
    let decrypted = cocoon.unwrap(&encrypted).ok()?;
    serde_json::from_slice(&decrypted).ok()
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
            auth(&secret, 1).await;
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

pub async fn totp_login_handler(
    AxumState(state): AxumState<AuthState>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<bool>, StatusCode> {
    let secret = get_auth(1).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let valid = totp_verify(&secret.data, &payload.code, &payload.email).await;
    if valid {
        Ok(Json(true))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}