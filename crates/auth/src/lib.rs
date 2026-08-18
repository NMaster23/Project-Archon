use directories::ProjectDirs;
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use std::fs;
use tui_prompts::{Prompt, TextPrompt, TextRenderStyle, TextState};
use totp_rs::{Algorithm, TOTP, Secret};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use axum::extract::State as AxumState;
use axum::Json;use axum::http::StatusCode;
use cocoon::Cocoon;
use rand::Rng;
use totp_rs::qrcodegen_image::image::EncodableLayout;
use signed_tokens::SigningKey;
use argon2::{self, Config};

#[derive(Clone)]
pub struct UserData {
    pub secret: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

#[derive(Clone)]
pub struct AuthState {
    pub start_time: Instant,
    pub pending_totp: Arc<RwLock<HashMap<String, UserData>>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
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

pub async fn auth(email: Option<&str>, input: &str, username: Option<&str>, password: Option<&str>, case: i32) -> Option<()> {
    let proj_dirs = ProjectDirs::from("com", "NMCreator", "Talos")?;
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir).ok()?;
    let entry = keyring::Entry::new("Talos", "encryption_key").ok()?;
    
    let encryption_password: Vec<u8> = match entry.get_password().ok().and_then(|h| hex::decode(h).ok()) {
        Some(key) => key,
        None => {
            let mut new_key = [0u8; 32];
            rand::rng().fill_bytes(&mut new_key);
            entry.set_password(&hex::encode(new_key)).ok()?;
            new_key.to_vec()
        }
    };
    let mut cocoon = Cocoon::new(&encryption_password);
    let auth_data = AuthData {
        data: input.to_string(),
        username: username.map(|s| s.to_string()),
        password: password.map(|s| s.to_string()),
    };
    let json = serde_json::to_string(&auth_data).ok()?;
    let encrypted: Vec<u8> = cocoon.wrap(json.as_bytes()).ok()?;
    let file_path = if case == 1 {
        let e = email?;
        config_dir.join(format!("{}_totp.info", e))
    } else if case == 2 {
        config_dir.join(format!("{}_api.info", email?))
    } else {
        return None;
    };
    fs::write(file_path, encrypted).ok()?;
    Some(())
}

pub async fn get_auth(email: Option<&str>, case: i32) -> Option<AuthData> {
    let proj_dirs = ProjectDirs::from("com", "NMCreator", "Talos")?;
    let config_dir = proj_dirs.config_dir();
    let file_path = match case {
        1 => {
            let e = email?;
            config_dir.join(format!("{}_totp.info", e))
        },
        2 => config_dir.join(format!("{}_api.info", email?)),
        _ => return None,
    };
    let encrypted = fs::read(file_path).ok()?;
    let entry = keyring::Entry::new("Talos", "encryption_key").ok()?;
    let hex_str = entry.get_password().ok()?;
    let password = hex::decode(hex_str).ok()?;
    let cocoon = Cocoon::new(&password);
    let decrypted = cocoon.unwrap(&encrypted).ok()?;
    serde_json::from_slice(&decrypted).ok()
}

pub async fn issue_session_token(email: &str) -> Option<String> {
    let keyring_entry = keyring::Entry::new("Talos", "session_signing_key").ok()?;
    let keyring_string = match keyring_entry.get_password() {
        Ok(key) => key,
        Err(_) => {
            let mut new_key = [0u8; 32];
            rand::rng().fill_bytes(&mut new_key);
            let hex_key = hex::encode(new_key);
            keyring_entry.set_password(&hex_key).ok()?;
            hex_key
        }
    };
    let raw_key = hex::decode(&keyring_string).ok()?;
    let signing_key = SigningKey::new(raw_key);
    let token = signed_tokens::sign(email.as_bytes(), &[signing_key]).ok()?;
    Some(token.to_string())
}

pub async fn verify_session_token(token: &str) -> Option<String> {
    let key = keyring::Entry::new("Talos", "session_signing_key").ok()?.get_password().ok()?;
    let hex_key = hex::decode(&key).ok()?;
    let signing_key = SigningKey::new(hex_key);
    let verified_token = signed_tokens::verify(token, &[signing_key]).ok()?;
    let session_id = verified_token.payload();
    Some(std::str::from_utf8(session_id).ok()?.to_string())
}

pub async fn totp_setup(email: &str) -> Option<SetupResponse> {
    let secret = Secret::generate_secret();
    let totp = TOTP::new(
        Algorithm::SHA256,
        6,
        1,
        30,
        secret.to_bytes().ok()?,
        Some("Project Archon".to_string()),
        email.to_string(),
    ).ok()?;
    let qr_base64 = totp.get_qr_base64().ok()?;
    let secret_str = secret.to_encoded().to_string();
    Some(SetupResponse {
        qr_code_base64: qr_base64,
        secret_key: secret_str,
    })
}

pub async fn totp_verify(temp_secret: &str, code: &str, email: &str) -> bool {
    let secret_bytes = match Secret::Encoded(temp_secret.to_string()).to_bytes() {
        Ok(b) => b,
        Err(_) => return false,
    };
    let totp = match TOTP::new(
        Algorithm::SHA256,
        6,
        1,
        30,
        secret_bytes,
        Some("Project Archon".to_string()),
        email.to_string(),
    ) {
        Ok(t) => t,
        Err(_) => return false,
    };
    totp.check_current(code).unwrap_or(false)
}

pub async fn totp_setup_handler(
    AxumState(state): AxumState<AuthState>,
    Json(payload): Json<SignUpRequest>,
) -> Result<Json<SetupResponse>, StatusCode> {
    let response = totp_setup(&payload.email).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let encrypted_password = argon2::hash_encoded(&payload.password.as_bytes(), &payload.username.as_bytes(), &Config::default()).expect("Failed to hash password");
    let data = UserData {
        username: payload.username,
        password: encrypted_password,
        secret: response.secret_key.clone()
    };
    if let Ok(mut pending) = state.pending_totp.write() {
        pending.insert(payload.email, data);
    }
    Ok(Json(response))
}

pub async fn totp_verify_handler(
    AxumState(state): AxumState<AuthState>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user_data_opt = {
        let pending = state.pending_totp.read().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        pending.get(&payload.email).cloned()
    };
    if let Some(user_data) = user_data_opt {
        let is_valid = totp_verify(&user_data.secret, &payload.code, &payload.email).await;
        if is_valid {
            auth(Some(&payload.email), &user_data.secret, Some(&user_data.username), Some(&user_data.password), 1).await;
            if let Ok(mut pending) = state.pending_totp.write() {
                pending.remove(&payload.email);
            }
            let token = issue_session_token(&payload.email).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(LoginResponse {
                token,
                username: user_data.username.clone(),
            }))
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
) -> Result<Json<LoginResponse>, StatusCode> {
    let secret = get_auth(Some(&payload.email), 1).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let valid = totp_verify(&secret.data, &payload.code, &payload.email).await;
    if valid {
        let token = issue_session_token(&payload.email).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        let username = secret.username.unwrap_or_else(|| payload.email.split('@').next().expect("Error splitting email").to_string());
        Ok(Json(LoginResponse {
            token,
            username
        }))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn password_login_handler(
    AxumState(_state): AxumState<AuthState>,
    Json(payload): Json<VerifyRequest2>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let secret = get_auth(Some(&payload.email), 1).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    match &secret.password {
        Some(saved_password) if argon2::verify_encoded(&saved_password, payload.password.as_bytes()).unwrap_or(false) => {
            let token = issue_session_token(&payload.email).await.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            let username = secret.username.unwrap_or_else(|| payload.email.split('@').next().expect("Error splitting email").to_string());
            Ok(Json(LoginResponse {
                token,
                username,
            }))
        },
        _ => Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn axum_auth(mut request: axum::extract::Request, next: axum::middleware::Next) -> Result<axum::response::Response, StatusCode> {
    let auth_head = request.headers_mut();
    if !auth_head.contains_key("Authorization") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let auth_str = auth_head.get("Authorization").ok_or(StatusCode::UNAUTHORIZED)?.to_str().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !auth_str.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let token = auth_str.strip_prefix("Bearer ").unwrap_or(auth_str);
    let email = verify_session_token(token).await;
    if email.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    request.extensions_mut().insert(email.unwrap_or_default());
    let response = next.run(request).await;
    Ok(response)
}