# Port the bounded Pandora route and sequence

## Summary

Implement only the maps, source assets and semantic state transitions required by the qualified Pandora route, preserving the CPU-free core and existing preview.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Talk to the room B resident and leave the house](talk-and-leave-house.md)
- [Decode the required Pandora route backgrounds](decode-pandora-backgrounds.md)
- [Decode the required Pandora progression dialogue](decode-pandora-dialogue.md)
- [Qualify and implement bounded cellar pot actions](qualify-pandora-pot-actions.md)
- [Decode the required Pandora actors and carrying poses](decode-pandora-scene-art.md)

- [Support the bounded Pandora event flag projection](pandora-event-flag-projection.md)

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

- Remaining integration split: [navigation/contact admission](qualify-pandora-navigation.md) and [fixed story continuation](port-pandora-story-state.md). These are required beyond the independently decoded visual resources; neither enables an unqualified live route.

- Parent camera adapter has an explicit opt-in path for the eight additional maps
  and Town A, delegating directly to authenticated `SourceCamera` decoding and
  settled clamping. No duplicate geometry or in-flight camera-pan claim. Red→green
  tests cover all sectors, endpoint/clamp equality, source mutation rejection,
  house/A parity and unsupported-map rejection; three camera tests and workspace
  Clippy pass. Independently reviewed; existing host still uses the old path.

- Parent transport now reserves canonical command10 for native A/pot action, capability-gated Lift / throw (Z), separate from B/Interact/acknowledgements. Manual submission pauses and Resume advances delayed A/recovery; no automatic story advancement. TDD covers same-origin/two-byte parsing, segmented transport, repeat/held-input rejection, dialogue isolation and actual inline button dispatch. Live profile stays disabled pending the qualified aggregate compiler/route.
