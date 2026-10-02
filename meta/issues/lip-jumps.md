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
