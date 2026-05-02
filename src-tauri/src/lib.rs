use std::sync::OnceLock;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

static BACKEND_PORT: OnceLock<u16> = OnceLock::new();

// Path to the backend directory, baked in at compile time for dev.
// Phase 9 (packaging) replaces this with a sidecar binary.
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

/// Resolve the path of the `pov` CLI binary.
///
/// In dev that's the script uv created in `<backend>/.venv/bin/pov`. In a
/// packaged build (phase 9) we'll replace this with the bundled sidecar.
fn resolve_pov_binary() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(BACKEND_DIR);
    p.push(".venv/bin/pov");
    p
}

#[tauri::command]
fn install_cli() -> Result<String, String> {
    let src = resolve_pov_binary();
    if !src.exists() {
        return Err(format!(
            "pov CLI binary not found at {}. Run `uv sync` in backend/ first.",
            src.display()
        ));
    }
    let dst = std::path::PathBuf::from("/usr/local/bin/pov");

    // Replace any existing symlink or file at the target.
    if dst.exists() || dst.is_symlink() {
        std::fs::remove_file(&dst)
            .map_err(|e| format!("could not remove existing {}: {}", dst.display(), e))?;
    }

    std::os::unix::fs::symlink(&src, &dst).map_err(|e| {
        format!(
            "could not symlink {} → {}: {} (you may need to run `sudo chown $(whoami) /usr/local/bin` once)",
            dst.display(), src.display(), e
        )
    })?;
    Ok(format!("linked {} → {}", dst.display(), src.display()))
}

fn bring_to_front(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![get_backend_port, open_in_editor, install_cli])
        .setup(|app| {
            // Spawn the Python backend and wait for the port line on stdout.
            let mut cmd = app.shell().command("uv");
            cmd = cmd.args(["--directory", BACKEND_DIR, "run", "python", "main.py"]);
            #[cfg(debug_assertions)]
            { cmd = cmd.env("POV_ENV", "dev"); }
            let (mut rx, _child) = cmd.spawn().expect("failed to spawn Python backend");

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

            // Tray icon — left-click brings main window to front.
            let show = MenuItem::with_id(app, "show", "Show pov", true, None::<&str>)?;
            let install = MenuItem::with_id(app, "install_cli", "Install CLI", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &install, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => bring_to_front(app),
                    "install_cli" => {
                        let result = install_cli();
                        // Surface the result by reusing the dialog plugin.
                        use tauri_plugin_dialog::DialogExt;
                        let (title, body) = match result {
                            Ok(msg) => ("CLI installed", msg),
                            Err(msg) => ("CLI install failed", msg),
                        };
                        app.dialog().message(body).title(title).show(|_| {});
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        bring_to_front(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
