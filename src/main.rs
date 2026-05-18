use serde_json::json;
use transcribe_rs::onnx::moonshine::StreamingModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::SpeechModel;
use std::path::PathBuf;
use std::sync::mpsc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use transcribe_rs::TranscribeOptions;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl
};
use oauth2::basic::BasicClient;
use oauth2::reqwest::{Client, ClientBuilder, Url};
use reqwest::redirect::Policy;
use std::{io, thread};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use app_dirs2::*;
use std::fs::File;
use std::io::{Write, Read};

const APP_INFO: AppInfo = AppInfo{name: "Talos", author: "NMCreator"};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct UserData {
    #[serde(rename = "sub")]
    id: String,
    email: String,
    #[serde(rename = "email_verified")]
    verified: bool,
    name: String,
    #[serde(default)]
    given_name: String,
    #[serde(default)]
    family_name: String,
    picture: String,
    #[serde(default)]
    locale: String,
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
}

fn encrypt(secret: &str, data_path: &PathBuf) {
    let mc = new_magic_crypt!("magickey", 256);
    let encrypted = mc.encrypt_str_to_base64(secret);
    let path = data_path.join("user.info");
    println!("{:?}", path);
    let mut file = File::create(path).unwrap();
    file.write_all(encrypted.as_bytes()).unwrap();
}

async fn get_data(data_path: &PathBuf) -> UserData {
    let path = data_path.join("user.info");
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let mc = new_magic_crypt!("magickey", 256);
    let decrypted = mc.decrypt_base64_to_string(&contents).unwrap();
    println!("{}", decrypted);
    let user_data = serde_json::from_str::<UserData>(&decrypted).unwrap();
    user_data
}

async fn user_info(token: &str) -> UserData {
    let userdata = Client::new()
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await.unwrap()
        .json::<UserData>()
        .await.unwrap();
    userdata
}

async fn oauth(data_path: &PathBuf) {
    let client = BasicClient::new(ClientId::new("In File".to_string()))
        .set_client_secret(ClientSecret::new("In File".to_string()))
        .set_auth_uri(AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap())
        .set_token_uri(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new("http://localhost:8080/redirect".to_string()).unwrap());
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/cloud-platform".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();
    println!("Browse to: {}", auth_url);
    let http_client = ClientBuilder::new()
        .redirect(Policy::none())
        .build()
        .expect("Client should build");
    let mut auth_code = String::new();
    io::stdin().read_line(&mut auth_code).unwrap();
    let url = Url::parse(&auth_code.trim().to_string()).unwrap();
    let token = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string());
    let token_result = client
        .exchange_code(AuthorizationCode::new(token.unwrap().trim().to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await.unwrap();
    let token = token_result.access_token().secret();
    let profile = user_info(&token).await;
    let saved_token = token_result.refresh_token().map(|t| t.secret().to_string()).unwrap();
    let user = UserData {
        id: profile.id,
        email: profile.email,
        verified: profile.verified,
        name: profile.name.clone(),
        given_name: profile.given_name,
        family_name: profile.family_name,
        picture: profile.picture,
        locale: profile.locale,
        access_token: token.to_string(),
        refresh_token: saved_token,
    };
    let user_data = serde_json::to_string(&user).unwrap();
    encrypt(&user_data, data_path);
}

async fn refresh_token(token: &str) -> String {
    let response = Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", "387354057252-tenk11q3gakltdvej1uo89lds9ik97pd.apps.googleusercontent.com"),
            ("client_secret", "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"),
            ("refresh_token", token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await.unwrap()
        .json::<serde_json::Value>()
        .await.unwrap();
    let token = response["access_token"].as_str().unwrap().to_string();
    token
}

async fn gemini(token: &str, prompt: &str) -> bool {
    let response = Client::new()
        .post("https://cloudaicompanion.googleapis.com/v1:generateContent".to_string())
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "model": "models/gemini-2.0-flash-001",
            "request": {
                "contents": [
                    {
                        "role": "user",
                        "parts": [{"text": prompt}]
                    }
                ],
                "generationConfig": {
                    "maxOutputTokens": 8192,
                    "temperature": 1.0,
                    "topP": 0.95
                }
            }
        }))
        .send()
        .await.unwrap();
    if response.status() == 401 { 
        println!("Error: 401");
        return false;
    }
    let body = response.text().await.unwrap();
    println!("{}", body);
    true
}

fn stt(tx_out: mpsc::Sender<String>) {
    let mut model = StreamingModel::load(
        &PathBuf::from("models\\moonshine-streaming-small-onnx"),
        4,  // threads
        &Quantization::default(),
    ).unwrap();
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no output device available");
    println!("{}", device.description().unwrap());
    let config = device.default_input_config().unwrap().into();
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec()).ok();
        },
        move |err| {
            println!("an error occurred on the input stream: {}", err);
        },
        None
    ).expect("failed to build stream");
    stream.play().unwrap();
    let mut audio = Vec::new();
    while let Ok(samples) = rx.recv() {
        let sample_rate = config.sample_rate as u32 as usize;
        let channels = config.channels as usize;
        for frame in samples.chunks_exact(channels) {
            let sample = frame.iter().sum::<f32>() / channels as f32;
            audio.push(sample);
        }
        
        if audio.len() >= sample_rate {
            let skip = (sample_rate / 16000).max(1) as usize;
            let filtered: Vec<f32> = audio.iter().step_by(skip).copied().collect();
            let rms = (filtered.iter().map(|x| x * x).sum::<f32>() / filtered.len() as f32).sqrt();
            if rms > 0.002 {
                let result = model.transcribe(&filtered, &TranscribeOptions::default()).unwrap();
                if !result.text.is_empty() && result.text != "Thank you." {
                    if tx_out.send(result.text.clone()).is_err() {
                        break;
                    }
                    println!("{:?}", result);
                }
            }
            audio.drain(..sample_rate);
        }
    }
}

#[tokio::main]
async fn main() {
    let data_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    let user_file = data_path.join("user.info");
    let mut user_data: UserData = if std::fs::exists(user_file).unwrap() {
        get_data(&data_path).await
    } else {
        let _ = std::fs::create_dir_all(&data_path);
        oauth(&data_path).await;
        get_data(&data_path).await
    };
    let (tx_out, rx_out) = mpsc::channel();
    thread::spawn(move || {
        stt(tx_out);
    });
    while let Ok(speech) = rx_out.recv() {
        if !gemini(user_data.access_token.as_str(), &speech).await {
            user_data.access_token = refresh_token(user_data.refresh_token.as_str()).await;
            gemini(user_data.access_token.as_str(), &speech).await;
        }
    }
}