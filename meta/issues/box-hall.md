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
