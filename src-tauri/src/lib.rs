use tauri::Manager;

/// Runs a pov-core command (see `pov_core::api`). Off the UI thread: commands run git and SQLite.
#[tauri::command]
async fn api(
    paths: tauri::State<'_, pov_core::Paths>,
    cmd: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || pov_core::api::dispatch(&paths, &cmd, args))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Sends a task to its agent, starting the agent when it isn't answering, and delegates it once
/// the agent took it (see `pov_core::tools::delegate`). Otherwise the task stays as it was.
#[tauri::command]
async fn delegate(
    paths: tauri::State<'_, pov_core::Paths>,
    launcher: tauri::State<'_, pov_core::tools::Launcher>,
    project_id: String,
    task_hash: String,
    tool: String,
    note: Option<String>,
) -> Result<(), String> {
    let paths = paths.inner().clone();
    let launcher = launcher.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> pov_core::Result<()> {
        let conn = pov_core::db::open(&paths)?;
        pov_core::tools::delegate(
            &conn,
            &paths,
            &launcher,
            &project_id,
            &task_hash,
            &tool,
            note.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
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
/// Dev: the debug `pov-cli` build in the workspace target dir. Release: the binary bundled
/// alongside the app executable in Contents/MacOS/.
fn resolve_pov_binary() -> std::path::PathBuf {
    #[cfg(debug_assertions)]
    {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/pov-cli")
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
        let hint = if cfg!(debug_assertions) {
            "\n\nBuild it with `cargo build -p pov-core --features cli --bin pov-cli`."
        } else {
            ""
        };
        return Err(format!("pov binary not found at {}.{hint}", src.display()));
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
        format!(
            "could not symlink {} → {}: {}",
            dst.display(),
            src.display(),
            e
        )
    })?;
    #[cfg(not(debug_assertions))]
    std::fs::copy(&src, &dst).map_err(|e| {
        format!(
            "could not copy {} → {}: {}",
            src.display(),
            dst.display(),
            e
        )
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            api,
            delegate,
            open_in_editor,
            install_cli,
            is_cli_installed
        ])
        .setup(|app| {
            let dev = cfg!(debug_assertions) || std::env::var("POV_ENV").is_ok_and(|v| v == "dev");
            let paths = pov_core::Paths::data_dir(dev)?;
            pov_core::storage::init(&paths)?;
            pov_core::db::init(&paths)?;
            // Edits made while the app was closed: marks follow their shifted tasks.
            pov_core::tasks::sync_all(&pov_core::db::open(&paths)?, &paths)?;
            // Kept in app state: watching stops when the watcher is dropped.
            app.manage(pov_core::watcher::start(&paths)?);
            app.manage(paths);
            app.manage(pov_core::tools::Launcher::default());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
