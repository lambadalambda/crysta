# Open Pandora’s Box in the portable slice

## Summary

Extend the accepted talk-and-leave slice into one continuous, source-qualified New Game → Pandora opening → regained-control journey. Prioritize required story progression over full-town simulation or presentation polish.

## Dependencies

- [Talk to the room B resident and leave the house](talk-and-leave-house.md)
- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Port the bounded Pandora route and sequence](port-pandora-sequence.md)

## Requirements

- Establish the actual required interactions, rooms, story flags and post-box control checkpoint from a fresh owned-ROM reference journey; no guessed prerequisites.
- Implement only the admitted route, interactions, dialogue and immediate persistent world/player changes, with no original CPU execution in the portable simulation loop.
- Preserve the accepted house/conversation/exterior regressions, deterministic snapshots and explicit unsupported boundaries.
- Keep any source assets, screenshots and raw traces local and ignored.
- Run the existing bounded Wasm feasibility issue independently; do not turn it into a full frontend rewrite or a dependency of story qualification.

## Acceptance Criteria

- A reproducible fresh reference route establishes prerequisite ordering and Pandora’s immediate state changes through regained control.
- A continuous input-only portable/browser replay reaches the same declared semantic checkpoint from New Game; it does not teleport, inject state or execute original CPU code.
- Required source assets and state changes have focused positive/negative tests, deterministic restored continuations and documented fidelity limits.
- Existing house/conversation regressions, native/Wasm checks and independent correctness/architecture reviews pass before closure.

## Notes

- First-tower arrival and combat are follow-up milestones, not additions to this issue.
- Full Crysta exploration, incidental NPC wandering, audio, general event-VM work and unrelated static-export help cleanup remain out of scope unless required by qualified source behavior.
