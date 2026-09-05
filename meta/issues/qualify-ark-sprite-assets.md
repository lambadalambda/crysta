# Qualify Ark's house sprite assets

## Summary

Locate and decode the ROM-backed graphics, palette and frame composition for Ark's ordinary standing/walking poses in the two covered house rooms.

## Dependencies

- [Decode graphics, palettes, sprites, and animation](decode-graphics-animation.md)
- [Qualify a fresh new-game house bootstrap](qualify-new-game-house-bootstrap.md)

- [Inspect read-only sprite hardware state](inspect-sprite-hardware-state.md)

## Requirements

- Trace resource loading and sprite composition to authenticated ROM sources; use runtime VRAM/CGRAM/OAM or draw-record evidence to validate, not as distributed assets.
- Preserve placement, transparency, flips, palette and ordering metadata rather than baking unexplained screenshots into an atlas.
- Provide bounded pure decoders and deterministic local export, with synthetic red-green tests and selected authenticated pose comparisons.

## Acceptance Criteria

- Required standing/walking frames can be composed from ROM-derived assets without original CPU execution.
- Selected player poses match authenticated reference tile/palette/composition evidence and image comparisons with explicit scene-effect exclusions.
- Raw evidence remains ignored; source extents, hashes, reproduction and limitations are documented.

## Notes

- Subissue of [render Ark in the portable house](render-ark-house-sprite.md).

- Completed: pure ROM decoding of21 ordinary frames plus7 horizontal mirrors, preserving source anchors, palette, transparency and priority/order. See [evidence and reproduction](../../docs/ark-sprites.md).
- Synthetic red→green tests,14 authenticated indexed compositions, selected fresh OAM/VRAM/CGRAM/draw-record and opaque-image comparisons, and deterministic28-raster exports pass. Raw artifacts remain ignored.
- Fresh native BGMODE writers and tilemap priority witnesses qualify opaque high first-background pixels above OBJ2; transparent/low pixels do not occlude. The first background is hardware BG2, correcting prior house terminology.
- Native transport and actual-browser integration are verified; independent reviews passed.
