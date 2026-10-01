# Walk into and out of tower 1

## Summary

Selector `$66` places Ark at (256,1024) and walks him up to 1007 in 20 frames; the exit back (cells (0,63) 32x2, selector `$55`) triggers at y 1025 and walks out to 1042 over 18 frames; on `$03` he lands at (216,816) and walks to (216,832) (`docs/tower-entry.md`).

## Dependencies

- [Enter the towers from the world map](enter-the-towers.md)

## Requirements

- Model the `$66` arrival and the `$55` walk-out and the return to `$03`.

## Acceptance Criteria

- Arrival and the way back match the native positions frame by frame on both ROMs.

## Notes

- Exit selectors are read by their kind (`$8D:88DF`): `$66` arrives with the door walk from (256,1024) to (256,1007), `$55` leaves through the bottom and lands on `$03` at (216,816), walking to (216,832) (`local_towers.rs`, both ROMs). The walk-in's frames follow the door arrival's pattern, two still frames shorter at the start than natively; the walk-out is 16 pixels, natively 17.
