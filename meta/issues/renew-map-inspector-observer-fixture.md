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

- Parent audit: `tools/pandora-qualification/OBSERVER-MIGRATION.md`, map-inspector integration section. Current test fails old `93a224...` versus observed `3833db...` at its second RGB checkpoint; one failure message alone is not renewal evidence.

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
- Parent owns index closure/integration. Other legacy wrappers remain unrenewed.
