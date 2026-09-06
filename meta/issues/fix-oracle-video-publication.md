# Make oracle video publication deterministic and race-free

## Summary

Independent Pandora replays match the complete frame log, machine state, non-pixel captures and provenance, but a few framebuffer captures intermittently contain zeroed regions. The project-authored shim writes `lastFrame` from ares’s video worker while `snes_setPixels` reads it without synchronization. Fix this observer defect before treating intermediate framebuffer captures as uniform fidelity evidence.

## Dependencies

- [Reference core](switch-reference-core.md)

## Requirements

- Remove torn publication and define a deterministic completed-frame observation boundary; a mutex that merely changes the race into nondeterministic old/new-frame selection is not sufficient.
- Prefer a small headless-oracle configuration/publication change. Audit callback completion, loading/shutdown and pixel format/dimensions; do not introduce a renderer or alter original game scripts/inputs.
- Preserve native machine execution and the exact qualifying command/save synchronization schedule. Compare all non-pixel state, not just final positions.
- Keep strict reference checks. If the corrected observer changes pixel epoch/provenance, document/version and independently requalify that boundary rather than adding ignored mismatch paths or blindly refreshing goldens.
- Preserve license/provenance records for any vendored configuration patch. The playable portable core and live preview remain unchanged.

## Acceptance Criteria

- A focused failing test/stress witness demonstrates the publication problem or missing deterministic policy; corrected tests pass under the supported lifecycle.
- Multiple fresh same-input oracle runs produce identical complete framebuffer captures at the declared boundary and unchanged qualified non-pixel state.
- Existing affected oracle/asset regressions are run; any observer-induced fixture changes are explained and reviewed separately from the synchronization fix.
- Independent correctness/concurrency/architecture review approves the fix, source/provenance policy and reproduction evidence.

## Notes

- Parent: [Open Pandora’s Box in the portable slice](open-pandora-portable-slice.md).
- Diagnosis: `tools/pandora-qualification/replay-diagnosis.json` and [source contract](../../docs/pandora-progression.md). Precise per-capture interleavings remain unproven; the unsynchronized access is established from source and same-binary nondeterminism.
- This is a reference-observer fix, not evidence of a Pandora game-script or emulator CPU failure. Intermediate `.pixels` files are not yet uniformly trustworthy; corresponding WRAM/VRAM/CGRAM/OAM artifacts match.

## Bounded fix and verification

- Headless-only `ARES_ORACLE_SYNCHRONOUS_VIDEO=1` selects the existing inline
  Screen publication path; upstream default stays threaded. Build + both project
  translation units enforce it. The sole upstream edit is a marked ISC config
  conditional, not a Screen/PPU algorithm rewrite.
- [Publication contract, lifecycle audit and reproduction](../../docs/oracle-video-publication.md)
  define `headless-sync-video-v1`: completed last `run_frame` flush, not the later
  synchronized save-state instant. No execution/input/capture schedule changed.
- ROM-free red/green policy regression and 512-frame actual-Screen stress pass,
  including caller-thread/completion checks, repeated quit and direct destruction.
  Oracle, asset and affected qualification tests were run; precise pass/skip and
  expected strict-gate failure results are in the publication document.
- Two fresh routes in the isolated oracle-video worktree, under
  `local/oracle-video-qualification/replay-Tk2j5r/{a,b}/journey`, reproduce all
  2,682 complete artifacts and the full log exactly. All 2,299 non-pixel files
  and complete logs equal each of the three old routes. Old pixel differences
  are 171/174/173 (source/parent/old-same-binary); final stable controls still match.
  [Public hashes/counts](../../tools/oracle-video-qualification/evidence.json)
  authenticate the producer/config and private exhaustive comparison reports.
- Independent source audit approved the synchronous design and supported
  lifecycle. Separate execution-capable final review approved the diff/evidence,
  independently rehashed/recompared every report and reran the new tests before
  the signed implementation commit. No production changes were requested.

**Still open for parent acceptance/migration:** the frozen Pandora checker fails
in normal and optimized Python on both fresh runs at `exit-trigger.pixels`, before
final semantic validation. No old reference or checker was changed. Parent/source
owner must independently repeat, then review/version observer provenance (include
build/unity/config header) and requalify affected prefix/selected pixel fixtures.
Stable observer reproducibility here is not an accepted portable RGB fidelity
claim. Shared indices/milestones, game assets/core/host and preview were untouched.
