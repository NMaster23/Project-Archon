use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TalosBus {
    VoiceTranscript(String),
    ScreenCapture(Vec<u8>),
    AiResponse(String),
    UserCredentials(String),
    TerminalOutput(String),
    ActionIntent { tool: String, args: String },
    Shutdown,
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