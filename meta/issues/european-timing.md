# Run the European version at its own timing and sound

## Summary

The European release runs at PAL rates; its movement, animation and music
timing may differ from the Japanese measurements.

## Dependencies

- [Read every ROM address through a per-revision layout](revision-layout.md)

## Acceptance Criteria

- Frame rate, movement and music tempo match a native European recording.

## Progress

- Both hosts pace the European ROM at the PAL frame period. The measured
  bedroom `$0F→$10` load now stays dark for 14 frames in the portable world
  rather than reusing the Japanese 17; a headless real-ROM door test and a
  no-ROM revision test cover the change. Native movement and load durations
  for the rest of the route are not yet qualified.
- A separate fresh-child empty-SRAM native test replays the European input
  fixture through Elle's wake-up and compares every boundary of the first 62
  held-Right bedroom frames (including the initial anchor) against a portable
  `World` after its English pages. All 63 `(map,x,y)` states match with no
  frame shift; this qualifies that ordinary movement leg, not the rest of
  the route's load/movement timing or host real-time pacing.
- The app and web audio player now use a revision-aware NMI/port-script cadence:
  533 stereo samples and 17,067 settling SPC cycles per Japanese frame; 640
  samples and 20,480 cycles per European frame. This corrects track-change,
  fade polling and alternate-frame SFX timing at PAL's approximate 50 Hz
  without changing the region-independent SPC playback clock. A native
  capture shows why 640 remains a per-video-frame approximation: its 600 PAL
  frames contain 383,946 stereo frames, with individual counts of 639, 640 or
  641.
- A fresh-child empty-SRAM native test holds the first bedroom page at its
  fully typed `$D5` boundary, captures those 600 PAL frames at 32 kHz, and
  compares the resulting track 4 log-energy envelope with a 42-second direct
  render from the European ROM's driver and song data. Whole-capture
  acquisition scores 0.940; 1.5-second anchors near 1, 6 and 11 seconds score
  0.920/0.816/0.934, with 512 stereo frames of endpoint drift. A deterministic
  1%-slower rendering retains correlation but is rejected at 3,968 frames of
  drift; synthetic positive, unrelated-envelope and 1%-slower controls cover
  the matcher gates. This qualifies the steady bedroom track's **tempo**, not
  exact PCM/waveform phase, a common start boundary, host real-time audio
  delivery, port-event timing, fades, sound effects or the other route tracks.
