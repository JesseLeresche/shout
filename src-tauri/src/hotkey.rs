use std::sync::mpsc::Sender;

use anyhow::{anyhow, Context, Result};
use tauri::App;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::audio::capture::AudioCmd;
use crate::ghost::GhostCmd;

/// Register the global shortcuts: hold-to-talk dictation and ghost toggle.
pub fn register(
    app: &App,
    dictation_hotkey: &str,
    ghost_hotkey: &str,
    audio_tx: Sender<AudioCmd>,
    ghost_tx: Sender<GhostCmd>,
) -> Result<()> {
    let dictation: Shortcut = dictation_hotkey
        .parse()
        .map_err(|e| anyhow!("invalid hotkey '{dictation_hotkey}': {e}"))?;
    app.global_shortcut()
        .on_shortcut(dictation, move |_app, _sc, event| {
            let cmd = match event.state() {
                ShortcutState::Pressed => AudioCmd::Start,
                ShortcutState::Released => AudioCmd::StopAndProcess,
            };
            let _ = audio_tx.send(cmd);
        })
        .context("register dictation shortcut")?;

    let ghost: Shortcut = ghost_hotkey
        .parse()
        .map_err(|e| anyhow!("invalid ghost hotkey '{ghost_hotkey}': {e}"))?;
    app.global_shortcut()
        .on_shortcut(ghost, move |_app, _sc, event| {
            if event.state() == ShortcutState::Pressed {
                let _ = ghost_tx.send(GhostCmd::Toggle);
            }
        })
        .context("register ghost shortcut")?;
    Ok(())
}
