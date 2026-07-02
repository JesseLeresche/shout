use std::path::Path;

use anyhow::{anyhow, Result};
use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};

/// One VAD-detected utterance. `start_sample` is the offset in the session's
/// 16 kHz timeline.
pub struct SpeechChunk {
    pub start_sample: i64,
    pub samples: Vec<f32>,
}

/// Silero VAD gate: feed 16 kHz mono audio, get back speech-only chunks.
pub struct Vad {
    vad: VoiceActivityDetector,
    /// Samples not yet forming a full window; sherpa's VAD expects
    /// window_size-sample frames per accept_waveform call.
    pending: Vec<f32>,
}

pub const VAD_SAMPLE_RATE: u32 = 16000;
const WINDOW: usize = 512;

impl Vad {
    pub fn load(model: &Path) -> Result<Self> {
        if !model.exists() {
            return Err(anyhow!("missing VAD model {}", model.display()));
        }
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(model.to_string_lossy().into_owned()),
                threshold: 0.5,
                min_silence_duration: 0.5,
                min_speech_duration: 0.25,
                window_size: WINDOW as i32,
                max_speech_duration: 20.0,
            },
            sample_rate: VAD_SAMPLE_RATE as i32,
            num_threads: 1,
            ..Default::default()
        };
        let vad = VoiceActivityDetector::create(&config, 120.0)
            .ok_or_else(|| anyhow!("failed to create Silero VAD from {}", model.display()))?;
        Ok(Self {
            vad,
            pending: Vec::new(),
        })
    }

    pub fn feed(&mut self, samples: &[f32]) -> Vec<SpeechChunk> {
        self.pending.extend_from_slice(samples);
        let mut out = Vec::new();
        let mut offset = 0;
        while self.pending.len() - offset >= WINDOW {
            self.vad.accept_waveform(&self.pending[offset..offset + WINDOW]);
            out.extend(self.drain());
            offset += WINDOW;
        }
        self.pending.drain(..offset);
        out
    }

    /// Flush trailing speech at end of session.
    pub fn finish(&mut self) -> Vec<SpeechChunk> {
        if !self.pending.is_empty() {
            let rest = std::mem::take(&mut self.pending);
            self.vad.accept_waveform(&rest);
        }
        self.vad.flush();
        self.drain()
    }

    fn drain(&mut self) -> Vec<SpeechChunk> {
        let mut out = Vec::new();
        while !self.vad.is_empty() {
            if let Some(seg) = self.vad.front() {
                out.push(SpeechChunk {
                    start_sample: seg.start() as i64,
                    samples: seg.samples().to_vec(),
                });
            }
            self.vad.pop();
        }
        out
    }
}
