# Open Yomi's box after the frozen return

## Summary

Select opens the menu once flag `$FE` is set, as natively
([menu](../../docs/menu.md)): the item room first (Use, Equip, the field's X,
descriptions), then weapons, armor and the status mirror.

## Dependencies

- [Keep the native save slot in the world](save-slot-block.md)

## Acceptance Criteria

- After the frozen return, Select opens the box and the item room works as
  natively; before it, Select does nothing.

## Sub-issues

1. [Open the box's hall with Select](box-hall.md)
2. [Equip at the box's weapon and armor doors](box-equip-doors.md)
3. [Use and equip at the box's item door](box-item-door.md)
