# Qualify and implement bounded cellar pot actions

## Summary

Qualify and implement source-derived FA/FB lifting, carrying and throws sufficient for the direct C cellar-door route, including a miss and two actual hits.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Do not replace hit testing with a pot counter or interaction-distance shortcut. Preserve consumed-cell membership, carried-object ownership, resident occupancy and carrying movement semantics.
- Source/native qualification must distinguish lift/hold/throw/flight, a miss, first/second hit and control restoration. Door reactions/whole story graph remain the parent runtime responsibility.
- Provide a dependency-free, deterministic pure core component with atomic error handling and canonical snapshots/continuation. Only admitted carrying/throw behavior; no general combat/projectile system.
- Keep source/art/capture data local; use red/green native-fixture and synthetic tests and independent review.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.
- Alice owns this detail, new `room-core::pots`, pot qualification tools and docs in `task/pandora-pots`. No parent runtime, assets, profile, or live-preview changes.
- [Pot component contract and qualification](../../docs/pandora-pots.md): three contiguous original segments, **2,184 frame calls**, source FA/FB replacements and reservation slots, carried 0020 movement, explicit release/admitted Up flight, miss and two actual contacts. Local native comparisons and the full core regression suite pass. Independent correctness and evidence/architecture/DRY reviews approved the bounded implementation (static/read-only reviews, not independent native execution); reviewed impossible queued-action snapshot admission was fixed red → green. Library Clippy with warnings denied and normal/optimized Python checks pass.
- TDD red boundaries retained during development: missing projector/module, full native type5 contact refusal, exact type29 staircase refusal, rejected-A delayed-queue poisoning, missing source door geometry, and the native break/reservation age22 boundary. Focused source/canonical snapshot/action negative tests accompany the implementation.
- Source findings: COP83 held horizontal movement lacks the ordinary COP84 54-phase restart. Type29 matches open in all Up tables but **not Right S-first**; only exact source door/stair words at (11,21), Up, receive a private collision-view alias. No global material admission changes.
- Per-frame observations are selected player/actor fields, not full WRAM. Held slots/counter/collision are checkpoint fields; source/script boundaries qualify intervening reservation release. No new native run or restore was used, and no raw captures/trace data are shipped.
- Parent reports semantics resolved in source `5b7b88b` / parent `42fe162`: frame logs and non-pixel captures match; intermittent pixel zeroing is the shim async publication race. This work never reads pixels and does not weaken exact pins or claim the renewed observer-fix replay has passed.
- Deliberate limit: general COP65 collision dispatch/ballistics remain unqualified; only the documented immutable direct-C Up lanes and exact released-flight contact are admitted. Parent owns the request/reaction graph and final acceptance/tracker closure. Issue remains open pending that handoff.
