//! Phase 1 integration tests. The STT test needs the Parakeet model in
//! ../models (run scripts/download-models.sh first).

use std::path::PathBuf;

use shout_lib::config::{Config, PARAKEET_DIR_NAME};
use shout_lib::llm::ollama;
use shout_lib::stt::parakeet::Parakeet;

fn model_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("models")
        .join(PARAKEET_DIR_NAME)
}

#[test]
fn parakeet_transcribes_bundled_test_wav() {
    let dir = model_dir();
    assert!(
        dir.exists(),
        "Parakeet model not found at {} — run scripts/download-models.sh",
        dir.display()
    );
    let parakeet = Parakeet::load(&dir).expect("load parakeet model");
    let text = parakeet
        .transcribe_file(&dir.join("test_wavs/0.wav"))
        .expect("transcribe test wav");
    println!("transcript: {text}");
    assert!(
        text.split_whitespace().count() >= 3,
        "expected a real transcript, got: {text:?}"
    );
}

#[test]
fn ollama_cleanup_mock_passthrough() {
    std::env::set_var("SHOUT_MOCK_LLM", "1");
    let cfg = Config::default();
    assert_eq!(ollama::cleanup(&cfg, "um hello world"), "um hello world");
    std::env::remove_var("SHOUT_MOCK_LLM");
}

#[test]
fn ollama_cleanup_falls_back_when_unreachable() {
    let cfg = Config {
        ollama_url: "http://127.0.0.1:1".into(),
        ..Default::default()
    };
    assert_eq!(ollama::cleanup(&cfg, "hello there"), "hello there");
}
