# Animate the river in native Crysta

## Summary

The user reports the exterior river does not flow. The native host currently caches a static background per map.

## Requirements

- Identify the original exterior background animation data and timing; add bounded native playback without fabricated animation or changes to gameplay cadence.
- Use regression tests and independent review before implementation commits.

## Acceptance Criteria

- Tests prove source-derived animation changes the river over time and repeats correctly; preserve static map rendering and existing qualified scenes.
- Relevant tests and strict lint gates pass; document fidelity limits.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Raw ROM assets, screenshots and reference captures remain ignored under `local/`.

## Source animation implementation

- Added a bounded immutable map-A graphics/palette animation decoder. It reads
  four original scene services and ordered partial transfers directly from ROM;
  static background exports and backdrop0 remain unchanged.
- River flow uses palette service `$06`, colors112–119, a six-tick transfer hold
  and42-tick cycle. Graphics services retain prior destinations across partial
  writes and cycle wrap. Full source timing/details and repeatable tests:
  [animation qualification](../../tools/crysta-animation-qualification/README.md).
- Five pure red/green tests and the explicit owned-ROM/native membership test
  pass in the parent checkout. Five retained map-A snapshots match a joint
  source state covering all768 graphics tiles and24 animated colors. This is
  membership evidence, not an asserted native video-frame phase alignment.
- Independent correctness/architecture review approved the decoder with no
  blockers. Native clock, cache invalidation and rendering are integrated separately.

## Completed native integration

- Native Session now owns a per-visit presentation clock, advanced by simulation
  updates even while stationary/in dialogue, reset on map change/debug placement,
  and frozen by a fatal world error. `background_tick` is included in diagnostics.
- Cached background rendering applies source transfers at the requested age;
  no intermediate redraw is needed. Per-cell graphics/palette dependency masks
  refresh only changed cells, including opacity/priority. Backward/startup seeks
  restore base values before applying partial transfers. Indoor maps and static
  inspector exports are unchanged.
- Stationary river regression was red before integration, then green. Full-map
  dirty-cache colors/priorities match brute source sampling across forward ages,
  wrap, backward/startup seeks and equal-key redraws. Tree transparency, refusal
  recovery and fatal-stop regressions still pass. Native:42 ROM-free tests plus
  five owned-ROM Session and two owned-ROM background tests pass; strict Clippy
  and changed-file formatting pass. Root fmt, strict workspace Clippy and release
  workspace tests pass too.
- Headless native renders at ages0,6,12,18,24,30,36,42 show river changes and exact
  river-region return at42. Phase sheet, GIF and frames remain under ignored
  `local/crysta-backgrounds/`. No full-scene RGB/native-phase equivalence claimed.
- Independent native integration review approved correctness/architecture with
  no blockers. The tree/backdrop fix and source decoder are separate commits.
  Archived; secondary leaf effects/color math remain outside this issue.
