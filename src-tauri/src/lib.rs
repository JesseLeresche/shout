use std::sync::mpsc;

pub mod audio;
pub mod config;
pub mod ghost;
pub mod hotkey;
pub mod inject;
pub mod llm;
pub mod pipeline;
pub mod pkm;
pub mod stt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = config::Config::load();
    let dictation_hotkey = cfg.hotkey.clone();
    let ghost_hotkey = cfg.ghost_hotkey.clone();
    let (audio_tx, audio_rx) = mpsc::channel();
    let (pipe_tx, pipe_rx) = mpsc::channel();
    let (ghost_tx, ghost_rx) = mpsc::channel();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            pipeline::spawn(cfg.clone(), pipe_rx, handle.clone());
            audio::capture::spawn(audio_rx, pipe_tx, handle.clone());
            ghost::spawn(cfg, ghost_rx, handle);
            hotkey::register(app, &dictation_hotkey, &ghost_hotkey, audio_tx, ghost_tx)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running shout");
}
