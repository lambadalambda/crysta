# Decode the required Pandora actors and carrying poses

## Summary

Compile source-derived presentation for required town/map13/cellar/box/tutorial actors, carried/thrown pots and Ark carrying poses, without introducing general NPC simulation.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Qualify only required source actor identities, scene/phase membership, palettes, frame bounds, anchors and draw ordering from source and retained native captures.
- Dynamic scripted phases are explicit presentation data, not an invented wandering scheduler. Keep collision ownership with the portable core/source profile compiler.
- Preserve existing Ark and frozen-house art. Account explicitly for priority/layer omissions and distinguish source composition from whole-scene native equality.
- Export no embedded copyrighted sprites/palettes; retain raw assets under ignored local paths. Positive/negative tests and independent review precede substantial commits.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.

## Work log

- 2026-03-24: Sprite owner started source/compiler qualification. Scope is additive sprite APIs, new Pandora scene tools/docs and this detail only; no tracker-index/core/host/map/text changes. Original journey/discovery and parent replay remain separately labeled evidence; parent acceptance is not asserted.
- Research preceded executable TDD for unknown source formats. Seven focused Rust tests now cover exact synthetic pixels/mirrors, list/operand mutation rejection, carry pairing, phase/motion resolution, departures and authenticated nonempty ROM rasters. Review-found malformed selector and bank-crossing cases were demonstrated red then green.
- Source/API review (`2d0b72cb-df1e-4202-9128-dd4733b851d3`) approved the bounded compiler after fixes to compressed table bounds, phase/motion referential integrity, public motion export and palette bank-relative checks. Static independent review, not independent test execution. Existing authenticated Ark28 and exact house10 source export are unchanged; assets tests and strict clippy pass.
- Additive compiler: 16 source art resources, 241 list records, 33 finite phase choices and nine C departure segments. C's fourth resident leaves first; the reaction roster shrinks to zero. Ark lift/standing/walking/throw pairing and pot flight list60 are explicit; core owns all carry physics, hits and input state.
- Required C window/color math and the opening guide's white-palette effect remain explicit unresolved presentation limits, not silently missing actors or source palette replacements. Town is source-spawn-frozen, not wandering simulation. Parent route acceptance and repaired-observer evidence remain separate gates; this issue is not closed.
- Source/API signed commit: `30add8d`. Consumer/carry contract and explicit ancillary-actor/effect exclusions are documented in [Pandora scene](../../docs/pandora-scene.md).
- Evidence: 482 independent pixel-exact source raster exports, 18 named OAM-piece witnesses and 24 native phase witnesses. Both original and parent `replay-JmCgU8/journey` hardware evidence pass the same strict metadata checker. All five hardware surfaces are hashed at every selected phase, including empty E/20; native `.pixels` are never read because of the separately diagnosed observer publication race.
- Tools/docs review (`01425b52-b218-4ea1-9638-b4f57362a313`) approved after adding full hardware hashes to phase-only checkpoints and an absolute-path reproduction command. Three Python tests pass normally/optimized, six local phase mutations reject, and a no-op native checker is caught. Review was static/read-only; test execution and capture comparisons were owner-run. No new native navigation or preview/browser changes.
