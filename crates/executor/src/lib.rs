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

const TOOLS: &[(&str, &str)] = &[
    ("cursor_move", "Move the cursor on screen"),
    ("mouse_click", "Click left (1), right (2), or middle (3) mouse button"),
    ("mouse_scroll", "Scroll mouse wheel vertically"),
    ("key_press", "Press a specific key"),
    ("key_type", "Type a full string of text"),
];

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
    pub key: String,
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
            let parsed_key = McpServer::parse_key_string(&input.key).await.unwrap();
            enigo.key(parsed_key, Direction::Click).unwrap();
            Ok(CallToolResult::text(format!("Pressed key: {} successfully.", input.key)))
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
    let router = McpRouter::new()
        .server_info("talos-executor", "0.1.0")
        .tool(cursor_move)
        .tool(mouse_click)
        .tool(mouse_scroll)
        .tool(key_press)
        .tool(key_type);
    let transport = HttpTransport::new(router);
    let app = transport.into_router();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Talos Executor running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
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
            let key_str = parsed["key"].as_str().unwrap();
            let key = McpServer::parse_key_string(key_str).await.unwrap();
            enigo.key(key, Direction::Click).map_err(|e| e.to_string())?;
            Ok("Pressed key".into())
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
    config["mcpServers"]["talos-executor"] = json!({
        "type": "http",
        "serverUrl": "http://127.0.0.1:3000/"
    });
    if config.get("mcpServers").and_then(|m| m.get("talos-executor")) == Some(&config) {
        return;
    }
    if let Some(parent) = &config_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let updated_json = serde_json::to_string_pretty(&config).expect("Failed to create json");
    fs::write(config_path, &updated_json).expect("Failed to save json");
}