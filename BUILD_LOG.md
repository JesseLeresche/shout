# shout build log

## 2026-07-02 — Phase 1 started

Environment verified (tool output in session): rustc 1.92.0, node v22.23.1, cmake
installed via brew, arm64 macOS. **Ollama running locally** at `localhost:11434`
(v0.24.0) with `qwen2.5:7b` available — matches the spec's dictation-cleanup model class.

Decisions/deviations from ARCHITECTURE.md:

- **`sherpa-rs` → official `sherpa-onnx` crate (1.13.x).** `sherpa-rs` README declares
  itself deprecated and points at the official Rust bindings maintained in the
  k2-fsa/sherpa-onnx repo. Same underlying library the spec locked; only the binding
  crate changed. The official crate ships a `nemo_parakeet.rs` example this code follows.
- **Vanilla JS frontend** (spec allowed "Svelte or vanilla") — no bundler, `frontendDist`
  points straight at `src/`.
- Added `pipeline.rs` (not in the proposed layout): the worker thread that owns the STT
  engine and runs transcribe → cleanup → inject. Audio capture stays in
  `audio/capture.rs`, PTT hotkey in `hotkey.rs`.

Model: `sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8` (482MB) downloaded from sherpa-onnx
release assets and extracted to `models/` (gitignored); includes a test WAV used by the
integration test.

Cleanup prompt validated against local Ollama (qwen2.5:7b) before wiring in: the
self-correction case "move the meeting to thursday no wait actually friday" came back
"…move the meeting to Friday…" — fillers stripped, punctuation fixed. Latency measured
45s/126s but is meaningless: the sherpa-onnx C++ build was saturating all cores at the
same time. Re-measure in Phase 2 on a quiet machine.

First `cargo check --tests`: sherpa-onnx static lib built successfully via cmake; 4
compile errors in my code from cpal 0.18 API drift (`sample_rate()` returns bare u32,
`build_input_stream` takes `StreamConfig` by value). Fixed.

### Phase 1 verification evidence (commit 4fe1b76)

- `cargo test`: 6/6 pass — config defaults/toml/env-override, Ollama mock passthrough,
  Ollama-unreachable → raw fallback, and Parakeet STT on the model's bundled test WAV
  (transcript: "Well, I don't wish to see it any more, observed Phebe…" — correct).
- Live app run (`npm run tauri dev`): hotkey Pressed/Released drove the PTT state
  machine; log evidence: "recording started (16000 Hz)" → "captured 6.42s of audio" →
  accurate transcript of real room speech → Ollama cleanup responded → "injected 65
  chars" with no hang.
- Two live bugs found & fixed: (1) model path resolution under `tauri dev` (CWD is
  src-tauri) — now uses CARGO_MANIFEST_DIR in dev builds; (2) injection hung when enigo
  ran off the main thread — now dispatched via `run_on_main_thread` with a 10s timeout.

**Open Phase 1 items:** (1) injection into a *controlled* target not confirmed — the
automated E2E ran while Jesse was actively using the machine, so the paste followed his
focus instead of the TextEdit target (side effect: ~65 stray chars may have landed in
his terminal; testing policy changed to never drive the desktop while the machine is in
use). (2) Windows build/test — no Windows machine here (see BLOCKERS.md).
(3) Fresh-context verifier subagent still to run. Session paused by Jesse mid-Phase-1
verification.
