use app_dirs2::{get_app_root, AppDataType, AppInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, vec};
use std::env::home_dir;
use talos_ai::{auth, get_auth, gemini_api, agy_setup};
use talos_audio::stt;
use talos_core::TalosBus;
use talos_ui::{dashboard, select_menu};

const APP_INFO: AppInfo = AppInfo{name: "Talos", author: "NMCreator"};

#[tokio::main]
async fn main() {
    let data_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = completed.clone();
    let speaking = Arc::new(AtomicBool::new(false));
    let speaking_clone = speaking.clone();
    let options = vec!["API", "OAuth"];
    let selection = select_menu(options).await;
    let oauth_or_api = Arc::new(AtomicBool::new(selection == 0));
    let stt_enabled = Arc::new(AtomicBool::new(true));
    let stt_enabled_clone = stt_enabled.clone();
    let (start_tx, start_rx) = tokio::sync::oneshot::channel();
    start_tx.send(()).unwrap();
    let (tx_out, mut rx_out) = tokio::sync::mpsc::unbounded_channel::<TalosBus>();
    let (tx_in, _rx_in) = tokio::sync::mpsc::unbounded_channel::<TalosBus>();
    let tx_out_stt = tx_out.clone();
    
    // Always spawn the STT thread, let the mute button control it internally!
    thread::spawn(move || {
        stt(tx_out_stt, speaking_clone, stt_enabled_clone);
    });
    let _ = start_rx.await;
    if oauth_or_api.load(Ordering::Relaxed) {
        let user_data = Some(if std::fs::exists(data_path.join("user_api.info")).unwrap() {
            let auth_data = get_auth(&data_path).await;
            completed_clone.store(true, Ordering::Relaxed);
            auth_data
        } else {
            let _ = std::fs::create_dir_all(&data_path);
            auth(&data_path).await;
            completed_clone.store(true, Ordering::Relaxed);
            println!("Created user_api.info");
            get_auth(&data_path).await
        });
        if completed.load(Ordering::Relaxed) {
            let data = user_data.expect("API data should be initialized");
            if let Err(e) = gemini_api(data.data.as_str(), rx_out, tx_in).await {
                eprintln!("Gemini session error: {:?}", e);
            }
        }
    } else {
        println!("OAuth selected, using gemini_oauth...");
        completed_clone.store(true, Ordering::Relaxed);
        let mut path = home_dir().ok_or("Home directory not found").unwrap();
        path.push(".gemini");
        path.push("oauth_creds.json");
        if !path.exists() {
            agy_setup(path).await;
        }
        let agy_session = talos_ai::AgySession::new(tx_out.clone()).expect("failed to create session");
        let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            dashboard(stt_enabled, ui_rx).await;
        });
        while let Some(event) = rx_out.recv().await {
            match event {
                TalosBus::VoiceTranscript(speech) => {
                    let processed = speech.trim().to_string();
                    if !processed.is_empty() {
                        let _ = ui_tx.send(format!("You: {}", processed));
                        agy_session.execute(&processed);
                    }
                }
                TalosBus::TerminalOutput(clean_txt) => {
                    let _ = ui_tx.send(format!("AI: {}", clean_txt));
                }
                _ => {}
            }
        }
    }
}
