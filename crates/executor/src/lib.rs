use std::io::Cursor;
use image::ImageFormat::{Jpeg, Png};
use rust_mcp_sdk::schema::Tool;
use xcap::image::{ImageFormat, RgbaImage};
use xcap::{Frame, Monitor};
use serde_json::json;

pub struct MouseMovement {
    pub x: i32,
    pub y: i32,
}

pub struct KeyboardInput {
    pub text: String,
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