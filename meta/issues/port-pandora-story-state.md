# Implement bounded Pandora story state and continuation

## Summary

Compose the existing house state with a fixed CPU-free Pandora continuation graph, authoritative wider flags, room locals, authentic dialogue boundaries and pot/door/box/tour ownership. Do not introduce a second checkpoint-start game or general event VM.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Support the bounded Pandora event flag projection](pandora-event-flag-projection.md)
- [Decode the required Pandora progression dialogue](decode-pandora-dialogue.md)
- [Qualify and implement bounded cellar pot actions](qualify-pandora-pot-actions.md)
- [Qualify bounded Pandora navigation and contact admission](qualify-pandora-navigation.md)

## Requirements

- Continuous existing New Game state/ticks/identity; new capability remains opt-in until host acceptance.
- One authoritative story projection; exact room-local and counter resets including same-map reload, preserving persistent highflags.
- Fixed map13 refusal/retry and direct C branch; unsupported C cancel/result2 must not grant or advance.
- Real hit output drives door state; preserve pot consumed ledger and launch collision profile through recovery.
- Box warning has two acknowledgements; opening requires the raw-coordinate polling gate, local1 AND local2 and successful COPDF handoff; forced tour owns control through final four-page return and grants `$243/$244` at their distinct boundaries.
- Versioned canonical snapshots and aggregate identity; restore must reject impossible stages/flags/ownership and resume every supported action exactly.
- Source-data geometry belongs to the authenticated host compiler, not speculative core constants.

## Acceptance Criteria

- Focused red → green state/branch/reset/malformed-restore tests and existing house behavior pass in dependency-free no_std core and Wasm.
- Immutable API is usable by the parent host; final aggregate native/browser acceptance remains the parent issue.
- Independent correctness/architecture/DRY review before small topical signed commits.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).

### Runtime implementation log

- Foundation red → green: typed invocation identities and exact immutable text
  catalog admission, including repeated D720 sites and corrected two-page warning.
- API and evidence boundaries are recorded in `docs/pandora-runtime.md`.
- Navigation/contact/forced pacing remains compiler-owned; no guessed core geometry.
- Issue remains open; no browser/native aggregate acceptance claimed.
- Unified flag storage committed with unchanged profile9 bytes and all five private
  old fixture suites passing; B highflag preservation tested at both widths.
- Fixed graph component red → green, independently reviewed. Review's forced-tour
  ownership restore defect fixed with regression tests. Aggregate wiring follows.
- Aggregate opt-in now extends the same GameData/GameState; live host remains
  disabled. Concrete compiler/actions/art API and 300-byte version2/profile10
  snapshot contract are documented in `docs/pandora-runtime.md`.
- Integration tests cover per-action restore, refusal/retry/direct grant pages,
  unsupported C answers, two genuine synthetic lifts/carries/hits with concurrent
  reaction/recovery, ledger retention, source-boundary $292, contact-only warning,
  same-map reload, forced tour and final controllable41. Isolated miss launch is
  explicitly synthetic, not claimed as a native carry-route witness.
- Independent data/aggregate reviews found and fixed ambiguous exit anchors,
  post-reload box collision selection, erased forced-motion ownership and
  counter/consumption inconsistency. Follow-up reviews confirmed these fixes.
  Box actor roster also remains pre-opening until the reconstruction boundary.
- Verification: strict all-target core Clippy; dependency-free no_std Wasm build;
  all five explicitly enabled private core fixture suites; six unchanged host
  room_preview tests including the authenticated fresh-house route. No browser or
  new aggregate native route/pacing acceptance is claimed by this issue.
- Parent/navigation still supply qualified new geometry/contact/presentation
  samples and own compiler/transport/render acceptance. Missing cue data fail
  atomically, never with guessed pacing. Keep this issue open until those gates.

### Source correction from navigation e98a7cc

- Opening is a facing-independent polling predicate on inclusive raw bounds and
  local1 AND local2, not a second Down/contact callback. COPDF success is a separate
  required boundary before grant22/reload; first-contact witness remains narrow.
- Shared AFCBB3 sheet lifetime is distinct from room visits. B/C/D/E/20 retain
  tile/attribute patches and removed pots; A/13/21 replace the resident sheet.
  Locals/counter and scene occupancy still reset/rebuild on every load. The new
  profile must rebuild A→D with the wooden door closed, leaving profile9 unchanged.
- Polling correction implemented red → green as `ce617cf`: two callback specs plus
  `BoxOpeningGate`; the masked `BoxAcquireControl` cue awaits compiler-certified
  COPDF success before grant22/reload. Profile11 supersedes profile10; the opening
  correction alone keeps the 300-byte version2 envelope. Inclusive corners,
  neutral/held input, all facings, both local requirements, Y359 exclusion, missing
  readiness and per-action restore are covered. Independent source-contract review
  approved; all five private fixture suites, strict core Clippy and Wasm pass.
- Reopening cellar tiles after replacement with persistent292 needs authenticated
  source load effects, not flag-derived inference; requested from navigation/parent.
- Adapter gaps remain distinct from these corrections: exact motion anchors do not
  encode the full ordered exit tables, and ordinary Room sampling cannot express
  E/20's Up-only type29 admission. Do not substitute guessed exits or globally
  classify type29 as floor.

### Shared-sheet correction completed

- Implemented in `051771b` (integrated here as `a2a6afe`), with TDD and independent
  correctness/architecture review. Finite AFCBB3 patches and consumed ledger survive
  B/C/D/E/20 and ROM-confirmed F/10/11 loads; scene occupancy/locals/counter rebuild.
  Replaced sheets discard patches; no off-screen cache or flag-inferred reopening.
- Public effective_room supplies owned patched geometry; current_room remains the
  borrowed base. PandoraOutput.sheet exposes resident/cellar/consumed state; the
  constructor remains unchanged, but all four C profiles require source-closed
  wooden cells. Exact API and schema3/profile11/320-byte layout are documented in
  docs/pandora-runtime.md; old profile9 remains unchanged.
- Merged verification passed: 131 core tests plus one doctest with all five private
  fixture suites enabled, strict all-target Clippy, no_std Wasm, and six unchanged
  host room_preview tests including the authenticated fresh-house route.
- Replaced-sheet C reentry with292 remains deliberately unsupported pending source
  load effects. This issue remains open for parent aggregate acceptance; no new
  native itinerary or browser acceptance is claimed.

### Bounded compiler integration corrections (in progress)

- Source adapter identifies four required core seams: scoped raw-material policy,
  preserve-player cue poses, Town door interaction/patch lifetime, and ordered
  first-coarse/fine exit admission using the existing decoder/planner.
- Split work into scoped classification, canonical cue poses, then Town/ordered
  exit admission. No host/compiler/source-fixture edits; live host stays disabled.
- Require TDD negative controls, atomic errors, canonical restore, old private
  fixture regression, Wasm/Clippy and independent review before signed commits.

- Preserve-player cue samples implemented red → green: Absolute or Preserve with
  optional facing. The same authoritative walking/pot pose feeds each sample;
  frozen owner coordinates and immutable prefix constraints validate restore.
  Schema4/profile12 retains 320 bytes and adds an active-motion witness to prevent
  erasing a preserve-only motion into its graph wait. No profile9 changes.
