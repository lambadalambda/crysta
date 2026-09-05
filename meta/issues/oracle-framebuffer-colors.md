# Preserve oracle framebuffer colors

## Summary

The project ares shim interprets the video callback's ARGB8888 pixels as RGB30,
producing corrupted colors in local captures. This blocks useful map visuals.

## Dependencies

- [Select and integrate the reference emulator](select-reference-emulator.md)

## Requirements

- Match the vendored `Screen::refreshPalette` ARGB8888 format.
- Retain the public XRGB8888 buffer layout and current geometry.
- Add a ROM-free regression for RGB channel preservation.

## Acceptance Criteria

- Known channel values survive conversion with only alpha removed.
- Existing oracle tests pass; a local loaded-map capture has correct colors.

## Notes

- Parent: [Build a local loaded-map inspector](loaded-map-inspector.md)
- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
