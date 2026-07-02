# TO-DO

Ranked by value-for-effort (state as of 2026-07-02; all four phases done and verified
— see BUILD_LOG.md).

## 1. Ship it as a real app (~30 min)
Dev mode dies with the terminal and rebuilds on every file save.
- `npm run tauri build` → `shout.app`
- Grant it Microphone + Accessibility once, add as a login item
- Flip the tray to hide the dock icon (`ActivationPolicy::Accessory`) so it's a
  proper background utility

## 2. BlackHole + aggregate device (~15 min, needs admin)
Ghost mode currently only hears *your* side of a meeting.
- `brew install blackhole-2ch`
- Audio MIDI Setup → create Aggregate Device (mic + BlackHole); route app output to a
  Multi-Output Device that includes BlackHole
- Set `ghost_input_device` to the aggregate's name
This is the spec's "fiddly bit" — pure user-side setup now; it unlocks real meeting
capture.

## 3. "Me" speaker enrollment (~1–2 hrs dev)
Record a ~10s enrollment clip once, embed with the 3dspeaker model already in
models/, cosine-match against diarized clusters → notes label `me` instead of
`speaker_N`. Big readability win for the PKM use case.

## 4. Ghost crash recovery (small)
Segments already spool to `~/.config/shout/sessions/` so a crash loses nothing — but
nothing yet notices orphaned session dirs on startup and offers to batch-process
them. Complete the durability story.

## 5. Index the vault with qmd (5 min)
The architecture chose Obsidian partly because it's "already searchable via qmd":
`qmd collection add ~/Documents/ShoutVault` (or your real vault path).

## Further out
- Windows validation (needs a Windows machine — see BLOCKERS.md)
- Per-app profile tuning after a week of real dictation (`[app_prompts]` in config)
- Point `SHOUT_OLLAMA_URL` at the tailnet box + server-side lockdown (Tailscale
  Serve, ACLs, never bind 0.0.0.0)
