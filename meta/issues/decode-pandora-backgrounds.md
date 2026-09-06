# Decode the required Pandora route backgrounds

## Summary

Compile and independently qualify only the first-background resources, source camera profiles and attributed grids needed by wider A, map13, E/20/21 and the forced41–44 tour.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Authenticate resource provenance and state-dependent setup. Maps41–44 inherit the controller-supplied graphics and ordered bank-first COP5A palette transfers; they are not ordinary standalone house recipes.
- Keep full source-derived grids distinct from route admission and dynamic occupancy. Preserve house/exterior recipes and priority semantics.
- Compare selected native tile words/indexed pixels/priority and camera evidence; document omitted layers/effects instead of claiming whole-frame RGB equality.
- Keep extracted pixels/grids/palettes local; source metadata and hashes only are committed.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.
- Owned by Alice in `task/pandora-backgrounds`; implementation confined to visual assets, dedicated background qualification tooling and this detail/doc.
- Compiler delivered in signed `6eeac80`: `assets::maps::visual::pandora::{PandoraBackground, SourceCamera}`. Existing nine-map static allowlist preserved. Full attributed initialization, caller-supplied admission halo and caller-applied source phase patches are explicit separate policies.
- Maps41–44 compile map21 inherited colors 0–15 plus controller41's 384-tile graphics, definitions/attributes and ordered bank-first palette transfers. Later tour maps are not standalone recipes. Raw/captured resources never initialize production.
- Dedicated `tools/pandora-background-qualification/compare.sh` qualifies both existing journeys using typed source plus WRAM/VRAM/CGRAM only: 17 settled states × two captures, every visible tileword and all 57,344 indexed/priority pixels per state, full definitions and source camera contracts. Normal/optimized strict comparisons agree exactly.
- Full-grid phase deltas remain explicit. E/20 have five changed C-sector tiles outside the cellar viewport (opened door/three lifted pots), not an adjusted base. Town-west fine-X ring alias differs from an unwrapped natural crop by 28 words/833 pixels; this edge is compared and reported, not masked. Source-selected animation payloads explain remaining natural-base pixel differences.
- Verification: full assets suite; authenticated local Japanese house/cavern/exterior background regressions; strict assets Clippy; 12 normal/optimized Python tests including input-level pipeline mutations and fine-scroll/nonuniform ring alias. Replacing the final comparator gate with a no-op is killed by four tests. Independent correctness/architecture and source-pixel reviews found no remaining blocker after stronger inheritance, camera, phase and viewport checks.
- Acceptance remains open for parent-owned observer repair / renewed strict route replay. Parent diagnosed intermittent `.pixels` zeroed regions as the async video-publication race (source `5b7b88b`, parent `42fe162`); nonpixel captures/provenance agree. This qualifier never reads `.pixels`, grants no wholeRGB equality and does not claim accepted parent replay. Shared issue index/milestones are deliberately untouched.
