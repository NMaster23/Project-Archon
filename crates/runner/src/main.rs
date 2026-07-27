use app_dirs2::{AppDataType, AppInfo, get_app_root};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;
use talos_ai::{gemini_api, manage_soul, self_improvement};
use talos_auth::{auth, get_auth};
use talos_core::{ClientToServer, ServerToClient, TalosBus};
use notify_rust::{Notification, Timeout};
use std::sync::{Arc, RwLock};
use std::time::Duration;

const APP_INFO: AppInfo = AppInfo {
    name: "Talos",
    author: "NMCreator",
};

pub async fn start_server() -> anyhow::Result<()> {
    let config_path = get_app_root(AppDataType::UserConfig, &APP_INFO)?.join("config.json");
    let config_val = talos_core::TalosConfig::load(&config_path, talos_core::CONFIG_TEMPLATE);
    let config = Arc::new(RwLock::new(config_val));
    let backend = config.read().unwrap().backend.clone();
    let use_api = backend == "API";
    let api_key = if use_api {
        println!("API selected, fetching auth...");
        let _data_path = get_app_root(AppDataType::UserConfig, &APP_INFO)?;
        if let Some(auth_data) = get_auth(None, 2).await {
            Some(auth_data.data)
        } else {
            auth(None, "INSERT_API_KEY_OR_SECRET", None, None, 2).await;
            println!("Created user_api.info with dummy data");
            Some(get_auth(None, 2).await.unwrap().data)
        }
    } else {
        println!("OAuth selected, using agy_communicate...");
        None
    };

    let listener = talos_transport::listen("0.0.0.0:9090").await?;
    println!("Server is listening on port 9090");
    let (bus_tx, _) = tokio::sync::broadcast::channel::<talos_core::SystemEvent>(100);
    
    let bus_tx_ui = bus_tx.clone();
    tokio::spawn(async move {
        talos_ui::server_dashboard(bus_tx_ui, config.clone()).await;
    });
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(600)).await;
            manage_soul().await;
            self_improvement().await;
        }
    });
    let api_key_arc = Arc::new(api_key);

    loop {
        let (stream, _) = listener.accept().await?;
        
        let bus_tx_conn = bus_tx.clone();
        let api_key_clone = api_key_arc.clone();

        tokio::spawn(async move {
            let mut conn = talos_transport::accept(stream).await.unwrap();
            conn.send_to_client(&ServerToClient::RequestToolRegistration).await.unwrap();
            let (tx_in, mut rx_in) = mpsc::unbounded_channel::<TalosBus>();
            let (tx_out, rx_out) = mpsc::unbounded_channel::<TalosBus>();

            let is_api = api_key_clone.is_some();
            if let Some(key) = api_key_clone.as_ref() {
                let tx_in_clone = tx_in.clone();
                let key_str = key.clone();
                tokio::spawn(async move {
                    if let Err(e) = gemini_api(&key_str, rx_out, tx_in_clone).await {
                        eprintln!("Gemini session error: {:?}", e);
                    }
                });
            }

            loop {
                tokio::select! {
                    Ok(message) = conn.recv_from_client() => {
                        let _ = bus_tx_conn.send(talos_core::SystemEvent::ClientEvent(message.clone()));
                        match message {
                            ClientToServer::VoiceTranscript(text) => {
                                let processed = text.trim().to_string();
                                if !processed.is_empty() {
                                    println!("User: {}", processed);
                                    if is_api {
                                        let _ = tx_out.send(TalosBus::VoiceTranscript(processed));
                                    } else {
                                        let tx_in_clone = tx_in.clone();
                                        tokio::spawn(async move {
                                            if let Err(e) = talos_ai::agy_communicate(true, tx_in_clone, &processed).await {
                                                eprintln!("AGY CLI Error: {:?}", e);
                                            }
                                        });
                                    }
                                }
                            }
                            ClientToServer::ToolRegistration { tools } => println!("Client registered tools {:?}", tools),
                            ClientToServer::ToolCallResult { call_id: _, success: _, result } => println!("Tool Result: {}", result),
                            _ => {}
                        }
                    }
                    Some(ai_msg) = rx_in.recv() => {
                        let _ = bus_tx_conn.send(talos_core::SystemEvent::BusEvent(ai_msg.clone()));
                        match ai_msg {
                            TalosBus::AiResponse(txt) => {
                                println!("AI: {}", txt);
                                let _ = conn.send_to_client(&ServerToClient::AiResponse(txt)).await;
                            }
                            TalosBus::TerminalOutput(txt) => {
                                println!("Terminal: {}", txt);
                                let _ = conn.send_to_client(&ServerToClient::TerminalOutput(txt)).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
    }
}

pub async fn run_client(server_addr: &str) -> anyhow::Result<()> {
    let config_path = get_app_root(AppDataType::UserConfig, &APP_INFO)?.join("config.json");
    let config = Arc::new(RwLock::new(talos_core::TalosConfig::load(&config_path, talos_core::CONFIG_TEMPLATE)));
    let (icon_enabled_path, _) = talos_ui::get_icon_paths();
    Notification::new()
        .summary("Microphone Unmuted")
        .body("Voice Control and STT Available. (Alt+M to Disable)")
        .icon(icon_enabled_path.to_str().unwrap_or(""))
        .timeout(Timeout::Milliseconds(6000))
        .show().ok();
    tokio::spawn(async move {
        if let Err(e) = talos_executor::tools().await {
            eprintln!("Critical Error: {:?}", e);
        }
    });
    let ws_url = format!("ws://{}", server_addr);
    let mut conn = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        loop {
            match talos_transport::connect(&ws_url).await {
                Ok(c) => break c,
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }).await.map_err(|_| anyhow::anyhow!("Timed out waiting for client to initialize"))?;
    let (stt_tx, mut stt_rx) = mpsc::unbounded_channel::<TalosBus>();
    let (ui_tx, ui_rx) = mpsc::unbounded_channel::<String>();

    let stt_disabled = Arc::new(AtomicBool::new(false));
    let stt_disabled_ui = stt_disabled.clone();
    tokio::spawn(async move {
        talos_ui::client_backend(stt_disabled_ui, ui_rx, config.clone()).await;
    });
    std::thread::spawn(move || {
        talos_audio::stt(stt_tx, Arc::new(AtomicBool::new(false)), stt_disabled)
    });
    
    loop {
        tokio::select! {
            Some(msg) = stt_rx.recv() => {
                if let TalosBus::VoiceTranscript(text) = msg {
                    conn.send_to_server(&ClientToServer::VoiceTranscript(text.clone())).await?;
                    let _ = ui_tx.send(format!("You: {}", text.trim()));
                }
            }
            Ok(msg) = conn.recv_from_server() => {
                match msg {
                    ServerToClient::AiResponse(text) | ServerToClient::TerminalOutput(text) => {
                        let _ = ui_tx.send(text);
                    }
                    ServerToClient::ExecuteToolCall { call_id, tool_name, args } => {
                        let result = talos_executor::call_tool(&tool_name, &args).await.unwrap_or_else(|e| e);
                        conn.send_to_server(&ClientToServer::ToolCallResult { call_id, success: true, result }).await?;
                    }
                    ServerToClient::RequestToolRegistration => {
                        conn.send_to_server(&ClientToServer::ToolRegistration { tools: talos_executor::get_tools().await }).await?;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "server" {
        start_server().await.unwrap();
        return;
    }
    let server_address = if args.len() > 2 {
        args[2].clone()
    } else {
        "127.0.0.1:9090".to_string()
    };
    run_client(&server_address).await.unwrap();
}
