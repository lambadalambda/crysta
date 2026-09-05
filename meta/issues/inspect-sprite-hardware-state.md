# Inspect read-only sprite hardware state

## Summary

Expose physical OAM bytes and the OBJ selection/priority-start registers through the reference oracle so player-sprite qualification can compare ROM-derived composition to live hardware state.

## Dependencies

- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Read the existing PPU state without CPU execution, memory writes or side-effecting hardware register reads.
- Preserve all544 physical OAM bytes, reconstructed OBJSEL and first-sprite priority index; document these as current state, not guaranteed pixel-output latches.
- Keep this reference-only API outside the portable core and live house runtime.

## Acceptance Criteria

- Tests verify extent/packing and demonstrate that repeated reads do not change serialized reference state.
- Two independent fresh-ROM processes reproduce selected sprite-state observations.
- Existing oracle tests pass; no raw OAM or ROM-derived imagery is committed.

## Notes

- Supports [Ark sprite qualification](qualify-ark-sprite-assets.md).
