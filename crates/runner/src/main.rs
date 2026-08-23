use app_dirs2::{AppDataType, AppInfo, get_app_root};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;
use talos_ai::{gemini_api, manage_soul, save_chats, self_improvement};
use talos_auth::{auth, get_auth, issue_session_token, verify_session_token};
use talos_core::{ClientToServer, ConfigFile, ServerToClient, TalosBus, UserPreferences};
use notify_rust::{Notification, Timeout};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use talos_core::TalosBus::UserCredentials;
use any_tts::{load_model, ModelType, SynthesisRequest, TtsConfig, TtsModel};
use talos_ui::get_user_preferences;

const APP_INFO: AppInfo = AppInfo {
    name: "Talos",
    author: "NMCreator",
};

pub async fn cron_scheduler() {
    
}

pub async fn start_server() -> anyhow::Result<()> {
    let server_config_path = get_app_root(AppDataType::UserConfig, &APP_INFO)?.join("server_config.json");
    let server_config = talos_core::ServerConfig::load(&server_config_path, talos_core::CONFIG_TEMPLATE);
    let config = Arc::new(RwLock::new(server_config));
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
            let _ = manage_soul().await;
            let _ = self_improvement().await;
        }
    });
    loop {
        let (stream, _) = listener.accept().await?;
        let bus_tx_conn = bus_tx.clone();
        tokio::spawn(async move {
            let mut conn = match talos_transport::accept(stream).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                    return;
                }
            };
            let first_msg = conn.recv_from_client().await;
            let email = match first_msg {
                Ok(ClientToServer::UserCredentials(token)) => {
                    verify_session_token(&token).await
                }
                _ => return,
            };
            if let Err(e) = conn.send_to_client(&ServerToClient::RequestToolRegistration).await {
                eprintln!("Send error: {}", e);
                return;
            }
            let (tx_in, mut rx_in) = mpsc::unbounded_channel::<TalosBus>();
            let (tx_out, rx_out) = mpsc::unbounded_channel::<TalosBus>();
            let tx_in_clone = tx_in.clone();
            tokio::spawn(async move {
                let _ = save_chats(rx_out).await;
            });
            let email_unwrapped = match email.clone() {
                Some(e) => e,
                None => {
                    eprintln!("Client send invalid token");
                    return;
                }
            };
            let config_dir = match get_app_root(AppDataType::UserConfig, &APP_INFO) {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                },
            };
            let prefs_path = config_dir.join(format!("{}_prefs.json", email_unwrapped));
            let user_prefs = UserPreferences::load(&prefs_path, "{}");
            let token_budget = user_prefs.max_output_tokens;
            if user_prefs.backend == "API" {
                let opt_auth_data = get_auth(email.as_deref(), 2).await;
                if let Some(auth_data) = opt_auth_data {
                    let api_key = auth_data.data;
                    tokio::spawn(async move {
                        gemini_api(&api_key, rx_out, tx_in_clone, token_budget).await;
                    });
                } else {
                    eprintln!("Authentication data missing");
                    return;
                }
            } else if user_prefs.backend == "AGY" {
                let tx_in_clone = tx_in.clone();
                tokio::spawn(async move {
                    if let Err(e) = talos_ai::agy_backend(rx_out, tx_in_clone).await {
                        eprintln!("AGY CLI Error: {:?}", e);
                    }
                });
            } else if user_prefs.backend == "LOCAL" {
                let tx_in_clone = tx_in.clone();
                tokio::spawn(async move {
                    if let Err(e) = talos_ai::local_backend(rx_out, tx_in_clone).await {
                        eprintln!("Local Backend Error: {:?}", e);
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
                                    let _ = tx_out.send(TalosBus::VoiceTranscript(processed));
                                }
                            }
                            ClientToServer::ToolRegistration { tools } => println!("Client registered tools {:?}", tools),
                            ClientToServer::ToolCallResult { call_id, tool_name, success: _, result } => {
                                let _ = tx_out.send(TalosBus::ToolCallResult {
                                    call_id,
                                    tool_name,
                                    result,
                                });
                            }
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
                            TalosBus::ActionIntent { call_id, tool, args } => {
                                let _ = conn.send_to_client(&ServerToClient::ExecuteToolCall {
                                    call_id,
                                    tool_name: tool,
                                    args
                                }).await;
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
    let config_path = get_app_root(AppDataType::UserConfig, &APP_INFO)?.join("config");
    let client_config_path = config_path.join("config.json");
    let client_config = Arc::new(RwLock::new(talos_core::ClientConfig::load(&client_config_path, talos_core::CONFIG_TEMPLATE)));
    let (icon_enabled_path, _) = talos_ui::get_icon_paths();
    let token_path = config_path.join("session.token");
    let token = match std::fs::read_to_string(&token_path) {
        Ok(token) => token.trim().to_string(),
        Err(_) => {
            let new_token = issue_session_token("user@local.client").await.expect("Failed to issue token");
            if let Some(parent) = token_path.parent() {
                std::fs::create_dir_all(parent).expect("Failed to create config dir");
            }
            std::fs::write(&token_path, &new_token).expect("Failed to write token");
            new_token
        }
    };
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
    let mut conn = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            match talos_transport::connect(&ws_url).await {
                Ok(c) => break c,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }).await.map_err(|_| anyhow::anyhow!("Timed out waiting for client to initialize"))?;
    let (stt_tx, mut stt_rx) = mpsc::unbounded_channel::<TalosBus>();
    let (ui_tx, ui_rx) = mpsc::unbounded_channel::<String>();
    let user_stt = client_config.read().expect("Error reading config").stt_disabled_by_default;
    let stt_disabled = Arc::new(AtomicBool::new(user_stt));
    let stt_disabled_ui = stt_disabled.clone();
    tokio::spawn(async move {
        talos_ui::client_backend(stt_disabled_ui, ui_rx, client_config.clone()).await;
    });
    std::thread::spawn(move || {
        if let Err(e) = talos_audio::stt(stt_tx, Arc::new(AtomicBool::new(false)), stt_disabled) {
            eprintln!("STT error: {:?}", e);
        }
    });
    let (tts_tx, mut tts_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        let model_path = dirs::data_local_dir().expect("Directory error").join("Talos/models/Qwen3-TTS");
        let model = tokio::task::spawn_blocking(move || {
            load_model(
                TtsConfig::new(ModelType::Qwen3Tts)
                    .with_model_path(model_path.to_str().expect("Error converting model path to string"))
            ).expect("Failed to load model")
        }).await.expect("Failed to spawn blocking task");
        while let Some(text) = tts_rx.recv().await {
            if let Err(e) = talos_audio::tts(&text.clone(), &*model).await {
                eprintln!("TTS Error: {}", e);
            }
        }
    });
    loop {
        conn = match talos_transport::connect(&ws_url).await {
            Ok(c) => c,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };
        let token_path = app_dirs2::get_app_root(AppDataType::UserConfig, &APP_INFO)?.join("config").join("session.token");
        let token = match std::fs::read_to_string(&token_path) {
            Ok(t) => t.trim().to_string(),
            Err(_) => {
                eprintln!("Error: Put session token in {:?}", &token_path);
                return Ok(())
            }
        };
        if let Err(e) = conn.send_to_server(&ClientToServer::UserCredentials(token)).await {
            eprintln!("Client credentials error: {:?}", e);
            continue;
        }
        loop {
            tokio::select! {
                Some(msg) = stt_rx.recv() => {
                    if let TalosBus::VoiceTranscript(text) = msg {
                        if let Err(e) = conn.send_to_server(&ClientToServer::VoiceTranscript(text.clone())).await {
                            break;
                        };
                        let _ = ui_tx.send(format!("You: {}", text.trim()));
                    }
                }
                res = conn.recv_from_server() => {
                    match res {
                        Ok(msg) => {
                            match msg {
                                ServerToClient::AiResponse(text) | ServerToClient::TerminalOutput(text) => {
                                    let _ = ui_tx.send(text.clone());
                                    let _ = tts_tx.send(text.clone());
                                }
                                ServerToClient::ExecuteToolCall { call_id, tool_name, args } => {
                                    let (success, result) = match talos_executor::call_tool(&tool_name, &args).await {
                                        Ok(output) => (true, output),
                                        Err(e) => (false, e),
                                    };
                                    if let Err(e) = conn.send_to_server(&ClientToServer::ToolCallResult {
                                        call_id,
                                        tool_name,
                                        success,
                                        result
                                    }).await {
                                        break;
                                    };
                                }
                                ServerToClient::RequestToolRegistration => {
                                    if let Err(e) = conn.send_to_server(&ClientToServer::ToolRegistration { tools: talos_executor::get_tools().await }).await {
                                        break;
                                    };
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "server" {
        start_server().await?;
        return Ok(());
    }
    let server_address = if args.len() > 2 {
        args[2].clone()
    } else {
        "127.0.0.1:9090".to_string()
    };
    run_client(&server_address).await?;
    Ok(())
}
