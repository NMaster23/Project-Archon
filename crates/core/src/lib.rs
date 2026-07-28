use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TalosBus {
    VoiceTranscript(String),
    ScreenCapture(Vec<u8>),
    AiResponse(String),
    UserCredentials(String),
    TerminalOutput(String),
    ActionIntent { tool: String, args: String },
    Shutdown,
    RenderWidget { plugin_id: String, widget_id: String, layout_json: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientToServer {
    VoiceTranscript(String),
    ScreenCapture(Vec<u8>),
    ToolCallResult { call_id: String, success: bool, result: String },
    ToolRegistration { tools: Vec<ToolDeclaration> },
    UserCredentials(String),
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerToClient {
    AiResponse(String),
    TerminalOutput(String),
    AudioPlayback(Vec<u8>),
    ExecuteToolCall { call_id: String, tool_name: String, args: String },
    RequestScreenCapture,
    ProcessingState(bool),
    RequestToolRegistration,
    Pong,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDeclaration {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    BusEvent(TalosBus),
    ClientEvent(ClientToServer),
    ServerEvent(ServerToClient),
    ToolsUpdate(Vec<ToolDeclaration>),
}

// ==========================================
// MASTER CONFIGURATION STRUCTS
// ==========================================

pub const CONFIG_TEMPLATE: &str = r#"{
  "_comment": "=== TALOS MASTER CONFIGURATION ===",

  "backend": "OAuth",
  "dashboard_port": 3000,
  "run_in_background": false,
  "start_on_boot": false,
  "debug_logging": false,

  "gemini_api_key": "",
  "model": "models/gemini-3.1-flash-live-preview",
  "system_prompt_override": "",
  "max_output_tokens": 8192,

  "stt_disabled_by_default": false,
  "input_device": "default",
  "output_device": "default",
  "silence_threshold_rms": 0.01,
  "push_to_talk_key": null,

  "auto_start_plugins": true,
  "plugin_directory": "./plugins",
  "allowed_mcp_servers": ["*"],
  
  "_note": "Plugins will inject their custom settings here",
  "example_plugin_theme": "dark"
}"#;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TalosConfig {
    pub ai_permissions: Vec<String>,
    pub backend: String,
    pub dashboard_port: u16,
    pub run_in_background: bool,
    pub start_on_boot: bool,
    pub debug_logging: bool,
    pub gemini_api_key: String,
    pub model: String,
    pub system_prompt_override: String,
    pub max_output_tokens: u32,
    pub stt_disabled_by_default: bool,
    pub input_device: String,
    pub output_device: String,
    pub silence_threshold_rms: f32,
    pub push_to_talk_key: Option<String>,
    pub auto_start_plugins: bool,
    pub plugin_directory: String,
    pub allowed_mcp_servers: Vec<String>,
    #[serde(flatten)]
    pub custom_settings: HashMap<String, serde_json::Value>,
}

impl TalosConfig {
    pub fn load(path: &std::path::Path, template: &str) -> Self {
        if path.exists() {
            let contents = std::fs::read_to_string(path).unwrap();
            serde_json::from_str(&contents).unwrap_or_else(|_| Self::default())
        } else {
            std::fs::write(path, template).ok();
            serde_json::from_str(template).unwrap_or_else(|_| Self::default())
        }
    }

    pub fn save(&self, path: &std::path::Path) {
        if let Ok(contents) = serde_json::to_string_pretty(self) {
            std::fs::write(path, contents).unwrap();
        }
    }
}