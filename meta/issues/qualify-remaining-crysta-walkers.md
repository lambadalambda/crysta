# Qualify the remaining Crysta walkers' movement cadence

## Summary

Only the map-`$0D` class-0 wanderer has source/native-qualified step timing.
Seven other drawn residents in the slice run the same `COP 26` random walk and
still use the approximate eight-tick, two-pixel projection.

## Dependencies

- [Verify native Crysta movement and animation cadence](verify-native-crysta-cadence.md)

## Requirements

- Derive each walker's step and idle timing from its own source chain: class,
  descriptor movement base, the `$80:8F6D`/`$80:8FB5` class tables, the
  movement streams and the display lists of its composition packet.
- Replace the single hard-coded admission with that derivation. Anything
  outside the understood subset keeps the existing projection.
- Qualify with a native witness where the source alone leaves a question.
- Keep the map-`$0D` result unchanged.

## Acceptance Criteria

- Every drawn `COP 26` walker in the 24 maps is either admitted with
  source-derived timing or listed with the reason it is not.
- Regression tests cover the derivation and refusals; the map-`$0D` tests pass.
- Timing doc and README record expected rates per walker.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Census (new-game flags): `$0A` `$83:89FB` (header `05 00`, mode `$20`);
  `$0A` `$83:8A19`/`8A23`/`8A2D` (header `02 04`/`09 04`, mode `$22`);
  `$15` `$83:8F67`, `$1A` `$83:90BD`, `$1B` `$83:90F7` (header `xx 00`,
  mode `$20`).
- `$80:8F6D` groups by `class & ~3`: movement `$68/$69/$60` (0.5 px/tick),
  `$78/$79/$70` (1 px/tick), `$88/$89/$80` (2 px/tick). `$80:8FB5` groups by
  `class & 3`: idle counts 16, 8, 1, 1. Map `$1A`'s down list is 31 ticks.

## Sub-issues

- [Witness a class-2 Crysta walker natively](witness-class-two-walker.md)
- [Derive each walker's step and idle timing from source](derive-walker-cadence-from-source.md)
- [Execute counted loops, timed waits and the map branch in resident scripts](execute-script-loops-and-waits.md)

## Findings

- The class is descriptor mode `& $0F`, not header byte 1. The exterior
  walkers with mode `$22` are class 2, which uses the class-0 movement row
  and idle count 1. No walker in the slice is class 4 or higher.
- Header byte 1 is the low byte of entity flags +`$04`; bit `$0004` selects
  the collision-checked movement path at `$80:D101`, with the same deltas on
  free cells.

## Result

- All eight drawn `COP 26` walkers in the 24 maps are admitted with
  source-derived timing (classes 0 and 2); none is refused. The class-2 row
  has a native witness in the town, retained in the qualification tool.
- Expected rates per walker are tabled in `docs/native-crysta-timing.md`.
- `COP 8E` waits stay a one-frame approximation, and a non-admitted step
  followed by `COP C1` finishes before the wait where hardware keeps moving;
  no slice resident does this.
