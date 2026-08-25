# Implement the classic renderer

## Summary

Render portable commands with SNES-authentic viewport, ordering, palette, and effect behavior.

## Dependencies

- [Decode graphics, palettes, sprites, and animation](decode-graphics-animation.md)
- [Define the deterministic portable core model](deterministic-core-model.md)
- [Export and compare reference frame state](reference-state-export.md)
- [Complete the Crysta and Pandora vertical slice](opening-vertical-slice.md)

## Requirements

- Model tile layers, ordered sprites, priorities, windows, scrolling, palettes, and color math used by covered areas.
- Separate logical rendering from host scaling and shaders.
- Generate framebuffer hashes at reference checkpoints.
- Support deterministic headless rendering.

## Acceptance Criteria

- Selected opening and first-tower frames match approved reference images or documented pixel differences.
- Renderer tests cover ordering, flips, palettes, clipping, and transitions.
- The renderer does not mutate authoritative game state.

## Notes

- Milestone: [M5 — Classic presentation and Chapter 1](../milestones.md#m5-classic-presentation-and-chapter-1)
- A temporary PPU emulation backend is acceptable if its boundary permits later semantic replacement.
