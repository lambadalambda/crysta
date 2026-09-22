# Qualify Crysta return arrivals and prevent reverse-door bounces

## Summary

Initially the explicit directional candidate reached all 24 Crysta maps, but
returned from only21 of23 non-opening maps. Raw destination placement omitted
the native arrival phase. Both missing returns now use measured source-bound
profiles, and all23 checked round trips pass.

## Dependencies

- [Qualify the directional resolver for Crysta's remaining collision types](qualify-crysta-directional-collision.md)

## Requirements

- Capture genuine input-only `$19 → $17` (selector14) and `$1E → $0A`
  (selector5) traversals from boot, without warps, memory patches or save states.
- Measure queued, loaded and settled coordinates, arrival-controller ownership,
  input handoff and reverse-exit suppression, including relevant town occupancy.
- Add failing replay tests, then implement only justified arrival behavior. No
  guessed position correction, timer, or collision bypass.
- Preserve conservative production collision and checked resident/load errors.

## Acceptance Criteria

- Both native arrival sequences have reproducible input recipes and pinned
  evidence, with raw ROM/captures remaining ignored.
- Portable arrival positioning, control handoff and exit suppression match the
  measured scope; unsupported selectors/modes remain explicit.
- Checked candidate round trips succeed for all 23 non-opening maps without
  intermediate position resets or ignored refusals/errors.
- Existing traversal, collision and error tests pass; independent review completed.

## Notes

- Initial return gaps: `$19 → $17` raw `(448,352)` and `$1E → $0A` raw
  `(784,752)`. See [collision](../../docs/collision.md).
- Scope is arrival sequencing, not global collision enablement or Pandora story
  progression qualification.
- Native captures and hostile Left+A replays now pin both initialized-to-free
  profiles: selector5 `(792,752) → (792,769)` over36 advances; selector14
  `(442,345) → (456,368)` over78. These are bounded measured profiles, not a
  generalized scheduler or departure/loader clock. Evidence, recipes and
  verification live in [arrival qualification](../../tools/arrival-qualification/README.md).
- Direct ROM verification corrected an older exit-gate annotation: `$8D:87A5`
  continues scanning when `$097C & $10` is **clear**, not set. Frame-end `$8000`
  arrival ownership must not be presented as proof that this gate suppresses
  every native scan.

## Completion

- Pure, lossless change-point profiles replay every initialized-to-free sample;
  both checked transition paths validate exact source identity and all operands.
  Explicit placement stays raw. Owned arrival bypasses ordinary walking/exits
  and host interaction, then releases fresh host walking history only after the
  measured free sample. Other records remain explicitly legacy/unqualified.
- Native town occupancy is retained: the nearby resident at `(776,768)` and
  dynamic block at `(48,47)` coexist with the measured arrival; no solid cell or
  resident was removed. See collision documentation for bounded evidence limits.
- Checked discovery/replay now passes **24/24 outbound, 23/23 returns**, with
  no intermediate position reset or accepted refusal/load error. Conservative
  production outbound coverage remains **19/24**.
- TDD: missing profile/API tests failed before implementation; review's changed
  destination bypass was reproduced red, then fixed by source-first validation.
  Portable tests cover all operand bytes (including conditional destination),
  source relocation, every sample/phase and saturation; ROM-backed tests cover
  both entry paths, hostile inputs, held endpoint/recovery, and ordinary resumption.
- Verification: full release workspace tests; nine native collision windows
  (**5,541 frames**) and legacy regressions; both baseline/hostile capture hashes
  and profiles; observer and extractor unit tests; strict clippy/rustdoc,
  formatting, tracker and repository safety checks. Independent source,
  architecture/correctness reviews and executable runtime verification passed.
- Broader collision/state qualification remains in the parent issue; this does
  not enable production directional collision or emulate native global scheduling.
