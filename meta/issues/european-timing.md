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
  fixture through Elle's wake-up and compares every boundary of two ordinary
  movement legs with a portable `World`: the first 62 held-Right bedroom
  frames (63 states including the anchor), then—after deliberately not
  comparing the doorway/load interval—the first 42 held-Down exterior frames
  from the shared `$10 (392,353)` landing (43 states). The second leg ends at
  `(392,413)` on both implementations with no frame shift. This qualifies
  those two movement legs, not the intervening load, later route movement,
  animation or host real-time pacing.
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
  exact PCM/waveform phase, a common start boundary or host real-time delivery.
- A bounded CPU-to-APU write capture now replays the native route while draining
  every frame without overflow. Route-local windows observe exact adjacent
  port-2/3 pairs for pot lift `$1100`, break `$1200`, door hit `$1300` and open
  `$001A`, plus the door reaction's `F1` fade and bounded `F0 → FF/upload → F4`
  patterns for its changes and the later spear windows. The portable route
  emits its source-derived track/fanfare cues and does not restart an unchanged
  track. This is command-pattern evidence, not PCM/DSP fidelity or native track
  identity for the spear windows: tracks `$34/$1C/$06` share stop parameter 5.
  It also exposes retained timing gaps rather than hiding them: portable door
  contact is four frames early and orders `$001A` before `$1200`, whereas native
  orders `$1200 → $001A → $1300`; the spear return remains 420 portable frames
  versus 405 native.
- A fresh European native witness source-pins the map-`$21` frozen-return
  player stream and every movement tick. Its 36-tick first leg applies +84 Y;
  its 16-tick second leg requests +15 but collision resolves +12 to Y=464.
  Portable now runs that same exact admitted profile through collision and
  releases at `(136,464)`, before separate manual Left/Up movement. This does
  not qualify other scripted-player movement or the route's remaining timing.
