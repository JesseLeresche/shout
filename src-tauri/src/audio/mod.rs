pub mod capture;
pub mod vad;

/// Magnitude of one frequency in a sample window (Goertzel — cheaper than an
/// FFT when only a handful of bands are needed).
fn goertzel(samples: &[f32], sample_rate: f32, freq: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0f32, 0f32);
    for &x in samples {
        let s0 = x + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0).sqrt() * 2.0 / samples.len() as f32
}

/// Levels for log-spaced speech-range bands (~120–3800 Hz), for the pill's
/// live spectrogram bars.
pub fn band_levels(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    const BANDS: usize = 12;
    let (lo, hi) = (120f32, 3800f32);
    let ratio = (hi / lo).powf(1.0 / (BANDS - 1) as f32);
    (0..BANDS)
        .map(|i| goertzel(samples, sample_rate as f32, lo * ratio.powi(i as i32)))
        .collect()
}

/// Streaming linear resampler (mono f32). Keeps continuity across chunks.
pub struct LinearResampler {
    step: f64,
    /// Fractional read position relative to the current chunk's first sample;
    /// -1.0 means "between prev and input[0]".
    pos: f64,
    prev: Option<f32>,
}

impl LinearResampler {
    pub fn new(src_rate: u32, dst_rate: u32) -> Self {
        Self {
            step: src_rate as f64 / dst_rate as f64,
            pos: 0.0,
            prev: None,
        }
    }

    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        if input.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::with_capacity((input.len() as f64 / self.step) as usize + 2);
        let mut pos = self.pos;
        while pos < input.len() as f64 - 1.0 {
            let sample = if pos < 0.0 {
                let s0 = self.prev.unwrap_or(input[0]);
                let frac = (pos + 1.0) as f32;
                s0 + (input[0] - s0) * frac
            } else {
                let i = pos as usize;
                let frac = (pos - i as f64) as f32;
                input[i] + (input[i + 1] - input[i]) * frac
            };
            out.push(sample);
            pos += self.step;
        }
        self.pos = pos - input.len() as f64;
        self.prev = input.last().copied();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_passthrough_at_equal_rates() {
        let mut r = LinearResampler::new(16000, 16000);
        let out = r.process(&[0.0, 1.0, 2.0, 3.0]);
        // one sample of latency at the chunk edge is fine; values must be linear
        assert!(out.len() >= 3);
        for (i, s) in out.iter().enumerate() {
            assert!((s - i as f32).abs() < 1e-4);
        }
    }

    #[test]
    fn resample_halves_rate() {
        let mut r = LinearResampler::new(48000, 16000);
        let input: Vec<f32> = (0..4800).map(|i| i as f32).collect();
        let out = r.process(&input);
        // 4800 samples at 48k ≈ 1600 at 16k
        assert!((out.len() as i64 - 1600).unsigned_abs() <= 2, "got {}", out.len());
    }
}
