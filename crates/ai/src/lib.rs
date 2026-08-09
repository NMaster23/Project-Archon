use gemini_live::{
    Auth, GenerationConfig, Modality, ReconnectPolicy, ServerEvent, Session, SessionConfig,
    SetupConfig, TransportConfig,
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use tokio::process::Command;
use std::string::ToString;
use std::time::Duration;
use talos_core::TalosBus;
use tokio::sync::mpsc;
use app_dirs2::{AppDataType, AppInfo, get_app_root};
use app_dirs2::AppDataType::UserConfig;
use mistralrs::{Model, ModelBuilder};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use turso::Builder;
use fastembed::{TextEmbedding, TextInitOptions, EmbeddingModel};
use tokio::io::AsyncWriteExt;

const APP_INFO: AppInfo = AppInfo {
    name: "Talos",
    author: "NMCreator",
};

pub const MEMORY_PROMPT: &str = r#"You are the background memory compression engine for Talos, a local developer assistant.
Distill the user's chat log into dense, permanent facts. Ignore all pleasantries, raw code outputs, and conversational filler.
Extract the data strictly using this format:
FACTS: [List any new user preferences, project constraints, or system configurations learned]
STATE: [What was successfully executed, built, or remains broken]
ENTITIES: [Comma-separated list of specific file paths, Rust crates, tools, or variables mentioned]
SUMMARY: [One concise sentence describing the final outcome of the interaction]"#;

pub const SOUL_PROMPT: &str = r#"Role:
You are an expert AI Architect and Prompt Engineer. Your task is to act as the "Soul-Weaver"—synthesizing scattered information into a powerful, cohesive Master System Prompt for a new AI assistant.

Objective:
I will provide you with a list of raw facts, desired behaviors, and constraints. Your goal is to analyze these inputs, extract the core intent, and weave them into a precise, highly effective System Instruction.

Execution Rules:

    Analyze: Review the raw inputs below to identify the core persona, the primary objective, and the strict boundaries.

    Synthesize: Combine this information into exactly two cohesive paragraphs.

    Paragraph 1 (The Essence): Define the AI's identity, role, tone, and primary mission. Breathe life into its persona and establish who it is and what it aims to achieve.

    Paragraph 2 (The Bounds): Establish the operational rules, formatting requirements, and strict constraints. Define what the AI must do and must never do.

    Refine: Ensure the language is direct, commanding, and leaves no room for ambiguity. Use imperative verbs (e.g., "Always format," "Never assume")."#;

pub const SKILL_PROMPT: &str = r#"Role:
  You are the Master Skill-Builder, an advanced AI diagnostic system. Your job is to analyze recent execution failures from the AI assistant and write a new, permanent Skill to ensure it never makes the same mistake again.

Objective:
  Write a comprehensive, highly structured markdown guide that teaches the AI exactly how to solve this specific class of problem flawlessly.

Execution Rules:
  1. You MUST output ONLY valid markdown. Do not include any conversational filler (e.g., "Here is your skill:", "Understood").
  2. The very top of your output MUST contain this exact YAML frontmatter block, replacing the bracketed text with your own short titles:
---
name: [a-short-kebab-case-name]
description: [One sentence explaining when the AI should use this skill]
---
  3. Below the frontmatter, write the Skill content strictly adhering to the "Skill Structure" defined below.
  4. DO NOT wrap your entire response in a markdown code block (```markdown). Start immediately with the `---` of the frontmatter.

Skill Structure:
  ## 1. Failure Analysis & Root Cause
  Provide a blunt, precise breakdown of exactly where and why the previous attempts failed based on the logs.

  ## 2. Core Operating Principles
  Define 2-3 absolute rules or algorithmic shifts the AI must adopt to approach this problem correctly.

  ## 3. Execution Protocol
  Provide a foolproof, sequential algorithm for the AI to follow. Use imperative language (e.g., "Verify X before attempting Y").

  ## 4. Anti-Patterns to Avoid
  List the specific mistakes from the logs as 'Anti-Patterns' and directly contrast them with the 'Correct Approach'.

<failure_logs>
{failures}
</failure_logs>"#;

pub const REACT_PROMPT: &str = r#"You are Talos, an expert developer assistant.
You operate in a strict ReAct (Reasoning and Acting) loop to solve user requests.

You have access to the following tools:
{tools}

YOUR SKILLS / PAST MISTAKES:
You have learned the following skills from past failures. You MUST obey these instructions:
{skills}

HOW TO BEHAVE:
You must resolve the user's request by following this exact cycle:
1. Thought: Explain your reasoning, analyze the current state, and plan your next move.
2. Action: Execute a tool by outputting a strict JSON object.
3. Observation: (The system will provide the tool's output to you).
... (Repeat Thought -> Action -> Observation until the task is complete)
4. Final Answer: Provide the final response to the user.

FORMATTING RULES:
When you need to use a tool, your output MUST follow this exact format:
Thought: [Your step-by-step reasoning]
Action: {"action": "tool_name", "args": {"arg1": "value"}}

When you have completed the task or need to ask the user a clarifying question, your output MUST follow this exact format:
Thought: I have the information I need.
Final Answer: [Your comprehensive response or question to the user]

CRITICAL CONSTRAINTS:
- ALWAYS include a "Thought:" line before an Action or Final Answer.
- Output exactly ONE Action per turn.
- Do NOT wrap your Action in Markdown code blocks (e.g., no ```json). Output raw JSON.
- Stop generating text immediately after outputting an Action. Wait for the Observation.
- Never fake an Observation; the system will provide it.
"#;

pub const SUCCESS_SKILL_PROMPT: &str = r#"You are an Expert AI Behavior Analyst. You are given a transcript of a highly successful execution chain where an AI agent perfectly solved a user's problem.

Your objective is to deconstruct this winning strategy and codify it into a reusable "Skill Cheatsheet" that future AI agents can instantly understand and execute.

### Instructions:
1. Analyze the transcript to identify the core user intent.
2. Map the exact sequence of tools used.
3. Extract the critical reasoning steps, decision points, and data transformations.
4. Output your analysis STRICTLY using the Markdown template provided below.

### Output Template:
# Skill: [Create a clear, descriptive 3-5 word title]

**Trigger Intent:** [1-2 sentences describing the specific user problem or request that should trigger this skill]

### Tool Chain Sequence
[List the exact sequence of tools used, e.g., ToolA -> ToolB -> ToolC]

### Step-by-Step Execution Strategy
* **Step 1: [Action Taken]** - [Explain the specific reasoning and what data was passed/extracted]
* **Step 2: [Action Taken]** - [Explain the specific reasoning and what data was passed/extracted]
* [Continue for all essential steps]

### Key Insights & Pitfalls to Avoid
* [Bullet points of any clever maneuvers, parameter specifics, or logical leaps the agent made that are crucial for success]

### Constraints:
* NO conversational filler or pleasantries.
* NO preambles or postscripts.
* Output ONLY the populated Markdown template.

***
[SUCCESSFUL EXECUTION CHAIN START]
{successes}
[SUCCESSFUL EXECUTION CHAIN END]
"#;

pub struct StreamingSource {
    receiver: std::sync::mpsc::Receiver<f32>,
}

pub struct AgySession {
    tx: UnboundedSender<String>,
}

impl AgySession {
    pub fn new(
        talos_bus_tx: UnboundedSender<TalosBus>,
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
                let _start_time = std::time::Instant::now();
                let start_time = std::time::Instant::now();
                let _last_chunk_time = start_time;
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
                let stripped = strip_ansi_escapes::strip(accumulated_output.as_bytes());
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
        std::num::NonZeroU16::new(1).unwrap_or(unsafe { std::num::NonZeroU16::new_unchecked(1) })
    }
    fn sample_rate(&self) -> std::num::NonZeroU32 {
        std::num::NonZeroU32::new(24000).unwrap_or(unsafe { std::num::NonZeroU32::new_unchecked(24000) })
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub async fn gemini_communicate_speech(
    mut session: Session,
    mut rx_out: UnboundedReceiver<TalosBus>,
    tx_in: UnboundedSender<TalosBus>,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
    let player = rodio::Player::connect_new(handle.mixer());
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
            },
            Some(TalosBus::ToolCallResult { call_id, tool_name, result }) => {
                let result: serde_json::Value = serde_json::from_str(&result).unwrap_or(serde_json::Value::Null);
                let tool_response = gemini_live::FunctionResponse {
                    id: call_id,
                    name: tool_name,
                    response: result,
                };
                if let Err(e) = session.send_tool_response(vec![tool_response]).await {
                    eprintln!("{}", e);
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
        if let Ok(memories) = retrieve_memories(&speech, 3).await {
            if !memories.is_empty() {
                let combined_memories = memories.join("\n- ");
                speech = format!(
                    "Relevant context from previous conversations:\n- {}\n\nUser Input: {}",
                    combined_memories,
                    speech.trim(),
                )
            }
        };
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
                        let _ = tx_in.send(TalosBus::ActionIntent {
                            call_id: tool_call.id,
                            tool: tool_call.name,
                            args: tool_call.args.to_string(),
                        });
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
    rx_out: UnboundedReceiver<TalosBus>,
    tx_in: UnboundedSender<TalosBus>,
    token_budget: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let ai_management_prompt = r#"You are Talos, an advanced, voice-operated Developer Assistant. You have direct, real-time access to the user's host operating system and terminal through an AGY CLI bridge.

When the user asks you to inspect files, write code, run commands, or manage the local project, respond with the exact instruction you want AGY to execute. Do not claim you ran a command yourself; the host app forwards your completed response to AGY and displays the result.

Operational Directives & Safety Rules

    Prefer the CLI for Data: If the user asks for information about their system (e.g., "What's in this folder?", "Read this code file"), issue a concise AGY instruction for the CLI rather than describing GUI steps.

    Chain Actions Logically: You can request multiple terminal/file actions in one instruction when they belong together.

    Destructive Actions: If a user asks you to delete files, format drives, or run potentially dangerous commands, ask for confirmation before issuing the AGY instruction."#;
    let executor_tools = executor::gemini_api_mcp().await;
    let gemini_tools = vec![gemini_live::Tool::FunctionDeclarations(
        executor_tools.into_iter().filter_map(|raw| {
            serde_json::from_value(raw).ok()
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
                max_output_tokens: Some(token_budget),
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

pub async fn agy_setup(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let _ = Command::new("cmd")
            .args(["/C", "curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd && agy"])
            .status()
            .await?;
    } else if cfg!(any(target_os = "linux", target_os = "macos")) {
        let _ = Command::new("sh")
            .args([
                "-c",
                "curl -fsSL https://antigravity.google/cli/install.sh | bash && agy",
            ])
            .status()
            .await?;
    } else {
        println!("Unsupported OS");
    }
    println!("Path successfully found at {:?}", path);
    Ok(())
}

pub async fn agy_communicate(
    new_chat: bool,
    talos_bus_tx: UnboundedSender<TalosBus>,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", "agy"]);
        c
    } else {
        Command::new("agy")
    };
    
    if new_chat {
        cmd.args(["--dangerously-skip-permissions", "-p", input.trim()]);
    } else {
        cmd.args(["--dangerously-skip-permissions", "-c", "-p", input.trim()]);
    }
    
    match cmd.output().await {
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

pub async fn agy_backend(mut rx_out: UnboundedReceiver<TalosBus>, tx_in: tokio::sync::mpsc::UnboundedSender<TalosBus>) -> Result<(), Box<dyn std::error::Error>> {
    let agy_session = AgySession::new(tx_in.clone())?;
    while let Some(event) = rx_out.recv().await {
        match event {
            TalosBus::VoiceTranscript(speech) => {
                let mut prompt = speech.trim().to_string();
                if prompt.is_empty() {
                    continue;
                }
                if let Ok(memories) = retrieve_memories(&prompt, 3).await {
                    if !memories.is_empty() {
                        let combined_memories = memories.join("\n- ");
                        prompt = format!(
                            "Relevant context from previous conversations:\n- {}\n\nUser Input: {}",
                            combined_memories,
                            speech.trim(),
                        )
                    }
                }
                tx_in.send(TalosBus::TerminalOutput(format!("You: {}", prompt))).ok();
                agy_session.execute(&prompt);
            }
            _ => {}
        }
    }
    Ok(())
}

pub async fn create_config() -> Result<(), Box<dyn std::error::Error>> {
    let app_root = get_app_root(UserConfig, &APP_INFO)?;
    let config_file = app_root.join("config.json");
    fs::write(config_file, talos_core::CONFIG_TEMPLATE)?;
    Ok(())
}

pub async fn save_chats(mut rx_out: UnboundedReceiver<TalosBus>) -> Result<(), Box<dyn std::error::Error>> {
    let app_root = get_app_root(UserConfig, &APP_INFO)?;
    let chat_location = app_root.join(".history").join("chats.db");
    if !chat_location.exists() {
        fs::create_dir_all(app_root.join(".history"))?;
    }
    let db = Builder::new_local(chat_location.to_str().ok_or("Invalid path")?).build().await?;
    let conn = db.connect()?;
    conn.execute(
    "CREATE TABLE IF NOT EXISTS chats (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        method TEXT NOT NULL,
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
    )",
        ()
    ).await?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS profile (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        fact TEXT UNIQUE NOT NULL
        )",
        ()
    ).await?;
    let mut rows = conn.query("SELECT COUNT(DISTINCT session_id) FROM chats", ()).await?;
    let current_session: i64 = if let Ok(Some(row)) = rows.next().await {
        row.get(0).unwrap_or(0)
    } else {
        0
    };
    let session_id = format!("session_{}", current_session + 1);
    while let Some(event) = rx_out.recv().await {
        match event {
            TalosBus::VoiceTranscript(speech) => {
                conn.execute(
                "INSERT INTO chats (session_id, role, content, method) VALUES (?1, ?2, ?3, ?4)",
                (session_id.clone(), "user", speech, "voice"),
                ).await?;
            }
            TalosBus::AiResponse(ai_response) => {
                conn.execute(
                    "INSERT INTO chats (session_id, role, content, method) VALUES (?1, ?2, ?3, ?4)",
                    (session_id.clone(), "assistant", ai_response, "ai"),
                ).await?;
            }
            TalosBus::ActionIntent { call_id, tool, args } => {
                let tool_mem = format!("Used tool: {}, with args: {}", tool, args);
                conn.execute(
                    "INSERT INTO chats (session_id, role, content, method) VALUES (?1, ?2, ?3, ?4)",
                    (session_id.clone(), "assistant", tool_mem, "tool-call"),
                ).await?;
            }
            TalosBus::TerminalOutput(output) => {
                conn.execute(
                    "INSERT INTO chats (session_id, role, content, method) VALUES (?1, ?2, ?3, ?4)",
                    (session_id.clone(), "system", output, "agy-cli"),
                ).await?;
            }
            TalosBus::ScreenCapture(_) => {
                conn.execute(
                    "INSERT INTO chats (session_id, role, content, method) VALUES (?1, ?2, ?3, ?4)",
                    (session_id.clone(), "system", "screencap", "system"),
                ).await?;
            }
            TalosBus::UserCredentials(_) => { return Ok(()); }
            TalosBus::Shutdown => { return Ok(()); }
            _ => {
                return Err(Box::<dyn std::error::Error>::from("Unknown error"));
            }
        }
    }
    Ok(())
}

pub async fn memory_parser(raw_output: &str) -> (String, String, String, String) {
    let fact_idx = raw_output.find("FACTS:");
    let state_idx = raw_output.find("STATE:");
    let entity_idx = raw_output.find("ENTITIES:");
    let summary_idx = raw_output.find("SUMMARY:");
    let facts_output = match (fact_idx, state_idx) {
        (Some(f), Some(s)) if f + 6 <= s => raw_output[(f + 6)..s].trim(),
        _ => "",
    }.to_string();
    let state_output = match (state_idx, entity_idx) {
        (Some(s), Some(e)) if s + 6 <= e => raw_output[(s + 6)..e].trim(),
        _ => "",
    }.to_string();
    let entity_output = match (entity_idx, summary_idx) {
        (Some(e), Some(s)) if e + 9 <= s => raw_output[(e + 9)..s].trim(),
        _ => "",
    }.to_string();
    let summary_output = match summary_idx {
        Some(s) if s + 8 <= raw_output.len() => raw_output[(s + 8)..].trim(),
        _ => "",
    }.to_string();
    (facts_output, state_output, entity_output, summary_output)
}

pub async fn manage_memory() -> Result<(), Box<dyn std::error::Error>> {
    let app_root = get_app_root(UserConfig, &APP_INFO)?;
    let chat_location = app_root.join(".history").join("chats.db");
    let db = Builder::new_local(chat_location.to_str().ok_or("Invalid path")?).build().await?;
    let conn = db.connect()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            level INTEGER NOT NULL,
            content TEXT NOT NULL,
            vector F32_BLOB(384) NOT NULL
        )",
        ()
    ).await?;
    let model = ModelBuilder::new("meta-llama/Llama-3.2-1B")
        .build()
        .await?;
    let mut embed_model = TextEmbedding::try_new(
        TextInitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true).with_intra_threads(4),
    )?;
    let mut rows = conn.query("SELECT COUNT(*) FROM memories WHERE level = 0", ()).await?;
    let _row = rows.next().await?.ok_or("No row")?;
    let mut session_query = conn.query("SELECT session_id FROM chats ORDER BY id DESC LIMIT 1", ()).await?;
    let mut chat = String::new();
    let target_session = if let Some(row) = session_query.next().await? {
        row.get::<String>(0)?
    } else {
        String::new()
    };
    if !target_session.is_empty() {
        let mut chat_rows = conn.query("SELECT role, content FROM chats WHERE session_id = ?1 ORDER BY id ASC", [target_session]).await?;
        while let Some(row) = chat_rows.next().await? {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            chat.push_str(format!("{}: {}\n", role, content).as_str());
        }
    }
    let message = format!("System: {}\n\nUser: {}", MEMORY_PROMPT, chat);
    let summary = model.chat(message).await.map_err(|e| e.to_string())?;
    let (facts, _state, _entities, clean_summary) = memory_parser(&summary).await;
    let docs = vec![clean_summary.clone()];
    let embeddings = embed_model.embed(docs, None)?;
    let memory_vector = format!("{:?}", &embeddings[0]);
    if !facts.is_empty() {
        conn.execute(
            "INSERT OR IGNORE INTO profile (fact) VALUES (?1)",
            [facts]
        ).await?;
    }
    conn.execute(
        "INSERT INTO memories (level, content, vector) VALUES (?1, ?2, vector32(?3))",
        (0, clean_summary, memory_vector)
    ).await?;
    let mut current_level = 0;
    loop {
        let mut count_query = conn.query("SELECT COUNT(*) FROM memories WHERE level = ?1", [current_level]).await?;
        let row_count = count_query.next().await?.ok_or("No row")?;
        let count: i64 = row_count.get(0)?;
        if count < 20 {
            break
        }
        if count >= 20 {
            let mut group_rows = conn.query("SELECT id, content FROM memories WHERE level = ?1 ORDER BY id ASC LIMIT 10", (current_level,)).await?;
            let mut del_ids = Vec::new();
            let mut combined_content = String::new();
            while let Some(row) = group_rows.next().await? {
                let id: i64 = row.get(0)?;
                let text: String = row.get(1)?;
                del_ids.push(id);
                combined_content.push_str(&text);
                combined_content.push_str("\n---\n");
            }
            let prompt = format!("System: {}\n\nUser: {}", MEMORY_PROMPT, combined_content);
            let new_summary = model.chat(prompt).await.map_err(|e| e.to_string())?;
            let (facts, _state, _entities, clean_summary) = memory_parser(&new_summary).await;
            let docs = vec![clean_summary.clone()];
            let embeddings = embed_model.embed(docs, None)?;
            let memory_vector = format!("{:?}", &embeddings[0]);
            if !facts.is_empty() {
                conn.execute(
                    "INSERT OR IGNORE INTO profile (fact) VALUES (?1)",
                    [facts]
                ).await?;
            }
            conn.execute(
                "INSERT INTO memories (level, content, vector) VALUES (?1, ?2, vector32(?3))",
                (current_level + 1, clean_summary, memory_vector)
            ).await?;
            for id in del_ids {
                conn.execute("DELETE FROM memories WHERE id = ?1", [id]).await?;
            }
            current_level += 1;
        }
    }
    Ok(())
}

pub async fn manage_soul() -> Result<(), Box<dyn std::error::Error>> {
    let app_root = get_app_root(UserConfig, &APP_INFO)?;
    let history_dir = app_root.join(".history");
    if !history_dir.exists() {
        fs::create_dir_all(&history_dir)?;
    }
    let chat_location = history_dir.join("chats.db");
    let db = Builder::new_local(chat_location.to_str().ok_or("Invalid path")?).build().await?;
    let conn = db.connect()?;
    let mut to_improve = conn.query(
        "SELECT fact FROM profile", ()
    ).await?;
    let mut combined_facts = String::new();
    while let Some(row) = to_improve.next().await? {
        let fact: String = row.get(0)?;
        combined_facts.push_str(&fact);
        combined_facts.push_str("\n---\n");
    }
    let model = ModelBuilder::new("meta-llama/Llama-3.2-1B")
        .build()
        .await?;
    let prompt = format!("{}\n\nFacts:\n{}", SOUL_PROMPT, combined_facts);
    let new_soul = model.chat(prompt).await.map_err(|e| e.to_string())?;
    let agent_dir = app_root.join("Agent");
    fs::create_dir_all(&agent_dir)?;
    let soul_file = agent_dir.join("agent.md");
    fs::write(&soul_file, &new_soul)?;
    let current_dir = std::env::current_dir()?;
    let rules_dir = current_dir.join(".gemini").join("rules");
    fs::create_dir_all(&rules_dir)?;
    let soul_file = rules_dir.join("talos_identity.md");
    fs::write(&soul_file, &new_soul)?;
    Ok(())
}

pub async fn get_db_conn() -> Result<turso::Connection, Box<dyn std::error::Error>> {
    let app_root = get_app_root(UserConfig, &APP_INFO)?;
    let hist_dir = app_root.join(".history");
    if !hist_dir.exists() {
        fs::create_dir_all(&hist_dir)?;
    }
    let chat_location = hist_dir.join("chats.db");
    let db = Builder::new_local(chat_location.to_str().ok_or("Invalid path")?).build().await?;
    let conn = db.connect()?;
    Ok(conn)
}

pub async fn self_improvement() -> Result<(), Box<dyn std::error::Error>> {
    let model = ModelBuilder::new("meta-llama/Llama-3.2-1B")
        .build()
        .await?;
    let app_root = get_app_root(UserConfig, &APP_INFO)?;
    let conn = get_db_conn().await?;
    let mut error_rows = conn.query(
        "SELECT content FROM chats WHERE content LIKE '%Error%' ORDER BY id DESC LIMIT 3",
        turso::params![]
    ).await?;
    let mut failures = String::new();
    while let Some(row) = error_rows.next().await? {
        let error: String = row.get(0)?;
        failures.push_str(&error);
        failures.push_str("\n---\n");
    }
    let prompt = SKILL_PROMPT.replace("{failures}", &failures);
    let skill_content = model.chat(prompt).await.map_err(|e| e.to_string())?;
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let skill_name = format!("skill_{}", timestamp);
    let user_skills_dir = app_root.join("Agent").join("Skills");
    fs::create_dir_all(&user_skills_dir)?;
    fs::write(user_skills_dir.join(format!("{}.md", skill_name)), &skill_content)?;
    let current_dir = std::env::current_dir()?;
    let agy_skill_dir = current_dir.join(".gemini").join("skills").join(&skill_name);
    fs::create_dir_all(&agy_skill_dir)?;
    fs::write(agy_skill_dir.join("SKILL.md"), &skill_content)?;
    Ok(())
}

pub async fn retrieve_memories(query: &str, limit: u32) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let conn = get_db_conn().await?;
    let mut embed_model = TextEmbedding::try_new(TextInitOptions::new(EmbeddingModel::AllMiniLML6V2))?;
    let embeddings = embed_model.embed(vec![query], None)?;
    let query_vector = format!("{:?}", &embeddings[0]);
    let mut rows = conn.query("SELECT content FROM memories ORDER BY vector_distance_cos(vector, vector32(?1)) LIMIT ?2", (query_vector, limit)).await?;
    let mut relevant_memories = Vec::new();
    while let Some(row) = rows.next().await? {
        relevant_memories.push(row.get::<String>(0)?);
    }
    Ok(relevant_memories)
}

pub async fn react_loop(tx_in: UnboundedSender<TalosBus>, mut rx_out: &mut UnboundedReceiver<TalosBus>, input: &str, model: &Model) -> Result<(), Box<dyn std::error::Error>> {
    let mut loaded_skills = String::new();
    let app_root = get_app_root(AppDataType::UserConfig, &APP_INFO)?;
    let user_skills_dir = app_root.join("Agent").join("Skills");
    
    if user_skills_dir.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(user_skills_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                    loaded_skills.push_str(&content);
                    loaded_skills.push_str("\n\n");
                }
            }
        }
    }
    if loaded_skills.is_empty() {
        loaded_skills = "No skills learned".to_string();
    }
    let active_prompt = REACT_PROMPT.replace("{loaded_skills}", &loaded_skills);
    let mut history = format!("System: {}\n\nUser: {}", REACT_PROMPT, input);
    loop {
        let output = model.chat(history.clone()).await?;
        if output.starts_with("{") {
            let json: serde_json::Value = serde_json::from_str(&output)?;
            let action = json["action"].as_str().unwrap();
            let args = json["args"].to_string();
            let call_id = format!("call_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u32);
            tx_in.send(TalosBus::ActionIntent {
                call_id: call_id.clone(),
                tool: action.to_string(),
                args: args.to_string(),
            });
            while let Some(msg) = rx_out.recv().await {
                if let TalosBus::ToolCallResult { call_id: response_id, result, .. } = msg {
                    if response_id == call_id {
                        history.push_str(&format!("\nAssistant: {}\nObservation: {}", output, result));
                        break;
                    }
                }
            }
        } else {
            tx_in.send(TalosBus::AiResponse(output.clone()))?;
            break Ok(());
        }
    }
}

pub async fn local_backend(mut rx_out: UnboundedReceiver<TalosBus>, tx_in: UnboundedSender<TalosBus>) -> Result<(), Box<dyn std::error::Error>> {
    let model = ModelBuilder::new("meta-llama/Llama-3.2-1B").build().await?;
    while let Some(msg) = rx_out.recv().await {
        if let TalosBus::VoiceTranscript(input) = msg {
            if let Err(e) = react_loop(tx_in.clone(), &mut rx_out, &input, &model).await {
                eprintln!("ReAct Error: {}", e);
            }
        }
    }
    Ok(())
}

pub async fn success_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let app_root = get_app_root(UserConfig, &APP_INFO)?;
    let model = ModelBuilder::new("meta-llama/Llama-3.2-1B").build().await?;
    let conn = get_db_conn().await?;
    let mut rows = conn.query(
        "SELECT content FROM chats WHERE content NOT LIKE '%Error%' AND length(content) > 300 ORDER BY id DESC LIMIT 3",
        ()
    ).await?;
    let mut successes = String::new();
    while let Some(row) = rows.next().await? {
        let success_text: String = row.get(0)?;
        successes.push_str(&success_text);
        successes.push_str("\n---\n");
    };
    let prompt = SUCCESS_SKILL_PROMPT.replace("{successes}", &successes);
    let skill_contents = model.chat(prompt).await?;
    if !skill_contents.is_empty() {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis();
        let skill_name = format!("{}_skill.md", timestamp);
        let skill_dirname = format!("{}_skill", skill_name);
        let skill_path = app_root.join("Agent").join("Skills");
        fs::create_dir_all(&skill_path)?;
        fs::write(skill_path.join(&skill_name), &skill_contents)?;
        let current_dir = std::env::current_dir()?;
        let agy_skill_dir = current_dir.join(".gemini").join("skills").join(&skill_name);
        fs::create_dir_all(&agy_skill_dir)?;
        fs::write(agy_skill_dir.join("SKILL.md"), &skill_contents)?;
    }
    Ok(())
}