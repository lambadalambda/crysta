# Start a new game and explore Ark's house

## Summary

User goal: start the game and walk around Ark's house through the portable implementation. The existing saved-checkpoint semantic preview is a baseline, not completion evidence.

## Dependencies

- [Implement a reference-qualified portable room slice](portable-room-slice.md)
- [Qualify a fresh new-game house bootstrap](qualify-new-game-house-bootstrap.md)
- [Qualify repeatable house movement and collision](qualify-repeatable-house-movement.md)

## Requirements

- Reach controllable Ark from an authenticated Japanese-ROM fresh new-game reference replay, without supplied SRAM, RAM patches or forcewarps.
- Implement the required portable startup/state initialization and an explicit user-facing Start/New Game flow, without original CPU execution in the simulation loop.
- Support repeatable ordinary exploration of the covered house rooms, including normal release/repress, turns, furniture/wall collision and required internal doorways. Do not retain the old one-activation-per-direction restriction as goal completion.
- Keep actual unsupported actions, events, timing and presentation gaps explicit. Do not broaden this into the whole Crysta/Pandora opening.
- Retain a usable local frontend and document the precise covered house boundary.

## Acceptance Criteria

- An automatic fresh-process reference replay reaches named new-game and controllable-house checkpoints twice identically.
- An automatic portable end-to-end test starts a new game and follows a repeatable house exploration route, including revisiting a direction/area and using covered internal doorways, with qualified state comparisons.
- A browser test exercises Start/New Game and the exploration route against the actual CPU-free host, with no scope-error pause on the required route.
- Native and Wasm core builds, synthetic tests, authenticated fixtures and independent reviews pass.
- All raw ROM-derived artifacts remain ignored. Each progress report identifies concrete checks passed and remaining gaps; the goal remains open until these criteria pass.

## Notes

- Parent: [opening vertical slice](opening-vertical-slice.md).
- New-game input confirmation was previously unqualified, not a proven ares/game-script stall.
- Existing preview starts from slot 1's saved room-F checkpoint; it has endpoint-only doorway pacing and fails closed on reactivation, mixed pairs, type16 and flagged cells.
