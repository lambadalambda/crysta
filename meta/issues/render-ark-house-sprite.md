# Render Ark standing and walking in the portable house

## Summary

Replace the cyan player marker with ROM-derived Ark sprites, facing and qualified standing/walking animation across the covered bedroom and adjoining house room. NPC dialogue is a later milestone.

## Dependencies

- [Start a new game and explore Ark's house](start-and-explore-arks-house.md)
- [Qualify Ark's house sprite assets](qualify-ark-sprite-assets.md)
- [Qualify Ark's standing and walking animation](qualify-ark-walking-animation.md)

## Requirements

- Decode only the player graphics, palette, frame composition and ordering needed by the existing ordinary-house slice.
- Derive facing/animation selection and timing from source and input-only reference captures, not assumed generic walking cycles.
- Keep simulation device-free and rendering separate. Preserve the existing movement, passive collision, two-way doorway and New Game behavior.
- Make unsupported actions, scene effects and transition-animation timing explicit; do not expand into combat, NPCs or a general animation VM.
- Keep ROMs, extracted sprites, palettes and reference images local/ignored.

## Acceptance Criteria

- New Game displays Ark standing at the source-derived bedroom position.
- Ordinary cardinal walking, turning, blocking and release display qualified facing and animation; selected poses/sequences compare with reference captures.
- The actual browser renders Ark through both covered rooms without breaking the 511-step house route.
- Synthetic decoding/state tests, authenticated visual/animation fixtures, snapshots, native/Wasm builds and independent reviews pass.

## Notes

- Child of the graphics/animation and player-input/movement parent issues.
- Existing BG1-only scene limitations remain unless a small player-specific ordering requirement demands otherwise.
