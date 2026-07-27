use directories::ProjectDirs;
use crossterm::{
    ExecutableCommand,
    event::{self, Event},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use serde::{Deserialize, Serialize};
use std::{env, fs};
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
use signed_tokens::SigningKey;

#[derive(Clone)]
pub struct UserData {
    pub secret: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Clone)]
pub struct AuthState {
    pub start_time: Instant,
    pub pending_totp: Arc<RwLock<HashMap<String, UserData>>>,
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
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyRequest {
    pub email: String,
    pub code: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyRequest2 {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub data: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
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

pub async fn auth(email: Option<&str>, input: &str, username: Option<&str>, password: Option<&str>, case: i32) {
    let proj_dirs = ProjectDirs::from("com", "NMCreator", "Talos").expect("Could not find project directories");
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir).unwrap();
    let entry = keyring::Entry::new("Talos", "encryption_key").unwrap();
    let encryption_password: Vec<u8> = match entry.get_password() {
        Ok(hex_str) => hex::decode(hex_str).unwrap(),
        Err(_) => {
            let mut new_key = [0u8; 32];
            OsRng.fill_bytes(&mut new_key);
            entry.set_password(&hex::encode(new_key)).unwrap();
            new_key.to_vec()
        }
    };
    let mut cocoon = Cocoon::new(&encryption_password);
    let auth_data = AuthData {
        data: input.to_string(),
        username: username.map(|s| s.to_string()),
        password: password.map(|s| s.to_string()),
    };
    let json = serde_json::to_string(&auth_data).unwrap();
    let encrypted: Vec<u8> = cocoon.wrap(json.as_bytes()).unwrap();
    let file_path = if case == 1 {
        let e = email.expect("Email required.");
        config_dir.join(format!("{}_totp.info", e))
    } else if case == 2 {
        config_dir.join("user_api.info")
    } else {
        return;
    };
    fs::write(file_path, encrypted).unwrap();
}

pub async fn get_auth(email: Option<&str>, case: i32) -> Option<AuthData> {
    let proj_dirs = ProjectDirs::from("com", "NMCreator", "Talos")?;
    let config_dir = proj_dirs.config_dir();
    let file_path = match case {
        1 => {
            let e = email?;
            config_dir.join(format!("{}_totp.info", e))
        },
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

pub async fn issue_session_token(email: &str) -> String {
    let keyring_entry = keyring::Entry::new("Talos", "session_signing_key").unwrap();
    let keyring_string = match keyring_entry.get_password() {
        Ok(key) => key,
        Err(_) => {
            let mut new_key = [0u8; 32];
            OsRng.fill_bytes(&mut new_key);
            let hex_key = hex::encode(new_key);
            keyring_entry.set_password(&hex_key).unwrap();
            hex_key
        }
    };
    let signing_key = SigningKey::new(keyring_string.as_bytes());
    let token = signed_tokens::sign(email.as_bytes(), &[signing_key]).unwrap();
    token.to_string()
}

pub async fn verify_session_token(token: &str) -> Option<String> {
    let key = keyring::Entry::new("Talos", "session_signing_key").unwrap().get_password().unwrap();
    let signing_key = SigningKey::new(key.as_bytes());
    let verified_token = signed_tokens::verify(&token, &[signing_key]).ok()?;
    let session_id = verified_token.payload();
    Some(std::str::from_utf8(session_id).ok()?.to_string())
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
    let data = UserData {
        username: payload.username,
        password: payload.password,
        secret: response.secret_key.clone()
    };
    if let Ok(mut pending) = state.pending_totp.write() {
        pending.insert(payload.email, data);
    }
    Json(response)
}

pub async fn totp_verify_handler(
    AxumState(state): AxumState<AuthState>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user_data_opt = {
        let pending = state.pending_totp.read().unwrap();
        pending.get(&payload.email).cloned()
    };
    if let Some(user_data) = user_data_opt {
        let is_valid = totp_verify(&user_data.secret, &payload.code, &payload.email).await;
        if is_valid {
            auth(Some(&payload.email), &user_data.secret, Some(&user_data.username), Some(&user_data.password), 1).await;
            if let Ok(mut pending) = state.pending_totp.write() {
                pending.remove(&payload.email);
            }
            let generated_token = issue_session_token(&payload.email).await;
            Ok(Json(LoginResponse { token: generated_token }))
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

pub async fn totp_login_handler(
    AxumState(_state): AxumState<AuthState>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<bool>, StatusCode> {
    let secret = get_auth(Some(&payload.email), 1).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let valid = totp_verify(&secret.data, &payload.code, &payload.email).await;
    if valid {
        Ok(Json(true))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn password_login_handler(
    AxumState(_state): AxumState<AuthState>,
    Json(payload): Json<VerifyRequest2>,
) -> Result<Json<bool>, StatusCode> {
    let secret = get_auth(Some(&payload.email), 1).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    match &secret.password {
        Some(saved_password) if saved_password == &payload.password => {
            Ok(Json(true))
        },
        _ => Err(StatusCode::UNAUTHORIZED)
    }
}