# Qualify bounded Pandora navigation and contact admission

## Summary

Derive immutable collision/occupancy profiles and transition/contact contracts needed to connect the accepted house route to map13, the cellar and completed Pandora tour. Asset availability alone does not qualify movement.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Decode the required Pandora route backgrounds](decode-pandora-backgrounds.md)
- [Qualify and implement bounded cellar pot actions](qualify-pandora-pot-actions.md)

## Requirements

- Authenticate source grids, material/sample admission, actor occupancy, ordered exits and settled destinations for wider A, map13, changed C, E/20/21 and final map41 control.
- Separate source stairs/forced transfers from ordinary wooden-door motion; explicitly document portable pacing and history-reset policy rather than interpreting snapshot frame labels as delay constants.
- Qualify box contact bounds/order and shared-house patch/pot lifecycle across required loads. Do not add a general collision/event dispatcher or initialize from captures.
- Preserve narrow existing house profiles until the new aggregate is explicitly enabled; unsupported materials/branches remain atomic errors.

## Acceptance Criteria

- Source-derived compiler/data contract plus independent native sample/transition evidence cover the continuous admitted route and boundary rejection tests.
- TDD, mutation controls, affected regressions and independent correctness/architecture review pass.
- Exact fidelity limits are documented; no raw captures or extracted grids committed.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Reverse-engineering/source discovery may precede tests; implementation must use red → green.
- Source discovery: [bounded navigation](../../docs/pandora-navigation.md) records material aliases, actual edge/actor samples, selector14 adjustment, box polling and pointer-cache lifecycle. Independent read-only correctness/evidence/architecture review approved the discovery-only increment; requested spatial-coverage and residue provenance clarifications were applied.
- Compiler `13418e7`: 16 immutable ROM-derived profiles, scoped material admission, source actor geometry/positions, ordered exits, separate stairs/forced transfers, exact pot/shared-sheet deltas and contact operands. Independent compiler/API/source review approved after provenance and projection corrections; old house/core/main files untouched.
- The corrected coordinate contract is essential: `$0968=rawY-8`; map13 Up probes rawY−24/−16, and the raw box-opening gate is X120..152/Y368..400. Y144 remains the admitted witness, not a proven native cutoff at Y145. First contact is separately X123..149/Y370..400; only centerline north recoil is endpoint-qualified.
- Native checker/reference: repaired-observer twins agree in normal and optimized Python: 2,613 eligible frames / 10,851 conservative edge samples against 16 fixed phase witnesses, eight settled transfers, contact370→recoil369 and positive opening368. The 36 repeated frozen-bird differences at `(28,25)` are explicitly enumerated, not native walking equivalence. Per-phase denominator/exclusions and exact WRAM/log/source hashes are retained.
- TDD and controls: five compiler tests (including owned-ROM, full-grid delta and geometric connectivity tests), five synthetic checker tests and nine input-level native mutation tests pass; no-op comparators fail as expected. `assets`/`room-core` regressions and standalone Clippy pass. Independent source/API and native-evidence reviews addressed nonvacuity, event provenance, serialization and projection issues.
- **Keep open pending parent integration/reproduction:** source-geometric connectivity is not proof of portable input cadence or forced/carry/recoil timing. No ordinary admission for second-hit temporary occupancy or forced tour. Do not expand the route if actual core movement cannot follow this bounded contract. Historical Y145 pulse failure remains unexplained; no source-owner observer/tools/docs contracts were changed.

### Parent reproduction

- Independent source export and normal/optimized tests/native comparison pass against both parent fixed twins: 2613 eligible frames, 10851 conservative samples; all 36 frozen-bird differences remain explicitly retained. Export pointer `local/pandora-navigation-qualification/parent-root.txt`; log `local/map-research/pandora-navigation-parent.txt`.
- Parent corrected the progression summary to distinguish the successful exact map13 witness from an unproved Y145 cutoff, and the box's first-contact geometry from its any-facing post-warning polling/readiness gate. No native fixture or source operand changed.
- Issue remains open: the owner is now adapting the source profiles to the portable aggregate and must prove a real input-cadence route without erasing frozen occupancy or broadening type29/material admission.

### Offline adapter groundwork

