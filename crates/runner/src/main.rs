use app_dirs2::{AppDataType, AppInfo, get_app_root};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, vec};
use talos_ai::{auth, gemini_api, get_auth};
use talos_audio::stt;
use talos_core::TalosBus;
use talos_executor::McpServer;
use talos_ui::{dashboard, select_menu};
use mcpkit_axum::McpRouter;

const APP_INFO: AppInfo = AppInfo {
    name: "Talos",
    author: "NMCreator",
};

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
    let (tx_in, mut rx_in) = tokio::sync::mpsc::unbounded_channel::<TalosBus>();
    let tx_out_stt = tx_out.clone();
    let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::spawn(
        talos_executor::start_mcpserver()
    );
    tokio::spawn(async move {
        dashboard(stt_enabled, ui_rx).await;
    });
    thread::spawn(move || {
        stt(tx_out_stt, speaking_clone, stt_enabled_clone);
    });
    let _ = start_rx.await;
    if oauth_or_api.load(Ordering::Relaxed) {
        let user_data = Some(
            if std::fs::exists(data_path.join("user_api.info")).unwrap() {
                let auth_data = get_auth(&data_path).await;
                completed_clone.store(true, Ordering::Relaxed);
                auth_data
            } else {
                let _ = std::fs::create_dir_all(&data_path);
                auth(&data_path).await;
                completed_clone.store(true, Ordering::Relaxed);
                println!("Created user_api.info");
                get_auth(&data_path).await
            },
        );
        if completed.load(Ordering::Relaxed) {
            let data = user_data.expect("API data should be initialized");
            let ui_tx_api = ui_tx.clone();
            tokio::spawn(async move {
                while let Some(msg) = rx_in.recv().await {
                    match msg {
                        TalosBus::TerminalOutput(txt) => {
                            let _ = ui_tx_api.send(txt);
                        }
                        TalosBus::AiResponse(txt) => {
                            let _ = ui_tx_api.send(txt);
                        }
                        _ => {
                            eprintln!("[Runner] Ignoring message type");
                        }
                    }
                }
            });
            if let Err(e) = gemini_api(data.data.as_str(), rx_out, tx_in).await {
                eprintln!("Gemini session error: {:?}", e);
            }
        }
    } else {
        println!("OAuth selected, using agy_communicate...");
        completed_clone.store(true, Ordering::Relaxed);
        let ui_tx_oauth = ui_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx_in.recv().await {
                match msg {
                    TalosBus::TerminalOutput(txt) => {
                        let _ = ui_tx_oauth.send(txt);
                    }
                    TalosBus::AiResponse(txt) => {
                        let _ = ui_tx_oauth.send(txt);
                    }
                    _ => {}
                }
            }
        });
        while let Some(event) = rx_out.recv().await {
            match event {
                TalosBus::VoiceTranscript(speech) => {
                    let processed = speech.trim().to_string();
                    if !processed.is_empty() {
                        let _ = ui_tx.send(format!("You: {}", processed));
                        let tx_in_clone = tx_in.clone();
                        tokio::spawn(async move {
                            if let Err(e) = talos_ai::agy_communicate(true, tx_in_clone, &processed).await {
                                eprintln!("[OAuth] AGY Error: {:?}", e);
                            }
                        });
                    }
                }
                _ => {}
            }
        }
    }
}