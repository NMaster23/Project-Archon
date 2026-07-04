pub enum TalosBus {
    VoiceTranscript(String),
    ScreenCapture(String),
    AiResponse(String),
    UserCredentials(String),
    TerminalOutput(String),
    ActionIntent { tool: String, args: String },
    Shutdown,
}
