# Show the glow when a voice speaks from the blue door and the Box

## Summary

When the voice speaks from the blue door, and in Yomi's box, the native game darkens the screen a little and shows a spinning bright square at the door; both are missing.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Research the effect (colour math, the square's sprite or BG, its motion) and play it where the scripts start it.

## Acceptance Criteria

- Both scenes match native frames during the voice.

## Notes

- A `Display` model takes the colour math scripts write (inline native code, `COP 76`) and `COP 6A`'s turning square; the app subtracts the fixed colour from the background outside the square, after the palette's tint (`crysta-runtime/src/display.rs`, `local_story.rs` `the_voice_glows_from_the_broken_door` and `the_closed_box_glows`: 7 steps of 18 frames up and down, the square at the door and on the box). Checked by eye against `local/effects/jp/box`; the square's edges are computed with floating point, not the native per-line table.
