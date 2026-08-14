use std::fs::{self, File};
use std::path::Path;
use std::string::ToString;
use serde_json::{Value, json};
use mslnk::{ShellLink,MSLinkError};

const LINUX_DESKTOP: &str = r"
[Desktop Entry]
Version=1.0
Name=Talos
Comment=Talos is a cross platform agentic assistant that can control the entire computer and can be deployed on a server.
Exec={app_path}
Path={app_dir}
Icon={icon_path}
Terminal=false
Type=Application
Categories=Utility;Development;
";

fn main() {
    setup();
    mcp_setup();
}

pub fn setup() {
    let dir = dirs::data_local_dir().expect("Could not get local dir").join("Talos").join("Models");
    fs::create_dir_all(&dir).expect("Could not create Models directory");
    let mut files_zip = reqwest::blocking::get("https://github.com/NMaster23/Project-Archon/releases/download/Non-User/models.zip").expect("Could not download models");
    let zip_path = &dir.join("Non-User.zip");
    let mut zip_file = File::create(zip_path).expect("Could not create file");
    std::io::copy(&mut files_zip, &mut zip_file).expect("Could not copy file");
    let zip = File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(zip).expect("Failed to open zip");
    archive.extract(dir).expect("Failed to extract zip");
    fs::remove_file(zip_path).expect("Could not remove file");
    let app_executable = match std::env::consts::OS {
        "windows" => "https://github.com/NMaster23/Project-Archon/releases/download/windows.exe",
        "linux" => "https://github.com/NMaster23/Project-Archon/releases/download/linux64",
        "macos" => "https://github.com/NMaster23/Project-Archon/releases/download/macos",
        _ => {
            eprintln!("Unknown/Unsupported Operating system");
            "https://github.com/NMaster23/Project-Archon/releases/"
        }
    };
    let mut app = reqwest::blocking::get(app_executable).expect("Could not download models");
    let app_path = dirs::data_local_dir().expect("Could not get local dir").join("Talos");
    let exe_path = app_path.join(if std::env::consts::OS == "windows" { "talos.exe" } else { "talos" });
    let mut app_file = File::create(&exe_path).expect("Could not create file");
    std::io::copy(&mut app, &mut app_file).expect("Could not copy file");
    create_shortcut(&exe_path);
}

#[cfg(target_os = "windows")]
pub fn create_shortcut(path: &Path) {
    let icon_path = dirs::data_local_dir().expect("Could not get local dir").join("Talos").join("Icon.ico");
    std::fs::write(&icon_path, include_bytes!("../../../assets/Icon.ico")).unwrap();
    let lnk = dirs::desktop_dir().expect("Could not get home dir").join("talos.lnk");
    let mut sl = ShellLink::new(path).expect("Could not create shortcut link");
    sl.set_icon_location(Some(icon_path.to_str().unwrap().to_string()));
    sl.create_lnk(lnk).expect("Could not create shortcut link");
}

#[cfg(target_os = "linux")]
pub fn create_shortcut(path: &Path) {
    let icon_path = dirs::data_local_dir().expect("Could not get local dir").join("Talos").join("Icon.png");
    std::fs::write(&icon_path, include_bytes!("../../../assets/Icon.png")).unwrap();
    let finalized_desktop = LINUX_DESKTOP
        .replace("{app_path}", path.to_str().unwrap())
        .replace("{app_dir}", path.parent().unwrap().to_str().unwrap())
        .replace("{icon_path}", icon_path.to_str().unwrap());
    let desktop_file = dirs::desktop_dir().expect("Could not get desktop directory").join("Talos.desktop");
    std::fs::write(&desktop_file, finalized_desktop).expect("Failed to write Linux shortcut");
    if let Some(desktop) = desktop_file.to_str() {
        std::process::Command::new("chmod").args(&["+x", desktop]).spawn().expect("Chmod failed");
    }
}

#[cfg(target_os = "macos")]
pub fn create_shortcut(path: &Path) {
    let desktop_link = dirs::desktop_dir().expect("Could not get desktop dir").join("Talos");
    std::os::unix::fs::symlink(&path, &desktop_link).expect("Could not create shortcut link");
    if let Some(app) = path.to_str() {
        std::process::Command::new("chmod").args(&["+x", app]).spawn().expect("Chmod failed");
    }
}

pub fn mcp_setup() {
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