# Lessons

One entry per lesson; one-line summary first.

## sherpa-rs is deprecated — use the official `sherpa-onnx` crate
The spec's locked crate (`sherpa-rs`) declares itself deprecated in its README; upstream
k2-fsa/sherpa-onnx publishes official Rust bindings (crate `sherpa-onnx`, 1.13.x) with
maintained examples (`rust-api-examples/examples/nemo_parakeet.rs`). Mattered because
building on an unmaintained binding would have been a dead end. API quirks: `create()`
returns `Option`, `Wave::read()` returns `Option`, `accept_waveform(i32, &[f32])`.

## cpal streams are !Send — give capture its own thread
`cpal::Stream` can't cross threads, and the global-shortcut handler must be
`Send + Sync`. Owning the stream inside a dedicated audio thread driven by an mpsc
channel (Start / StopAndProcess) sidesteps the whole class of Send/Sync errors.

## context-mode hook blocks curl/wget in Bash
Downloads must go through `python3 -c "urllib.request.urlretrieve(...)"` (or ctx_execute
for fetch-and-analyze). ctx_execute cannot be used for downloads that must persist —
its sandbox FS is discarded.
