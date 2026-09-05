# Inspect read-only sprite hardware state

## Summary

Expose physical OAM bytes and the OBJ selection/priority-start registers through the reference oracle so player-sprite qualification can compare ROM-derived composition to live hardware state.

## Dependencies

- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Read the existing PPU state without CPU execution, memory writes or side-effecting hardware register reads.
- Preserve all 544 physical OAM bytes, reconstructed OBJSEL and first-sprite priority index; document these as current state, not guaranteed pixel-output latches.
- Keep this reference-only API outside the portable core and live house runtime.

## Acceptance Criteria

- Tests verify extent/packing and demonstrate that repeated reads do not change existing read-only reference state surfaces.
- Two independent fresh-ROM processes reproduce selected sprite-state observations.
- Existing oracle tests pass; no raw OAM or ROM-derived imagery is committed.

## Notes

- Supports [Ark sprite qualification](qualify-ark-sprite-assets.md).

- Test finding: `save_state` calls synchronized serialization and is not a passive
  before/after probe. Use CPU/frame/WRAM/VRAM/CGRAM/APU/framebuffer/audio reads
  instead; do not attribute serializer synchronization to the new accessor.

## Verified completion

- Added `Session::sprite_state()` and a fixed546-byte native read-only ABI.
  Two fresh6800 boots reproduce OAM hash/OBJSEL2/priority-start0; existing
  exported reference surfaces remain unchanged across repeated reads.
- `cargo test -p oracle` and strict oracle Clippy pass. Independent source/ABI/
  read-only contract review found no blockers; minor compactness/docs nits fixed.
- [API, source correspondence and test finding](../../docs/sprite-hardware.md).
  This does not close the separate sprite decoding/rendering milestone.
