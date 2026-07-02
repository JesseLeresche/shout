use std::path::Path;

use anyhow::{anyhow, Result};
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig, Wave};

/// NVIDIA Parakeet TDT (NeMo transducer) via sherpa-onnx — fast offline STT for dictation.
pub struct Parakeet {
    recognizer: OfflineRecognizer,
}

impl Parakeet {
    pub fn load(dir: &Path) -> Result<Self> {
        let need = |name: &str| -> Result<String> {
            let p = dir.join(name);
            if !p.exists() {
                return Err(anyhow!("missing model file {}", p.display()));
            }
            Ok(p.to_string_lossy().into_owned())
        };
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.transducer = OfflineTransducerModelConfig {
            encoder: Some(need("encoder.int8.onnx")?),
            decoder: Some(need("decoder.int8.onnx")?),
            joiner: Some(need("joiner.int8.onnx")?),
        };
        config.model_config.tokens = Some(need("tokens.txt")?);
        config.model_config.provider = Some("cpu".into());
        config.model_config.num_threads = 4;
        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            anyhow!("failed to create Parakeet recognizer from {}", dir.display())
        })?;
        Ok(Self { recognizer })
    }

    /// Transcribe mono f32 samples at any sample rate (sherpa-onnx resamples).
    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> String {
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(sample_rate as i32, samples);
        self.recognizer.decode(&stream);
        stream
            .get_result()
            .map(|r| r.text.trim().to_string())
            .unwrap_or_default()
    }

    pub fn transcribe_file(&self, path: &Path) -> Result<String> {
        let wave = Wave::read(&path.to_string_lossy())
            .ok_or_else(|| anyhow!("failed to read wav {}", path.display()))?;
        Ok(self.transcribe(wave.samples(), wave.sample_rate() as u32))
    }
}
