# Renew map-inspector fixture for completed video publication

## Summary

The existing SRAM-based local capture integration test has an old threaded-observer RGB pin. Independently qualify its own unchanged input/capture recipe under the completed-publication policy, without treating the empty-SRAM Pandora route as substitute evidence.

## Dependencies

- [Make oracle video publication deterministic and race-free](fix-oracle-video-publication.md)

## Requirements

- Authenticate the existing slot3 SRAM/ROM, producer policy and exact no-save 1601/1841 observation schedule.
- Reproduce old/new complete capture evidence and repeat the fixed output. Require complete WRAM/VRAM/CGRAM equality, not merely player/map assertions, before reviewing pixel epoch changes.
- Preserve old evidence metadata; explicitly version observer provenance and only update independently reviewed RGB pins. Do not change route, conversion, saves or suppress test assertions.
- Keep raw inputs/captures/binaries ignored. Other legacy wrapper renewal is separate work, not implied by this fixture.

## Acceptance Criteria

- Exact fixed runs agree, non-pixel invariants hold, and migration evidence is independently reviewed.
- Focused negative controls and existing `map-inspector` local-capture integration pass without skipped owned inputs or weakened checks.

## Notes

- Parent audit: `tools/pandora-qualification/OBSERVER-MIGRATION.md`, map-inspector integration section. Pre-renewal test failed old `93a224...` versus observed `3833db...` at its second RGB checkpoint; one failure message alone was not renewal evidence.

## Bounded renewal evidence

- Old producer reproduced at `8034889` (`0db0aad^`); fixed twins built from
  `7c5c90b`. The capture driver is byte-identical between these sources. Owned
  slot3 SRAM/ROM are authenticated; no production code or shared index changed.
- Complete private captures/manifests/binaries/build records:
  `/Users/lainsoykaf/repos/ilar-task-capture-renewal/local/map-inspector-renewal/{old,fixed-a,fixed-b}`.
- Byte-preserved old test and hash-only evidence metadata are archived under
  `tools/map-inspector-qualification/epochs/threaded-video-v0/`.
- Independent static architecture review approved the bounded tooling; a separate
  execution-capable reviewer recomputed all inventories, decoded all six BMPs,
  compared all 394,240 nonpixel surface bytes per run, and reran all three retained
  producers. Only the second RGB changes; complete fixed captures agree.
- Reviewed renewal and reproducible commands:
  [`tools/map-inspector-qualification/README.md`](../../tools/map-inspector-qualification/README.md).
  Active frame1841 RGB is now `3833db...`; all original assertions remain, with
  whole-nonpixel-manifest and complete observer-source gates added.
- Validation: full owned-input `cargo test --locked -p map-inspector` passes
  (49 unit + 10 integration); Python gates pass 13 tests in each mode; all ten
  targeted mutations are detected normally and under `-O` (20/20).
- Parent owns index closure/integration. Other legacy wrappers remain unrenewed.

## Parent acceptance

- Parent independently recomputed the retained old/fixed/twin audit normally and optimized; both outputs exactly match the reviewed `migration.json`. Reports: `local/map-inspector-renewal/parent-audit{,-O}.json`.
- All 13 validator tests pass normally/optimized and 20/20 mutations are detected. The rebuilt parent full `map-inspector` suite passes with owned inputs: 51 unit + 10 integration tests, including actual native RGB/nonpixel capture validation. Log: `local/map-research/pandora-parent-full-host.txt`. The initial 120-second orchestration deadline interrupted a later long opening test; the complete rerun with an adequate deadline passed, rather than counting the interrupted run as green.
- A trivial formatting-allocation lint in the new test hash helper was corrected without changing pins/assertions; local capture tests and strict workspace Clippy pass afterward.
- Reviewed scope and all nonpixel/input/save/conversion invariants remain unchanged. This supersedes the earlier red SRAM-fixture status in the Pandora observer audit and component handoffs. Other legacy wrappers remain explicitly unrenewed.
- Issue archived; this fixture renewal does not enable portable Pandora gameplay.
