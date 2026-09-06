# Qualify the fresh house room and actor roster

## Summary

Inventory the fresh post-intro house, replacing the single-NPC assumption with an authenticated account of rooms, spawn conditions, residents, props and nonvisual actors.

## Dependencies

- [Render the first NPC in the adjoining house room](render-first-house-npc.md)

## Requirements

- Decode the room/exit and actor-list sources; separate interior connectivity from the exterior boundary.
- Corroborate the selected fresh event/branch path, actor membership, positions, facing/pose/resource references and draw-list order through fresh input-only captures.
- Identify actors whose native movement, visibility or scene effects prevent a faithful frozen setup; report these explicitly instead of silently omitting them.
- No patches, forcewarps, snapshot restores or captured data as production initialization.

## Acceptance Criteria

- Source-linked roster and room boundary, with a disposition for every entity, are reproducible from two fresh boots.
- Required renderer/decoder contracts and any genuine scope blockers are documented.

## Notes

- Subissue of [complete house setup](complete-house-scene-setup.md).
