# Qualify Ark's house sprite assets

## Summary

Locate and decode the ROM-backed graphics, palette and frame composition for Ark's ordinary standing/walking poses in the two covered house rooms.

## Dependencies

- [Decode graphics, palettes, sprites, and animation](decode-graphics-animation.md)
- [Qualify a fresh new-game house bootstrap](qualify-new-game-house-bootstrap.md)

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
