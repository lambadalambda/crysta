# Open the box's hall with Select

## Summary

Select opens Yomi's box (`COP 2D 0020` → `$85:A179`, European `$85:A211`)
once flag `$FE` is set: the hall the tour starts in (`$041`'s background),
Yomi on a door as the cursor, the hotspot's label below, and the menu HUD
(level, life, item, gems). The pad moves Yomi between the hotspots; Select
leaves. The field waits meanwhile (`$049A`).

## Dependencies

- [Keep the native save slot in the world](save-slot-block.md)

## Requirements

- The opening and closing as natively (frames, fades).
- The hotspots, their order under the pad, Yomi's place and pose at each,
  and the labels.

## Acceptance Criteria

- Native traces (JP and EU) of opening, moving across the hotspots and
  closing match frame by frame: the hotspot, Yomi's place, the label.
- Before `$FE`, Select does nothing.

## Notes

- Runtime: `crysta_runtime::menu` steps the hall; the world opens it on
  Select and waits while it runs. Not modelled yet: Ark's put-away pose
  (`$85:A0F0`, anim `$1C`) and its return (`$85:A113`), the window's
  gradient, the pad at S+61, Yomi's ease after a closing press, and the
  repeat counted from the physical press when it falls in the redraw.
- The host draws the hall, Yomi, the label and the HUD, matching native
  captures (`local/menu/*-hall.rgb`) on both ROMs except the wall lamps'
  tile animation and the item icon ([icon](hud-item-icon.md)).
- The wall lamps animate by tiles, not colours: 4 frames of 6, a
  24-frame cycle (JP capture). The hall's helper `$85:D1EE` runs
  `COP 8C $3C` (`$80:A30C`: `7F:0008` = `$3C`, then `$8D:93D8`, `$8D:940D`,
  `$8D:9470`, the town animation's consumers) and `COP 95` in a loop;
  `$8D:93D8` reads `$9B:8000 + 2n`, whose bit 15 points it at the second
  base `$9C:8000` (list `$8D:9407`); the records there are not decoded.
