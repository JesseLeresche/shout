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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "qwen2.5:7b".into(),
            hotkey: "alt+space".into(),
            parakeet_model_dir: None,
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

    /// Where the Parakeet model lives: explicit config > repo models/ (dev builds)
    /// > ~/.config/shout/models.
    pub fn parakeet_dir(&self) -> PathBuf {
        if let Some(d) = &self.parakeet_model_dir {
            return d.clone();
        }
        #[cfg(debug_assertions)]
        {
            let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("manifest dir has parent")
                .join("models")
                .join(PARAKEET_DIR_NAME);
            if dev.exists() {
                return dev;
            }
        }
        dirs::home_dir()
            .map(|h| h.join(".config/shout/models").join(PARAKEET_DIR_NAME))
            .unwrap_or_else(|| PathBuf::from("models").join(PARAKEET_DIR_NAME))
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
