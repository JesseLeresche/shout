use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::config::Config;

const SYSTEM_PROMPT: &str = "You are a dictation cleanup engine. Rewrite the user's raw speech \
transcript into clean written text: fix punctuation, capitalization, and obvious transcription \
errors; remove filler words (um, uh, you know) and false starts; apply the speaker's \
self-corrections. Preserve the meaning, tone, and language of the original. Output ONLY the \
cleaned text — no commentary, no quotation marks around it.";

/// Clean up a raw transcript via Ollama. Falls back to the raw text on any
/// failure so dictation never blocks on the LLM being reachable.
/// SHOUT_MOCK_LLM=1 skips the network entirely (offline passthrough).
pub fn cleanup(cfg: &Config, raw: &str) -> String {
    if std::env::var("SHOUT_MOCK_LLM").is_ok() {
        return raw.to_string();
    }
    match request(cfg, raw) {
        Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
        Ok(_) => raw.to_string(),
        Err(e) => {
            eprintln!("shout: ollama cleanup failed ({e:#}); using raw transcript");
            raw.to_string()
        }
    }
}

fn client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("http client")
    })
}

fn request(cfg: &Config, raw: &str) -> Result<String> {
    let url = format!(
        "{}/v1/chat/completions",
        cfg.ollama_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": cfg.ollama_model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": raw },
        ],
        "temperature": 0.2,
        "stream": false,
    });
    let resp = client().post(&url).json(&body).send()?.error_for_status()?;
    let v: serde_json::Value = resp.json()?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("unexpected response shape from {url}"))
}
