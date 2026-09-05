# Render the static cavern from decoded graphics

## Summary

Produce a local full-map visual artifact from ROM-resolved cavern graphics,
palettes and metatiles rather than framebuffer atlas samples.

## Dependencies

- [Resolve map loading scripts and qualify additional layers](resolve-map-loading-scripts.md)
- [Qualify static map loading and decode the cavern layer](static-map-cavern.md)

## Requirements

- Qualify cavern graphics/palette/metatile resource layout against loader evidence.
- Add pure bounded SNES graphics and map rendering primitives with synthetic tests.
- Preserve tile palette, flip and priority metadata; distinguish static rendering
  from animation, sprites and hardware compositing.
- Export an inspectable full-map artifact from an authenticated local ROM without
  SRAM or emulator execution; keep all derived assets ignored.

## Acceptance Criteria

- Cavern map ID resolves to graphics, palette, metatiles and a rendered full layer.
- Synthetic tests cover planar pixels, color conversion, tile attributes and
  placement, and malformed resource bounds.
- Owned-ROM tests and runtime evidence qualify the decoded resources and compare
  representative static map pixels with the oracle, documenting exclusions.
- Local browser QA confirms the visual artifact is usable.
- Documentation states the supported scope and outstanding rendering semantics.

## Notes

- Subissue of [graphics decoding](decode-graphics-animation.md) and
  [map formats](decode-map-collision-formats.md).
- Initial scope is the portal cavern ($0128), not a generic scene renderer.
