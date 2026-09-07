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

## Acceptance evidence

- **Continuous browser acceptance passed** on the parent's authorized8877 host
  in named session `pandora-journey-alice`, after reload and actual New Game.
  Signed verifier `b897899` emitted11409 UI inputs with11410 full-canvas checks;
  all projected GET fields/raw snapshot hashes matched, with187 checkpoints saved.
- The reviewed proof computed181 blocking-dialogue no-op omissions from11590
  offline actions. All18 visible-unready arrival updates and every manual action
  remained. No state injection, POST bypass, hidden request or forced ack occurred.
- All34 direct invocations, source grants, pot miss/two hits/recovery, persistent
  patches/reloads and final four-direction movement passed. Required backgrounds,
  door/pot patches and typed carry/flight had nonvacuous visible-pixel evidence.
- Final UI tick11409: map`$0041`,(136,208), owner/player and walking, no dialogue or
  error. Raw snapshot SHA256:
  `8aab7a37cb0c1115438e8f177c7213fabb0cee14f94d7eaed0cf58635c7484d5`.
  A subsequent GET and paused DOM independently matched the saved endpoint.
- Complete private result: `local/pandora-browser/full-result.json`; proof/input
  pins and audit are documented in [browser acceptance](../../docs/pandora-browser.md).
  The earlier904 prefix remains historical failed/partial evidence, not a pass.
- All17 synthetic tests (32 combined) pass. Bitmap-load timeout remains a deferred
  nonblocking robustness follow-up, not part of this acceptance claim.
- Parent owns final producer pinning and shared index/archive follow-through;
  no production, parent issue or shared-index files were edited here.
- The [test-only canonical proof](../../docs/pandora-cadence.md) was accepted
  after independent source/log review and parent clean debug/release reproduction,
  full-array comparison and actual artifact/hash/Git-linkage checks. Continuous
  fresh raw-core mirrors reject concealed input failure; only the global tick is
  erased in read-only continuation hashing. The pinned proof is expectation only.

## Parent acceptance

Parent reproduced the complete post-run audit and verified the exact served HTML
and result bytes; private browser evidence is also retained in parent
`local/pandora-browser/`. Final producer/workspace gates and live startup smoke
checks pass; see [umbrella acceptance](open-pandora-portable-slice.md#parent-acceptance).
The accepted journey remains one continuous input-only run, not an additional
native replay. Bitmap-load timeout hardening remains optional and out of scope.
