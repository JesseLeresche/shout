use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::anyhow;
use tauri::{AppHandle, Emitter};

use crate::{config::Config, inject, llm::ollama, stt::parakeet::Parakeet};

pub enum PipeJob {
    /// Snapshot of an in-progress recording, for streaming partial transcripts.
    Partial { samples: Vec<f32>, sample_rate: u32 },
    /// The finished recording: transcribe, clean up, inject.
    Final { samples: Vec<f32>, sample_rate: u32 },
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
    // What live-typing mode has typed so far for the current utterance.
    let mut live_typed = String::new();

    while let Ok(mut job) = rx.recv() {
        // Partials can pile up while a decode runs; keep only the newest,
        // and never skip past a Final.
        while matches!(job, PipeJob::Partial { .. }) {
            match rx.try_recv() {
                Ok(newer) => job = newer,
                Err(_) => break,
            }
        }

        match job {
            PipeJob::Partial {
                samples,
                sample_rate,
            } => {
                if (samples.len() as f32) < sample_rate as f32 * 0.5 {
                    continue;
                }
                let text = parakeet.transcribe(&samples, sample_rate);
                if text.is_empty() {
                    continue;
                }
                let _ = app.emit("shout:partial", serde_json::json!({ "text": text }));
                if cfg.live_typing {
                    if let Err(e) = live_replace(&app, &mut live_typed, &text) {
                        eprintln!("shout: live typing failed: {e:#}");
                    }
                }
            }
            PipeJob::Final {
                samples,
                sample_rate,
            } => {
                let _ = app.emit("shout:partial", serde_json::json!({ "text": null }));
                // Ignore accidental taps shorter than ~200ms of audio.
                if (samples.len() as f32) < sample_rate as f32 * 0.2 {
                    status(&app, "idle", Some("recording too short".into()));
                    continue;
                }
                let peak = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
                eprintln!("shout: peak level {peak:.4}");
                if std::env::var("SHOUT_DEBUG_WAV").is_ok() {
                    sherpa_onnx::write("/tmp/shout-last.wav", &samples, sample_rate as i32);
                }
                status(&app, "transcribing", None);
                let raw = parakeet.transcribe(&samples, sample_rate);
                eprintln!("shout: transcript: {raw:?}");
                if raw.is_empty() {
                    // Erase any live-typed partials that came from noise.
                    let _ = live_replace(&app, &mut live_typed, "");
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
                    // First remove the live-typed words "scratch that" themselves.
                    let _ = live_replace(&app, &mut live_typed, "");
                    match last_injected.take() {
                        Some(n) => {
                            status(&app, "injecting", None);
                            match delete_on_main_thread(&app, n) {
                                Ok(()) => status(&app, "idle", Some("scratched".into())),
                                Err(e) => {
                                    status(&app, "error", Some(format!("scratch failed: {e:#}")))
                                }
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
                let result = if cfg.live_typing {
                    // Correct the streamed raw text into the cleaned version.
                    live_replace(&app, &mut live_typed, &cleaned)
                } else {
                    inject_on_main_thread(&app, &cleaned)
                };
                live_typed.clear();
                match result {
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
    }
}

/// Live-typing: morph what's already at the cursor (`current`) into `target`
/// by erasing past the common prefix and typing the difference.
fn live_replace(app: &AppHandle, current: &mut String, target: &str) -> anyhow::Result<()> {
    if current == target {
        return Ok(());
    }
    let prefix = current
        .chars()
        .zip(target.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let backspaces = current.chars().count() - prefix;
    let addition: String = target.chars().skip(prefix).collect();
    if backspaces == 0 && addition.is_empty() {
        return Ok(());
    }
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let result = inject::delete_chars(backspaces)
            .and_then(|_| inject::type_text_at_cursor(&addition));
        let _ = tx.send(result);
    })
    .map_err(|e| anyhow!("dispatch to main thread: {e}"))?;
    rx.recv_timeout(Duration::from_secs(10))
        .map_err(|_| anyhow!("live typing timed out"))??;
    *current = target.to_string();
    Ok(())
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
