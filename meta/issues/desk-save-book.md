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

## Notes

- `COP 9C` spawns at the parent plus (dx, dy), dx mirrored (`$80:A56B`); the book clears `+$04`/`+$06`, takes the `$A2:C000` art and loops on pose `$42` (`crysta-runtime/tests/local_art.rs` `the_desk_shows_its_save_book`). Checked on the Japanese ROM and by eye in the app; no native capture of the desk exists.
