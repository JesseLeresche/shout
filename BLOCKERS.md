# Blockers — things that need Jesse

## Live checks needing you at an unlocked machine (~3 minutes total)
Everything below is code-complete and headlessly verified; these confirm the live
desktop legs (the lock screen swallows global hotkeys, so I can't automate them while
you're away). The dev app should be running (`npm run tauri dev` if not).
1. **Dictation**: click into any text field, hold `alt+space`, say a sentence,
   release → cleaned text should appear at your cursor within ~2s. If nothing
   appears: System Settings → Privacy & Security → Accessibility → enable your
   terminal (or shout.app).
2. **Scratch that**: immediately dictate the words "scratch that" → the previous
   dictation should be erased.
3. **Ghost mode**: press `alt+shift+g`, chat for ~30s, press again → a note appears
   in `~/Documents/ShoutVault/Meetings/` (pill shows "processing meeting…" while the
   3GB Whisper model loads+runs; allow a minute or two).
4. **Eyes-on UI**: pill appears bottom-center during activity; tray menu has
   ghost/show/quit; settings form saves to ~/.config/shout/config.toml.

## macOS permissions (needed for end-to-end dictation)
- **Microphone**: macOS will prompt on first recording — click Allow.
- **Accessibility**: System Settings → Privacy & Security → Accessibility → enable for
  the terminal you run `npm run tauri dev` from (or shout.app for a bundle). Without it
  the Cmd+V injection keystroke is silently dropped by macOS.

## Windows acceptance (Phase 1 says "Mac + Windows")
All crates chosen are cross-platform (cpal/WASAPI, enigo, arboard, global-shortcut),
but there is no Windows machine in this environment to build or test on. Needs a
Windows box: `npm install && npm run tauri dev`, then verify hotkey → dictation → paste.

## BlackHole install (Phase 4 system-audio loopback)
Decision (per ARCHITECTURE.md open question): BlackHole virtual device + macOS Aggregate
Device, selected via `ghost_input_device` config — no ScreenCaptureKit bridge. Installing
BlackHole needs admin: `brew install blackhole-2ch`, then create an Aggregate Device
(mic + BlackHole) in Audio MIDI Setup. Mic-only ghost capture works without any of this.

## Real tailnet Ollama host (Phase 2)
Config + `SHOUT_OLLAMA_URL` support any URL; local Ollama (localhost:11434, qwen2.5:7b)
is used for development. If you want the tailnet box wired in, provide its MagicDNS name.
