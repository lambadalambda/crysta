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

## Result

- Handler: `COP 8E` plays the selected list once; the next command runs
  T(n) = Σ(duration + 1) frames after `COP 80 n; COP 8E`, in the dispatch that
  meets the list end (any word with bit 15 set). `COP 80` restarts the list.
- `cadence::pose_ticks` reads every list of the resident's packet (the table
  runs up to the lowest list it points at; an entry that does not end within
  sixteen records is `None`). Actors wait T - 1 frames, yield once for T = 1
  and continue for T = 0; an unknown packet or list keeps the one-frame wait.
  `COP 80` now resets the pose age on every selection. Walk lists also end at
  any bit-15 word now; all eight walkers derive as before.
- Independent review found one regression, fixed: lists were read to 16
  records while the art decoder reads 64, so map `$1D`'s `$83:919B` (a
  304-tick pose) fell back to the short wait and, with the restart on every
  `COP 80`, stuck on its first raster. Lists now read to 64 records, and a
  same-pose `COP 80` restarts the raster only where the list length is
  known. A test fails with the old unconditional restart. Table entries that
  point back into the table now end it.
- Red: the pure wait test, the same-pose restart test and the town walker's
  75-tick pose section failed before, and pass after. 46 runtime unit tests,
  every runtime integration test, workspace fmt/Clippy/tests and 65 app tests
  with the ROM pass.
