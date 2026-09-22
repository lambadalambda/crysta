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

## Completed

- Reproduced map `$000A`, `(360,472)`: the third Right update refuses
  `UnsupportedType(6)`. Strict atomic refusal preserves the queued Right input;
  neutral/Left/Up repeatedly retry it, and skipped actor ticks freeze the scene.
  The audio worker continues independently, matching the clarified report.
- Added opt-in `World::step_interactive`: a refusal keeps position, resets delayed
  movement, restores standing/facing and ticks residents once. Strict APIs retain
  atomic refusal; rejected frames do not test/rearm exits. No collision type was
  newly admitted and the production/candidate qualification boundary is unchanged.
- Native Session now uses that recovery path. Checked movement/interaction loading
  errors stop once, flush a named diagnostic and show `WORLD STOPPED` in the title;
  the event loop waits for input rather than repeatedly using the failed world.
- Red-before/green-after session regression covers the reported tree edge. The
  headless host script `at:A:360:472,right:3,wait:8,left:16,up:32` now ends at
  `(344,429)` instead of remaining trapped. This debug placement is a host
  regression, not a new native input-only qualification witness.
- Runtime and native integration received independent correctness/architecture
  reviews. Runtime regression gates and root release workspace tests pass; root
  fmt and strict workspace Clippy pass. Native: 39 ROM-free tests and three
  explicit owned-ROM session tests pass, with strict Clippy. Changed Rust files
  are formatted; two pre-existing wrapping differences in untouched app
  `frame.rs` tests remain outside this fix.
- Private reproduction, before/after views and test logs remain under
  `local/crysta-outdoor/`. Archived; broader collision qualification stays open.
