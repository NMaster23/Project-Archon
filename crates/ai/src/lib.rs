use std::env::home_dir;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use talos_core::TalosBus;
use serde::{Serialize, Deserialize};
use gemini_live::{Session, SessionConfig, TransportConfig, Auth, SetupConfig, GenerationConfig, Modality, ReconnectPolicy, ServerEvent};
use std::time::{Duration, Instant};
use std::process::Command;
use portable_pty::{CommandBuilder, native_pty_system, PtySize};
use tokio::sync::{mpsc, oneshot};

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub data: String,
}

pub struct AgySession {
    tx: mpsc::UnboundedSender<String>,
}

impl AgySession {
    pub fn new(talos_bus_tx: mpsc::UnboundedSender<TalosBus>) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let mut pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let cmd = CommandBuilder::new("bash");
        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        writer.write_all(b"stty -echo; export PS1=\"\"; export PROMPT_COMMAND=\"\"\n")?;
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        std::thread::spawn(move || {
            let mut buffer = [0; 1024];
            std::thread::sleep(std::time::Duration::from_millis(50));
            while let Ok(bytes) = reader.read(&mut buffer) {
                if bytes < buffer.len() { break; }
            }
            while let Some(command) = rx.blocking_recv() {
                let sentinel = "__TALOS_CMD_COMPLETE__";
                let full_input = format!("{}\necho {}\n", command.trim(), sentinel);

                if writer.write_all(full_input.as_bytes()).is_err() {
                    break;
                }
                let mut accumulated_output = String::new();
                while let Ok(bytes_read) = reader.read(&mut buffer) {
                    if bytes_read == 0 { break; }

                    let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
                    accumulated_output.push_str(&chunk);
                    if accumulated_output.contains(sentinel) {
                        let clean_output = accumulated_output
                            .replace(&format!("echo {}\r\n", sentinel), "")
                            .replace(&format!("echo {}\n", sentinel), "")
                            .replace(&format!("{}\r\n", sentinel), "")
                            .replace(&format!("{}\n", sentinel), "")
                            .replace(sentinel, "")
                            .trim()
                            .to_string();
                        let _ = talos_bus_tx.send(TalosBus::TerminalOutput(clean_output));
                        break;
                    }
                }
            }
            let _ = child.kill();
        });

        Ok(Self { tx })
    }
    pub fn execute(&self, command: &str) {
        self.tx.send(command.to_string()).ok();
    }
}

pub async fn auth(path: &PathBuf) {
    let mut api_key = String::new();
    println!("Please enter your Gemini API key:");
    std::io::stdin().read_line(&mut api_key).unwrap();
    let auth_data = AuthData { data: api_key.trim().to_string() };
    let json = serde_json::to_string(&auth_data).unwrap();
    fs::write(path.join("user_api.info"), json).unwrap();
}

pub async fn get_auth(path: &PathBuf) -> AuthData {
    let json = fs::read_to_string(path.join("user_api.info")).unwrap();
    serde_json::from_str(&json).unwrap()
}

pub async fn gemini_communicate_speech(mut session: Session, mut rx_out: tokio::sync::mpsc::UnboundedReceiver<TalosBus>, mut tx_in: tokio::sync::mpsc::UnboundedSender<TalosBus>) -> Result<(), Box<dyn std::error::Error>> {
    let handle = rodio::DeviceSinkBuilder::open_default_sink()
        .expect("open default audio stream");
    let player = rodio::Player::connect_new(&handle.mixer());
    player.play();
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
                            speech.push_str(&" ".to_string());
                        }
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
                    let source = rodio::buffer::SamplesBuffer::new(std::num::NonZeroU16::new(1).unwrap(), std::num::NonZeroU32::new(24000).unwrap(), samples);
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
            .args(["-c", "curl -fsSL https://antigravity.google/cli/install.sh | bash && agy"])
            .status()
            .expect("failed to execute process");
    } else {
        println!("Unsupported OS");
    }
    println!("Path successfully found at {:?}", path);
}