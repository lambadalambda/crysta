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
- At goal start, the preview used a saved checkpoint and rejected reactivation/corners/partial/flagged terrain. That historical baseline is retained only under explicitly named checkpoint controls.

## Verified completion

- **Covered boundary:** source-derived default-name New Game, bedroomF and
  adjoining house room10, their shared doorway both ways, and ordinary revisits.
  Intro/dialogue presentation is explicitly omitted; marker/staticBG1 rendering,
  passive collision and endpoint-only doorway timing remain declared limitations.
- Fresh `Session::new` replay passed twice with matching13 checkpoints/7,100-row
  stream, both negative controls rejected, and22 native source-writer stops
  authenticated. A separate fresh house replay passed twice with661 identical
  captured rows and ownership/grid/event pins.
- `local_house` launches `verify-house ROM semantic-preview` in two fresh
  CPU-free processes. The separately required authenticated-route test compares
  all441 ordinary walking/stream steps while preserving core state across70
  semantic doorway updates, ending tick511, F392,191. Initial state is
  ROM-queue-derived304,112—not the saved472,176 start or captured WRAM.
- `tools/verify-house-browser.js` clicked actual New Game, then exercised real
  keyboard bindings/request serialization through both doors and repeated
  Right/Left walking. Two runs produced identical nine-checkpoint/final JSON;
  error=null at tick511. Mobile390px startup has no horizontal overflow.
- Full workspace tests with required fixtures, strict workspace Clippy,
  native/Wasm builds, formatting, Node UI tests and safety/tracker gates pass.
  Independent source/compiler/core/UI/integration reviews found no blockers.
- Implementation: `0e36d82`; [run instructions, exact route and limitations](../../docs/playable-house.md).
  Broader opening, additional house exits/interiors, sprites/NPCs, combat/audio
  and direct browser Wasm hosting remain outside this completed bounded goal.
