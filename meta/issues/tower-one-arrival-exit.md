# Walk into and out of tower 1

## Summary

Selector `$66` places Ark at (256,1024) and walks him up to 1007 in 20 frames; the exit back (cells (0,63) 32x2, selector `$55`) triggers at y 1025 and walks out to 1042 over 18 frames; on `$03` he lands at (216,816) and walks to (216,832) (`docs/tower-entry.md`).

## Dependencies

- [Enter the towers from the world map](enter-the-towers.md)

## Requirements

- Model the `$66` arrival and the `$55` walk-out and the return to `$03`.

## Acceptance Criteria

- Arrival and the way back match the native positions frame by frame on both ROMs.
