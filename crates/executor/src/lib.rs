use std::{env, fs};
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use image::ImageFormat::{Jpeg, Png};
use serde_json::{Value, json};
use std::io::{Cursor, Write};
use xcap::image::{ImageFormat, RgbaImage};
use xcap::{Frame, Monitor};
use mcpkit::prelude::*;
use axum;
use schemars::JsonSchema;
use serde::Serialize;
use std::net::SocketAddr;
use tower_mcp::{BoxError, CallToolResult, HttpTransport, McpRouter, ToolBuilder};
use std::time::Instant;
use base64::engine::general_purpose;
use base64::prelude::*;
use imageproc::drawing::Canvas;
use webp_screenshot_rust::{WebPScreenshot, CaptureConfig, WebPConfig, ScreenCapture};

const TOOLS: &[(&str, &str)] = &[
    ("cursor_move", "Move the cursor on screen"),
    ("mouse_click", "Click left (1), right (2), or middle (3) mouse button"),
    ("mouse_scroll", "Scroll mouse wheel vertically"),
    ("key_press", "Press a specific key or combination"),
    ("key_type", "Type a full string of text"),
    ("view_screen", "Capture the screen and return it as an image to view"),
];

const API_TOOLS: &str = r#"[
  {
    "name": "cursor_move",
    "description": "Move the cursor on screen",
    "parameters": {
      "type": "object",
      "properties": {
        "x": { "type": "integer" },
        "y": { "type": "integer" }
      },
      "required": ["x", "y"]
    }
  },
  {
    "name": "mouse_click",
    "description": "Click left (1), right (2), or middle (3) mouse button",
    "parameters": {
      "type": "object",
      "properties": {
        "button": { "type": "integer" }
      },
      "required": ["button"]
    }
  },
  {
    "name": "mouse_scroll",
    "description": "Scroll mouse wheel vertically",
    "parameters": {
      "type": "object",
      "properties": {
        "lines": { "type": "integer" }
      },
      "required": ["lines"]
    }
  },
  {
    "name": "key_press",
    "description": "Press a specific key or combination",
    "parameters": {
      "type": "object",
      "properties": {
        "keys": { "type": "array", "items": { "type": "string" } }
      },
      "required": ["keys"]
    }
  },
  {
    "name": "key_type",
    "description": "Type a full string of text",
    "parameters": {
      "type": "object",
      "properties": {
        "text": { "type": "string" }
      },
      "required": ["text"]
    }
  },
  {
    "name": "view_screen",
    "description": "Capture the screen and return it as an image to view",
    "parameters": { "type": "object", "properties": {} }
  }
]"#;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CursorMoveInput {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MouseClickInput {
    pub button: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MouseScrollInput {
    pub lines: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeyPressInput {
    pub keys: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeyTypeInput {
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

pub async fn tools() -> Result<(), BoxError> {
    mcp_setup().await;

    let cursor_move = ToolBuilder::new("cursor_move")
        .description("Move the cursor to a specific coordinate on the screen.")
        .handler(|input: CursorMoveInput| async move {
            let mut enigo = McpServer::get_enigo().unwrap();
            enigo.move_mouse(input.x, input.y, Coordinate::Abs).unwrap();
            Ok(CallToolResult::text(format!("Moved cursor to X: {} Y: {} on screen", input.x, input.y)))
        })
        .build();
    let mouse_click = ToolBuilder::new("mouse_click")
        .description("Click left, right, or middle mouse button.")
        .handler(|input: MouseClickInput| async move {
            let mut enigo = McpServer::get_enigo().unwrap();
            let mut clicked_button = String::new();
            if input.button == 1 {
                enigo.button(Button::Left, Direction::Click).unwrap();
                clicked_button = "Left Button".to_string();
            } else if input.button == 2 {
                enigo.button(Button::Right, Direction::Click).unwrap();
                clicked_button = "Right Button".to_string();
            } else if input.button == 3 {
                enigo.button(Button::Middle, Direction::Click).unwrap();
                clicked_button = "Middle Button".to_string();
            }
            Ok(CallToolResult::text(format!("Clicked button: {} successfully", clicked_button)))
        })
        .build();
    let mouse_scroll = ToolBuilder::new("mouse_scroll")
        .description("Scroll the mouse wheel a certain amount of line.")
        .handler(|input: MouseScrollInput| async move {
            let mut enigo = McpServer::get_enigo().unwrap();
            enigo.scroll(input.lines, Axis::Vertical).unwrap();
            Ok(CallToolResult::text(format!("Scrolled the mouse wheel {} successfully.", input.lines)))
        })
        .build();
    let key_press = ToolBuilder::new("key_press")
        .description("Press any key on the keyboard.")
        .handler(|input: KeyPressInput| async move {
            let mut enigo = McpServer::get_enigo().unwrap();
            let mut parsed_keys = Vec::new();
            for k in &input.keys {
                if let Some(parsed) = McpServer::parse_key_string(k).await {
                    parsed_keys.push(parsed);
                }
            }
            if parsed_keys.is_empty() {
                return Ok(CallToolResult::text("No Valid Keys Provided"));
            }
            for key in parsed_keys.iter().take(parsed_keys.len() - 1) {
                enigo.key(*key, Direction::Press).unwrap();
            }
            if let Some(last_key) = parsed_keys.last() {
                enigo.key(*last_key, Direction::Click).unwrap();
            }
            for key in parsed_keys.iter().take(parsed_keys.len().saturating_sub(1)).rev() {
                enigo.key(*key, Direction::Release).unwrap();
            }
            Ok(CallToolResult::text(format!("Pressed keys: {:?} successfully.", input.keys)))
        })
        .build();
    let key_type = ToolBuilder::new("key_type")
        .description("Type a string of keys.")
        .handler(|input: KeyTypeInput| async move {
            let mut enigo = McpServer::get_enigo().unwrap();
            enigo.text(&input.text).unwrap();
            Ok(CallToolResult::text(format!("Typed text: {} successfully.", input.text)))
        })
        .build();
    let view_screen = ToolBuilder::new("view_screen")
        .description("Take a capture of the screen to view")
        .handler(|_: ()| async move {
            let config = CaptureConfig {
                webp_config: WebPConfig::high_quality(),
                include_cursor: true,
                use_hardware_acceleration: true,
                ..Default::default()
            };
            let mut screenshot = WebPScreenshot::with_config(config).unwrap();
            let results = screenshot.capture_all_displays();
            if let Some(Ok(capture)) = results.into_iter().find(|r| r.is_ok()) {
                let img = image::load_from_memory(&capture.data).unwrap().to_rgba8();
                let base64_img = general_purpose::STANDARD.encode(encode(img).await);
                Ok(CallToolResult::image(base64_img, "image/jpeg"))
            } else {
                Ok(CallToolResult::text("Failed to capture any displays."))
            }
        })
        .build();
    let router = McpRouter::new()
        .server_info("talos-executor", "0.1.0")
        .tool(cursor_move)
        .tool(mouse_click)
        .tool(mouse_scroll)
        .tool(key_press)
        .tool(key_type)
        .tool(view_screen);
    let transport = HttpTransport::new(router);
    let app = transport.into_router();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Talos Executor running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn encode(mut image: RgbaImage) -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    let grid_color = image::Rgba([255, 0, 0, 255]);
    let spacing = 100;
    for x in (0..image.width()).step_by(spacing) {
        for y in 0..image.height() {
            image.put_pixel(x, y, grid_color);
        }
    }
    for y in (0..image.height()).step_by(spacing) {
        for x in 0..image.width() {
            image.put_pixel(x, y, grid_color);
        }
    }
    image
        .write_to(&mut buffer, Jpeg)
        .expect("Failed to save image");
    buffer.into_inner()
}

pub async fn call_tool(tool_name: &str, args: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(args).map_err(|e| e.to_string())?;
    let mut enigo = McpServer::get_enigo().unwrap();
    match tool_name {
        "cursor_move" => {
            enigo.move_mouse(parsed["x"].as_i64().unwrap() as i32, parsed["y"].as_i64().unwrap() as i32, Coordinate::Abs).map_err(|e| e.to_string())?;
            Ok("Moved cursor".into())
        }
        "mouse_click" => {
            let button = match parsed["button"].as_i64().unwrap() {
                1 => {
                    Button::Left
                }
                2 => {
                    Button::Right
                }
                3 => {
                    Button::Middle
                }
                _ => {
                    Button::Left
                }
            };
            enigo.button(button, Direction::Click).map_err(|e| e.to_string())?;
            Ok("Clicked button".into())
        }
        "mouse_scroll" => {
            let lines = parsed["lines"].as_i64().unwrap() as i32;
            enigo.scroll(lines, Axis::Vertical).unwrap();
            Ok("Scrolled the mouse wheel".into())
        }
        "key_press" => {
            let mut parsed_keys = Vec::new();
            if let Some(keys_array) = parsed["keys"].as_array() {
                for k in keys_array {
                    if let Some(key_str) = k.as_str() {
                        if let Some(parsed) = McpServer::parse_key_string(key_str).await {
                            parsed_keys.push(parsed);
                        }
                    }
                }
            }
            for key in parsed_keys.iter().take(parsed_keys.len().saturating_sub(1)) {
                enigo.key(*key, Direction::Press).map_err(|e| e.to_string())?;
            }
            if let Some(last_key) = parsed_keys.last() {
                enigo.key(*last_key, Direction::Click).map_err(|e| e.to_string())?;
            }
            for key in parsed_keys.iter().take(parsed_keys.len().saturating_sub(1)).rev() {
                enigo.key(*key, Direction::Release).map_err(|e| e.to_string())?;
            }
            Ok("Pressed keys".into())
        }
        "key_type" => {
            enigo.text(parsed["type"].as_str().unwrap()).map_err(|e| e.to_string())?;
            Ok("Typed text".into())
        }
        _ => Err("Unknown tool".into()),
    }
}

pub async fn get_tools() -> Vec<talos_core::ToolDeclaration> {
    TOOLS.iter().map(|(name, desc)| talos_core::ToolDeclaration {
        name: name.to_string(),
        description: desc.to_string(),
    }).collect()
}

pub async fn mcp_setup() {
    let config_path = std::env::home_dir().unwrap()
        .join(".gemini")
        .join("config")
        .join("mcp_config.json");
    let mut config: Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_else(|_| "".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = json!({})
    }
    let mut changed = false;

    let new_server = json!({
        "type": "http",
        "serverUrl": "http://127.0.0.1:3000/"
    });
    
    if config.get("mcpServers").and_then(|m| m.get("talos-executor")) != Some(&new_server) {
        config["mcpServers"]["talos-executor"] = new_server;
        changed = true;
    }

    let chrome_devtools = json!({
        "command": "npx",
        "args": [
            "-y",
            "chrome-devtools-mcp@latest",
            "--browser-url=http://127.0.0.1:9222"
        ]
    });

    if config.get("mcpServers").and_then(|m| m.get("chrome-devtools")) != Some(&chrome_devtools) {
        config["mcpServers"]["chrome-devtools"] = chrome_devtools;
        changed = true;
    }

    if changed {
        if let Some(parent) = &config_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let updated_json = serde_json::to_string_pretty(&config).expect("Failed to create json");
        fs::write(config_path, &updated_json).expect("Failed to save json");
    }
}

pub async fn gemini_api_mcp() -> Vec<serde_json::Value> {
    serde_json::from_str(API_TOOLS).expect("Failed to parse json")
}