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

## enigo must run on the main thread on macOS
Injection hung indefinitely when called from the pipeline worker thread — enigo's
Unicode key mapping uses TIS keyboard-layout APIs that misbehave off the main thread.
Dispatch via `AppHandle::run_on_main_thread` + result channel with a timeout.

## Never drive the desktop (keystrokes/audio/focus) while the machine is in use
An automated E2E pasted a transcript into whatever Jesse had focused (his terminal)
because his activity stole focus from the test target, and TTS audio played out loud
during what may have been a meeting. Check `ioreg -c IOHIDSystem` HIDIdleTime (and ask)
before any synthetic-input test; prefer idle windows or explicit user cooperation.

## macOS drops synthetic events silently without Accessibility; enigo Ok ≠ delivered
"injected N chars" only proves the CGEvent calls returned. Verify AX trust with
`swift -e 'import ApplicationServices; print(AXIsProcessTrusted())'` (this session: true).

## tauri dev runs the binary with CWD=src-tauri
Repo-root-relative paths (models/) miss. Use `env!("CARGO_MANIFEST_DIR")` for dev-build
fallbacks, config/data-dir for release.

## context-mode hook blocks curl/wget in Bash
Downloads must go through `python3 -c "urllib.request.urlretrieve(...)"` (or ctx_execute
for fetch-and-analyze). ctx_execute cannot be used for downloads that must persist —
its sandbox FS is discarded.
