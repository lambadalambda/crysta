# Verify the continuous Pandora browser journey

## Summary

Own the bounded input-only New Game → final controllable map41 browser verifier.
Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).

## Dependencies

- [Port the bounded Pandora route and sequence](port-pandora-sequence.md)

## Requirements

- Only actual UI controls may advance the browser; GET inspection is allowed.
- Project the fixed offline route only by omitting proved dialogue-paused no-op
  movement/neutral actions. Preserve every manual action, recovery, ordinary cue
  tick, source invocation and reload. Distinguish projected ticks/snapshot identity.
- Use source-derived fresh-game replay provenance, never inferred flags, injected
  states, disabled dialogue pauses or native-frame timing claims.
- Check source dialogue, all34 direct invocations, flags/control ownership, final
  movement and nonvacuous background, world-patch and carry pixels.
- No production, shared-index or parent issue edits. Parent owns isolated host
  enablement and final HTML pinning; never use the parent session or live8765.

## Acceptance Criteria

- Red → green synthetic tests cover projection/control/composition failures.
- Independent correctness and compactness review precedes signed topical commits.
- An isolated enabled host passes the retained-promise real-browser journey.

## Notes

- Open: isolated8877 enabled; actual New Game and904 continuous UI commands /
  independent canvas checks matched the parent fresh replay's full states and raw
  snapshots. Stopped at first paused movement905 without omitting any input.
- Full acceptance is blocked by the missing canonical continuation/fresh projected
  replay proof and source-backed C/Box dialogue-before-arrival deadlocks. See
  [verifier contract/evidence](../../docs/pandora-browser.md).
- All13 synthetic tests (28 combined) pass; independent correctness/compactness
  review approved after a red→green initialization-failure fix. Bitmap-load timeout
  is a deferred nonblocking robustness follow-up.
- Shared issue-index and roadmap registration are deliberately parent-owned per
  assignment; tracker currently reports exactly those two membership omissions.
