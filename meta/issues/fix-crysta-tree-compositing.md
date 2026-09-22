# Correct native Crysta tree compositing

## Summary

The user reports trees do not look fully correct, possibly a transparency issue.

## Requirements

- Identify the source-backed layer/transparency behavior at exterior tree pixels; correct only the demonstrated native rendering gap.
- Use regression tests and independent review before implementation commits.

## Acceptance Criteria

- A focused regression distinguishes incorrect and corrected composition; compare an exterior tree view to source/native evidence without weakening renderer qualification.
- Relevant tests and strict lint gates pass; document fidelity limits.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Raw ROM assets, screenshots and reference captures remain ignored under `local/`.

## Completed

- Root cause: native background loading reused the inspector BMP, whose index-zero
  pixels deliberately contain a gray checkerboard. Transparent holes/edges in
  the exterior trees therefore showed diagnostic colors rather than the backdrop.
- Added a native-only background presentation boundary. For map A, transparent
  indices now receive source palette entry32, while opaque pixels and priority
  remain unchanged. `$8D:8C52..8C62` copies staged `$7F0640/41` to `$7F0600/01`;
  the admitted loading recipe supplies color32 from the exterior palette. The
  source instruction sequence is checked; neither `$15ED` nor captured colors
  initialize production. The inspector's static exports are untouched.
- Existing `landed-A` capture's six files match the original exterior reference
  pins. World rectangle `[384,701,414,762)` contains 1,783 opaque and47 transparent
  pixels. All1,830 match source palette/backdrop with the reference ares color ramp;
  the fixed host matches all1,830 with its existing natural RGB expansion. Exactly
  the47 checkerboard pixels changed. Private comparisons/screenshots and red/green
  logs are under `local/crysta-backgrounds/`.
- The owned-ROM session test failed before correction and passes afterward.
  Two synthetic compositor tests and the explicit source-palette mutation /
  changed-consumer refusal test pass. Native suite:41 ROM-free tests pass;
  four owned-ROM session regressions and the source-mutation test pass. Strict
  native Clippy, changed-file formatting and diff checks pass.
- Independent correctness/architecture review approved with no blockers. Archived.
  This does not implement additive leaf effects, secondary scrolling, color math,
  or whole-exterior RGB equivalence; river animation is a separate tracked task.
