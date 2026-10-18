# Draw the HUD's item icon

## Summary

Both HUDs show the field's X item as an OBJ icon: in the box at (160,8),
palette 1, priority 3; with no item tiles `$24 $25 $34 $35`; an item's
icon from `$0648` → `$09C7` (`docs/menu.md`, HUD). Neither HUD draws it.

## Dependencies

- [Open the box's hall with Select](box-hall.md)

## Acceptance Criteria

- The box's hall matches its native captures with the icon included, with
  and without an item set.
