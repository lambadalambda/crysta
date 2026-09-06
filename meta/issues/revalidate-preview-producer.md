# Revalidate the preview producer without renewing observation pins

## Summary

Registering ROM-only preview modules changes the whole-file map-inspector producer source pin. Revalidate the new producer within headless-sync-video-v1 while preserving the original observer migration evidence and all output pins.

## Dependencies

- [Render bounded Pandora world patches](render-pandora-world-patches.md)
- [Renew map-inspector fixture for completed video publication](renew-map-inspector-observer-fixture.md)

## Requirements

- Authenticate the old main source and exact reviewed module-registration-only delta; no whitespace normalization or arbitrary line stripping.
- Retain original observer.json, migration.json, threaded epoch archive and producer envelopes unchanged. Separate current producer identity from historical migration authentication explicitly.
- Fresh independently built producer and two new singleton processes use the exact owned SRAM/input/capture schedule. Compare all ten files and complete manifests/nonpixel evidence against accepted fixed roots and each other.
- Same epoch/policy; no output pin change or generic producer registry. Additional source hashes bind new module hooks without retroactively changing historical inventories.
- Reject identity substitution and any non-registration main change; no weakened whole-file source gate.

## Acceptance Criteria

- Exact source-delta proof, retained private build/process evidence and no-output-change bridge pass independent review.
- Historical normal/optimized audits still reproduce; current Rust source/capture gate and focused negative tests pass.
- Parent independently reproduces checks before archive. Future additional module registration requires explicit revalidation, not an implicit wildcard.

## Progress

- Historical audit separation implemented with a red→green explicit-source API
  control. Both normal/optimized retained audits byte-reproduce frozen
  `migration.json`; 14 tests per mode and all 20 existing mutations pass.
- Independently hashed old worktree `8034889` main bytes:
  `7736b543c442e6e4c2789fb13f6f177d1335e78f11810d5023a49c313b27a4d3`.
- Historical split reviewed independently before signed commit `5a4d742`.
- Proposed bounded bridge tooling has 12 normal/optimized ROM-free tests:
  authenticated exact-byte insertion, identity substitution, non-registration
  main changes even when resealed, all historical/additional source pins,
  current process provenance, every capture file and capture alias rejection.
  Four mocked recorder tests cover isolated targets, singleton invocation,
  existing-output refusal and post-capture source/descriptor rejection. Independent
  static review's literal-registration and orchestration coverage findings were
  addressed. Both historical audits and all original mutation checks still pass.
- Current capture mode requires explicit descriptor/fixed-source flags and uses
  a fresh private target for each of two new processes. Final real execution and
  Rust source-gate switch remain pending the parent's final registration relay;
  no descriptor or final bridge report has been pinned prematurely.
- Private `local/provisional-main-proof.json` authenticates old Git blob/worktree,
  exact current navigation-only delta, ROM and 8,192-byte SRAM. `local/historical.json`
  and `local/historical-O.json` reproduce the frozen report. No production, shared
  index or frontend files changed; only private input symlinks added under local.
