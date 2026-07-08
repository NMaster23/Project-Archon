pub enum TalosBus {
    VoiceTranscript(String),
    ScreenCapture(Vec<u8>),
    AiResponse(String),
    UserCredentials(String),
    TerminalOutput(String),
    ActionIntent { tool: String, args: String },
    Shutdown,
}
