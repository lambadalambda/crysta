# Execute counted loops, timed waits and the map branch in resident scripts

## Summary

The resident interpreter steps over `COP 02`, `COP 03`, `COP C1` and
`COP 0A`. On the town walker `$83:8A2D` that drops the one-frame loop gap
after each action and runs its four counted loops (16 steps, then pose
6/7/8 waits) as a single pass. `COP C1` entry delays are lost too.

## Dependencies

- [Derive each walker's step and idle timing from source](derive-walker-cadence-from-source.md)

## Requirements

- `COP 02 n`: loop count `n` and loop start after the operand (`$80:85DF`).
- `COP 03`: decrement; nonzero jumps to the loop start and yields one frame;
  zero continues (`$80:85F8`). One loop level per actor, as the game stores it.
- `COP C1 n`: suspend for `n` frames, resume on frame F+n+1 (`$80:AB17`).
  An admitted action in progress keeps moving meanwhile.
- `COP 0A word target`: branch when `word & $7FFF` equals the map; bit 15
  inverts (`$80:8720`).

## Acceptance Criteria

- Pure tests for each service, red before green.
- With source cadence the town walker's action-to-action period is 33 ticks
  for a step and 17 for an idle, as natively witnessed, and it plays its
  pose 6/7/8 waits after 16 steps.
- Workspace gates and app tests with the ROM pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Handler facts: `tools/crysta-cadence-qualification/REPORT.md`.
