# Hide Ark's head behind a door's top as he enters

## Summary

When Ark walks up into a doorway, his head draws over the door's top; natively the top covers it.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Find what the native game draws in front (BG priority tiles, sprite priority) at doorways and draw it so.

## Acceptance Criteria

- Entering a door upward matches native frames: the head is behind the door's top.
