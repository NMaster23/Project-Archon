pub enum SystemEvent {
    VoiceTranscript(String),
    ScreenCapture(String), // Base64
    AiResponse(String),
    ActionIntent { tool: String, args: String },
    Shutdown,
}
