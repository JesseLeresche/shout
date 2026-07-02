use anyhow::{Context, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Insert text at the cursor of the frontmost app. Clipboard-paste primary
/// (reliable for long/Unicode text), direct typing as fallback.
pub fn inject_text(text: &str) -> Result<()> {
    match paste_via_clipboard(text) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("shout: clipboard paste failed ({e:#}); falling back to typing");
            type_text_at_cursor(text)
        }
    }
}

fn paste_via_clipboard(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new().context("open clipboard")?;
    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(text.to_string())
        .context("set clipboard")?;
    std::thread::sleep(std::time::Duration::from_millis(80));

    let mut enigo = Enigo::new(&Settings::default()).context("init enigo")?;
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;
    enigo
        .key(modifier, Direction::Press)
        .context("press paste modifier")?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .context("press V")?;
    enigo
        .key(modifier, Direction::Release)
        .context("release paste modifier")?;

    // Give the target app time to read the clipboard before restoring it.
    std::thread::sleep(std::time::Duration::from_millis(200));
    if let Some(prev) = previous {
        let _ = clipboard.set_text(prev);
    }
    Ok(())
}

/// Type text directly at the cursor (used by live-typing mode and as the
/// clipboard-paste fallback).
pub fn type_text_at_cursor(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    let mut enigo = Enigo::new(&Settings::default()).context("init enigo")?;
    enigo.text(text).context("type text")?;
    Ok(())
}

/// Erase the last `n` characters at the cursor ("scratch that", live-typing).
pub fn delete_chars(n: usize) -> Result<()> {
    if n == 0 {
        return Ok(());
    }
    let mut enigo = Enigo::new(&Settings::default()).context("init enigo")?;
    for _ in 0..n.min(4000) {
        enigo
            .key(Key::Backspace, Direction::Click)
            .context("backspace")?;
    }
    Ok(())
}
