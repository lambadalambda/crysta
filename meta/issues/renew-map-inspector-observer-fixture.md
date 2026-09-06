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
