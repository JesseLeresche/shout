use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::anyhow;
use tauri::{AppHandle, Emitter};

use crate::{config::Config, inject, llm::ollama, stt::parakeet::Parakeet};

pub struct PipeJob {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

/// Last emitted status, so a webview that loads late can pull it
/// (events emitted before the listener attaches are lost).
static LAST_STATUS: std::sync::Mutex<Option<(String, Option<String>)>> =
    std::sync::Mutex::new(None);

pub fn last_status() -> serde_json::Value {
    let guard = LAST_STATUS.lock().unwrap();
    let (state, detail) = guard
        .clone()
        .unwrap_or_else(|| ("starting".into(), None));
    serde_json::json!({ "state": state, "detail": detail })
}

pub fn status(app: &AppHandle, state: &str, detail: Option<String>) {
    *LAST_STATUS.lock().unwrap() = Some((state.to_string(), detail.clone()));
    let _ = app.emit(
        "shout:status",
        serde_json::json!({ "state": state, "detail": detail }),
    );
    // The overlay pill is visible whenever something is happening.
    use tauri::Manager;
    if let Some(pill) = app.get_webview_window("pill") {
        let _ = if state == "idle" {
            pill.hide()
        } else {
            pill.show()
        };
    }
}

/// Name of the frontmost app (the dictation target), used for per-app style
/// profiles. macOS only; returns None elsewhere or on failure.
fn frontmost_app() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to get name of first process whose frontmost is true",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!name.is_empty()).then_some(name)
    }
    #[cfg(not(target_os = "macos"))]
    None
}

/// Run injection on the main thread: enigo's key mapping uses macOS TIS APIs
/// that misbehave (hang) when called from other threads.
fn inject_on_main_thread(app: &AppHandle, text: &str) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel();
    let text = text.to_string();
    app.run_on_main_thread(move || {
        let _ = tx.send(inject::inject_text(&text));
    })
    .map_err(|e| anyhow!("dispatch to main thread: {e}"))?;
    rx.recv_timeout(Duration::from_secs(10))
        .map_err(|_| anyhow!("injection timed out after 10s (missing Accessibility permission?)"))?
}

/// Worker thread that owns the STT engine: mic samples in → cleaned text at cursor.
pub fn spawn(cfg: Config, rx: Receiver<PipeJob>, app: AppHandle) {
    std::thread::spawn(move || run(cfg, rx, app));
}

fn run(cfg: Config, rx: Receiver<PipeJob>, app: AppHandle) {
    // Warm Ollama concurrently with the STT model load.
    {
        let cfg = cfg.clone();
        std::thread::spawn(move || ollama::warm(&cfg));
    }
    status(&app, "loading-model", None);
    let parakeet = match Parakeet::load(&cfg.parakeet_dir()) {
        Ok(p) => {
            eprintln!("shout: Parakeet model loaded, dictation ready");
            p
        }
        Err(e) => {
            eprintln!("shout: FAILED to load Parakeet model: {e:#}");
            status(
                &app,
                "error",
                Some(format!(
                    "failed to load Parakeet model: {e:#} — run scripts/download-models.sh"
                )),
            );
            return;
        }
    };
    status(&app, "idle", None);

    // Chars of the last injected text, for "scratch that".
    let mut last_injected: Option<usize> = None;

    while let Ok(job) = rx.recv() {
        // Ignore accidental taps shorter than ~200ms of audio.
        if (job.samples.len() as f32) < job.sample_rate as f32 * 0.2 {
            status(&app, "idle", Some("recording too short".into()));
            continue;
        }
        let peak = job.samples.iter().fold(0f32, |m, s| m.max(s.abs()));
        eprintln!("shout: peak level {peak:.4}");
        if std::env::var("SHOUT_DEBUG_WAV").is_ok() {
            sherpa_onnx::write("/tmp/shout-last.wav", &job.samples, job.sample_rate as i32);
        }
        status(&app, "transcribing", None);
        let raw = parakeet.transcribe(&job.samples, job.sample_rate);
        eprintln!("shout: transcript: {raw:?}");
        if raw.is_empty() {
            status(&app, "idle", Some("heard nothing".into()));
            continue;
        }

        // "scratch that": erase the previous dictation instead of injecting.
        let normalized: String = raw
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .collect();
        if normalized.trim() == "scratch that" {
            match last_injected.take() {
                Some(n) => {
                    status(&app, "injecting", None);
                    match delete_on_main_thread(&app, n) {
                        Ok(()) => status(&app, "idle", Some("scratched".into())),
                        Err(e) => status(&app, "error", Some(format!("scratch failed: {e:#}"))),
                    }
                }
                None => status(&app, "idle", Some("nothing to scratch".into())),
            }
            continue;
        }

        let target_app = frontmost_app();
        status(&app, "cleaning", None);
        let cleaned = ollama::cleanup(&cfg, &raw, target_app.as_deref());
        eprintln!("shout: cleaned: {cleaned:?}");
        status(&app, "injecting", None);
        match inject_on_main_thread(&app, &cleaned) {
            Ok(()) => {
                eprintln!("shout: injected {} chars", cleaned.chars().count());
                last_injected = Some(cleaned.chars().count());
                let _ = app.emit(
                    "shout:result",
                    serde_json::json!({ "raw": raw, "cleaned": cleaned }),
                );
                status(&app, "idle", None);
            }
            Err(e) => status(&app, "error", Some(format!("inject failed: {e:#}"))),
        }
    }
}

fn delete_on_main_thread(app: &AppHandle, n: usize) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(inject::delete_chars(n));
    })
    .map_err(|e| anyhow!("dispatch to main thread: {e}"))?;
    rx.recv_timeout(Duration::from_secs(10))
        .map_err(|_| anyhow!("scratch timed out"))?
}
