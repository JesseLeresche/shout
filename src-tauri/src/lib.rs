use std::sync::mpsc;

pub mod audio;
pub mod config;
pub mod hotkey;
pub mod inject;
pub mod llm;
pub mod pipeline;
pub mod stt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = config::Config::load();
    let hotkey_str = cfg.hotkey.clone();
    let (audio_tx, audio_rx) = mpsc::channel();
    let (pipe_tx, pipe_rx) = mpsc::channel();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            pipeline::spawn(cfg, pipe_rx, handle.clone());
            audio::capture::spawn(audio_rx, pipe_tx, handle);
            hotkey::register(app, &hotkey_str, audio_tx)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running shout");
}
