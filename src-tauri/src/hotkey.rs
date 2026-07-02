use std::sync::mpsc::Sender;

use anyhow::{anyhow, Context, Result};
use tauri::App;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::audio::capture::AudioCmd;

/// Register the push-to-talk shortcut: hold to record, release to process.
pub fn register(app: &App, hotkey: &str, audio_tx: Sender<AudioCmd>) -> Result<()> {
    let shortcut: Shortcut = hotkey
        .parse()
        .map_err(|e| anyhow!("invalid hotkey '{hotkey}': {e}"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _sc, event| {
            let cmd = match event.state() {
                ShortcutState::Pressed => AudioCmd::Start,
                ShortcutState::Released => AudioCmd::StopAndProcess,
            };
            let _ = audio_tx.send(cmd);
        })
        .context("register global shortcut")?;
    Ok(())
}
