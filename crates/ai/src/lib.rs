use gemini_live::{
    Auth, GenerationConfig, Modality, ReconnectPolicy, ServerEvent, Session, SessionConfig,
    SetupConfig, TransportConfig,
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::ToString;
use std::time::Duration;
use talos_core::TalosBus;
use tokio::sync::mpsc;
use app_dirs2::{AppDataType, AppInfo, get_app_root};

const APP_INFO: AppInfo = AppInfo {
    name: "Talos",
    author: "NMCreator",
};

pub struct StreamingSource {
    receiver: std::sync::mpsc::Receiver<f32>,
}

pub struct AgySession {
    tx: mpsc::UnboundedSender<String>,
}

impl AgySession {
    pub fn new(
        talos_bus_tx: mpsc::UnboundedSender<TalosBus>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut child = pair.slave.spawn_command(CommandBuilder::new("agy"))?;
        let mut writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        std::thread::spawn(move || {
            let mut buffer = [0; 1024];
            let first_output_timeout = Duration::from_secs(30);
            let idle_timeout = Duration::from_secs(2);
            let (output_tx, output_rx) = std::sync::mpsc::channel::<String>();

            std::thread::spawn(move || {
                while let Ok(bytes_read) = reader.read(&mut buffer) {
                    if bytes_read == 0 {
                        break;
                    }
                    let chunk = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                    if output_tx.send(chunk).is_err() {
                        break;
                    }
                }
            });

            while let Some(command) = rx.blocking_recv() {
                while output_rx.try_recv().is_ok() {}

                let safe_cmd = format!("{}\n", command.trim());
                if writer.write_all(safe_cmd.as_bytes()).is_err() {
                    break;
                }
                if writer.flush().is_err() {
                    break;
                }
                let _ = talos_bus_tx.send(TalosBus::TerminalOutput("__PROCESSING_START__".to_string()));
                let mut accumulated_output = String::new();
                let start_time = std::time::Instant::now();
                let mut last_chunk_time = start_time;
                let mut got_first_chunk = false;

                loop {
                    match output_rx.try_recv() {
                        Ok(chunk) => {
                            accumulated_output.push_str(&chunk);
                            last_chunk_time = std::time::Instant::now();
                            got_first_chunk = true;
                        }
                        Err(_) => {
                            if !got_first_chunk && start_time.elapsed() > first_output_timeout {
                                let timeout_msg = format!("AGY command timeout or no output: '{}'", command.trim());
                                let _ = talos_bus_tx.send(TalosBus::TerminalOutput(timeout_msg));
                                break;
                            }
                            if got_first_chunk && last_chunk_time.elapsed() > idle_timeout {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                    }
                }
                let stripped = strip_ansi_escapes::strip(&accumulated_output.as_bytes());
                let text = String::from_utf8_lossy(&stripped);
                let clean_output = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .filter(|line| *line != command.trim())
                    .filter(|line| !line.starts_with("agy>"))
                    .filter(|line| !line.starts_with("Choose") && !line.ends_with("model:"))
                    .filter(|line| !line.contains("Gemini") && !line.contains("gemini"))
                    .filter(|line| !line.contains("(High)") && !line.contains("(Low)") && !line.contains("(Default)"))
                    .filter(|line| {
                        !line.chars().all(|c| {
                            matches!(
                                c,
                                '⠋' | '⠙'
                                    | '⠹'
                                    | '⠸'
                                    | '⠼'
                                    | '⠴'
                                    | '⠦'
                                    | '⠧'
                                    | '⠇'
                                    | '⠏'
                                    | '-'
                                    | '\\'
                                    | '|'
                                    | '/'
                                    | ' '
                            )
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !clean_output.is_empty() {
                    let formatted_output = format!("AI: {}", clean_output);
                    let _ = talos_bus_tx.send(TalosBus::TerminalOutput(formatted_output));
                }
                let _ = talos_bus_tx.send(TalosBus::TerminalOutput("__PROCESSING_END__".to_string()));
            }
            let _ = child.kill();
        });

        Ok(Self { tx })
    }

    pub fn execute(&self, command: &str) {
        self.tx.send(command.to_string()).ok();
    }
}

impl Iterator for StreamingSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        match self.receiver.try_recv() {
            Ok(sample) => Some(sample),
            Err(_) => Some(0.0),
        }
    }
}

impl rodio::Source for StreamingSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> std::num::NonZeroU16 {
        std::num::NonZeroU16::new(1).unwrap()
    }
    fn sample_rate(&self) -> std::num::NonZeroU32 {
        std::num::NonZeroU32::new(24000).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub async fn gemini_communicate_speech(
    mut session: Session,
    mut rx_out: tokio::sync::mpsc::UnboundedReceiver<TalosBus>,
    tx_in: tokio::sync::mpsc::UnboundedSender<TalosBus>,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
    let player = rodio::Player::connect_new(&handle.mixer());
    let (tx_audio, rx_audio) = std::sync::mpsc::channel::<f32>();
    let stream_source = StreamingSource { receiver: rx_audio };
    player.append(stream_source);
    player.play();
    let mut audio_buffer: Vec<u8> = Vec::new();
    let agy_session = AgySession::new(tx_in.clone())?;
    loop {
        let mut speech = String::new();
        match rx_out.recv().await {
            Some(TalosBus::VoiceTranscript(speech_input)) => {
                let processed = speech_input.trim().to_string();
                if !processed.is_empty() {
                    speech.push_str(&processed);
                }
            }
            Some(_) => continue,
            None => break,
        }
        loop {
            match tokio::time::timeout(Duration::from_secs(1), rx_out.recv()).await {
                Ok(Some(TalosBus::VoiceTranscript(input_speech))) => {
                    let processed = input_speech.trim().to_string();
                    if !processed.is_empty() {
                        if !speech.is_empty() {
                            speech.push(' ');
                        }
                        speech.push_str(&processed);
                    }
                }
                Ok(Some(_)) => continue,
                Ok(None) => break,
                Err(_) => break,
            }
        }
        if speech.is_empty() {
            continue;
        }
        session.send_text(&speech).await?;
        let _ = tx_in.send(TalosBus::TerminalOutput(format!("You: {}", speech)));
        let mut gemini_response = String::new();
        while let Some(event) = session.next_event().await {
            match event {
                ServerEvent::ModelAudio(audio) => {
                    audio_buffer.extend_from_slice(&audio);
                    let valid_len = audio_buffer.len() - (audio_buffer.len() % 2);
                    for chunk in audio_buffer[..valid_len].chunks_exact(2) {
                        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0;
                        tx_audio.send(sample)?;
                    }
                    audio_buffer.drain(..valid_len);
                }
                ServerEvent::ModelText(text) => {
                    gemini_response.push_str(&text);
                    let _ = tx_in.send(TalosBus::AiResponse(text));
                }
                ServerEvent::ToolCall(tool_calls) => {
                    for tool_call in tool_calls {
                        let msg = format!("AI called tool: {} with args: {}", tool_call.name, tool_call.args);
                        let _ = tx_in.send(TalosBus::TerminalOutput(msg));
                    }
                }
                ServerEvent::TurnComplete => {
                    let final_msg = format!("AI: {}", gemini_response);
                    let _ =
                        tx_in.send(TalosBus::TerminalOutput(final_msg));
                    if !gemini_response.trim().is_empty() {
                        agy_session.execute(&gemini_response);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    break;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub async fn gemini_api(
    api_key: &str,
    rx_out: tokio::sync::mpsc::UnboundedReceiver<TalosBus>,
    tx_in: tokio::sync::mpsc::UnboundedSender<TalosBus>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ai_management_prompt = r#"You are Talos, an advanced, voice-operated Developer Assistant. You have direct, real-time access to the user's host operating system and terminal through an AGY CLI bridge.

When the user asks you to inspect files, write code, run commands, or manage the local project, respond with the exact instruction you want AGY to execute. Do not claim you ran a command yourself; the host app forwards your completed response to AGY and displays the result.

Operational Directives & Safety Rules

    Prefer the CLI for Data: If the user asks for information about their system (e.g., "What's in this folder?", "Read this code file"), issue a concise AGY instruction for the CLI rather than describing GUI steps.

    Chain Actions Logically: You can request multiple terminal/file actions in one instruction when they belong together.

    Destructive Actions: If a user asks you to delete files, format drives, or run potentially dangerous commands, ask for confirmation before issuing the AGY instruction."#;
    let executor_tools = executor::gemini_api_mcp().await;
    let gemini_tools = vec![gemini_live::Tool::FunctionDeclarations(
        executor_tools.into_iter().map(|raw| {
            serde_json::from_value(raw).unwrap()
        }).collect()
    )];
    let session = Session::connect(SessionConfig {
        transport: TransportConfig {
            auth: Auth::ApiKey(api_key.to_string()),
            ..Default::default()
        },
        setup: SetupConfig {
            model: "models/gemini-3.1-flash-live-preview".into(),
            tools: Some(gemini_tools),
            system_instruction: Some(gemini_live::Content {
                parts: vec![gemini_live::Part {
                    text: Some(ai_management_prompt.to_string()),
                    inline_data: None,
                }],
                role: Some("system".to_string()),
            }),
            generation_config: Some(GenerationConfig {
                response_modalities: Some(vec![Modality::Audio, Modality::Text]),
                ..Default::default()
            }),
            ..Default::default()
        },
        reconnect: ReconnectPolicy::default(),
    })
    .await?;
    gemini_communicate_speech(session, rx_out, tx_in).await?;
    Ok(())
}

pub async fn agy_setup(path: PathBuf) {
    if cfg!(target_os = "windows") {
        let _ = Command::new("cmd")
            .args(["/C", "curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd && agy"])
            .status()
            .expect("failed to execute process");
    } else if cfg!(any(target_os = "linux", target_os = "macos")) {
        let _ = Command::new("sh")
            .args([
                "-c",
                "curl -fsSL https://antigravity.google/cli/install.sh | bash && agy",
            ])
            .status()
            .expect("failed to execute process");
    } else {
        println!("Unsupported OS");
    }
    println!("Path successfully found at {:?}", path);
}

pub async fn agy_communicate(
    new_chat: bool,
    talos_bus_tx: mpsc::UnboundedSender<TalosBus>,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(&["/C", "agy"]);
        c
    } else {
        Command::new("agy")
    };
    
    if new_chat {
        cmd.args(["-p", input.trim()]);
    } else {
        cmd.args(["-c", "-p", input.trim()]);
    }
    
    match cmd.output() {
        Ok(agy_output) => {
            let mut text = String::from_utf8(agy_output.stdout).unwrap_or_default();
            let stderr = String::from_utf8(agy_output.stderr).unwrap_or_default();
            
            if !stderr.trim().is_empty() {
                if !text.trim().is_empty() {
                    text.push('\n');
                }
                text.push_str(&stderr);
            }
            
            if !text.is_empty() {
                let cleaned = text.trim().to_string();
                let formatted = format!("AI: {}", cleaned);
                let _ = talos_bus_tx.send(TalosBus::TerminalOutput(formatted));
            }
        }
        Err(e) => {
            let _ = talos_bus_tx.send(TalosBus::TerminalOutput(format!("AI: Error - {}", e)));
        }
    }
    
    Ok(())
}

pub async fn create_config() {
    let app_root = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    let config_file = app_root.join("config.json");
    fs::write(config_file, talos_core::CONFIG_TEMPLATE).unwrap();
}