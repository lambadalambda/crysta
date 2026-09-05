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
- Existing first-background-only (hardware BG2) scene limitations remain unless a small player-specific ordering requirement demands otherwise.

## Verified result

- Completed: New Game renders ordinary Down-facing Ark from the ROM; qualified cardinal standing/walking, blocked holds, turns, releases and mirrors are driven by deterministic core state. Both doorways retain the explicit standing/endpoint-timing policy.
- Twenty-eight exact normal/mirrored rasters are compiled once; high opaque background pixels occlude OBJ2. No copied framebuffer atlas or silent marker fallback.
- Two real-browser511-step routes agree, with512 full-canvas pixel comparisons and21 distinct keys per run; all28 exported rasters have ROM-backed tests. No scope errors, browser errors or390px overflow.
- Integrated source/animation fresh replays, full workspace fixture tests, strict Clippy, native/Wasm builds, Node tests and independent reviews pass. See [playable-house boundary](../../docs/playable-house.md).
- NPC dialogue, outdoor Crysta, direct browser Wasm hosting, idle gestures, shadows, scene effects and native doorway-animation timing remain outside this milestone.
