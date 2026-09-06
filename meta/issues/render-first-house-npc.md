# Render the first NPC in the adjoining house room

## Summary

Make the covered house feel inhabited by displaying one source-derived NPC in room $0010. This is a bounded rendering feature and groundwork for later conversation, not an NPC AI or dialogue implementation.

## Dependencies

- [Render Ark standing and walking in the portable house](render-ark-house-sprite.md)
- [Decode graphics, palettes, sprites, and animation](decode-graphics-animation.md)

## Requirements

- Identify an appropriate room10 NPC from ROM spawn/script data, corroborated by fresh input-only reference captures.
- Decode its required ordinary pose, graphics, palette, placement and scene ordering from ROM sources; do not ship captured pixels or RAM-derived initialization.
- Make any frozen ordinary-pose policy explicit. Do not invent NPC movement, collision, conversation or event behavior.
- Keep the player core device-free and preserve the supported New Game/two-room route.

## Acceptance Criteria

- The actual browser shows the qualified NPC in room10 and removes it on returning to the bedroom.
- Selected source-to-reference placement/composition evidence and synthetic decoding/render tests pass.
- The existing511-step house route, sprite occlusion checks and deterministic snapshots remain valid.
- Unsupported behaviors and raw-local evidence boundaries are documented; independent review passes.
