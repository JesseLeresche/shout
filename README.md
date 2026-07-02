# shout

Local, self-hosted voice dictation (a Wispr Flow clone). Hold a hotkey anywhere on your
desktop, speak, release — your words are transcribed locally, cleaned up by your own
Ollama server, and typed at the cursor. Audio and transcripts never leave your machine
except to the Ollama URL you configure. See `ARCHITECTURE.md` for the full design.

[![CI](https://github.com/JesseLeresche/shout/actions/workflows/ci.yml/badge.svg)](https://github.com/JesseLeresche/shout/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Download

Grab the latest `.dmg` from [Releases](https://github.com/JesseLeresche/shout/releases).
It's unsigned (no Apple Developer cert), so Gatekeeper will refuse to open it on first
launch — right-click the app and choose **Open**, or run:

```sh
xattr -cr /Applications/shout.app
```

The dictation model isn't bundled in the `.dmg` (it's ~480MB). One-time setup:

```sh
curl -fsSL -o download-models.sh \
  https://raw.githubusercontent.com/JesseLeresche/shout/main/scripts/download-models.sh
chmod +x download-models.sh
./download-models.sh
```

This installs to `~/.config/shout/models`, which is where the bundled app looks. Without
it, the hotkey will appear to do nothing — check Console.app for `shout:` log lines if
dictation isn't producing text.

Found a bug or have a feature idea? [Open an issue](https://github.com/JesseLeresche/shout/issues).

## Requirements

- Rust ([rustup.rs](https://rustup.rs)) + Node 20+ (`npm install` once for the Tauri CLI)
- Xcode Command Line Tools (`xcode-select --install`) — C/C++ toolchain for the bundled
  whisper.cpp/sherpa-onnx native builds
- `cmake` (builds the bundled sherpa-onnx STT library)
- STT models: `./scripts/download-models.sh` (lands in `./models` inside a checkout)
- Optional: an [Ollama](https://ollama.com) server for cleanup — pull the default model
  with `ollama pull qwen2.5:7b` (or set `ollama_model` in config to one you already have).
  Without Ollama running, raw transcripts are injected unchanged (set `SHOUT_MOCK_LLM=1`
  to skip the network call entirely).

## Run

```sh
npm install
npm run tauri dev
```

Hold **alt+space**, speak, release.

## Ghost mode (meeting capture)

Press **alt+shift+g** to start capturing; press again to stop. Speech (silence removed
by Silero VAD) is buffered to `~/.config/shout/sessions/`, then batch-transcribed with
Whisper Large V3, diarized, summarized via Ollama, and written as one markdown note to
`<vault>/Meetings/` (vault defaults to `~/Documents/ShoutVault`; set `vault_dir` to your
Obsidian vault).

Ghost models are separate (~3.6GB total): `./scripts/download-models.sh --ghost`.

**System audio (the other side of a call):** install
[BlackHole 2ch](https://github.com/ExistentialAudio/BlackHole), create an Aggregate
Device in Audio MIDI Setup combining your mic + BlackHole, route app output to a
Multi-Output Device that includes BlackHole, and set `ghost_input_device` to the
aggregate device's name. Without it, ghost mode captures your mic only.

### macOS permissions

- **Microphone** — prompted on first recording.
- **Accessibility** (System Settings → Privacy & Security → Accessibility) — required
  for the paste keystroke that inserts text at the cursor. Grant it to your terminal
  when running via `npm run tauri dev`, or to shout.app when running the bundle.

## Configuration

`~/.config/shout/config.toml` (all keys optional):

```toml
ollama_url = "http://localhost:11434"   # or your tailnet host
ollama_model = "qwen2.5:7b"             # dictation cleanup (small, fast)
ollama_summary_model = "qwen2.5:7b"     # ghost summaries (bigger is fine, batch)
hotkey = "alt+space"
# input_device = "MacBook Pro Microphone"  # pin if a virtual device (Steam/Teams) is the system default
ghost_hotkey = "alt+shift+g"
# vault_dir = "/path/to/ObsidianVault"
# ghost_input_device = "Shout Aggregate"  # mic+BlackHole aggregate for system audio
# parakeet_model_dir = "/path/to/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"
# whisper_model = "/path/to/ggml-large-v3.bin"
```

`SHOUT_OLLAMA_URL` overrides `ollama_url`.

Per-app cleanup styles (config only):

```toml
[app_prompts]
Slack = "casual tone, contractions are fine"
Mail = "professional email prose"
```

Say **"scratch that"** as its own dictation to erase the previous one.
The app lives in the tray; closing the window hides it.

While you hold the hotkey, the pill shows a **live partial transcript** (~750ms
cadence). With `live_typing = true` in config, raw partials are typed at your cursor
as you speak and corrected to the cleaned text on release — immediate, but the text
visibly rewrites itself and can misbehave in terminals/vim; the default (pill-only
streaming) never touches the target app until the final clean paste.

## Tests

```sh
cd src-tauri && cargo test
```

## License

MIT — see [LICENSE](LICENSE).
