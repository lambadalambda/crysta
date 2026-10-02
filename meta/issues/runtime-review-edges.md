# Close the runtime's review edge cases

## Summary

Edge cases from the reviews of the underworld work. None breaks a tested path.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- `COP 9C` children's `+$04` flags, and the interaction test on `+$04` bit 8 (`world.rs`, `faced_resident`).
- `LDA $002C,X` reads the parent (a guess; `actors/native.rs`).
- Mode-4 objects as foes.
- The reload when the pending map is the current map.
- Pokes and group deletions from Ark's own script apply a frame late.
- `place()` does not clear a jump.

## Acceptance Criteria

- Each case is traced against the ROM and fixed or confirmed.
