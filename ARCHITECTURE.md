# shout — local, self-hosted voice dictation + ambient PKM capture

A local clone of Wispr Flow: system-wide push-to-talk dictation with AI cleanup, plus a
desktop-only "ghost mode" that transcribes meetings into an Obsidian knowledge base.
Fully local / self-hosted — no audio leaves your devices except to your own Ollama server
over a Tailscale tailnet.

> **Status:** all four phases below are implemented and shipping (see `BUILD_LOG.md`).
> This doc doubles as the original design record and a map of the current module layout.

## Guiding decisions (locked)

| Decision | Choice | Why |
|---|---|---|
| Desktop framework | **Tauri 2 (Rust core + web UI)** | Leanest; no Electron bloat; Rust does the native glue directly |
| Dictation STT | **NVIDIA Parakeet** via sherpa-onnx | Fast (RTFx >2000), ~7–8% WER, real-time feel |
| Ghost-mode STT | **Whisper Large V3** via whisper.cpp | Max accuracy; latency irrelevant (batch) |
| LLM cleanup/summarize | **Ollama** (OpenAI-compatible) on a tailnet box | One always-on host; every device points at it |
| PKM target | **Obsidian markdown vault** | Local-first; already searchable via `qmd` |
| Ghost mode scope | **Desktop only, batch** | Meeting capture → PKM, not real-time |

## Two independent pipelines, shared plumbing

```
                    ┌──────────────── shared ────────────────┐
                    │  Ollama client (reqwest → tailnet)      │
                    │  Config (toml)  •  Obsidian writer      │
                    │  STT layer (sherpa-onnx / whisper.cpp)  │
                    └─────────────────────────────────────────┘

DICTATION (all desktop OSes, latency-critical)
  hotkey down → mic capture (cpal) → hotkey up
    → Parakeet STT → Ollama cleanup (style/format) → inject at cursor

GHOST MODE (desktop only, batch, accuracy-first)
  continuous capture (mic + system loopback)
    → Silero VAD (drop silence) → segment buffer to disk
    → [batch] Whisper Large V3 + diarization
    → Ollama summarize (action items, topics)
    → write Obsidian note (one per meeting)
```

The two only touch via shared modules, so Phase 1 (dictation) ships before ghost mode exists.

## Rust crate selection for the hard native parts

| Need | Crate / approach | Notes |
|---|---|---|
| Global hotkey (works unfocused) | `tauri-plugin-global-shortcut` | First-class in Tauri 2 |
| Text injection at cursor | **clipboard-paste** primary, `enigo` fallback | Set clipboard → send Cmd/Ctrl+V → restore clipboard is more reliable for long/Unicode text than synthesizing every keystroke |
| Mic capture | `cpal` | Cross-platform (CoreAudio / WASAPI / ALSA) |
| System audio loopback (meetings) | Win: `cpal` WASAPI loopback · Mac: ScreenCaptureKit or BlackHole virtual device | Mac loopback is the fiddly bit — see open questions |
| VAD | `voice_activity_detector` (Silero v5 ONNX) | Gate ghost-mode transcription |
| STT — Parakeet & Whisper | `sherpa-rs` (sherpa-onnx bindings) + `whisper-rs` (whisper.cpp) | sherpa-onnx runs Parakeet; whisper-rs for accurate batch |
| Diarization (who spoke) | sherpa-onnx speaker diarization (pyannote seg + embedding ONNX), **or Python sidecar** | ⚠️ Weakest link in pure-Rust; pyannote-quality may need a sidecar |
| LLM call | `reqwest` → `http://<tailnet-host>:11434/v1/chat/completions` | Ollama's OpenAI-compatible API |
| Tray icon + settings window | Tauri 2 built-in tray + a web view | UI is small: overlay pill + settings |
| Config | `serde` + `toml` | ~/.config/shout/config.toml |

## Repo layout

```
shout/
├── src-tauri/               # Rust core
│   ├── src/
│   │   ├── main.rs
│   │   ├── hotkey.rs        # global shortcut registration + PTT state machine
│   │   ├── audio/
│   │   │   ├── capture.rs   # cpal mic + loopback
│   │   │   └── vad.rs       # Silero gating
│   │   ├── stt/
│   │   │   ├── parakeet.rs  # sherpa-onnx (dictation)
│   │   │   └── whisper.rs   # whisper.cpp (ghost mode)
│   │   ├── llm/ollama.rs    # cleanup + summarize prompts
│   │   ├── inject.rs        # clipboard-paste / enigo
│   │   ├── pkm/obsidian.rs  # markdown note writer
│   │   ├── ghost.rs         # batch pipeline orchestration
│   │   ├── pipeline.rs      # dictation pipeline orchestration
│   │   └── config.rs
│   └── tauri.conf.json
├── src/                     # web UI (overlay pill + settings) — vanilla JS/HTML
├── models/                  # downloaded ONNX / ggml models
└── ARCHITECTURE.md
```

## Ollama server (the one tailnet box)

- Host on the most capable always-on machine; expose **only** over the tailnet
  (Tailscale Serve + MagicDNS + ACLs). Never bind Ollama to `0.0.0.0` publicly.
- Two model roles:
  - **Dictation cleanup** — small fast instruct (Llama 3.x 8B / Qwen 7–8B), sub-second.
  - **Ghost summarize** — bigger (14–32B) OK since batch.
- Sizing: one box with ~16GB+ VRAM (4070Ti/4080-class) or an M-series Mac w/ 32GB+
  unified memory covers both roles + Whisper Large V3.

## Obsidian note schema (ghost mode)

One note per meeting: `Meetings/YYYY-MM-DD-HHmm-<slug>.md`

```markdown
---
date: 2026-07-02T14:30
source: <app or "room mic">
speakers: [me, speaker_1, speaker_2]
duration_min: 47
tags: [meeting, ghost-capture]
---
## Summary
<LLM-generated: TL;DR + action items + decisions>

## Transcript
> **speaker_1** (00:02): ...
> **me** (00:15): ...
```

## Open questions / known risks

- **Mac system-audio loopback** is the fiddliest native piece — ScreenCaptureKit audio
  vs. a BlackHole virtual device. Decide during Phase 4.
- **Pure-Rust diarization quality** may not match pyannote; a Python sidecar for ghost
  mode is an acceptable fallback (batch, so latency is fine).
- **Tailnet round-trip latency** for dictation cleanup vs. Wispr's 200ms network budget —
  measure early; LAN/tailnet should be fine.
- **Legal:** meeting capture of others hits two-party-consent laws in some jurisdictions.
  Local-first, encrypted at rest, explicit toggle.
- **Mobile** (iOS keyboard extension / Android IME) is a separate native effort, out of
  scope for the Tauri desktop core; optional future Flutter companion for PKM browsing.

## Phased build

1. **Dictation MVP** — hotkey → mic → Parakeet → Ollama cleanup → inject. Mac + Windows.
2. **Ollama/Tailscale hardening** — lock down host, measure latency, tune models.
3. **Polish** — overlay pill UI, tray, settings, per-app style profiles, "scratch that".
4. **Ghost mode** — loopback capture + VAD + batch Whisper + diarization → Obsidian.
