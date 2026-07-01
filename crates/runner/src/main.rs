use app_dirs2::{get_app_root, AppDataType, AppInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, vec};
use talos_ai::{UserData, APIData, auth, get_auth, gemini_api};
use talos_audio::stt;
use talos_core::TalosBus;
use talos_ui::select_menu;

const APP_INFO: AppInfo = AppInfo{name: "Talos", author: "NMCreator"};

#[tokio::main]
async fn main() {
    let data_path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    let user_file = data_path.join("user_oauth.info");
    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = completed.clone();
    let speaking = Arc::new(AtomicBool::new(false));
    let speaking_clone = speaking.clone();
    let options = vec!["API", "OAuth"];
    let selection = select_menu(options).await;
    let oauth_or_api = Arc::new(AtomicBool::new(selection == 0));
    let oauth_or_api_clone = oauth_or_api.clone();
    println!("Selected mode: {}", if selection == 0 { "API" } else { "OAuth" });
    let (start_tx, start_rx) = tokio::sync::oneshot::channel();
    start_tx.send(()).unwrap();
    let (tx_out, mut rx_out) = tokio::sync::mpsc::unbounded_channel::<TalosBus>();
    let (tx_in, mut _rx_in) = tokio::sync::mpsc::unbounded_channel::<TalosBus>();
    thread::spawn(move || {
        talos_audio::stt(tx_out, speaking_clone);
    });
    let _ = start_rx.await;
    let mut user_data: Option<UserData> = None;
    let mut api_data: Option<APIData> = None;
    if oauth_or_api.load(Ordering::Relaxed) {   
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
    if oauth_or_api.load(Ordering::Relaxed) && completed.load(Ordering::Relaxed) {
        let api_data = api_data.expect("API data should be initialized");
        if let Err(e) = gemini_api(api_data.api_key.as_str(), rx_out, tx_in).await {
            eprintln!("Gemini session error: {:?}", e);
        }
    }
}
