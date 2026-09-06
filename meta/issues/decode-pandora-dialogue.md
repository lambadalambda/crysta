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
- Implemented bounded immutable `assets::text::pandora::PandoraDialogue`: 33 resources / 76 fixed logical pages, 34 ordered direct invocations, retained choice contexts, exact D3/D5/D4 semantics, source-derived native glyphs/layout and default controller labels. Existing house API, fourteen pages and two catalogs remain compatible.
- [Dialogue contract and reproduction](../../docs/pandora-dialogue.md); new `tools/pandora-dialogue-qualification/reference.json` is the ordered request/page/ack manifest. Raw outputs remain ignored under `local/`; only tooling/metadata/hashes are committed.
- Independent complete source reconstruction passes. Original and parentfresh each compare 73 pages / 380,672 native font-cell pixels (97,371 foreground); discovery compares 67 / 355,968 (92,434 foreground) and supplies retry `$88B722` plus refusal `$88B7E3`. Combined selected coverage spans all 76 pages, with per-set source-only inventories explicit.
- TDD red → green, Rust/owned-ROM regressions, normal/optimized Python tests and mutation controls pass. Independent static reviews approved correctness/architecture and pixel nonvacuity; review-found native orchestration coverage was added and its hash-guard mutation killed. Existing house native regression remains 67,904 matching pixels.
- Source boundary correction handed back to the parent: `$88AE50` is glyph `$56`, not an acknowledgement. Warning `$88ADF2` has only `$88AE29 D5` and `$88AE5E D3`. The parent has since corrected its progression doc; this task did not edit it.
- Parent diagnosed intermittent `.pixels` corruption as the shim's async video-publication race (source `5b7b88b`, parent `42fe162`). This qualification strictly uses separately named WRAM/VRAM sets, not RGB exceptions or golden substitution. Renewed observer-fixed strict replay and parent integration/acceptance remain pending; this issue remains open and no shared index/milestone changed.

- Bounded follow-up: appended only map13 `$88B7E3` result0/2 refusal (D5 `$88B807`, D3 `$88B82D`) so the existing retry context has a usable path. Preserved 34 direct invocations and every prior page via a frozen 74-page metadata/bitmap-hash digest. Discovery supplies 12,288 additional matching pixels (2,959 foreground); both pages remain source-only in the direct sets. No C `$2F` expansion; local1/branch execution stays parent-owned. Owned-ROM and Python red → green tests added; normal/optimized source/native checks repeated. Independent static correctness/architecture/nonvacuity review approved this follow-up after reported green reruns. Separate video observer fix `6eda` remains outside this qualification.

### Parent host adapter

- Opt-in Pandora art compilation preserves all 18 house rasters and adds 76 pages plus six prompt-specific choice crops. Native catalog1 has different map13/C labels: transport now keys contexts by the actual prompt page, retaining legacy global catalogs only when no context map is advertised. Live gameplay still uses the house-only profile.
- Dynamic page width/background policy is retained; this is explicit high-contrast presentation, not native RGBA. Every source page/crop pixel is checked, including the two refusal pages and two-page box warning.
- Parent real UI regression: `local/map-research/conversation-context-browser.json`, 1671 input steps / 1683 canvas checks, final A `(538,815)` with `$20/$26/$FB`. Host 47 unit tests, strict Clippy and 15 browser-verifier tests/controller harness pass. The full host suite separately fails the existing `local_capture` observer hash after the synchronous-video fix; that migration remains tracked by the source owner, not hidden by this adapter acceptance.

- Parent refusal follow-up reproduced from a fresh export (pointer `local/pandora-dialogue-qualification/parent-refusal-root.txt`): 20 checker tests normal/-O, strict source reconstruction, fixed-observer parentfresh 73 pages / 380,672 native font-cell pixels, discovery 67 / 355,968, including both refusal pages. Combined coverage still spans 76 pages. Logs `local/map-research/pandora-refusal-parent.txt`; no `.pixels` substitution or route-policy relaxation.

## Parent component acceptance

- The 33 resources / 76 pages and context-specific labels are accepted. Native coverage is the union of direct and historical discovery evidence; retry/refusal pages remain source-only in the direct sets. The warning has two acknowledgement boundaries. Scheduling, high-contrast host colors and native window/RGBA fidelity remain distinct.
- Parent source replay now passes the reviewed synchronous observer epoch strictly on both independently reproduced roots, normally and optimized; this supersedes earlier pending observer/source handoffs, not the fidelity limits above. Independent component reproduction and closure audit found no remaining blocker for these component criteria.
- Issue archived. The live profile remains house-only; `port-pandora-sequence` and its navigation/story/renderer work stay open. Full map-inspector SRAM capture renewal and other unrenewed legacy wrappers remain separate, not reported green by these tests.
