# Verify the continuous Pandora browser journey

## Summary

Own the bounded input-only New Game → final controllable map41 browser verifier.
Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).

## Dependencies

- [Port the bounded Pandora route and sequence](port-pandora-sequence.md)

## Requirements

- Only actual UI controls may advance the browser; GET inspection is allowed.
- Project the fixed offline route only by omitting proved blocking-dialogue no-op
  movement/neutral actions. Visible-but-unready dialogue must retain normal arrival
  updates and keep text painted, with acknowledgement/choices disabled. Preserve
  every manual action, recovery, ordinary cue tick, source invocation and reload.
  Distinguish projected ticks/snapshot identity.
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
- Parent resolves the source-backed C/Box arrival conflict through read-only
  `dialogue_ready`, preserving visible requests and the core timeline. The
  verifier adapts only blocking/ready input and inspection checks; no production
  or cadence-producer ownership transfers. See
  [verifier contract/evidence](../../docs/pandora-browser.md).
- Full acceptance remains gated on the independently qualified canonical proof
  and updated isolated host/UI relay. No further input to8877 until then;904 is
  historical partial evidence only. Parent's181/18 observational counts are not
  assumed omission permissions. Final producer pin remains held.
- All17 synthetic tests (32 combined) pass after red→green readiness adaptation.
  Bitmap-load timeout remains a deferred nonblocking robustness follow-up.
- Parent reports shared issue-index and roadmap registration complete; these
  files remain outside this worktree's owned edits.
