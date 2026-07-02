# shout

Local, self-hosted voice dictation (a Wispr Flow clone). Hold a hotkey anywhere on your
desktop, speak, release — your words are transcribed locally, cleaned up by your own
Ollama server, and typed at the cursor. Audio and transcripts never leave your machine
except to the Ollama URL you configure. See `ARCHITECTURE.md` for the full design.

## Requirements

- Rust + Node 20+ (`npm install` once for the Tauri CLI)
- `cmake` (builds the bundled sherpa-onnx STT library)
- STT models: `./scripts/download-models.sh`
- Optional: an [Ollama](https://ollama.com) server for cleanup — without it, raw
  transcripts are injected unchanged (set `SHOUT_MOCK_LLM=1` to skip the network call
  entirely).

## Run

```sh
npm install
npm run tauri dev
```

Hold **alt+space**, speak, release.

### macOS permissions

- **Microphone** — prompted on first recording.
- **Accessibility** (System Settings → Privacy & Security → Accessibility) — required
  for the paste keystroke that inserts text at the cursor. Grant it to your terminal
  when running via `npm run tauri dev`, or to shout.app when running the bundle.

## Configuration

`~/.config/shout/config.toml` (all keys optional):

```toml
ollama_url = "http://localhost:11434"   # or your tailnet host
ollama_model = "qwen2.5:7b"
hotkey = "alt+space"
# parakeet_model_dir = "/path/to/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"
```

`SHOUT_OLLAMA_URL` overrides `ollama_url`.

## Tests

```sh
cd src-tauri && cargo test
```
