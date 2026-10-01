# Draw the save book on Ark's desk

## Summary

The bedroom's save point (`$83:8D4F`) has no visible book. Its hidden
parent spawns the book with `COP 9C` (a child at an offset, here (0,-16)),
which the runtime steps over. Reported by the user on 2026-10-01.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Spawn `COP 9C` children at the parent's position plus the offset (dx
  mirrored with the parent), as `$80:A56B` does.
- Run the book's script (`$88:D641`) far enough to draw its pose.

## Acceptance Criteria

- The book shows on the desk in the bedroom on both ROMs, and saving still works.
