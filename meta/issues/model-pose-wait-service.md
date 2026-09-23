# Model the COP 8E pose wait from source

## Summary

The resident interpreter treats `COP 8E` as a one-frame wait. Natively it
holds for longer: the town walker `$83:8A2D` pauses about 75 frames over four
`{COP 23, pose, COP 8E, COP 03}` passes. Standing residents that cycle poses
with `COP 8E` are affected too.

## Dependencies

- [Execute counted loops, timed waits and the map branch in resident scripts](execute-script-loops-and-waits.md)

## Requirements

- Read the `COP 8E` handler and state exactly how long it holds (for
  example one repetition of the selected display list) and what it writes.
- Implement that length from the resident's own packet where the source
  chain is admitted; otherwise keep today's approximation.
- Confirm against the native town capture (pose 6/7/8 section).

## Acceptance Criteria

- Pure tests for the wait length, red before green.
- The town walker's pose section matches the native frame count.
- Workspace gates and app tests with the ROM pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
