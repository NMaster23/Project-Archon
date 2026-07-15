use app_dirs2::{AppDataType, AppInfo, get_app_root};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, vec};
use tokio::sync::mpsc;
use talos_ai::{auth, gemini_api, get_auth};
use talos_audio::stt;
use talos_core::{ClientToServer, ServerToClient, TalosBus};
use talos_ui::{dashboard, select_menu};

const APP_INFO: AppInfo = AppInfo {
    name: "Talos",
    author: "NMCreator",
};

pub async fn start_server() -> anyhow::Result<()> {
    let listener = talos_transport::listen("0.0.0.0:9090").await?;
    println!("Server is listening on port 9090 (Using AGY CLI mode)");

    loop {
        let (stream, _) = listener.accept().await?;
        
        tokio::spawn(async move {
            let mut conn = talos_transport::accept(stream).await.unwrap();
            conn.send_to_client(&ServerToClient::RequestToolRegistration).await.unwrap();
            let (tx_in, mut rx_in) = mpsc::unbounded_channel::<TalosBus>();
            
            loop {
                tokio::select! {
                    // Receive from client and forward to AGY
                    Ok(message) = conn.recv_from_client() => {
                        match message {
                            ClientToServer::VoiceTranscript(text) => {
                                let processed = text.trim().to_string();
                                if !processed.is_empty() {
                                    println!("User: {}", processed);
                                    let tx_in_clone = tx_in.clone();
                                    // Spawn agy_communicate in the background
                                    tokio::spawn(async move {
                                        if let Err(e) = talos_ai::agy_communicate(true, tx_in_clone, &processed).await {
                                            eprintln!("AGY CLI Error: {:?}", e);
                                        }
                                    });
                                }
                            }
                            ClientToServer::ToolRegistration { tools } => println!("Client registered tools {:?}", tools),
                            ClientToServer::ToolCallResult { call_id: _, success: _, result } => println!("Tool Result: {}", result),
                            _ => {}
                        }
                    }
                    
                    // Receive from AGY and forward to client
                    Some(ai_msg) = rx_in.recv() => {
                        match ai_msg {
                            TalosBus::AiResponse(txt) => {
                                println!("AGY: {}", txt);
                                let _ = conn.send_to_client(&ServerToClient::AiResponse(txt)).await;
                            }
                            TalosBus::TerminalOutput(txt) => {
                                println!("AGY Terminal: {}", txt);
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

pub async fn run_client() -> anyhow::Result<()> {
    tokio::spawn(async move {
        if let Err(e) = talos_executor::tools().await {
            eprintln!("Critical Error: {:?}", e);
        }
    });
    let mut conn = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        loop {
            match talos_transport::connect("ws://127.0.0.1:9090").await {
                Ok(c) => break c,
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }).await.map_err(|_| anyhow::anyhow!("Timed out waiting for client to initialize"))?;
    let (stt_tx, mut stt_rx) = mpsc::unbounded_channel::<TalosBus>();
    let (ui_tx, ui_rx) = mpsc::unbounded_channel::<String>();
    
    let stt_enabled = Arc::new(AtomicBool::new(true));
    let stt_enabled_ui = stt_enabled.clone();
    tokio::spawn(async move {
        dashboard(stt_enabled_ui, ui_rx).await;
    });
    std::thread::spawn(move || {
        talos_audio::stt(stt_tx, Arc::new(AtomicBool::new(false)), stt_enabled)
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
    } else if args.len() > 1 && args[1] == "client" {
        run_client().await.unwrap();
        return;
    }
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
