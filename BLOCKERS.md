# Blockers — things that need Jesse

Nothing hard-blocked yet. Items below are user actions the app will need at runtime;
development continues regardless.

## macOS permissions (needed for end-to-end dictation)
- **Microphone**: macOS will prompt on first recording — click Allow.
- **Accessibility**: System Settings → Privacy & Security → Accessibility → enable for
  the terminal you run `npm run tauri dev` from (or shout.app for a bundle). Without it
  the Cmd+V injection keystroke is silently dropped by macOS.

## Windows acceptance (Phase 1 says "Mac + Windows")
All crates chosen are cross-platform (cpal/WASAPI, enigo, arboard, global-shortcut),
but there is no Windows machine in this environment to build or test on. Needs a
Windows box: `npm install && npm run tauri dev`, then verify hotkey → dictation → paste.

## Real tailnet Ollama host (Phase 2)
Config + `SHOUT_OLLAMA_URL` support any URL; local Ollama (localhost:11434, qwen2.5:7b)
is used for development. If you want the tailnet box wired in, provide its MagicDNS name.
