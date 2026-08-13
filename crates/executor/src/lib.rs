use std::fs;
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use image::ImageFormat::Jpeg;
use serde_json::{Value, json};
use std::io::Cursor;
use xcap::image::RgbaImage;
use mcpkit::prelude::*;
use schemars::JsonSchema;
use std::net::SocketAddr;
use std::process::Stdio;
use tokio::process::Command;
use tower_mcp::{BoxError, CallToolResult, HttpTransport, McpRouter, ToolBuilder};
use base64::engine::general_purpose;
use base64::prelude::*;
use imageproc::{drawing::{draw_text_mut, draw_hollow_rect_mut}, rect::Rect};
use webp_screenshot_rust::{WebPScreenshot, CaptureConfig, WebPConfig};
use ocrs::{OcrEngine, OcrEngineParams};
use ab_glyph::{FontRef, PxScale};
use rten::Model;
use ocrs::TextItem;
use tokio::io::{AsyncBufReadExt, BufReader};

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

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

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
                other.chars().next().map(Key::Unicode)
            }
        }
    }
}

pub async fn tools() -> Result<(), BoxError> {
    mcp_setup().await;

    let cursor_move = ToolBuilder::new("cursor_move")
        .description("Move the cursor to a specific coordinate on the screen.")
        .handler(|input: CursorMoveInput| async move {
            let mut enigo = match McpServer::get_enigo() {
                Ok(e) => e,
                Err(e) => return Ok(CallToolResult::error(e)),
            };
            if let Err(e) = enigo.move_mouse(input.x, input.y, Coordinate::Abs) {
                return Ok(CallToolResult::error(e.to_string()));
            }
            Ok(CallToolResult::text(format!("Moved cursor to X: {} Y: {} on screen", input.x, input.y)))
        })
        .build();
    let mouse_click = ToolBuilder::new("mouse_click")
        .description("Click left, right, or middle mouse button.")
        .handler(|input: MouseClickInput| async move {
            let mut enigo = match McpServer::get_enigo() {
                Ok(e) => e,
                Err(e) => return Ok(CallToolResult::error(e)),
            };
            let mut clicked_button = String::new();
            if input.button == 1 {
                if let Err(e) = enigo.button(Button::Left, Direction::Click) { return Ok(CallToolResult::error(e.to_string())); }
                clicked_button = "Left Button".to_string();
            } else if input.button == 2 {
                if let Err(e) = enigo.button(Button::Right, Direction::Click) { return Ok(CallToolResult::error(e.to_string())); }
                clicked_button = "Right Button".to_string();
            } else if input.button == 3 {
                if let Err(e) = enigo.button(Button::Middle, Direction::Click) { return Ok(CallToolResult::error(e.to_string())); }
                clicked_button = "Middle Button".to_string();
            }
            Ok(CallToolResult::text(format!("Clicked button: {} successfully", clicked_button)))
        })
        .build();
    let mouse_scroll = ToolBuilder::new("mouse_scroll")
        .description("Scroll the mouse wheel a certain amount of line.")
        .handler(|input: MouseScrollInput| async move {
            let mut enigo = match McpServer::get_enigo() {
                Ok(e) => e,
                Err(e) => return Ok(CallToolResult::error(e)),
            };
            if let Err(e) = enigo.scroll(input.lines, Axis::Vertical) {
                return Ok(CallToolResult::error(e.to_string()));
            }
            Ok(CallToolResult::text(format!("Scrolled the mouse wheel {} successfully.", input.lines)))
        })
        .build();
    let key_press = ToolBuilder::new("key_press")
        .description("Press any key on the keyboard.")
        .handler(|input: KeyPressInput| async move {
            let mut enigo = match McpServer::get_enigo() {
                Ok(e) => e,
                Err(e) => return Ok(CallToolResult::error(e)),
            };
            let mut parsed_keys = Vec::new();
            for k in &input.keys {
                if let Some(parsed) = McpServer::parse_key_string(k).await {
                    parsed_keys.push(parsed);
                }
            }
            if parsed_keys.is_empty() {
                return Ok(CallToolResult::text("No Valid Keys Provided"));
            }
            for key in parsed_keys.iter().take(parsed_keys.len().saturating_sub(1)) {
                if let Err(e) = enigo.key(*key, Direction::Press) { return Ok(CallToolResult::error(e.to_string())); }
            }
            if let Some(last_key) = parsed_keys.last() && let Err(e) = enigo.key(*last_key, Direction::Click) {
                return Ok(CallToolResult::error(e.to_string()));
            }
            for key in parsed_keys.iter().take(parsed_keys.len().saturating_sub(1)).rev() {
                if let Err(e) = enigo.key(*key, Direction::Release) { return Ok(CallToolResult::error(e.to_string())); }
            }
            Ok(CallToolResult::text(format!("Pressed keys: {:?} successfully.", input.keys)))
        })
        .build();
    let key_type = ToolBuilder::new("key_type")
        .description("Type a string of keys.")
        .handler(|input: KeyTypeInput| async move {
            let mut enigo = match McpServer::get_enigo() {
                Ok(e) => e,
                Err(e) => return Ok(CallToolResult::error(e)),
            };
            if let Err(e) = enigo.text(&input.text) {
                return Ok(CallToolResult::error(e.to_string()));
            }
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
            let mut screenshot = match WebPScreenshot::with_config(config) {
                Ok(s) => s,
                Err(e) => return Ok(CallToolResult::error(e.to_string())),
            };
            let results = screenshot.capture_all_displays();
            if let Some(Ok(capture)) = results.into_iter().find(|r| r.is_ok()) {
                let img = match image::load_from_memory(&capture.data) {
                    Ok(i) => i.to_rgba8(),
                    Err(e) => return Ok(CallToolResult::error(e.to_string())),
                };
                let edited = match image_edit(img).await {
                    Ok(e) => e,
                    Err(e) => return Ok(CallToolResult::error(e)),
                };
                let base64_img = general_purpose::STANDARD.encode(&edited.0);
                Ok(CallToolResult::image(base64_img, "image/jpeg"))
            } else {
                Ok(CallToolResult::error("Failed to capture any displays.".to_string()))
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
    println!("Talos Executor running on https://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn image_edit(mut image: RgbaImage) -> Result<(Vec<u8>, Vec<(i32, i32, String)>), String> {
    let mut buffer = Cursor::new(Vec::new());
    let grid_color = image::Rgba([255, 0, 0, 255]);
    let spacing = 100;
    let font = FontRef::try_from_slice(include_bytes!("../../../assets/Fonts/JetBrainsMono-Regular.ttf")).unwrap();
    for (i, x) in (0..image.width()).step_by(spacing).enumerate() {
        let col_label = format!("{}", (b'A' + (i % 26) as u8) as char);
        for (j, y) in (0..image.height()).step_by(spacing).enumerate() {
            let row_label = format!("{}", j);
            image.put_pixel(x, y, grid_color);
            draw_text_mut(&mut image, grid_color, x as i32 + 5, y as i32 + 5, PxScale::from(15.0), &font, &format!("{}{}", col_label, row_label));
        }
    }
    let ocr = ocr_init().await?;
    let img_tensor = ocrs::ImageSource::from_bytes(image.as_raw(), image.dimensions()).map_err(|e| e.to_string())?;
    let ocr_input = ocr.prepare_input(img_tensor).map_err(|e| e.to_string())?;
    let text_lines_raw = ocr.find_text_lines(&ocr_input, Default::default());
    let text_lines = ocr.recognize_text(&ocr_input, &text_lines_raw);
    let mut click_targets = Vec::new();
    let flat_lines: Vec<_> = text_lines.into_iter().flatten().flatten().collect();
    for (id, line) in flat_lines.iter().enumerate() {
        let bounding_box = line.bounding_rect();
        let min_x = bounding_box.left();
        let min_y = bounding_box.top();
        let max_x = bounding_box.right();
        let max_y = bounding_box.bottom();
        let x_cent = (min_x + max_x) / 2;
        let y_cent = (min_y + max_y) / 2;
        let rect = Rect::at(min_x, min_y)
            .of_size((max_x - min_x) as u32, (max_y - min_y) as u32);
        draw_hollow_rect_mut(&mut image, rect, image::Rgba([255, 255, 0, 255]));
        let tag = format!("[{}]", id);
        draw_text_mut(&mut image, image::Rgba([0, 255, 0, 255]), min_x, min_y - 15, PxScale::from(14.0), &font, &tag);
        click_targets.push((x_cent, y_cent, line.to_string()));
    }
    image
        .write_to(&mut buffer, Jpeg)
        .map_err(|e| e.to_string())?;
    Ok((buffer.into_inner(), click_targets))
}

pub async fn call_tool(tool_name: &str, args: &str) -> Result<String, String> {
    let parsed: Value = serde_json::from_str(args).map_err(|e| e.to_string())?;
    let mut enigo = McpServer::get_enigo()?;
    match tool_name {
        "cursor_move" => {
            let x = parsed["x"].as_i64().ok_or("Missing x")? as i32;
            let y = parsed["y"].as_i64().ok_or("Missing y")? as i32;
            enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| e.to_string())?;
            Ok("Moved cursor".into())
        }
        "mouse_click" => {
            let button = match parsed["button"].as_i64().unwrap_or(1) {
                1 => Button::Left,
                2 => Button::Right,
                3 => Button::Middle,
                _ => Button::Left,
            };
            enigo.button(button, Direction::Click).map_err(|e| e.to_string())?;
            Ok("Clicked button".into())
        }
        "mouse_scroll" => {
            let lines = parsed["lines"].as_i64().ok_or("Missing lines")? as i32;
            enigo.scroll(lines, Axis::Vertical).map_err(|e| e.to_string())?;
            Ok("Scrolled the mouse wheel".into())
        }
        "key_press" => {
            let mut parsed_keys = Vec::new();
            if let Some(keys_array) = parsed["keys"].as_array() {
                for k in keys_array {
                    if let Some(key_str) = k.as_str() && let Some(parsed) = McpServer::parse_key_string(key_str).await {
                        parsed_keys.push(parsed);
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
            let text = parsed.get("text").and_then(|v| v.as_str()).ok_or("Missing text")?;
            enigo.text(text).map_err(|e| e.to_string())?;
            Ok("Typed text".into())
        }
        "view_screen" => {
            let config = CaptureConfig {
                webp_config: WebPConfig::high_quality(),
                include_cursor: true,
                use_hardware_acceleration: true,
                ..Default::default()
            };
            let mut screenshot = match WebPScreenshot::with_config(config) {
                Ok(s) => s,
                Err(e) => return Err(e.to_string()),
            };
            let results = screenshot.capture_all_displays();
            if let Some(Ok(capture)) = results.into_iter().find(|r| r.is_ok()) {
                let img = match image::load_from_memory(&capture.data) {
                    Ok(i) => i.to_rgba8(),
                    Err(e) => return Err(e.to_string()),
                };
                let edited = match image_edit(img).await {
                    Ok(e) => e,
                    Err(e) => return Err(e.to_string()),
                };
                let base64_img = general_purpose::STANDARD.encode(&edited.0);
                Ok(json!({
                    "image": base64_img,
                    "targets": edited.1
                }).to_string())
            } else {
                Err("Failed to capture any displays.".to_string())
            }
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
    let home = match std::env::home_dir() {
        Some(h) => h,
        None => return,
    };
    let config_path = home
        .join(".gemini")
        .join("config")
        .join("mcp_config.json");
    let mut config: Value = if config_path.exists() {
        let content = fs::read_to_string(&config_path).unwrap_or_else(|_| "".to_string());
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
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(updated_json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(config_path, &updated_json);
        }
    }
}

pub async fn gemini_api_mcp() -> Vec<Value> {
    serde_json::from_str(API_TOOLS).unwrap_or_default()
}

pub async fn ocr_init() -> Result<OcrEngine, String> {
    let detection_model = Model::load_file("models/ocrs/text-detection-ssfbcj81.rten").map_err(|e| e.to_string())?;
    let recognition_model = Model::load_file("models/ocrs/text-rec-checkpoint-s52qdbqt.rten").map_err(|e| e.to_string())?;
    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    }).map_err(|e| e.to_string())
}