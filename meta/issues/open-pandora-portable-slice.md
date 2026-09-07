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

<a id="parent-acceptance"></a>
## Parent acceptance

Accepted after the continuous source-qualified New Game → required interactions →
Pandora opening → mandatory first tour → restored player control route. The core
executes no original CPU code. Browser evidence:11,409 actual UI inputs and11,410
full-canvas checks, all34 direct invocations, final map`$0041` at(136,208); the181
omitted blocking-dialogue no-ops have a separately replayed canonical proof.
All18 visible-unready arrivals and all manual actions remain.

- [Cadence proof](../../docs/pandora-cadence.md) and
  [browser acceptance](../../docs/pandora-browser.md) retain exact identities,
  evidence boundaries and source-composition limits.
- Parent independently reproduced historical/current producer audits in normal
  and optimized Python, all31 tests per mode and20 mutation controls. The
  [same-output bridge](../../tools/map-inspector-qualification/README.md) preserves
  every original observation pin and all ten files; other legacy wrappers remain
  audited but unrenewed.
- Final `cargo test --locked --workspace` passed with all five private core fixture
  suites enabled, including the current producer gate and fresh local capture.
  Strict locked workspace/all-target Clippy, room-core Wasm build,32 browser-helper
  tests, repository safety and tracker checks passed. Private logs:
  `local/map-research/pandora-final-{workspace,clippy,wasm,browser-tests}.txt`;
  bridge reproduction: `local/map-inspector-preview-parent/`.
- Existing house511/2244 and conversation/exterior1671 real-browser regressions
  remain accepted; the post-readiness1671 run also passed. Both profile lifecycle
  tests preserve checkpoint/New Game/reset and immutable art.
- Reviewed `serve-room` enablement is live at `http://127.0.0.1:8765/` after final
  gates. Actual controls passed enabled checkpoint, New Game, DOM-acknowledged
  neutral, exact checkpoint reset and repeat New Game smoke checks. Left paused
  at fresh tick0. Private evidence: `local/map-research/pandora-live-{smoke,neutral}.json`.
- Independent component closure review found no remaining bounded gameplay/render
  blocker; source, compiler/core, host/UI, browser and producer changes each have
  scoped correctness/architecture reviews.

This completes the Pandora milestone, not M4 or the full opening vertical slice.
No equipment, world return, tower arrival, combat, audio or production Wasm
frontend is claimed. The direct34 browser route does not qualify optional refusal/
retry playback, native scheduler timing or whole-screen native RGB equivalence.
