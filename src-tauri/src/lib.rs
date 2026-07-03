use std::sync::mpsc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

pub mod audio;
pub mod config;
pub mod ghost;
pub mod hotkey;
pub mod inject;
pub mod llm;
pub mod models;
pub mod pipeline;
pub mod pkm;
pub mod stt;

#[tauri::command]
fn get_config() -> config::Config {
    config::Config::load()
}

#[tauri::command]
fn get_status() -> serde_json::Value {
    pipeline::last_status()
}

#[tauri::command]
fn list_models() -> Vec<serde_json::Value> {
    models::status(&config::Config::load())
}

#[tauri::command]
fn download_model(id: String, app: tauri::AppHandle) -> Result<(), String> {
    models::download(&id, app)
}

#[tauri::command]
fn save_config(cfg: config::Config) -> Result<(), String> {
    let path = config::Config::config_path().ok_or("no home directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let toml = toml::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, toml).map_err(|e| e.to_string())?;
    // The streaming-mode toggle applies immediately; other changes on restart.
    pipeline::LIVE_TYPING.store(cfg.live_typing, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = config::Config::load();
    let dictation_hotkey = cfg.hotkey.clone();
    let ghost_hotkey = cfg.ghost_hotkey.clone();
    let (audio_tx, audio_rx) = mpsc::channel();
    let (pipe_tx, pipe_rx) = mpsc::channel();
    let (ghost_tx, ghost_rx) = mpsc::channel();
    let ghost_tx_tray = ghost_tx.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_status,
            list_models,
            download_model
        ])
        .on_window_event(|window, event| {
            // Closing the settings window hides to tray; quit via tray menu.
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            pipeline::spawn(cfg.clone(), pipe_rx, handle.clone());
            audio::capture::spawn(cfg.input_device.clone(), audio_rx, pipe_tx, handle.clone());
            ghost::spawn(cfg, ghost_rx, handle);
            hotkey::register(app, &dictation_hotkey, &ghost_hotkey, audio_tx, ghost_tx)?;

            // Tray: ghost toggle + show settings + quit.
            let ghost_i =
                MenuItem::with_id(app, "ghost", "Start/stop ghost capture", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show shout", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit shout", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&ghost_i, &show_i, &quit_i])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("window icon").clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "ghost" => {
                        let _ = ghost_tx_tray.send(ghost::GhostCmd::Toggle);
                    }
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // The pill is positioned on each show (pipeline::status) on
            // whichever monitor holds the cursor.
            if app.get_webview_window("pill").is_none() {
                eprintln!("shout: WARNING pill window was not created");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running shout");
}
