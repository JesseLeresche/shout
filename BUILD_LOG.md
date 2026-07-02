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

## 2026-07-02 — Ghost mode drafted during pause (NOT compiled or verified)

While paused (quiet file-writes only — no builds/launches/audio per Jesse's pause),
drafted the full Phase 4 pipeline: `audio/vad.rs` (Silero via sherpa-onnx, 16k),
`audio/mod.rs` streaming linear resampler (device rate → 16k, with unit tests),
`stt/whisper.rs` (whisper-rs 0.16), `pkm/obsidian.rs` (note writer per the spec schema,
with unit test), `ghost.rs` (toggle-hotkey session: continuous capture → VAD chunks
spooled to ~/.config/shout/sessions → batch Whisper → sherpa-onnx pyannote diarization
with single-speaker fallback → Ollama summarize with 300s timeout → Meetings/ note),
`llm/ollama.rs` summarize(), config additions, second global shortcut (alt+shift+g).

Mac loopback decision (spec left it open): BlackHole + Aggregate Device via
`ghost_input_device` config; no ScreenCaptureKit bridge. Documented in README/BLOCKERS.

Diarization: official sherpa-onnx crate ships pyannote segmentation + speaker embedding
+ clustering, so no Python sidecar needed (spec's fallback avoided).

**None of this has been compiled yet** — first `cargo check`, model downloads
(--ghost, ~3.6GB), and live verification wait for resume.

## 2026-07-02 — Phase 3 drafted during pause (NOT compiled or verified)

Same quiet-only constraint. Drafted: overlay pill (transparent always-on-top window,
shown by the Rust status emitter whenever state ≠ idle; requires tauri
`macos-private-api` feature), tray icon with ghost-toggle/show/quit menu (close-to-tray
on the main window), settings form in the main window backed by `get_config`/
`save_config` commands (writes ~/.config/shout/config.toml; hotkey changes need
restart), per-app style profiles (`[app_prompts]` in config, frontmost app looked up
via osascript at dictation time, appended to the cleanup system prompt), and
"scratch that" (normalized transcript match → backspaces equal to the previous
injection's char count, capped at 4000, on the main thread).

Resume checklist: `cargo check --tests` (new deps: whisper-rs — compiles whisper.cpp,
long first build; chrono), `cargo test`, `./scripts/download-models.sh --ghost`,
re-verify dictation E2E on idle machine, ghost E2E, then verifier subagents per phase.

## 2026-07-02 — Resumed; Phases 3–4 compile, Phase 2 measured

- First compile of the pause-drafted code: whisper.cpp built clean; **5 Rust errors
  total** (cpal 0.18 `Device::name` → `description().name()`; whisper-rs 0.16.0's
  released API differs from its README — `full_n_segments()` returns `c_int`, segment
  text via `get_segment(i).to_str()`). Fixed; `cargo check --tests` clean, **9/9 tests
  pass** (new: resampler ×2, obsidian note schema ×1).
- Ghost models downloaded to models/: silero_vad.onnx, pyannote segmentation 3.0,
  3dspeaker eres2net embedding, ggml-large-v3.bin (3.1GB). Script bug fixed en route
  (`$SEG…` — bash swallowed the unicode ellipsis into the variable name under `set -u`).
- Transient full DNS outage mid-resume (~2 min, even python resolution failed);
  waited it out with a background probe, then reran both jobs.

### Phase 2 latency evidence (quiet machine, local Ollama 0.24)

| model | cold load | warm (2 short cleanups) |
|---|---|---|
| qwen2.5:7b | ~12s | 701ms, 404ms |
| qwen2.5:3b | ~9s | 654ms, 286ms |

Cleanup quality identical on the test set (self-correction applied, fillers stripped).
Mitigation shipped: `keep_alive: "30m"` on every request + best-effort warm-up at app
startup, concurrent with the STT model load. Default stays qwen2.5:7b (quality
headroom; warm latency fine). Tailnet host hardening remains a server-side task for
Jesse's box (BLOCKERS.md) — client honors `SHOUT_OLLAMA_URL`/config for any host.
Startup warm-up observed working live: "shout: ollama model qwen2.5:7b warmed".

## 2026-07-02 — Ghost batch pipeline verified end-to-end (headless)

Found and fixed a real bug via the new E2E: sherpa's Silero VAD expects exactly
window_size (512-sample) frames per `accept_waveform` — feeding a whole buffer
returned ONE chunk for 15s of 3-utterance audio. `Vad` now windows internally.

Evidence (tests/ghost.rs, `cargo test` output): TTS fixtures (two macOS voices,
generated with `say -o` — no audio played) stitched with 1s gaps →
- VAD: 3 utterances detected
- Whisper Large V3: near-verbatim transcripts of all three sentences
- Diarization: **2 speakers found, correctly attributed [1,0,1]**
  (Samantha/Daniel/Samantha) — pure Rust, no Python sidecar
- Note written matching the ARCHITECTURE.md schema exactly (frontmatter, ## Summary,
  ## Transcript with `> **speaker_N** (MM:SS):` lines)

Full suite: **10/10 tests pass** (ghost E2E takes ~54s; loads the 3.1GB model).

Still needing a live, unlocked machine: dictation injection into a controlled target,
live ghost session via hotkey (mic path), and eyes-on Phase 3 UI checks (pill, tray,
settings, scratch-that). Screen locked while Jesse is away — global hotkeys and
synthetic keys are swallowed by the lock screen (IOConsoleLocked=Yes), which also
explains the one failed E2E attempt right after resume.

## 2026-07-02 — Phase 1 verifier subagent verdict: PASS (code-verifiable criteria)

Fresh-context subagent, instructed not to trust this log, independently: ran the full
test suite (all green), re-ran the Parakeet test with --nocapture and confirmed the
real transcript, traced every pipeline stage to file:line, and grepped the tree for
the privacy invariant (only network client is the Ollama one; no telemetry). Verdict:
**PASS on all code-verifiable criteria, no bugs found.** Open items it confirmed as
environment-limited, not failures: live injection E2E (needs Jesse — BLOCKERS.md),
Windows build (no Windows machine), tailnet latency. (It also caught that an earlier
"6/6 tests" line here went stale after Phases 3/4 added tests — current suite is 10/10
including the ghost E2E, which postdates its 9/9 count.)

## 2026-07-02 — Phases 2 & 3 verifier subagent verdict: PASS

Fresh-context subagent verified all Phase 2 and Phase 3 criteria: config layering
(default/toml/env, with unit tests), mock passthrough + fallback-to-raw, keep_alive +
startup warm-up, and **independent latency measurements** (its own python calls:
620ms–2s warm under concurrent-compile CPU contention — corroborates the quiet-machine
404–701ms). Phase 3: pill/tray/settings/per-app-profiles/scratch-that all traced to
code with no defects; settings JS field names verified against the Config serde names
(no mismatch, non-form fields preserved on save). `cargo check` clean. Remaining
eyes-on items match BLOCKERS.md's live checklist exactly. **No bugs found.**

## 2026-07-02 — Phase 4 verifier subagent verdict: PASS (all 8 criteria)

Fresh-context subagent independently traced the whole ghost pipeline to code, verified
the privacy invariant in ghost paths, and **ran the E2E itself** (84s, exit 0): 3 VAD
chunks from the 3-sentence fixture, near-verbatim Whisper transcripts, 2 speakers with
correct [1,0,1] attribution via real pyannote+3dspeaker diarization (not the
fallback), and a schema-exact note. Minor deviations it flagged are now addressed:
filename slug derived from transcript content (re-verified:
`2026-07-02-1618-good-morning-everyone-lets.md`, suite green at 98s) and the missing
"me" speaker label documented in BLOCKERS.md as a deliberate lean-scope limitation.
Its honest-gaps list (live mic toggle, BlackHole loopback, the 500ms worker drain
loop, live summarize round-trip) matches BLOCKERS.md's live checklist.

## 2026-07-02 — Phase 1 dictation E2E confirmed live by Jesse ("works like a charm")

Two live-test bugs found and fixed first:
1. **Silent mic**: system default input was "Steam Streaming Microphone" (virtual,
   Steam-installed) → 5s of digital zeros (peak 0.0000). Fix: `input_device` config
   pin (+ settings-form field + device-name logging); Jesse's config pins the
   built-in mic.
2. **UI stuck on "starting…"**: pipeline emitted "idle" before the webview listener
   attached; events aren't replayed. Fix: last status cached, UI pulls via
   `get_status` on load.

Then the full loop, user-at-keyboard, real mic, log evidence:
capturing from "MacBook Pro Microphone" (48kHz) → 3.72s captured, peak 0.956 →
Parakeet: "Hello World Quick Test to see if this is working." → Ollama cleanup:
"Hello, World. Quick test to see if this is working." → **injected 51 chars at the
cursor** — confirmed by Jesse: "works like a charm". **Phase 1 fully verified.**
