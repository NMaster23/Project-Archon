pub type TalosBus = String;

pub enum SystemEvent {
    VoiceTranscript(String),
    ScreenCapture(String),
    AiResponse(String),
    UserCredentials(String),
    ActionIntent { tool: String, args: String },
    Shutdown,
}
