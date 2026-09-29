# Keep the native save slot in the world

## Summary

Model the native slot block ([saves](../../docs/saves.md)): WRAM `$7E:0600–07FF`
and `$7F:8000–82F9`, 1274 bytes, as the save payload. The world writes what
it models into it (map, position, facing, flags, items, money, Prime Blue) and
passes every other byte through, so a round trip loses nothing.

## Dependencies

- [Port menus, inventory, configuration, and saves](menus-inventory-save.md)

## Requirements

- Typed accessors over the block; unknown bytes kept as they were loaded or
  as a new game sets them (`$86:B93F`).
- Capture a block from a world and resume a world from a block, as the load
  does: saved map, position + (8,16), facing + 1, the prologue when flag
  `$20` is clear.

## Acceptance Criteria

- A world captured and resumed plays on identically; a slot decoded from a
  native SRAM dump resumes at the desk (472,176) facing up, on both ROMs.
