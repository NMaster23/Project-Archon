use std::fs;
use std::path::PathBuf;
use talos_core::TalosBus;
use serde::{Serialize, Deserialize};
use gemini_live::{Session, SessionConfig, TransportConfig, Auth, SetupConfig, GenerationConfig, Modality, ReconnectPolicy};

#[derive(Serialize, Deserialize)]
pub struct APIData {
    pub api_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserData {
    pub access_token: String,
}

pub async fn auth(path: &PathBuf) {
    let mut api_key = String::new();
    println!("Please enter your Gemini API key:");
    std::io::stdin().read_line(&mut api_key).unwrap();
    let data = APIData { api_key: api_key.trim().to_string() };
    let json = serde_json::to_string(&data).unwrap();
    fs::write(path.join("user_api.info"), json).unwrap();
}

pub async fn get_auth(path: &PathBuf) -> APIData {
    let json = fs::read_to_string(path.join("user_api.info")).unwrap();
    serde_json::from_str(&json).unwrap()
}

pub async fn gemini_communicate(mut session: Session, mut rx_out: tokio::sync::mpsc::UnboundedReceiver<String>, mut tx_in: tokio::sync::mpsc::UnboundedSender<String>) -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn gemini_api(api_key: &str, rx_out: tokio::sync::mpsc::UnboundedReceiver<TalosBus>, tx_in: tokio::sync::mpsc::UnboundedSender<TalosBus>) -> Result<(), Box<dyn std::error::Error>> {
    println!("API completed (UI hook removed)");
    let session = Session::connect(SessionConfig {
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
