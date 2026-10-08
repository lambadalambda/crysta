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

## Progress

- Research: `docs/runtime-review-edges.md`.
- Leave (the runtime's rule is the native one, or no chapter-1 content reaches it): mode-4 objects as foes (the hit scans read `+$04` and the profile only); the `$047C` reload (`$8D:86F8`, mode and selector cleared); Ark's script's pokes and deletions (no chapter-1 `COP DF` script makes them); `COP 9C` and the bit-8 interaction (only the save desks; one frame).
- To fix: children that copy their parent's flags (`9A`, `9C`, `A1`, `A3`, `A5`, `E6`) start hidden, `+$04 | $8000`, bit 12 clear (`$80:BCD2`); `+$2C` is the previous entity in the list, not the parent (the `$11D` orb's trail stays where the orb was); bit 12 gates the actors run in the nested frame of a transfer's fade (`$80:C85E`): enemies stand still then; `place()` clears the fall, the drop, the rope, the thrust and the recoil.
- Done: the copying children, the nested frame's bit 12, `place()` and `+$2C` are fixed (`22003b3`, `cbed4c3`, `3e3abe0`); the rest are left as above.
