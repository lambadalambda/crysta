# Port the bounded Pandora route and sequence

## Summary

Implement only the maps, source assets and semantic state transitions required by the qualified Pandora route, preserving the CPU-free core and existing preview.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Talk to the room B resident and leave the house](talk-and-leave-house.md)

## Requirements

- Derive the scope from the qualified source contract rather than assumed story order. Split substantial map/asset/mechanic work into focused child issues after discovery.
- Admit only required exterior traversal/re-entry, rooms, interactions, dialogue and immediate world/player changes through regained control.
- Keep semantic progression in the dependency-free device-free core, with immutable source-derived data and versioned aggregate identity/snapshots.
- Extend event operations only when the qualified sequence needs them; no general native event interpreter, scheduler or CPU fallback.
- Retain actual UI actions, readable ROM-derived text, explicit choices and visible unsupported boundaries.

## Acceptance Criteria

- Focused red → green tests cover required progression, missing/reordered prerequisites, interruption/reset, persistent effects and restored continuation.
- A continuous real-browser New Game-to-Pandora journey matches the admitted reference semantic checkpoints and regains control.
- House511/2244 and conversation/exterior1671 routes remain valid, or any intentional source-backed contract revision has explicit replacement evidence.
- Native/Wasm builds, workspace tests, lint, browser checks, repository safety/tracker and independent correctness/architecture review pass.

## Notes

- Parent: [Open Pandora’s Box in the portable slice](open-pandora-portable-slice.md).
- Do not enable an unqualified placeholder sequence while source discovery is incomplete.
