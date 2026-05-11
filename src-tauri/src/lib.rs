use std::sync::OnceLock;

use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

static BACKEND_PORT: OnceLock<u16> = OnceLock::new();

#[cfg(debug_assertions)]
const BACKEND_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../backend");

#[tauri::command]
fn get_backend_port() -> u16 {
    *BACKEND_PORT.get().expect("backend port not set")
}

#[tauri::command]
fn open_in_editor(path: String) {
    // -t opens the file in the default text editor registered with Launch Services.
    std::process::Command::new("open")
        .args(["-t", &path])
        .spawn()
        .ok();
}

/// Path to the `pov` CLI binary.
///
/// Dev: the uv-managed venv script. Release: the PyInstaller binary bundled
/// alongside the app executable in Contents/MacOS/.
fn resolve_pov_binary() -> std::path::PathBuf {
    #[cfg(debug_assertions)]
    {
        std::path::PathBuf::from(BACKEND_DIR).join(".venv/bin/pov")
    }
    #[cfg(not(debug_assertions))]
    {
        std::env::current_exe()
            .expect("could not get current exe")
            .parent()
            .expect("exe has no parent dir")
            .join("pov-cli")
    }
}

fn install_dst() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".local/bin/pov")
}

#[tauri::command]
fn is_cli_installed() -> bool {
    let dst = install_dst();
    dst.exists() || dst.is_symlink()
}

#[tauri::command]
fn install_cli() -> Result<String, String> {
    let src = resolve_pov_binary();
    if !src.exists() {
        return Err(format!(
            "pov binary not found at {}.",
            src.display()
        ));
    }
    let dst = install_dst();

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {}", parent.display(), e))?;
    }

    if dst.exists() || dst.is_symlink() {
        std::fs::remove_file(&dst)
            .map_err(|e| format!("could not remove existing {}: {}", dst.display(), e))?;
    }

    // Dev: symlink (source changes are picked up automatically).
    // Release: copy (user can move the .app without breaking the CLI).
    #[cfg(debug_assertions)]
    std::os::unix::fs::symlink(&src, &dst).map_err(|e| {
        format!("could not symlink {} → {}: {}", dst.display(), src.display(), e)
    })?;
    #[cfg(not(debug_assertions))]
    std::fs::copy(&src, &dst).map_err(|e| {
        format!("could not copy {} → {}: {}", src.display(), dst.display(), e)
    })?;

    Ok(format!(
        "Installed pov to {}\n\nMake sure ~/.local/bin is on your PATH.",
        dst.display()
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![get_backend_port, open_in_editor, install_cli, is_cli_installed])
        .setup(|app| {
            // Spawn the Python backend and wait for it to print its port.
            #[cfg(debug_assertions)]
            let backend_cmd = app.shell()
                .command("uv")
                .args(["--directory", BACKEND_DIR, "run", "python", "main.py"])
                .env("POV_ENV", "dev");

            #[cfg(not(debug_assertions))]
            let backend_cmd = app.shell()
                .sidecar("pov-backend")
                .expect("pov-backend sidecar not configured");

            let (mut rx, _child) = backend_cmd.spawn().expect("failed to spawn backend");

            tauri::async_runtime::block_on(async {
                while let Some(event) = rx.recv().await {
                    if let CommandEvent::Stdout(line) = event {
                        if let Ok(text) = String::from_utf8(line) {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(port) = val["port"].as_u64() {
                                    BACKEND_PORT.set(port as u16).ok();
                                    break;
                                }
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
