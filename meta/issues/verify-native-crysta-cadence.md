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
