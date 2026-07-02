use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use tauri::{AppHandle, Emitter};

use crate::audio::capture::{start_recording_on, ActiveRecording};
use crate::audio::vad::{SpeechChunk, Vad, VAD_SAMPLE_RATE};
use crate::audio::LinearResampler;
use crate::config::Config;
use crate::llm::ollama;
use crate::pipeline::status;
use crate::pkm::obsidian::{self, MeetingNote, TranscriptLine};
use crate::stt::whisper::Whisper;

pub enum GhostCmd {
    Toggle,
}

/// Ghost mode worker: continuous capture → Silero VAD (drop silence) →
/// segments buffered to disk → on stop: batch Whisper + diarization →
/// Ollama summary → one Obsidian note per meeting.
pub fn spawn(cfg: Config, rx: Receiver<GhostCmd>, app: AppHandle) {
    std::thread::spawn(move || run(cfg, rx, app));
}

struct GhostSession {
    rec: ActiveRecording,
    resampler: LinearResampler,
    vad: Vad,
    chunks: Vec<SpeechChunk>,
    started_at: DateTime<Local>,
    spool_dir: PathBuf,
}

fn run(cfg: Config, rx: Receiver<GhostCmd>, app: AppHandle) {
    let mut session: Option<GhostSession> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(GhostCmd::Toggle) => {
                if let Some(s) = session.take() {
                    status(&app, "ghost-processing", None);
                    match finish_session(&cfg, s) {
                        Ok(path) => {
                            eprintln!("shout: ghost note written: {}", path.display());
                            let _ = app.emit(
                                "shout:ghost-note",
                                serde_json::json!({ "path": path.to_string_lossy() }),
                            );
                            status(&app, "idle", Some(format!("note: {}", path.display())));
                        }
                        Err(e) => {
                            eprintln!("shout: ghost processing failed: {e:#}");
                            status(&app, "error", Some(format!("ghost: {e:#}")));
                        }
                    }
                } else {
                    match start_session(&cfg) {
                        Ok(s) => {
                            eprintln!("shout: ghost capture started");
                            status(&app, "ghost-recording", None);
                            session = Some(s);
                        }
                        Err(e) => {
                            eprintln!("shout: ghost start failed: {e:#}");
                            status(&app, "error", Some(format!("ghost: {e:#}")));
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some(s) = &mut session {
                    let raw = s.rec.drain();
                    let resampled = s.resampler.process(&raw);
                    let new = s.vad.feed(&resampled);
                    spool_chunks(&s.spool_dir, &mut s.chunks, new);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Ghost input: explicit ghost device (e.g. mic+BlackHole aggregate) >
/// the dictation mic > system default.
fn ghost_device(cfg: &Config) -> Option<&str> {
    cfg.ghost_input_device
        .as_deref()
        .or(cfg.input_device.as_deref())
}

fn start_session(cfg: &Config) -> Result<GhostSession> {
    let vad_model = Config::models_root().join("silero_vad.onnx");
    let vad = Vad::load(&vad_model)?;
    let rec = start_recording_on(ghost_device(cfg))?;
    let started_at = Local::now();
    let spool_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".config/shout/sessions")
        .join(started_at.format("%Y-%m-%d-%H%M%S").to_string());
    std::fs::create_dir_all(&spool_dir).context("create session spool dir")?;
    let resampler = LinearResampler::new(rec.sample_rate(), VAD_SAMPLE_RATE);
    Ok(GhostSession {
        rec,
        resampler,
        vad,
        chunks: Vec::new(),
        started_at,
        spool_dir,
    })
}

/// Spool each utterance to disk so a crash loses at most the tail.
fn spool_chunks(spool_dir: &Path, chunks: &mut Vec<SpeechChunk>, new: Vec<SpeechChunk>) {
    for chunk in new {
        let path = spool_dir.join(format!("seg{:04}.wav", chunks.len()));
        sherpa_onnx::write(
            &path.to_string_lossy(),
            &chunk.samples,
            VAD_SAMPLE_RATE as i32,
        );
        chunks.push(chunk);
    }
}

fn finish_session(cfg: &Config, s: GhostSession) -> Result<PathBuf> {
    let GhostSession {
        rec,
        mut resampler,
        mut vad,
        mut chunks,
        started_at,
        spool_dir,
    } = s;

    // Stop the stream and flush the tail through resampler + VAD.
    let tail = rec.stop();
    let resampled = resampler.process(&tail);
    let fed = vad.feed(&resampled);
    spool_chunks(&spool_dir, &mut chunks, fed);
    let flushed = vad.finish();
    spool_chunks(&spool_dir, &mut chunks, flushed);

    let duration_min =
        ((Local::now() - started_at).num_seconds().max(0) as f32 / 60.0).round() as u32;

    if chunks.is_empty() {
        anyhow::bail!("no speech detected in this session");
    }
    eprintln!("shout: ghost processing {} speech segments", chunks.len());

    // Batch STT: Whisper Large V3, loaded per session so RAM is freed after.
    let whisper = Whisper::load(&cfg.whisper_model_path())?;
    let mut lines: Vec<(i64, usize, String)> = Vec::new(); // (start_sample, chunk_idx, text)
    for (i, chunk) in chunks.iter().enumerate() {
        let text = whisper.transcribe(&chunk.samples)?;
        if !text.is_empty() {
            lines.push((chunk.start_sample, i, text));
        }
    }
    drop(whisper);

    // Diarize concatenated speech-only audio; map speakers back per chunk.
    let speaker_of = diarize_chunks(&chunks);
    let num_speakers = speaker_of.iter().copied().max().unwrap_or(0) + 1;
    let speaker_names: Vec<String> = (1..=num_speakers).map(|k| format!("speaker_{k}")).collect();

    let transcript: Vec<TranscriptLine> = lines
        .iter()
        .map(|(start, idx, text)| TranscriptLine {
            speaker: format!("speaker_{}", speaker_of.get(*idx).copied().unwrap_or(0) + 1),
            at_secs: (*start as f64 / VAD_SAMPLE_RATE as f64) as u32,
            text: text.clone(),
        })
        .collect();

    let transcript_text: String = transcript
        .iter()
        .map(|l| format!("{}: {}", l.speaker, l.text))
        .collect::<Vec<_>>()
        .join("\n");
    let summary = ollama::summarize(cfg, &transcript_text)
        .unwrap_or_else(|e| format!("*Summary unavailable ({e}).*"));

    let note = MeetingNote {
        started_at,
        source: ghost_device(cfg).unwrap_or("default mic").to_string(),
        speakers: speaker_names,
        duration_min,
        summary,
        transcript,
    };
    let path = obsidian::write_meeting_note(&cfg.vault_dir(), &note)?;
    // Note written — the spooled wavs are no longer needed.
    let _ = std::fs::remove_dir_all(&spool_dir);
    Ok(path)
}

/// Returns a 0-based speaker index per chunk. Falls back to a single speaker
/// when diarization models are absent or diarization fails.
pub fn diarize_chunks(chunks: &[SpeechChunk]) -> Vec<usize> {
    use sherpa_onnx::{
        FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
        OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
        SpeakerEmbeddingExtractorConfig,
    };

    let root = Config::models_root();
    let seg_model = root.join("sherpa-onnx-pyannote-segmentation-3-0/model.onnx");
    let emb_model = root.join("3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx");
    let fallback = vec![0; chunks.len()];
    if !seg_model.exists() || !emb_model.exists() {
        eprintln!("shout: diarization models missing, labeling single speaker");
        return fallback;
    }

    let config = OfflineSpeakerDiarizationConfig {
        segmentation: OfflineSpeakerSegmentationModelConfig {
            pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                model: Some(seg_model.to_string_lossy().into_owned()),
            },
            num_threads: 2,
            ..Default::default()
        },
        embedding: SpeakerEmbeddingExtractorConfig {
            model: Some(emb_model.to_string_lossy().into_owned()),
            num_threads: 2,
            ..Default::default()
        },
        clustering: FastClusteringConfig {
            num_clusters: -1, // auto
            threshold: 0.5,
        },
        min_duration_on: 0.3,
        min_duration_off: 0.5,
    };
    let Some(sd) = OfflineSpeakerDiarization::create(&config) else {
        eprintln!("shout: diarization init failed, labeling single speaker");
        return fallback;
    };

    // Concatenate speech-only chunks, remembering each chunk's range.
    let mut concat = Vec::new();
    let mut ranges = Vec::with_capacity(chunks.len());
    for c in chunks {
        let start = concat.len();
        concat.extend_from_slice(&c.samples);
        ranges.push((start as f32, concat.len() as f32));
    }
    let Some(result) = sd.process(&concat) else {
        eprintln!("shout: diarization failed, labeling single speaker");
        return fallback;
    };
    let segments = result.sort_by_start_time();
    let sr = VAD_SAMPLE_RATE as f32;

    // Majority-overlap speaker per chunk.
    ranges
        .iter()
        .map(|(start, end)| {
            let (cs, ce) = (start / sr, end / sr);
            let mut best = (0usize, 0.0f32);
            for seg in &segments {
                let overlap = (seg.end.min(ce) - seg.start.max(cs)).max(0.0);
                if overlap > best.1 {
                    best = (seg.speaker as usize, overlap);
                }
            }
            best.0
        })
        .collect()
}
