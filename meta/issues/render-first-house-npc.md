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

## Verified result

- Completed: source record`$83:8D7C` produces room16 position424,416 and the ordinary Right-facing pose. One16×33 resident raster is decoded from ROM; palette base208 and priority2 remain explicit.
- Two fresh empty-SRAM boots and four settled samples agree on native composition/OAM,256 graphics tiles,16 palette words and313/313 opaque framebuffer pixels. Integrated replay: `local/house-npc-qualification/replay-QNlHv5`. See [source/evidence contract](../../docs/house-npc.md).
- Native host emits a source-ordered scene list; equalY gives Ark precedence. Browser checks both room membership and actual canvas pixels. Two511-step runs agree with512 canvas checks each,99 NPC-visible states and30,987 NPC pixels per run. The reverse/tie ordering branches are synthetic tests, not native-overlap claims.
- Full workspace fixture tests, strict Clippy, Wasm, Node/browser tests, normal/optimized qualification checks, safety/tracker checks and independent reviews pass. Initial/final player snapshot hashes are unchanged.
- Frozen ordinary presentation only: NPC collision, dialogue, AI and event behavior remain unimplemented. Shared large-component column15 wrapping is outside the selected pose and is explicitly deferred.
