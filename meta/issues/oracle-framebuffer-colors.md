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

## Completion record

- `1473b92` corrects the shim to remove ARGB8888 alpha without shifting RGB
  channels, matching vendored `Screen::refreshPalette`.
- ROM-free compile-time assertions failed with the old RGB30 conversion and
  pass with the corrected conversion. Public framebuffer geometry is unchanged.
- Independent review approved; workspace oracle tests pass, and local cavern
  captures were visually inspected with correct colors. The map-inspector
  integration test now pins both qualified RGB hashes.
