pub type TalosBus = String;

pub enum SystemEvent {
    VoiceTranscript(String),
    ScreenCapture(String),
    AiResponse(String),
    ActionIntent { tool: String, args: String },
    Shutdown,
}
