use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::AppHandle;

use crate::pipeline::{status, PipeJob};

pub enum AudioCmd {
    Start,
    StopAndProcess,
}

/// Spawn the audio worker thread. It owns the cpal stream (which is !Send, so
/// it must be created and dropped on one thread) and forwards finished
/// recordings to the pipeline.
pub fn spawn(rx: Receiver<AudioCmd>, pipe_tx: Sender<PipeJob>, app: AppHandle) {
    std::thread::spawn(move || run(rx, pipe_tx, app));
}

struct ActiveRecording {
    stream: cpal::Stream,
    buf: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

fn run(rx: Receiver<AudioCmd>, pipe_tx: Sender<PipeJob>, app: AppHandle) {
    let mut active: Option<ActiveRecording> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCmd::Start => {
                if active.is_some() {
                    continue;
                }
                match start_recording() {
                    Ok(rec) => {
                        eprintln!("shout: recording started ({} Hz)", rec.sample_rate);
                        status(&app, "recording", None);
                        active = Some(rec);
                    }
                    Err(e) => {
                        eprintln!("shout: mic error: {e:#}");
                        status(&app, "error", Some(format!("mic: {e:#}")));
                    }
                }
            }
            AudioCmd::StopAndProcess => {
                if let Some(rec) = active.take() {
                    drop(rec.stream);
                    let samples = std::mem::take(&mut *rec.buf.lock().unwrap());
                    eprintln!(
                        "shout: captured {:.2}s of audio",
                        samples.len() as f32 / rec.sample_rate as f32
                    );
                    let _ = pipe_tx.send(PipeJob {
                        samples,
                        sample_rate: rec.sample_rate,
                    });
                }
            }
        }
    }
}

fn start_recording() -> Result<ActiveRecording> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no input device available"))?;
    let supported = device
        .default_input_config()
        .context("no default input config")?;
    let sample_rate = supported.sample_rate();
    let channels = supported.channels() as usize;
    let buf = Arc::new(Mutex::new(Vec::<f32>::new()));
    let err_fn = |e| eprintln!("shout: audio stream error: {e}");

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let b = buf.clone();
            device.build_input_stream(
                supported.config(),
                move |data: &[f32], _| push_mono(&b, data, channels, |s| s),
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let b = buf.clone();
            device.build_input_stream(
                supported.config(),
                move |data: &[i16], _| push_mono(&b, data, channels, |s| s as f32 / 32768.0),
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let b = buf.clone();
            device.build_input_stream(
                supported.config(),
                move |data: &[u16], _| {
                    push_mono(&b, data, channels, |s| (s as f32 - 32768.0) / 32768.0)
                },
                err_fn,
                None,
            )?
        }
        f => return Err(anyhow!("unsupported sample format {f:?}")),
    };
    stream.play().context("start input stream")?;
    Ok(ActiveRecording {
        stream,
        buf,
        sample_rate,
    })
}

/// Take channel 0 of interleaved frames, converting to f32.
fn push_mono<T: Copy>(
    buf: &Arc<Mutex<Vec<f32>>>,
    data: &[T],
    channels: usize,
    conv: impl Fn(T) -> f32,
) {
    let mut b = buf.lock().unwrap();
    for frame in data.chunks(channels) {
        b.push(conv(frame[0]));
    }
}
