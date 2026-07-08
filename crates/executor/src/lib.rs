use std::io::Cursor;
use image::ImageFormat::{Jpeg, Png};
use rust_mcp_sdk::schema::Tool;
use xcap::image::{ImageFormat, RgbaImage};
use xcap::{Frame, Monitor};
use serde_json::{json, Value};
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

pub struct MouseMovement {
    pub x: i32,
    pub y: i32,
}

pub struct KeyboardInput {
    pub text: String,
}

pub struct McpControl;

impl McpControl {
    pub fn handle_tool_execution(tool_name: &str, arguments: &Value) -> Result<String, String> {
        match tool_name {
            "mouse_click" => Self::execute_mouse_click(arguments),
            "type_text" => Self::execute_type_text(arguments),
            "press_key" => Self::execute_press_key(arguments),
            "scroll" => Self::execute_scroll(arguments),
            _ => Err(format!("Unknown tool requested: {}", tool_name)),
        }
    }

    fn execute_mouse_click(args: &Value) -> Result<String, String> {
        let x = args.get("x").and_then(|v| v.as_i64()).ok_or("Missing or invalid 'x' coordinate")? as i32;
        let y = args.get("y").and_then(|v| v.as_i64()).ok_or("Missing or invalid 'y' coordinate")? as i32;
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| e.to_string())?;
        enigo.button(Button::Left, Direction::Click).map_err(|e| e.to_string())?;
        Ok(format!("Successfully clicked at coordinates ({}, {})", x, y))
    }

    fn execute_type_text(args: &Value) -> Result<String, String> {
        let text = args.get("text").and_then(|v| v.as_str()).ok_or("Missing or invalid 'text' argument")?;
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo.text(text).map_err(|e| e.to_string())?;
        Ok(format!("Successfully typed the requested text."))
    }
    fn execute_press_key(args: &Value) -> Result<String, String> {
        let key_str = args.get("key").and_then(|v| v.as_str()).ok_or("Missing or invalid 'key' argument")?;
        let key = Self::parse_key_string(key_str).ok_or_else(|| format!("Unsupported key: {}", key_str))?;

        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo.key(key, Direction::Click).map_err(|e| e.to_string())?;
        Ok(format!("Successfully pressed the '{}' key.", key_str))
    }

    fn execute_scroll(args: &Value) -> Result<String, String> {
        let lines = args.get("lines").and_then(|v| v.as_i64()).ok_or("Missing or invalid 'lines' argument")? as i32;
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo.scroll(lines, Axis::Vertical).map_err(|e| e.to_string())?;
        Ok(format!("Successfully scrolled {} lines.", lines))
    }

    fn parse_key_string(key_str: &str) -> Option<Key> {
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
            "super" | "win" | "cmd" => Some(Key::Meta), // Super/Win are now grouped under Meta
            other => {
                if other.chars().count() == 1 {
                    // `Key::Layout` is gone, use `Key::Unicode` for characters
                    Some(Key::Unicode(other.chars().next().unwrap()))
                } else {
                    None
                }
            }
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

pub async  fn encode(frame: Frame, image_format: ImageFormat) {
    let image = RgbaImage::from_raw(frame.width, frame.height, frame.raw)
        .expect("Failed to create image");
    let mut buffer = Cursor::new(Vec::new());
    let output_format = match image_format {
        Jpeg => ImageFormat::Jpeg,
        Png => ImageFormat::Png,
        _ => ImageFormat::Jpeg,
    };
    image.write_to(&mut buffer, output_format).expect("Failed to save image");
    talos_core::TalosBus::ScreenCapture(buffer.into_inner());
    println!("screen captured");
}

pub async fn available_tools() -> Result<Vec<Tool>, rust_mcp_sdk::GenericSendError> {
    let tools = vec![
        Tool {
            name: "mouse_click".to_string(),
            description: Some("Move the cursor to a specific coordinate and left click.".to_string()),
            input_schema: serde_json::from_value(json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer", "description": "X coordinate on the screen" },
                    "y": { "type": "integer", "description": "Y coordinate on the screen" }
                },
                "required": ["x", "y"]
            })).unwrap(),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            title: None,
            output_schema: None,
        },
        Tool {
            name: "type_text".to_string(),
            description: Some("Type a string of text using the keyboard.".to_string()),
            input_schema: serde_json::from_value(json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "The exact text to type" }
                },
                "required": ["text"]
            })).unwrap(),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            title: None,
            output_schema: None,
        },
        Tool {
            name: "press_key".to_string(),
            description: Some("Press a specific special key (e.g., 'enter', 'tab', 'escape').".to_string()),
            input_schema: serde_json::from_value(json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "The name of the key to press" }
                },
                "required": ["key"]
            })).unwrap(),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            title: None,
            output_schema: None,
        },
        Tool {
            name: "scroll".to_string(),
            description: Some("Scroll the mouse wheel.".to_string()),
            input_schema: serde_json::from_value(json!({
                "type": "object",
                "properties": {
                    "lines": { "type": "integer", "description": "Positive to scroll down, negative to scroll up" }
                },
                    "required": ["lines"]
                })).unwrap(),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            title: None,
            output_schema: None,
        }
    ];
    Ok(tools)
}