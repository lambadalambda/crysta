# Lift and throw pots as the native game does

## Summary

A pot lifts and throws only at the three positions the cellar's native
route recorded: lifts at (104,352), (88,352) and (40,352), throws facing up
from (136,368) or (184,368), and only while Ark stands still
(`crates/room-core/src/pots.rs`, `docs/pandora-pots.md`). Anywhere else the
A press is lost, so throwing seems random. Reported by the user
2026-09-30.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Research the native lift, carry and throw on both ROMs: which cells lift
  from which positions and facings, whether A works while walking, the
  flight in each facing (path, speed, height, range), what stops it (walls,
  residents, pots, the room's edge), the break and its sounds, and the
  recovery timing.
- Generalize the pot component to those rules; keep the three recorded
  cellar segments and the door contacts exact.

## Acceptance Criteria

- Lifts and throws in all four facings, standing and walking, match native
  traces on both ROMs; the cellar's recorded segments still pass.
