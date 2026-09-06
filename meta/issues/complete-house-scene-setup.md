# Complete the fresh house scene with all residents

## Summary

Set up the whole fresh-game house correctly rather than displaying only Ark and one resident. Establish the actual room/actor inventory first; do not assume every linked entity is a visible NPC or that every house exit leads indoors.

## Dependencies

- [Render the first NPC in the adjoining house room](render-first-house-npc.md)
- [Qualify the fresh house room and actor roster](qualify-house-scene-roster.md)
- [Decode the complete ordinary house actor set](decode-house-scene-actors.md)
- [Qualify navigation through all fresh house rooms](qualify-house-room-navigation.md)
- [Qualify complete house background profiles](qualify-house-background-profiles.md)

## Requirements

- Account for all rooms, visible residents and scene objects belonging to the fresh house, using source data and fresh input-only references.
- Render all admitted residents with correct source-derived positions, ordinary poses, palettes, transparency and ordering; account for any missing scene objects needed by the setup.
- Preserve walking, the qualified doorway pair, deterministic snapshots and CPU-free simulation. Qualify additional interior transitions separately if the inventory requires them.
- Distinguish correct initial/frozen presentation from unsupported AI, conversations, NPC collision, story-event progression and full native scene effects. No guessed placements or copied framebuffer assets.

## Acceptance Criteria

- A documented room/actor inventory explains every included resident and every excluded nonvisual/controller entity.
- The actual browser shows the complete qualified house roster, with correct membership on room changes and source-backed scene composition.
- Source/native asset comparisons, synthetic tests, authenticated walking/house regressions, pixel checks, native/Wasm builds and independent reviews pass.

## Census result and admitted scope

Structural scenes are B,C,D,E,F,10,11,20,21. The fresh playable pass admits B,C,D,F,10,11 with nine residents plus the F table object; E/20 copies are rejected by fresh gates and21 is phase-dependent. D/A exterior, C/E cellar and exceptional F/122 remain gated/unqualified. See [house census](../../docs/house-scene.md). Frozen ordinary setup is explicit for resident presentation; no NPC AI/dialogue claim.

## Completion

- All six admitted fresh rooms B,C,D,F,10,11 now render their complete source roster: nine frozen setup residents plus the F table object. Stable actor IDs, source poses/palettes/mirroring and native world-Y/tie ordering are compiled once; the first NPC raster remains unchanged.
- Source-derived collision/camera profiles, internal doorway graph and persistent C/B wooden-door collision/visual patch are integrated. Exterior/cellar gates and exceptional scene exits retain the documented limits. No NPC AI/dialogue or original CPU execution enters the simulation loop.
- Parent independently reran census, backgrounds, full actor art and navigation qualifications. Two identical actual-browser2,244-step runs checked2,245 full canvases,15 checkpoints, all six maps and1,626,662 visible resident/table pixels each; all ten actors had nonvacuous evidence. The original511-step route passed512 canvas checks with its expected endpoints.
- 287 workspace tests with required walking/admission/material/fresh-house fixtures, strict workspace Clippy, Wasm build, UI/browser-verifier tests, source-negative controls, safety/tracker checks and independent final integration review passed.390px layout has no overflow; browser errors are empty.
- See `docs/playable-house.md` for reproduction and explicit frozen-presentation/secondary-effect boundaries. Raw outputs remain ignored under `local/`.
