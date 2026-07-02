
## 2026-07-02 — Post-phase polish (live with Jesse)

Iterated interactively: streaming partial transcripts (pill live-caption by default,
opt-in live typing at the cursor via stable-prefix diff, settings toggle hot-applied
through an atomic), pill placement fixes found by live testing (cursor's monitor via
physical-pixel containment, work-area-relative so it clears the Dock,
visibleOnAllWorkspaces so it floats over other apps' Spaces), and a live spectrogram
in the pill (12 Goertzel bands at 20fps with auto-gain). All confirmed working by
Jesse in use.
