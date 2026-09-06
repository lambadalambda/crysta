# Decode the required Pandora progression dialogue

## Summary

Extend the bounded ROM text decoder and qualify every request/page/choice context required by the direct Pandora route through the final tutorial acknowledgement.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Preserve source page order, acknowledgement/auto-return boundaries, names and explicit choice catalog semantics; do not invent text or acknowledgements.
- Authenticate new control/font/layout behavior before admission. Existing house dialogue and its renderer transport remain compatible.
- Compare independently reconstructed pages and selected native font-cell pixels; distinguish source-only pages and finite presentation omissions.
- Keep raw text and font/bitmap exports ignored; commit only tooling, metadata and hashes.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.
- Implemented bounded immutable `assets::text::pandora::PandoraDialogue`: 32 resources / 74 fixed logical pages, 34 ordered direct invocations, retained choice contexts, exact D3/D5/D4 semantics, source-derived native glyphs/layout and default controller labels. Existing house API, fourteen pages and two catalogs remain compatible.
- [Dialogue contract and reproduction](../../docs/pandora-dialogue.md); new `tools/pandora-dialogue-qualification/reference.json` is the ordered request/page/ack manifest. Raw outputs remain ignored under `local/`; only tooling/metadata/hashes are committed.
- Independent complete source reconstruction passes. Original and parentfresh each compare 73 pages / 380,672 native font-cell pixels (97,371 foreground); discovery compares 65 / 343,680 (89,475 foreground) and supplies retry `$88B722`. Combined selected coverage spans all 74 pages, with per-set source-only inventories explicit.
- TDD red → green, Rust/owned-ROM regressions, normal/optimized Python tests and mutation controls pass. Independent static reviews approved correctness/architecture and pixel nonvacuity; review-found native orchestration coverage was added and its hash-guard mutation killed. Existing house native regression remains 67,904 matching pixels.
- Source boundary correction handed back to the parent: `$88AE50` is glyph `$56`, not an acknowledgement. Warning `$88ADF2` has only `$88AE29 D5` and `$88AE5E D3`. The outside-owned progression doc was not edited.
- Parent diagnosed intermittent `.pixels` corruption as the shim's async video-publication race (source `5b7b88b`, parent `42fe162`). This qualification strictly uses separately named WRAM/VRAM sets, not RGB exceptions or golden substitution. Renewed observer-fixed strict replay and parent integration/acceptance remain pending; this issue remains open and no shared index/milestone changed.
