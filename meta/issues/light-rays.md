# Draw the light rays in the rooms

## Summary

Most rooms show rays of light falling from the windows (user screenshots,
shop capture). The slice draws none.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Find which layer and data draw the rays, and in which maps, and draw
  them with their motion.

## Acceptance Criteria

- The rays in the bedroom and the shop match native frames.

## Resolution

- The rays are the rooms' second layer (`10 02`, the town clouds' shared
  sheet, cells `$08..$24`, palette 7), added in full onto the view
  (`$96:BC1D`: `CGWSEL $82`, `CGADSUB $21`); they stand still. The cellars
  `$0E`, `$20`, `$21` load none. A render of the bedroom and the Elder's
  house matches the native screenshots (2026-09-24).
