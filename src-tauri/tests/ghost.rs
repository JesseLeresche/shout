//! Ghost-mode batch pipeline E2E, headless: TTS wav fixtures → Silero VAD →
//! Whisper Large V3 → diarization → Obsidian note. Requires the ghost models
//! (scripts/download-models.sh --ghost). Slow: loads the 3.1GB whisper model.

use std::path::PathBuf;

use shout_lib::audio::vad::Vad;
use shout_lib::config::Config;
use shout_lib::ghost::diarize_chunks;
use shout_lib::pkm::obsidian::{write_meeting_note, MeetingNote, TranscriptLine};
use shout_lib::stt::whisper::Whisper;

fn fixture(name: &str) -> Vec<f32> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let wave = sherpa_onnx::Wave::read(&path.to_string_lossy())
        .unwrap_or_else(|| panic!("read fixture {name}"));
    assert_eq!(wave.sample_rate(), 16000, "fixtures must be 16kHz");
    wave.samples().to_vec()
}

#[test]
fn ghost_batch_pipeline_end_to_end() {
    let models = Config::models_root();
    for needed in ["silero_vad.onnx", "ggml-large-v3.bin"] {
        assert!(
            models.join(needed).exists(),
            "missing {needed} — run scripts/download-models.sh --ghost"
        );
    }

    // Stitch: speaker A, silence, speaker B, silence, speaker A.
    let gap = vec![0.0f32; 16000]; // 1s silence
    let mut samples = Vec::new();
    for name in ["s1.wav", "s2.wav", "s3.wav"] {
        samples.extend(fixture(name));
        samples.extend(&gap);
    }

    // VAD should split this into (roughly) one chunk per sentence.
    let mut vad = Vad::load(&models.join("silero_vad.onnx")).expect("load VAD");
    let mut chunks = vad.feed(&samples);
    chunks.extend(vad.finish());
    println!("VAD chunks: {}", chunks.len());
    assert!(
        (2..=6).contains(&chunks.len()),
        "expected 2-6 utterances, got {}",
        chunks.len()
    );

    // Batch Whisper transcription.
    let whisper = Whisper::load(&models.join("ggml-large-v3.bin")).expect("load whisper");
    let texts: Vec<String> = chunks
        .iter()
        .map(|c| whisper.transcribe(&c.samples).expect("transcribe chunk"))
        .collect();
    let combined = texts.join(" ").to_lowercase();
    println!("transcripts: {texts:?}");
    for expected in ["quarterly", "marketing budget", "action item"] {
        assert!(
            combined.contains(expected),
            "transcript missing {expected:?}: {combined}"
        );
    }

    // Diarization: two distinct TTS voices; must label every chunk and
    // ideally find 2 speakers (soft assertion — clustering thresholds vary).
    let speakers = diarize_chunks(&chunks);
    assert_eq!(speakers.len(), chunks.len());
    let n_speakers = speakers.iter().max().unwrap() + 1;
    println!("speakers found: {n_speakers} ({speakers:?})");

    // Summarize via the mock path (offline-safe), as the pipeline would.
    std::env::set_var("SHOUT_MOCK_LLM", "1");
    let summary = shout_lib::llm::ollama::summarize(&Config::default(), &combined)
        .unwrap_or_else(|e| format!("*Summary unavailable ({e}).*"));
    std::env::remove_var("SHOUT_MOCK_LLM");
    assert!(!summary.is_empty());

    // Note writing per the ARCHITECTURE.md schema.
    let vault = std::env::temp_dir().join("shout-ghost-e2e-vault");
    let _ = std::fs::remove_dir_all(&vault);
    let note = MeetingNote {
        started_at: chrono::Local::now(),
        source: "fixture".into(),
        speakers: (1..=n_speakers).map(|k| format!("speaker_{k}")).collect(),
        duration_min: 1,
        summary,
        transcript: chunks
            .iter()
            .zip(&texts)
            .zip(&speakers)
            .map(|((c, t), s)| TranscriptLine {
                speaker: format!("speaker_{}", s + 1),
                at_secs: (c.start_sample / 16000) as u32,
                text: t.clone(),
            })
            .collect(),
    };
    let path = write_meeting_note(&vault, &note).expect("write note");
    let body = std::fs::read_to_string(&path).unwrap();
    println!("note at {}:\n{body}", path.display());
    assert!(body.contains("## Summary"));
    assert!(body.contains("## Transcript"));
    assert!(body.contains("tags: [meeting, ghost-capture]"));
    let _ = std::fs::remove_dir_all(&vault);
}
