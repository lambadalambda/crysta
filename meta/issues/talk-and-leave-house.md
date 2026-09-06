# Talk to the room B resident and leave the house

## Summary

Deliver the next continuous playable journey: interact with the room B resident, advance the required source dialogue, commit the progression change and leave through the formerly gated house exterior. Favor semantic progression over incidental presentation fidelity.

## Dependencies

- [Complete the fresh house scene with all residents](complete-house-scene-setup.md)
- [Qualify the room B conversation and exterior progression](qualify-house-conversation-progression.md)
- [Decode the required opening dialogue presentation](decode-house-conversation-text.md)
- [Qualify the first exterior landing profile](qualify-house-exterior-profile.md)

## Requirements

- Source-derived interaction admission, dialogue requests and completion/flag semantics; do not grant event0026 merely on room entry or conversation start.
- A small serializable semantic event runner, with only required dialogue/flag/control operations; no original CPU execution or general native NPC scheduler in the simulation loop.
- A ROM-derived readable presentation of the required dialogue; simplified layout/pacing is acceptable, invented dialogue is not.
- Source-derived gate change, exterior transition and bounded destination landing/walking. Explicitly retain unqualified boundaries.
- Preserve the six-room house, wooden door, walking, frozen residents and deterministic snapshots.

## Acceptance Criteria

- A fresh input-only reference journey establishes conversation state changes and the exterior transition; retained fixtures support fast development checks.
- A CPU-free browser journey talks, advances dialogue, changes the gate and reaches the exterior with correct semantic checkpoints.
- Movement is locked appropriately during dialogue; acknowledgement, interruption/reset and snapshot continuation are deterministic.
- Focused red/green tests, existing house regressions, native/Wasm builds and independent reviews pass. Full visual/audio fidelity remains outside this milestone.
