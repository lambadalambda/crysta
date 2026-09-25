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
  for the rest of the route, plus native sound-tempo comparison, are not yet
  qualified. The SPC driver/song data comparison is not itself a tempo test.
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
  without changing the region-independent SPC playback clock. The 640-sample
  approximation and its native PCM/tempo parity remain unqualified.
