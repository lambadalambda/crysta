# Hide Ark behind the stairway's frame on the stairs

## Summary

Going down the stairs, Ark's sprites are right, but he walks in front of
the stairway's frame instead of disappearing into it. Reported by the user
on 2026-10-01.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Research how the native game hides Ark on the stairs (mask sprite, OBJ
  priority, high-priority tiles) and draw it so.

## Acceptance Criteria

- Stair walks in both directions match native frames on both ROMs.
