# Qualify the room B conversation and exterior progression

## Summary

Establish source interaction targeting, dialogue requests/control flow, event0026 completion semantics, D exterior gate lifetime and the first exterior transition endpoints from a fresh input-only journey.

## Dependencies

- [Complete the fresh house scene with all residents](complete-house-scene-setup.md)

## Requirements

- Bounded subissue of [talk and leave the house](talk-and-leave-house.md).
- ROM/source-driven compilation, no capture-seeded production data or original CPU in simulation.
- Independent correctness/architecture review before substantial commits.

## Acceptance Criteria

- Source metadata and a reproducible fresh route pin semantic before/during/after conversation checkpoints, negative early-grant controls, gate change and transition endpoint. Do not implement portable core or presentation.

## Completion

- Source resident838B96 callback888EDE, acknowledgement/control targeting, first/repeat two-option catalogs and all branch routing are qualified. Event0026 is granted after the first request finishes, BEFORE its choice/follow-up; the runner must not delay it until all conversation pages finish.
- D gate838CC8 is created/stamped only at room initialization while0026 is clear; it does not subscribe to live flag changes. Post-conversation D reload omits cell1415 occupancy. Exterior A initializes Ark504752 and settles504769; a real Down/Right walking segment is retained.
- One fresh empty-SRAM discovery journey,31 semantic checkpoints and16 checker mutation tests passed, including normal/optimized checks and independent review. Parent separately repeated the single fresh journey successfully at ignored `local/house-conversation-qualification/replay-OYXAH7/journey`.
- Reproduction, source pins and explicit evidence limits are in `docs/house-conversation.md`. Text/font, core/UI and exterior asset integration remain open in the umbrella issue.
