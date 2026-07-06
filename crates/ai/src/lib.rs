use std::fs;
use std::io::Write;
use std::path::PathBuf;
use talos_core::TalosBus;
use serde::{Serialize, Deserialize};
use gemini_live::{Session, SessionConfig, TransportConfig, Auth, SetupConfig, GenerationConfig, Modality, ReconnectPolicy, ServerEvent};
use std::time::Duration;
use std::process::Command;
use portable_pty::{CommandBuilder, native_pty_system, PtySize};
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub data: String,
}

pub struct AgySession {
    tx: mpsc::UnboundedSender<String>,
}

impl AgySession {
    pub fn new(talos_bus_tx: mpsc::UnboundedSender<TalosBus>) -> Result<Self, Box<dyn std::error::Error>> {
        println!("PTY Start");
        let pty_system = native_pty_system();
        let mut pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = CommandBuilder::new("cmd");
            c.args(&["/C", "agy"]);
            c
        } else {
            let mut c = CommandBuilder::new("agy");
            c
        };
        cmd.env("CI", "true");
        cmd.env("TERM", "dumb");
        cmd.env("NO_COLOR", "1");
        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        std::thread::spawn(move || {
            let mut buffer = [0; 512];
            while let Ok(bytes_read) = reader.read(&mut buffer) {
                if bytes_read == 0 { break; }
                let clean_bytes = strip_ansi_escapes::strip(&buffer[..bytes_read]);
                let chunk_str = String::from_utf8_lossy(&clean_bytes).to_string();
                if !chunk_str.is_empty() {
                    let _ = talos_bus_tx.send(TalosBus::TerminalOutput(chunk_str));
                }
            }
            let _ = child.kill();
        });
        std::thread::spawn(move || {
            while let Some(command) = rx.blocking_recv() {
                let input = format!("{}\r\n", command.trim());
                if writer.write_all(input.as_bytes()).is_err() {
                    break;
                }
            }
        });
        println!("PTY End");
        Ok(Self { tx })
    }
    pub fn execute(&self, command: &str) {
        println!("PTY Execute Start");
        self.tx.send(command.to_string()).ok();
        println!("PTY Execute End");
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

pub async fn agy_communicate(new_chat: bool, talos_bus_tx: mpsc::UnboundedSender<TalosBus>, input: &str) -> Result<(), Box<dyn std::error::Error>>  {
    println!("PTY Start");
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(&["/C", "agy"]);
        c
    } else {
        let mut c = std::process::Command::new("agy");
        c
    };
    if new_chat {
        cmd.args(["-p", input.trim()]);
    } else {
        cmd.args(["-c", "-p", input.trim()]);
    }
    let agy_output = cmd.output()?;
    let text = String::from_utf8(agy_output.stdout)?.to_string();
    if !text.is_empty() {
        talos_bus_tx.send(TalosBus::TerminalOutput(text))?;
    }
    println!("PTY End");
    Ok(())
}