use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=web_frontend/package.json");
    println!("cargo:rerun-if-changed=web_frontend/vite.config.ts");
    println!("cargo:rerun-if-changed=web_frontend/src");
    println!("cargo:rerun-if-changed=web_frontend/public");
    println!("cargo:rerun-if-changed=web_frontend/index.html");

    let npm_command = if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    };

    let install_status = Command::new(npm_command)
        .arg("install")
        .current_dir("web_frontend")
        .status()
        .expect("Failed to run npm install. Is npm installed and in your PATH?");

    if !install_status.success() {
        panic!("npm install failed! See the output above for details.");
    }

    let build_status = Command::new(npm_command)
        .arg("run")
        .arg("build")
        .current_dir("web_frontend")
        .status()
        .expect("Failed to run npm run build.");

    if !build_status.success() {
        panic!("npm run build failed! Check your Vite/React code for errors.");
    }
}
