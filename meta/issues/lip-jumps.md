# Model the lips' jumps

## Summary

A lip (attribute 8, `$80:CF30`) jumps Ark down only on `$127`, the Hole's rim, with a guessed 64 pixels (2 a frame for 32 frames) into the exit. The lips of `$117`-`$119` in tower 4 are not modelled.

## Dependencies

- [Play tower 4](tower-four.md)
- [Finish the underworld](underworld-end.md)

## Requirements

- Take the jump's distance and frames from `$80:CF50`.
- Model the lips on every map.

## Acceptance Criteria

- The jumps on `$117`-`$119` and `$127` match native frames.

## Notes

- Marker: `world/fall.rs` (`LIP`, `JUMP`).

## Progress

- Lips are on most tower floors, not only tower 4 and the Hole (attribute 8: `$10F` alone has 186 cells); the runtime refused them as walls.
- room-core admits type 8 on the ordinary resolver with the `$097C & 4` contract (`Room::with_type8_special_bit_clear`): Open in the Down/Left/Right first tables, Partial in Up's, and the O/P/S tables' entries 8 as a second sample; a test checks it against the directional translation at every remainder.
- The drop is the ROM's for every map: two class-8 samples (`$08`, `$13`, `$1F`, `$80:CF30`) start it (`$80:CED1`, `CF50`), stream `$27` (3 pixels down a frame, walls off), exits fire, and `$80:CDA2` ends it: a landing, or a pit fall when only pits are under Ark. The Hole's guessed 64-pixel jump is gone.
- Open: a comparison with a native trace (no trace of a lip drop yet); the carrying variant's poses (`$84:9F13`).
