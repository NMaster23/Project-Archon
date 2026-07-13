use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use image::ImageFormat::{Jpeg, Png};
use serde_json::{Value, json};
use std::io::Cursor;
use xcap::image::{ImageFormat, RgbaImage};
use xcap::{Frame, Monitor};
use mcpkit::prelude::*;
use mcpkit_axum::prelude::*;

pub struct MouseMovement {
    pub x: i32,
    pub y: i32,
}

pub struct KeyboardInput {
    pub text: String,
}

pub struct McpServer;

impl McpServer {
    pub fn new() -> Self {
        Self
    }

    pub fn get_enigo() -> Result<Enigo, String> {
        Enigo::new(&Settings::default()).map_err(|e| e.to_string())
    }
    pub async fn parse_key_string(key_str: &str) -> Option<Key> {
        match key_str.to_lowercase().as_str() {
            "enter" | "return" => Some(Key::Return),
            "tab" => Some(Key::Tab),
            "space" => Some(Key::Space),
            "escape" | "esc" => Some(Key::Escape),
            "backspace" => Some(Key::Backspace),
            "up" => Some(Key::UpArrow),
            "down" => Some(Key::DownArrow),
            "left" => Some(Key::LeftArrow),
            "right" => Some(Key::RightArrow),
            "shift" => Some(Key::Shift),
            "control" | "ctrl" => Some(Key::Control),
            "alt" => Some(Key::Alt),
            "super" | "win" | "cmd" => Some(Key::Meta),
            other => {
                if other.chars().count() == 1 {
                    Some(Key::Unicode(other.chars().next().unwrap()))
                } else {
                    None
                }
            }
        }
    }
}

#[mcp_server(name = "talos-executor", version = "0.1.0")]
impl McpServer {
    #[tool(description = "Move the cursor to a specific coordinate on the screen.")]
    pub async fn cursor_move(&self, x: i32, y: i32) -> ToolOutput {
        let mut enigo = Self::get_enigo().unwrap();
        enigo.move_mouse(x, y, Coordinate::Abs).unwrap();
        ToolOutput::text(format!("Moved cursor to X: {} Y: {} on screen", x, y))
    }
    #[tool(description = "Click left, right, or middle mouse button.")]
    pub async fn mouse_click(&self, button: i32) -> ToolOutput {
        let mut enigo = Self::get_enigo().unwrap();
        let mut clicked_button = String::new();
        if button == 1 {
            enigo.button(Button::Left, Direction::Click).unwrap();
            clicked_button = "Left Button".to_string();
        } else if button == 2 {
            enigo.button(Button::Right, Direction::Click).unwrap();
            clicked_button = "Right Button".to_string();
        } else if button == 3 {
            enigo.button(Button::Middle, Direction::Click).unwrap();
            clicked_button = "Middle Button".to_string();
        }
        ToolOutput::text(format!("Clicked button: {} successfully", clicked_button))
    }
    #[tool(description = "Scroll the mouse wheel a certain amount of line.")]
    pub async fn mouse_scroll(&self, lines: i32) -> ToolOutput {
        let mut enigo = Self::get_enigo().unwrap();
        enigo.scroll(lines, Axis::Vertical).unwrap();
        ToolOutput::text(format!("Scrolled the mouse wheel {} successfully.", lines))
    }
    #[tool(description = "Press any key on the keyboard.")]
    pub async fn key_press(&self, key: String) -> ToolOutput {
        let mut enigo = Self::get_enigo().unwrap();
        let parsed_key = Self::parse_key_string(&key).await.unwrap();
        enigo.key(parsed_key, Direction::Click).unwrap();
        ToolOutput::text(format!("Pressed key: {} successfully.", key))
    }
    #[tool(description = "Type a string of keys.")]
    pub async fn key_type(&self, text: String) -> ToolOutput {
        let mut enigo = Self::get_enigo().unwrap();
        enigo.text(&text).unwrap();
        ToolOutput::text(format!("Typed text: {} successfully.", text))
    }
    #[resource(
        uri_pattern = "talos://placeholder",
        name = "PlaceHolder",
        description = "Placeholder resource (server exposes no real resources)",
        mime_type = "text/plain"
    )]
    pub async fn placeholder_resource(&self, uri: &str) -> ResourceContents {
        ResourceContents::text(uri, "")
    }

    #[prompt(description = "Placeholder prompt (server exposes no real prompts)")]
    pub async fn placeholder_prompt(&self) -> GetPromptResult {
        GetPromptResult {
            description: None,
            messages: vec![],
        }
    }
}

pub async fn screen_cap() {
    let monitor = Monitor::from_point(0, 0).unwrap();
    let (video_recorder, sx) = monitor.video_recorder().unwrap();
    tokio::spawn(async move {
        loop {
            match sx.recv() {
                Ok(frame) => {
                    println!("frame: {:?}", frame.width);
                    encode(frame, Jpeg).await;
                }
                _ => continue,
            }
        }
    });
    println!("start");
    video_recorder.start().unwrap();
}

pub async fn encode(frame: Frame, image_format: ImageFormat) {
    let image =
        RgbaImage::from_raw(frame.width, frame.height, frame.raw).expect("Failed to create image");
    let mut buffer = Cursor::new(Vec::new());
    let output_format = match image_format {
        Jpeg => ImageFormat::Jpeg,
        Png => ImageFormat::Png,
        _ => ImageFormat::Jpeg,
    };
    image
        .write_to(&mut buffer, output_format)
        .expect("Failed to save image");
    talos_core::TalosBus::ScreenCapture(buffer.into_inner());
    println!("screen captured");
}

// TODO: Implement start_mcpserver() here