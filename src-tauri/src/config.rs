use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PARAKEET_DIR_NAME: &str = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Base URL of the Ollama server (OpenAI-compatible API).
    pub ollama_url: String,
    /// Model used for dictation cleanup.
    pub ollama_model: String,
    /// Push-to-talk shortcut, e.g. "alt+space".
    pub hotkey: String,
    /// Directory holding the Parakeet ONNX model files. When unset, falls back
    /// to ./models (dev checkout) then ~/.config/shout/models.
    pub parakeet_model_dir: Option<PathBuf>,
    /// Ghost-mode toggle shortcut.
    pub ghost_hotkey: String,
    /// Input device for ghost mode by name (e.g. an Aggregate Device that
    /// combines the mic with a BlackHole loopback). Default input when unset.
    pub ghost_input_device: Option<String>,
    /// Obsidian vault root; meeting notes go to <vault>/Meetings/.
    pub vault_dir: Option<PathBuf>,
    /// Model used for ghost-mode summarization (batch, can be bigger).
    pub ollama_summary_model: String,
    /// Path to the Whisper ggml model file. Defaults under the models dir.
    pub whisper_model: Option<PathBuf>,
    /// Per-app style instructions for cleanup, keyed by app name
    /// (e.g. Slack = "casual tone, contractions fine").
    pub app_prompts: std::collections::HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "qwen2.5:7b".into(),
            hotkey: "alt+space".into(),
            parakeet_model_dir: None,
            ghost_hotkey: "alt+shift+g".into(),
            ghost_input_device: None,
            vault_dir: None,
            ollama_summary_model: "qwen2.5:7b".into(),
            whisper_model: None,
            app_prompts: Default::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".config/shout/config.toml"))
    }

    /// Load from ~/.config/shout/config.toml (if present), then apply env overrides.
    pub fn load() -> Self {
        let mut cfg = Self::config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| Self::from_toml_str(&s))
            .unwrap_or_default();
        cfg.apply_env();
        cfg
    }

    fn from_toml_str(s: &str) -> Self {
        toml::from_str(s)
            .map_err(|e| eprintln!("shout: invalid config.toml, using defaults: {e}"))
            .unwrap_or_default()
    }

    fn apply_env(&mut self) {
        if let Ok(url) = std::env::var("SHOUT_OLLAMA_URL") {
            if !url.is_empty() {
                self.ollama_url = url;
            }
        }
    }

    /// Root directory for models: repo models/ (dev builds) > ~/.config/shout/models.
    pub fn models_root() -> PathBuf {
        #[cfg(debug_assertions)]
        {
            let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("manifest dir has parent")
                .join("models");
            if dev.exists() {
                return dev;
            }
        }
        dirs::home_dir()
            .map(|h| h.join(".config/shout/models"))
            .unwrap_or_else(|| PathBuf::from("models"))
    }

    pub fn parakeet_dir(&self) -> PathBuf {
        self.parakeet_model_dir
            .clone()
            .unwrap_or_else(|| Self::models_root().join(PARAKEET_DIR_NAME))
    }

    pub fn whisper_model_path(&self) -> PathBuf {
        self.whisper_model
            .clone()
            .unwrap_or_else(|| Self::models_root().join("ggml-large-v3.bin"))
    }

    /// Where meeting notes are written; defaults to ~/Documents/ShoutVault.
    pub fn vault_dir(&self) -> PathBuf {
        self.vault_dir.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join("Documents/ShoutVault"))
                .unwrap_or_else(|| PathBuf::from("ShoutVault"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
        assert_eq!(cfg.hotkey, "alt+space");
        assert!(cfg.parakeet_model_dir.is_none());
    }

    #[test]
    fn parses_partial_toml() {
        let cfg = Config::from_toml_str("ollama_model = \"llama3.1:8b\"");
        assert_eq!(cfg.ollama_model, "llama3.1:8b");
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
    }

    #[test]
    fn env_overrides_ollama_url() {
        std::env::set_var("SHOUT_OLLAMA_URL", "http://tailnet-box:11434");
        let mut cfg = Config::default();
        cfg.apply_env();
        std::env::remove_var("SHOUT_OLLAMA_URL");
        assert_eq!(cfg.ollama_url, "http://tailnet-box:11434");
    }
}
