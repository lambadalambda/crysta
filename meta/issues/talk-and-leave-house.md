# Talk to the room B resident and leave the house

## Summary

Deliver the next continuous playable journey: interact with the room B resident, advance the required source dialogue, commit the progression change and leave through the formerly gated house exterior. Favor semantic progression over incidental presentation fidelity.

## Dependencies

- [Complete the fresh house scene with all residents](complete-house-scene-setup.md)
- [Qualify the room B conversation and exterior progression](qualify-house-conversation-progression.md)
- [Decode the required opening dialogue presentation](decode-house-conversation-text.md)
- [Qualify the first exterior landing profile](qualify-house-exterior-profile.md)

## Requirements

- Source-derived interaction admission and request/choice semantics: grant event0026 after the first request completes, before its choice/follow-up, never merely on room entry or interaction start.
- A small serializable semantic event runner, with only required dialogue/flag/control operations; no original CPU execution or general native NPC scheduler in the simulation loop.
- A ROM-derived readable presentation of the required dialogue; simplified layout/pacing is acceptable, invented dialogue is not.
- Source-derived gate change, exterior transition and bounded destination landing/walking. Explicitly retain unqualified boundaries.
- Preserve the six-room house, wooden door, walking, frozen residents and deterministic snapshots.

## Acceptance Criteria

- A fresh input-only reference journey establishes conversation state changes and the exterior transition; retained fixtures support fast development checks.
- A CPU-free browser journey talks, advances dialogue, changes the gate and reaches the exterior with correct semantic checkpoints.
- Movement is locked appropriately during dialogue; acknowledgement, interruption/reset and snapshot continuation are deterministic.
- Focused red/green tests, existing house regressions, native/Wasm builds and independent reviews pass. Full visual/audio fidelity remains outside this milestone.

## Implementation notes

- Groundwork: the dependency-free `events` module runs bounded linear `ShowPage`/`SetFlag` sequences, with canonical page-wait cursors and the source-compatible 64-byte flag block. Effects after a page cannot run before its acknowledgement. Native opcode decoding, sequence selection and full game-state ownership are separate integration responsibilities.
- Browser groundwork accepts immutable bounded dialogue page rasters and a capability-gated one-shot acknowledgement command6. Dialogue pauses movement; no held direction, neutral step, key repeat or queued action auto-advances pages. Missing page art hides the panel and fails visibly.
- These APIs are not yet enabled by the host. Source conversation/gate qualification is complete and independently replayed; text and exterior assets remain separate integration gates. The source requires real two-option choices and a grant before the first choice, not after all follow-up pages.
- Synthetic red/green tests, required existing core/host fixture regressions, strict package Clippy, Wasm core build and independent runner/controller/panel reviews passed. This issue remains open until the real talk-and-leave journey is integrated and verified.