- Resumed bounded implementation: adapt authentic ROM text/profiles/motions into the delivered `PandoraData`, then run an input-only New Game itinerary with per-step restoration. Live host remains disabled. Initial API blockers reported to parent: `Room` lacks scoped material classification; Town lacks the source-required A wooden-door action/patch. Core owner separately owns resident-sheet cache correction. New `pandora_progression.rs` and offline tools only; finite semantic pacing must preserve source prerequisites and explicit completion ownership.
- [Offline adapter groundwork](../../docs/pandora-portable-qualification.md): authentic text,14 raw rooms/source pots, six COP14 arrival samples and per-action restored1701-action house prefix pass four tests. Shared-sheet fix `a2a6afe` applied as `d11c3df`; all four C grids satisfy the new closed wooden-door constraint without adaptation. No aggregate compiler or completed Pandora itinerary yet. Remaining core seams: scoped material classifier, Town door action, preserve-player cue samples and ordered exit selection.
- Independent source advisor closed selector1 Down / selector2 Up stationary arrival and the bounded post-recoil COPDF readiness certificate. Completed northern recoil restores `$097C=0`; only the admitted ordinary walking/neutral/warning subset may carry that certificate. Handoff preserves position and sets Down; gate proximity/timers alone never certify readiness.
- Independent correctness/architecture/evidence review found no source-data or lifecycle blocker. Its narrow expected-error test finding was fixed: rejected acknowledgement must return exactly `SliceError::Interaction`, so a harness restoration/divergence error cannot masquerade as expected rejection. Four owned-ROM tests, `assets`/`room-core` regressions and standalone all-targets Clippy pass. This approves groundwork only; the integration gates and full itinerary remain open.
- Early core APIs applied: preserve-pose `051b127`→`ea81720`, raw-material `73f51ff`→`85c6fe1`, raw pot lanes `51d558c`→`5f0821b`. Adapter reconstruction samples now use Absolute; rooms install only source map/halo/Up-scoped aliases, without raw-word changes. Exact contacts carry the completed-rest witness. TDD missing-policy/missing-contact reds followed by six green owned-ROM tests; core suite and strict standalone Clippy pass. Independent compatibility/correctness/compactness review found no blocker. Town doors/ordered exits remain core-owner work; cue catalog and aggregate route remain follow-up.
- Cue source mapping closed all33 keys, including the six destination startup delays and exact no-COPC1 FirstHit/right/left return windows. `compile_cues` expands source-authenticated fixed recipes into preserving samples with logical COPC1 units and finite cooperative boundaries, never default timers. TDD missing-function red→seven owned-ROM tests green; strict standalone Clippy passes. Graph-atomic local/292/243 visibility and direct BoxEntry/pre-warning delay omissions are explicit semantic limits, not native scheduler equivalence. Independent catalog/source/architecture review approved the bounded33-cue increment with no blocker; no aggregate route claimed.
- Shared world-patch/runtime API handoff: moved the existing source-object compiler into public `pandora_navigation::source_objects(image, &Navigation)`; progression imports it. The scan covers the entire admitted C halo in cell order, never a three-pot hardcoded catalog or whole-sheet FA/FB scan. TDD public-symbol red→new halo-expansion/outside-halo mutation control green. Added public navigation API docs; standalone public-module Clippy with `-D warnings -D missing_docs -D clippy::missing_errors_doc` passes, as do seven adapter and six navigation tests. Parent alone owns registration/world-patch/presentation changes.
- Actual-workspace navigation lint handoff: temporary public registration exposed145 test-target pedantic errors absent standalone configuration. Behavior-preserving separators, must-use docs/API annotations, signed reinterpretations, bounds naming and hash writing resolve them; only two function-local long-RE-table/test allowances. Actual `cargo clippy -p map-inspector --all-targets -- -D warnings` is green; all six owned-ROM navigation tests pass before/after. Independent review found no blocker. Temporary `main.rs` registration restored byte-for-byte and is not committed. Aggregate/progression must also pass actual workspace lints before its handoff.
- Complete aggregate compiler: public `pandora_progression::compile(&Rom)` combines the existing house base with14 raw profiles, shared source objects, authentic text,33 cues, six travel motions, complete ordered exits and source Town door actions. Canonical manifest binds every supplied field plus a hash-only ROM-derived base NewGame serialization, never a captured/runtime initializer. TDD aggregate red exposed authentic Town ordinal8 `$818DB3` width80; no record was removed/shrunk. Core fix `0fd1f08`→`cb57353` unblocked construction. All nine compiler/harness tests and actual workspace all-target Clippy pass; main registration restored. Independent compiler/source/API/identity review approved pending this now-green correction. Full input-only itinerary remains separate follow-up.
