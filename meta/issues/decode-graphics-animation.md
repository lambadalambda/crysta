# Decode graphics, palettes, sprites, and animation

## Summary

Extract the visual data needed for classic rendering while preserving palette and ordering semantics.

## Dependencies

- [Implement and verify the compression codec](compression-codec.md)
- [Classify ROM code, data, and indirect dispatch](classify-rom-code-data.md)

## Requirements

- Decode relevant SNES tile formats and palettes.
- Model sprite placement, animation timing, priority, flips, and palette selection.
- Identify DMA/resource grouping used during map transitions.
- Provide inspection exports without committing generated assets.

## Acceptance Criteria

- Representative player, enemy, map, UI, and effect graphics render correctly in a local inspector.
- Animation fixtures match reference frame timing.
- Extraction is deterministic from a verified ROM.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Do not flatten priority or palette relationships into presentation-only PNG metadata.

## Subissues

- [Render the static cavern from decoded graphics](render-static-cavern.md):
  ROM-only full first-background viewer with pure 4bpp/palette/word primitives.

## Progress

The cavern now has source/VRAM/WRAM/CGRAM equality, 672 tilemap words and a
256-pixel effect-matched reference patch. The static artifact renders the full
1280×512 layer in its natural ROM palette. See [static graphics](../../docs/static-graphics.md).
This parent remains open: other graphics families/maps, sprites, animation,
resource caching and final scene effects still need qualification.
