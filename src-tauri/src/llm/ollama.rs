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
    match request(
        cfg,
        &cfg.ollama_model,
        SYSTEM_PROMPT,
        raw,
        Duration::from_secs(15),
    ) {
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

const SUMMARY_PROMPT: &str = "You summarize meeting transcripts into markdown for a personal \
knowledge base. Produce: a 2-4 sentence TL;DR paragraph, then '### Action items' as bullets \
(with owner when identifiable), then '### Decisions' as bullets. Only include things actually \
present in the transcript; write 'none' under a heading when there are none. Output only the \
markdown, no preamble.";

/// Summarize a meeting transcript via Ollama (batch: generous timeout, no
/// silent fallback — the caller decides what to write when this fails).
pub fn summarize(cfg: &Config, transcript: &str) -> Result<String> {
    if std::env::var("SHOUT_MOCK_LLM").is_ok() {
        return Ok("*Summary skipped (SHOUT_MOCK_LLM set).*".into());
    }
    let text = request(
        cfg,
        &cfg.ollama_summary_model,
        SUMMARY_PROMPT,
        transcript,
        Duration::from_secs(300),
    )?;
    if text.trim().is_empty() {
        return Err(anyhow!("empty summary from model"));
    }
    Ok(text.trim().to_string())
}

fn request(
    cfg: &Config,
    model: &str,
    system: &str,
    user: &str,
    timeout: Duration,
) -> Result<String> {
    let url = format!(
        "{}/v1/chat/completions",
        cfg.ollama_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
        "temperature": 0.2,
        "stream": false,
    });
    let resp = client()
        .post(&url)
        .timeout(timeout)
        .json(&body)
        .send()?
        .error_for_status()?;
    let v: serde_json::Value = resp.json()?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("unexpected response shape from {url}"))
}
