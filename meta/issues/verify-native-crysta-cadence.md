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
