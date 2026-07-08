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
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use ratatui::Frame;
use tui_prompts::{Prompt, TextPrompt, TextRenderStyle, TextState};
use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_prompts::{Status, State};

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    pub data: String,
}

pub struct StreamingSource {
    receiver: std::sync::mpsc::Receiver<f32>,
}

pub struct App<'a> {
    apikey_state: TextState<'a>,
}

impl<'a> App<'a> {
    fn draw_ui(&mut self, frame: &mut Frame) {
        TextPrompt::from("Gemini API Key")
            .with_render_style(TextRenderStyle::Password)
            .draw(frame, frame.area(), &mut self.apikey_state);
    }
}

pub struct AgySession {
    tx: mpsc::UnboundedSender<String>,
}

impl AgySession {
    pub fn new(talos_bus_tx: mpsc::UnboundedSender<TalosBus>) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let cmd = if cfg!(target_os = "windows") {
            CommandBuilder::new("cmd")
        } else {
            CommandBuilder::new("bash")
        };
        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        if cfg!(target_os = "windows") {
            writer.write_all(b"@echo off\nprompt $g\n")?;
        } else {
            writer.write_all(b"stty -echo; export PS1=\"\"; export PROMPT_COMMAND=\"\"\n")?;
        }
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        std::thread::spawn(move || {
            let mut buffer = [0; 1024];
            std::thread::sleep(std::time::Duration::from_millis(50));
            while let Ok(bytes) = reader.read(&mut buffer) {
                if bytes < buffer.len() { break; }
            }
            while let Some(command) = rx.blocking_recv() {
                let start_signal = "__START__";
                let end_signal = "__TALOS_CMD_COMPLETE__";
                let safe_cmd = command.trim().replace("\"", "\\\"");
                let agy_command = format!("agy -p \"{}\"", safe_cmd);
                let full_input = format!("echo {}& {} & echo {}\n", start_signal, agy_command, end_signal);

                if writer.write_all(full_input.as_bytes()).is_err() {
                    break;
                }
                let mut accumulated_output = String::new();
                while let Ok(bytes_read) = reader.read(&mut buffer) {
                    if bytes_read == 0 { break; }

                    let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
                    accumulated_output.push_str(&chunk);
                    if accumulated_output.contains(&format!("\n{}", end_signal)) {
                        let clean_output = accumulated_output
                            .split(start_signal)
                            .last()
                            .unwrap_or("")
                            .split(end_signal)
                            .next()
                            .unwrap_or("")
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
    let mut stdout = std::io::stdout();
    enable_raw_mode().unwrap();
    stdout.execute(EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App {
        apikey_state: TextState::new(),
    };
    let api_key = loop {
        terminal.draw(|f| app.draw_ui(f)).unwrap();
        if let Event::Key(key) = event::read().unwrap() {
            // Feed the key to the prompt
            app.apikey_state.handle_key_event(key);

            // Check if they hit Enter (Done) or Esc (Aborted)
            if app.apikey_state.status() == Status::Done {
                break app.apikey_state.value().to_string();
            } else if app.apikey_state.status() == Status::Aborted {
                break String::new(); // Fallback if they cancel
            }
        }
    };
    disable_raw_mode().unwrap();
    std::io::stdout().execute(LeaveAlternateScreen).unwrap();
    if !api_key.is_empty() {
        let auth_data = AuthData { data: api_key.trim().to_string() };
        let json = serde_json::to_string(&auth_data).unwrap();
        fs::write(path.join("user_api.info"), json).unwrap();
    }
}

pub async fn get_auth(path: &PathBuf) -> AuthData {
    let json = fs::read_to_string(path.join("user_api.info")).unwrap();
    serde_json::from_str(&json).unwrap()
}

pub async fn gemini_communicate_speech(mut session: Session, mut rx_out: tokio::sync::mpsc::UnboundedReceiver<TalosBus>, _tx_in: tokio::sync::mpsc::UnboundedSender<TalosBus>) -> Result<(), Box<dyn std::error::Error>> {
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
        let _ = _tx_in.send(TalosBus::TerminalOutput(format!("You: {}", speech)));
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
                    let _ = _tx_in.send(TalosBus::TerminalOutput(format!("AI: {}", gemini_response)));
                    break;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub async fn gemini_api(api_key: &str, rx_out: tokio::sync::mpsc::UnboundedReceiver<TalosBus>, _tx_in: tokio::sync::mpsc::UnboundedSender<TalosBus>) -> Result<(), Box<dyn std::error::Error>> {
    let ai_management_prompt = r#"You are Talos, an advanced, voice-operated Developer Assistant. You have direct, real-time access to the user's host operating system and terminal. Your goal is to help the user navigate their computer, write code, manage files, and automate GUI tasks completely hands-free.

Your Capabilities (Tools)
You have two primary domains of control via your tools:

    OS Control (Mouse & Keyboard): You can move the mouse, click (mouse_click), type text (type_text), press special keys (press_key), and scroll (scroll).

    Terminal Control (AGY CLI): You can execute terminal commands, manage files, and write code using the run_agy_cli tool.

Operational Directives & Safety Rules

    Think Before You Act: Before executing any tool, briefly state what you are about to do out loud so the user is aware. (e.g., "I'm going to open the terminal and list your files now.")

    Prefer the CLI for Data: If the user asks for information about their system (e.g., "What's in this folder?", "Read this code file"), always use run_agy_cli rather than trying to use the mouse and keyboard to open a GUI app.

    Chain Actions Logically: You can use multiple tools in sequence. For example, if asked to write a script and run it, use the CLI to create the file, write the code, and execute it in one fluid process.

    Spatial Awareness: You do not inherently know where UI elements are on the screen unless the user provides exact X/Y coordinates. If a user asks you to click something but hasn't provided coordinates (and you haven't received a screen capture yet), ask them for the coordinates or suggest a keyboard shortcut alternative.

    Destructive Actions: If a user asks you to delete files, format drives, or run potentially dangerous commands via the CLI, you must ask for verbal confirmation before proceeding."#;

    let session = Session::connect(SessionConfig {
        transport: TransportConfig {
            auth: Auth::ApiKey(api_key.to_string()),
            ..Default::default()
        },
        setup: SetupConfig {
            model: "models/gemini-3.1-flash-live-preview".into(),
            system_instruction: Some(gemini_live::Content {
                parts: vec![gemini_live::Part {
                    text: Some(ai_management_prompt.to_string()),
                    inline_data: None,
                }],
                role: Some("system".to_string()),
            }),
            generation_config: Some(GenerationConfig {
                response_modalities: Some(vec![Modality::Audio]),
                ..Default::default()
            }),
            ..Default::default()
        },
        reconnect: ReconnectPolicy::default(),
    }).await?;
    gemini_communicate_speech(session, rx_out, _tx_in).await?;
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
    println!("Command Start");
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
    let agy_output = cmd.output()?;
    let text = String::from_utf8(agy_output.stdout)?.to_string();
    if !text.is_empty() {
        talos_bus_tx.send(TalosBus::TerminalOutput(text))?;
    }
    println!("PTY End");
    Ok(())
}