use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use tauri::{AppHandle, Emitter};

use crate::config::{Config, PARAKEET_DIR_NAME};

enum Kind {
    File { dest: &'static str },
    Archive { dest_dir: &'static str },
}

type SentinelFn = fn(&Config) -> PathBuf;

struct ModelDef {
    id: &'static str,
    label: &'static str,
    category: &'static str,
    approx_mb: u32,
    url: &'static str,
    kind: Kind,
    sentinel: SentinelFn,
}

const REGISTRY: &[ModelDef] = &[
    ModelDef {
        id: "parakeet",
        label: "Parakeet (dictation)",
        category: "dictation",
        approx_mb: 480,
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2",
        kind: Kind::Archive { dest_dir: PARAKEET_DIR_NAME },
        sentinel: |cfg| cfg.parakeet_dir().join("tokens.txt"),
    },
    ModelDef {
        id: "whisper",
        label: "Whisper Large V3 (ghost)",
        category: "ghost",
        approx_mb: 3100,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        kind: Kind::File { dest: "ggml-large-v3.bin" },
        sentinel: |cfg| cfg.whisper_model_path(),
    },
    ModelDef {
        id: "silero_vad",
        label: "Silero VAD (ghost)",
        category: "ghost",
        approx_mb: 2,
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx",
        kind: Kind::File { dest: "silero_vad.onnx" },
        sentinel: |_cfg| Config::models_root().join("silero_vad.onnx"),
    },
    ModelDef {
        id: "diarize_seg",
        label: "Diarization segmentation (ghost)",
        category: "ghost",
        approx_mb: 6,
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
        kind: Kind::Archive { dest_dir: "sherpa-onnx-pyannote-segmentation-3-0" },
        sentinel: |_cfg| Config::models_root().join("sherpa-onnx-pyannote-segmentation-3-0/model.onnx"),
    },
    ModelDef {
        id: "diarize_emb",
        label: "Diarization embedding (ghost)",
        category: "ghost",
        approx_mb: 28,
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx",
        kind: Kind::File { dest: "3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx" },
        sentinel: |_cfg| Config::models_root().join("3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx"),
    },
];

/// Status of every known STT model, for the settings window's Models section.
pub fn status(cfg: &Config) -> Vec<serde_json::Value> {
    REGISTRY
        .iter()
        .map(|def| {
            serde_json::json!({
                "id": def.id,
                "label": def.label,
                "category": def.category,
                "approx_mb": def.approx_mb,
                "installed": (def.sentinel)(cfg).exists(),
            })
        })
        .collect()
}

/// Kick off a background download of the given model, reporting progress via
/// `shout:model-progress` events. Always writes to the default models root —
/// a custom `parakeet_model_dir`/`whisper_model` override is for pointing at
/// a model you already have, not a download destination.
pub fn download(id: &str, app: AppHandle) -> Result<(), String> {
    let def = REGISTRY
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| format!("unknown model id {id}"))?;

    std::thread::spawn(move || {
        let root = Config::models_root();
        if let Err(e) = std::fs::create_dir_all(&root) {
            emit_error(&app, def.id, &format!("create models dir: {e}"));
            return;
        }

        let download_target = match &def.kind {
            Kind::File { dest } => root.join(dest),
            Kind::Archive { dest_dir } => root.join(format!("{dest_dir}.tar.bz2")),
        };

        if let Err(e) = stream_download(&app, def.id, def.url, &download_target) {
            emit_error(&app, def.id, &e);
            return;
        }

        if matches!(def.kind, Kind::Archive { .. }) {
            let _ = app.emit(
                "shout:model-progress",
                serde_json::json!({"id": def.id, "phase": "extracting"}),
            );
            let result = Command::new("tar")
                .args(["xjf", download_target.to_string_lossy().as_ref()])
                .current_dir(&root)
                .status();
            let _ = std::fs::remove_file(&download_target);
            match result {
                Ok(s) if s.success() => {}
                Ok(s) => {
                    emit_error(&app, def.id, &format!("tar exited with {s}"));
                    return;
                }
                Err(e) => {
                    emit_error(&app, def.id, &format!("failed to run tar: {e}"));
                    return;
                }
            }
        }

        let _ = app.emit(
            "shout:model-progress",
            serde_json::json!({"id": def.id, "phase": "done"}),
        );
    });

    Ok(())
}

fn emit_error(app: &AppHandle, id: &str, message: &str) {
    eprintln!("shout: model download {id} failed: {message}");
    let _ = app.emit(
        "shout:model-progress",
        serde_json::json!({"id": id, "phase": "error", "message": message}),
    );
}

fn stream_download(
    app: &AppHandle,
    id: &str,
    url: &str,
    dest: &std::path::Path,
) -> Result<(), String> {
    let mut resp = reqwest::blocking::get(url).map_err(|e| format!("request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("server returned {}", resp.status()));
    }
    let total = resp.content_length();
    let mut file = std::fs::File::create(dest).map_err(|e| format!("create file: {e}"))?;
    let mut buf = [0u8; 65536];
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();
    loop {
        let n = resp.read(&mut buf).map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("write: {e}"))?;
        downloaded += n as u64;
        if last_emit.elapsed().as_millis() >= 250 {
            let _ = app.emit(
                "shout:model-progress",
                serde_json::json!({"id": id, "phase": "downloading", "downloaded": downloaded, "total": total}),
            );
            last_emit = Instant::now();
        }
    }
    let _ = app.emit(
        "shout:model-progress",
        serde_json::json!({"id": id, "phase": "downloading", "downloaded": downloaded, "total": total}),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_expected_models() {
        let ids: Vec<_> = REGISTRY.iter().map(|d| d.id).collect();
        assert_eq!(
            ids,
            vec!["parakeet", "whisper", "silero_vad", "diarize_seg", "diarize_emb"]
        );
    }

    #[test]
    fn status_matches_sentinel_paths() {
        let cfg = Config::default();
        let rows = status(&cfg);
        assert_eq!(rows.len(), REGISTRY.len());
        for (row, def) in rows.iter().zip(REGISTRY.iter()) {
            assert_eq!(row["id"], def.id);
            assert_eq!(row["installed"], (def.sentinel)(&cfg).exists());
        }
    }

}
