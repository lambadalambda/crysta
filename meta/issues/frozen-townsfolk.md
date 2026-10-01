# Freeze the townsfolk after the freeze

## Summary

After the freeze, the people still walk around and are not blue.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Research how the frozen state stops their scripts and tints them (palette), and model it.

## Acceptance Criteria

- The frozen town matches native frames: still, blue people.

## Notes

- `COP BB`'s palette field is kept and drawn: palettes 4 and 5 shift to OBJ palette 3, `$CC:2A6C` with colour 14 `$08DF` (`local_art.rs`, `local_story.rs`). After the real frozen return nobody walks in `$0A..$11` (probe, 600 frames); the walking the user saw is not reproduced. The source of colour 14's patch is not found.
