# Show Ark's stair poses

## Summary

Going up or down stairs moves Ark correctly, but his sprite and facing are wrong.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Research the native stair walk's poses and facing per direction and draw them.

## Acceptance Criteria

- Stair walks in both directions match native frames on both ROMs.

## Notes

- Each stair motion plays its resource-1 list (`$13..$16`, `docs/ark-poses.md`), timed to the native fixtures in `local/poses/`; Ark lands facing Down (`local_story.rs` `ark_plays_the_stair_lists_down_from_e_and_stands_facing_down`, `local_art.rs`). Checked on the Japanese ROM for the list timing; the European lists share the code.
