# Model COP AA and AF

## Summary

`COP AA` (`$90:8B01`, `8CB2`, `8D02`: the light room and the tower doors)
and `COP AF` (`$93:DA8C`, `DB04`, `DB93`: Shadowkeeper's room `$123`) are
stepped over. `COP AA` starts an HDMA child (`docs/light-room.md`).

## Dependencies

- [List the COP services the underworld scripts use](cop-inventory.md)

## Requirements

- Read the handlers and model what the scripts need, or show that stepping
  over is right.

## Acceptance Criteria

- A unit test per modelled service, checked against a native trace.

## Progress

- Done: `AF` (the streams alone, in `moving_pose`), with a unit test.
- Open: `AA` starts effect entities whose scripts use `8A`, `93`, `AE` and
  `EC` (palette cycles and blends: `$90:8BA2` in the light room, `$90:8D0E`
  and `8D29` at the tower doors); the inventory now walks them.
- Done: `AA` starts a hidden effect entity after its parent; its `EC`
  blends OBJ colours (tower 1's head matches the native CGRAM at steps 1,
  30, 60), `AE` ends it, `93` waits on the palette cycle `8A` records.
  `$123`'s darkness keeps its own model.
- `AF` runs only in Shadowkeeper's phases (`$93:DA8C`, `DB04`, `DB93`);
  its native check moves to [Play Shadowkeeper's full fight](shadowkeeper-full-fight.md).
