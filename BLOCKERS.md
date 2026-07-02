# Blockers — things that need Jesse

## ~~Live checks~~ — done 2026-07-02
Dictation and ghost mode both confirmed live by Jesse ("works like a charm" /
"There seems to all be working"); evidence in BUILD_LOG.md. Note: the system default
input was a silent Steam virtual mic — your config pins
`input_device = "MacBook Pro Microphone"`; change it in the settings window if you
switch mics.

## macOS permissions (needed for end-to-end dictation)
- **Microphone**: macOS will prompt on first recording — click Allow.
- **Accessibility**: System Settings → Privacy & Security → Accessibility → enable for
  the terminal you run `npm run tauri dev` from (or shout.app for a bundle). Without it
  the Cmd+V injection keystroke is silently dropped by macOS.

## Windows acceptance (Phase 1 says "Mac + Windows")
All crates chosen are cross-platform (cpal/WASAPI, enigo, arboard, global-shortcut),
but there is no Windows machine in this environment to build or test on. Needs a
Windows box: `npm install && npm run tauri dev`, then verify hotkey → dictation → paste.

## Known limitation: no "me" speaker label (Phase 4)
The spec's example note shows `speakers: [me, speaker_1, …]`. Identifying which
diarized cluster is *you* needs a voice-enrollment sample and embedding comparison —
deliberately out of scope for the lean build. All speakers are labeled speaker_N.
If you want it: record a ~10s enrollment clip and we add a cosine-similarity match
against the existing 3dspeaker embeddings.

## BlackHole install (Phase 4 system-audio loopback)
Decision (per ARCHITECTURE.md open question): BlackHole virtual device + macOS Aggregate
Device, selected via `ghost_input_device` config — no ScreenCaptureKit bridge. Installing
BlackHole needs admin: `brew install blackhole-2ch`, then create an Aggregate Device
(mic + BlackHole) in Audio MIDI Setup. Mic-only ghost capture works without any of this.

## Real tailnet Ollama host (Phase 2)
Config + `SHOUT_OLLAMA_URL` support any URL; local Ollama (localhost:11434, qwen2.5:7b)
is used for development. If you want the tailnet box wired in, provide its MagicDNS name.
