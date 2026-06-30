use serde_json::Value;
use serde_json::json;
use tokio::time::interval;
use transcribe_rs::onnx::moonshine::StreamingModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::SpeechModel;
use base64::prelude::*;
use xcap::image;
use std::io::Cursor;
use std::rc::Rc;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;
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
use gemini_live::session::{Session, SessionConfig, ReconnectPolicy};
use gemini_live::transport::{Auth, TransportConfig};
use gemini_live::types::*;
use std::num::{NonZeroU16, NonZeroU32};
use rodio::buffer::SamplesBuffer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use xcap::Monitor;

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
    client_id: String,
    client_secret: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct APIData {
    api_key: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct OauthCredentials {
    access_token: String,
    scope: String,
    token_type: String,
    id_token: String,
    expiry_date: String,
    refresh_token: String,
}

fn encrypt(encrypt: &str, data_path: &PathBuf, file_name: &str) {
    let mc = new_magic_crypt!("magickey", 256);
    let encrypted = mc.encrypt_str_to_base64(encrypt);
    let path = data_path.join(file_name);
    println!("{:?}", path);
    let mut file = File::create(path).unwrap();
    file.write_all(encrypted.as_bytes()).unwrap();
}

async fn auth(data_path: &PathBuf) {
    println!("Please enter your API key: [Input needed]");
    // Dummy key since no actual input section is requested
    let api_key = "dummy_key_replace_me".to_string();
    let api_data = APIData {
        api_key,
    };
    println!("{:?}", api_data.api_key);
    let api_data = serde_json::to_string(&api_data).unwrap();
    encrypt( api_data.as_str(), data_path, "user_api.info");
}

async fn get_auth(data_path: &PathBuf) -> APIData {
    let path = data_path.join("user_api.info");
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let mc = new_magic_crypt!("magickey", 256);
    let decrypted = mc.decrypt_base64_to_string(&contents).unwrap();
    println!("Decrypted: {}", decrypted);
    let api_data = serde_json::from_str::<APIData>(&decrypted).unwrap();
    api_data
}

async fn gemini_communicate(mut session: Session, mut rx_out: tokio::sync::mpsc::UnboundedReceiver<String>, mut tx_in: tokio::sync::mpsc::UnboundedSender<String>) -> Result<(), Box<dyn std::error::Error>> {
    let handle = rodio::DeviceSinkBuilder::open_default_sink()
        .expect("open default audio stream");
    let player = rodio::Player::connect_new(&handle.mixer());
    player.play();
    loop {
        let mut speech = String::new();
        match rx_out.recv().await {
            Some(speech_input) => {
                let processed = speech_input.trim().to_string();
                if !processed.is_empty() {
                    speech.push_str(&processed);
                }
            }
            None => break,
        }
        loop {
            match tokio::time::timeout(Duration::from_secs(1), rx_out.recv()).await {
                Ok(Some(input_speech)) => {
                    let processed = input_speech.trim().to_string();
                    if !processed.is_empty() {
                        if !speech.is_empty() {
                            speech.push_str(&" ".to_string());
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
        if speech.is_empty() {
            continue;
        }
        session.send_text(&speech).await?;
        println!("User: {}", speech);
        let mut gemini_response = String::new();
        let mut audio_buffer: Vec<u8> = Vec::new();
        while let Some(event) = session.next_event().await {
            match event {
                ServerEvent::ModelAudio(audio) => {
                    print!("{:?}", audio.len());
                    audio_buffer.extend_from_slice(&audio);
                    let valid_len = audio_buffer.len() - (audio_buffer.len() % 2);
                    let samples: Vec<f32> = audio_buffer[..valid_len]
                        .chunks_exact(2)
                        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0)
                        .collect();
                    audio_buffer.drain(..valid_len);
                    let source = SamplesBuffer::new(NonZeroU16::new(1).unwrap(), NonZeroU32::new(24000).unwrap(), samples);
                    player.append(source);
                }
                ServerEvent::ModelText(text) => {
                    print!("{text}");
                    gemini_response.push_str(&text);
                }
                ServerEvent::TurnComplete => {
                    println!("\n--- turn done ---");
                    println!("Gemini: {}", gemini_response);
                    break;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

async fn gemini_api(api_key: &str, mut rx_out: tokio::sync::mpsc::UnboundedReceiver<String>, mut tx_in: tokio::sync::mpsc::UnboundedSender<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("API completed (UI hook removed)");
    let mut session = Session::connect(SessionConfig {
        transport: TransportConfig {
            auth: Auth::ApiKey(api_key.to_string()),
            ..Default::default()
        },
        setup: SetupConfig {
            model: "models/gemini-3.1-flash-live-preview".into(),
            generation_config: Some(GenerationConfig {
                response_modalities: Some(vec![Modality::Audio]),
                ..Default::default()
            }),
            ..Default::default()
        },
        reconnect: ReconnectPolicy::default(),
    }).await?;
    gemini_communicate(session, rx_out, tx_in).await?;
    Ok(())
}

async fn get_data_other() -> OauthCredentials {
    let user = whoami::realname().unwrap_or_else(|_| "<unknown>".to_string());
    let home = dirs::home_dir().expect("No home directory found");
    let oauth_creds = home.clone().join(".gemini").join("oauth_creds.json");
    let credentials_file = File::open(oauth_creds).expect("Error");
    let credentials: OauthCredentials = serde_json::from_reader(credentials_file).unwrap();
    credentials
}

async fn authenticate_other() -> String {
    let credentials = get_data_other().await;
    let client_id = "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
    let client_secret = "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";
    let client = Client::new();
    let token_resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await.unwrap();
    let token: Value = token_resp.json().await.unwrap();
    token["access_token"].as_str().unwrap();
}

async fn gemini_other(mut rx_out: tokio::sync::mpsc::UnboundedReceiver<String>, mut tx_in: tokio::sync::mpsc::UnboundedSender<String>) -> Result<(), Box<dyn std::error::Error>> {
    let access_token = authenticate_other().await;
    let model = "gemini-3.1-flash-live-preview";
    let mut session = Session::connect(SessionConfig {
        transport: TransportConfig {
            auth: Auth::BearerToken(access_token),
            ..Default::default()
        },
        setup: SetupConfig {
            model: "models/gemini-3.1-flash-live-preview".into(),
            generation_config: Some(GenerationConfig {
                response_modalities: Some(vec![Modality::Audio]),
                ..Default::default()
            }),
            ..Default::default()
        },
        reconnect: ReconnectPolicy::default(),
    }).await?;
    gemini_communicate(session, rx_out, tx_in).await    
}

fn stt(tx_out: tokio::sync::mpsc::UnboundedSender<String>, speaking: Arc<AtomicBool>) {
    let mut model = StreamingModel::load(
        &PathBuf::from("models\\moonshine-streaming-small-onnx"),
        4,
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
            if speaking.load(Ordering::Relaxed) {
                audio.clear();
            }
            let filtered: Vec<f32> = audio.iter().step_by(skip).copied().collect();
            let rms = (filtered.iter().map(|x| x * x).sum::<f32>() / filtered.len() as f32).sqrt();
            if rms > 0.002 {
                let result = model.transcribe(&filtered, &TranscribeOptions::default()).unwrap();
                if !result.text.is_empty() {
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

async fn screencap(tx_img: tokio::sync::mpsc::UnboundedSender<String>) {
    let monitors = Monitor::all().unwrap();
    let primary = monitors.into_iter().next().expect("No monitor");
    let mut tick = interval(Duration::from_secs(1));
    let mut prev_frame: Option<image::DynamicImage> = None;
    loop {
        tick.tick().await;
        if let Ok(capture) = primary.capture_image() {
            let current_frame = image::DynamicImage::ImageRgba8(capture);
            let tx = tx_img.clone();
            tokio::task::spawn_blocking(move || {
                let resized = current_frame.resize(1280, 720, image::imageops::FilterType::Triangle);
                let mut cursor = Cursor::new(Vec::new());
                if resized.write_to(&mut cursor, image::ImageFormat::Jpeg).is_ok() {
                    let jpg_bytes = cursor.into_inner();
                    let b64 = BASE64_STANDARD.encode(&jpg_bytes);
                    let _ = tx.send(b64);
                }
            });
        }
    }
}

#[tokio::main]
async fn main() {
    let data_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    let user_file = data_path.join("user_gcp.info");
    let gcp_or_api = Arc::new(AtomicBool::new(true));
    let gcp_or_api_clone = gcp_or_api.clone();
    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = completed.clone();
    let speaking = Arc::new(AtomicBool::new(false));
    let speaking_clone = speaking.clone();
    
    println!("Please select mode (true for API, false for GCP) [Input needed]");
    // Dummy mode selection
    let (start_tx, start_rx) = tokio::sync::oneshot::channel();
    start_tx.send(()).unwrap();
    
    let (tx_out, mut rx_out) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (tx_in, mut _rx_in) = tokio::sync::mpsc::unbounded_channel::<String>();
    thread::spawn(move || {
        stt(tx_out, speaking_clone);
    });
    
    // We run the rest of main sequentially since there's no UI loop blocking it
    let _ = start_rx.await;
    let mut user_data: Option<UserData> = None;
    let mut api_data: Option<APIData> = None;
    if gcp_or_api.load(Ordering::Relaxed) {   
        api_data = Some(if std::fs::exists(data_path.join("user_api.info")).unwrap() {
            let auth_data = get_auth(&data_path).await;
            println!("API completed, UI shown as false.");
            completed_clone.store(true, Ordering::Relaxed);
            auth_data
        } else {
            let _ = std::fs::create_dir_all(&data_path);
            auth(&data_path).await;
            completed_clone.store(true, Ordering::Relaxed);
            println!("Created user_api.info");
            get_auth(&data_path).await
        });
    }
    if gcp_or_api.load(Ordering::Relaxed) && completed.load(Ordering::Relaxed) {
        let api_data = api_data.expect("API data should be initialized");
        if let Err(e) = gemini_api(api_data.api_key.as_str(), rx_out, tx_in).await {
            eprintln!("Gemini session error: {:?}", e);
        }
    }
}