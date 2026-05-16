use transcribe_rs::onnx::moonshine::StreamingModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::SpeechModel;
use std::path::PathBuf;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use transcribe_rs::TranscribeOptions;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl
};
use oauth2::basic::BasicClient;
use oauth2::reqwest::*;
use crate::redirect::Policy;
use std::io;
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use app_dirs2::*;
use std::fs::File;
use std::io::{Write, Read};

const APP_INFO: AppInfo = AppInfo{name: "Talos", author: "NMCreator"};

fn encrypt(secret: &str) {
    let mc = new_magic_crypt!("magickey", 256);
    let encrypted = mc.encrypt_str_to_base64(secret);
    let data_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    if std::fs::exists(&data_path).unwrap() {
        return;
    } else {
        std::fs::create_dir_all(&data_path);
    }
    let path = data_path.join("client.secret");
    println!("{:?}", path);
    let mut file = File::create(path).unwrap();
    file.write_all(encrypted.as_bytes()).unwrap();
}

fn stt() {
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
                    println!("{:?}", result);
                }
            }
            audio.drain(..sample_rate);
        }
    }
}

async fn oauth() {
    let client = BasicClient::new(ClientId::new("681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com".to_string()))
        .set_client_secret(ClientSecret::new("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".to_string()))
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
    let token = token_result.refresh_token().unwrap().secret();
    encrypt(token);
}

#[tokio::main]
async fn main() {
    oauth().await;
}