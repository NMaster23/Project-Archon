use std::fs;
use std::io::Write;
use std::path::PathBuf;
use talos_core::TalosBus;
use serde::{Serialize, Deserialize};
use gemini_live::{Session, SessionConfig, TransportConfig, Auth, SetupConfig, GenerationConfig, Modality, ReconnectPolicy, ServerEvent};
use std::time::Duration;
use std::process::Command;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub data: String,
}

pub struct StreamingSource {
    receiver: std::sync::mpsc::Receiver<f32>,
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
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> std::num::NonZeroU16 { std::num::NonZeroU16::new(1).unwrap() }
    fn sample_rate(&self) -> std::num::NonZeroU32 { std::num::NonZeroU32::new(24000).unwrap() }
    fn total_duration(&self) -> Option<Duration> { None }
}

pub async fn auth(path: &PathBuf) {
    let mut api_key = String::new();
    println!("Please enter your Gemini API key:");
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    reader.read_line(&mut api_key).await.unwrap();
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
    let (tx_audio, rx_audio) = std::sync::mpsc::channel::<f32>();
    let stream_source = StreamingSource { receiver: rx_audio };
    player.append(stream_source);
    player.play();
    let mut audio_buffer: Vec<u8> = Vec::new();
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
        while let Some(event) = session.next_event().await {
            match event {
                ServerEvent::ModelAudio(audio) => {
                    print!("{:?}", audio.len());
                    audio_buffer.extend_from_slice(&audio);
                    let valid_len = audio_buffer.len() - (audio_buffer.len() % 2);
                    for chunk in audio_buffer[..valid_len].chunks_exact(2) {
                        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0;
                        tx_audio.send(sample)?;
                    }
                    audio_buffer.drain(..valid_len);
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

pub struct AgySession {
    process: tokio::process::Child,
}

impl AgySession {
    pub fn new() -> Self {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = tokio::process::Command::new("cmd");
            c.args(&["/C", "agy"]);
            c
        } else {
            tokio::process::Command::new("agy")
        };
        let process = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to start agy");
        Self { process }
    }

    pub async fn prompt(&mut self, input: &str) -> String {
        let stdin = self.process.stdin.as_mut().unwrap();
        tokio::io::AsyncWriteExt::write_all(stdin, format!("{}\n", input).as_bytes()).await.unwrap();
        tokio::io::AsyncWriteExt::flush(stdin).await.unwrap();

        let stdout = self.process.stdout.as_mut().unwrap();
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut response = String::new();
        let mut buf = String::new();
        
        while let Ok(Ok(bytes)) = tokio::time::timeout(std::time::Duration::from_millis(500), tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut buf)).await {
            if bytes == 0 { break; }
            response.push_str(&buf);
            buf.clear();
        }
        response
    }
}


pub async fn agy_communicate(new_chat: bool, talos_bus_tx: mpsc::UnboundedSender<TalosBus>, input: &str) -> Result<(), Box<dyn std::error::Error>>  {
    println!("Command Start");
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(&["/C", "agy"]);
        c
    } else {
        let mut c = Command::new("agy");
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