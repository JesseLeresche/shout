use std::path::Path;

use anyhow::{anyhow, Context, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Whisper Large V3 via whisper.cpp — accuracy-first batch STT for ghost mode.
pub struct Whisper {
    ctx: WhisperContext,
}

impl Whisper {
    pub fn load(model: &Path) -> Result<Self> {
        if !model.exists() {
            return Err(anyhow!(
                "missing Whisper model {} — run scripts/download-models.sh --ghost",
                model.display()
            ));
        }
        let ctx = WhisperContext::new_with_params(model, WhisperContextParameters::default())
            .context("load whisper model")?;
        Ok(Self { ctx })
    }

    /// Transcribe 16 kHz mono f32 samples.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        let mut state = self.ctx.create_state().context("create whisper state")?;
        state.full(params, samples).context("run whisper")?;
        let mut text = String::new();
        for i in 0..state.full_n_segments() {
            if let Some(seg) = state.get_segment(i) {
                if let Ok(s) = seg.to_str() {
                    text.push_str(s.trim());
                    text.push(' ');
                }
            }
        }
        Ok(text.trim().to_string())
    }
}
