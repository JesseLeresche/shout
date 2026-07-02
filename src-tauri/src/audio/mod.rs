pub mod capture;
pub mod vad;

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
