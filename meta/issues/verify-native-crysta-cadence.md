# Verify native Crysta movement and animation cadence

## Summary

The user confirmed the tree/river fixes work, but reports NPC movement appears
too fast and asks what movement/animation speed should actually be.

## Requirements

- Separate host simulation updates per real second from NPC movement/animation
  durations per original-game tick; do not choose a slowdown by eye.
- Establish the reference clock and trace the relevant actor velocity/wait/pose
  timing using source and bounded native observations where available.
- Measure the native host cadence and correct demonstrated discrepancies with
  regression coverage, preserving deterministic headless scripts and audio timing.
- Record what is verified and what remains approximate; independent review before
  substantial implementation commits.

## Acceptance Criteria

- Document expected versus actual rates, including at least one wandering NPC.
- Any correction has red/green regression coverage and relevant tests/lints pass.
- Timing evidence and logs remain local; no raw ROM assets are committed.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Related: [Walking residents](crysta-walking-residents.md).
- Initial source observations: native `about_to_wait` advances unconditionally
  under `ControlFlow::Poll`; actor runtime explicitly approximates both tile
  movement and waits as eight frames. These are separate possible speed errors.

## Host correction

- Confirmed134.6 updates/second in the original native event loop, against nominal
  Japanese NTSC60.098814 Hz. Fixed host pacing with monotonic deadlines,
  bounded4-step catch-up, suspension rebase and latched keyboard interaction.
- New native measurement:480 updates /7.9874 seconds =60.0945 Hz. Per-frame
  monotonic elapsed time and timing policy are now logged. Audio and deterministic
  headless stepping are unchanged. Clock/input tests pass red→green; existing
  Session regressions and strict native Clippy pass. Independent review approved.
- [Timing contract and remaining NPC questions](../../docs/native-crysta-timing.md).
- Issue remains open while source/native NPC per-tick timing is investigated;
  no arbitrary global NPC slowdown has been applied.

## Resident raster countdown

- Confirmed a separate off-by-one in resident raster playback: raw duration7
  was displayed for7 ticks rather than8. `$80:ED99..EDA4` stores the raw byte at
  actor+$0E; `$80:C72C..C731` decrements and resumes only when negative.
- Corrected the raster player's hold length to duration+1, preserving the existing
  looping presentation policy rather than claiming full actor script timing.
  A four-record duration7 list now takes32 ticks. Zero lasts1 tick (a one-record
  list still looks held), and255 lasts256 without overflow.
- Two pure regressions failed before correction and pass afterward; seven
  owned-ROM art tests and strict runtime Clippy pass. Translation/wait timing
  remains a separate, active investigation.
- Independent raster-cadence review confirmed the generic16-bit actor countdown
  and approved with no blockers. Root formatting, strict workspace Clippy and
  release workspace tests pass after the correction.
