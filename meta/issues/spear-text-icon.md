# Draw the weapon's text icon with a clear background

## Summary

Talking to the weapon shows its page icon with a solid blue background; natively the background is clear.

## Dependencies

- [Fix the slice bugs the user reported on 2026-10-01](slice-bug-report.md)

## Requirements

- Find the icon's tiles, palette and colour 0 in the native window and draw them so.

## Acceptance Criteria

- The weapon's text page matches the native frame.

## Notes

- On a page without a window the prompt skips colour 3, as `$85:947F` does (`crysta-app/src/window.rs`). Checked by unit test and the native code, not a pixel compare with the capture.
