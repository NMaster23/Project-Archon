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
