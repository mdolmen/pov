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
        .invoke_handler(tauri::generate_handler![get_backend_port])
        .setup(|app| {
            // Spawn the Python backend and wait for the port line on stdout.
            let (mut rx, _child) = app
                .shell()
                .command("uv")
                .args(["--directory", BACKEND_DIR, "run", "python", "main.py"])
                .spawn()
                .expect("failed to spawn Python backend");

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
            // Will display selected tasks in a later phase.
            let show = MenuItem::with_id(app, "show", "Show pov", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => bring_to_front(app),
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
