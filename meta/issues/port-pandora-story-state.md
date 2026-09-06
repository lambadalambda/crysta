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
- Box warning has two acknowledgements and requires a second contact; forced tour owns control through final four-page return and grants `$243/$244` at their distinct boundaries.
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
