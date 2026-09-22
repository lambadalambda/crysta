# Qualify Crysta return arrivals and prevent reverse-door bounces

## Summary

The explicit directional candidate reaches all 24 Crysta maps, but two return
routes bounce into the reverse doorway after an alignment nudge. The host uses
raw destination coordinates and omits the native selector-driven arrival phase.

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
