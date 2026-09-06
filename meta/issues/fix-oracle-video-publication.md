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
