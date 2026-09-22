# Fix native Crysta outdoor stalls

## Summary

The user reports pre-existing crashes or hangs while running outside, including
on the exterior path between trees near the pond. The window remains visible in
the supplied screenshot. The user clarified that the window stays open and the
music continues, but gameplay stops responding. Distinguish an event-loop stall
from an input/movement state lock rather than assuming a process crash.

## Dependencies

- [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)

## Requirements

- Reproduce the outdoor failure with a bounded deterministic input/position case.
- Distinguish collision refusal, input/state lock, actor/render work and process failure.
- Fix the demonstrated cause without silently admitting unqualified collision paths.
- Keep regression tests ROM-free where possible, with optional owned-ROM coverage.

## Acceptance Criteria

- A regression reproduces the reported class of outdoor failure before the fix.
- The corrected path remains responsive or clearly reports unsupported behavior
  instead of appearing to crash/hang; actual walking fixes remain source-backed.
- Relevant native/runtime tests pass and the fix receives independent review.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- User observed this before music was added; do not assume an audio regression.
- Investigate missing resident art separately in
  [Crysta exterior placeholders](fix-crysta-exterior-placeholders.md).
