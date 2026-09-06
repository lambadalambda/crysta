# Port the bounded Pandora route and sequence

## Summary

Implement only the maps, source assets and semantic state transitions required by the qualified Pandora route, preserving the CPU-free core and existing preview.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Talk to the room B resident and leave the house](talk-and-leave-house.md)
- [Decode the required Pandora route backgrounds](decode-pandora-backgrounds.md)
- [Decode the required Pandora progression dialogue](decode-pandora-dialogue.md)
- [Qualify and implement bounded cellar pot actions](qualify-pandora-pot-actions.md)
- [Decode the required Pandora actors and carrying poses](decode-pandora-scene-art.md)

- [Support the bounded Pandora event flag projection](pandora-event-flag-projection.md)

## Requirements

- Derive the scope from the qualified source contract rather than assumed story order. Split substantial map/asset/mechanic work into focused child issues after discovery.
- Admit only required exterior traversal/re-entry, rooms, interactions, dialogue and immediate world/player changes through regained control.
- Keep semantic progression in the dependency-free device-free core, with immutable source-derived data and versioned aggregate identity/snapshots.
- Extend event operations only when the qualified sequence needs them; no general native event interpreter, scheduler or CPU fallback.
- Retain actual UI actions, readable ROM-derived text, explicit choices and visible unsupported boundaries.

## Acceptance Criteria

- Focused red → green tests cover required progression, missing/reordered prerequisites, interruption/reset, persistent effects and restored continuation.
- A continuous real-browser New Game-to-Pandora journey matches the admitted reference semantic checkpoints and regains control.
- House511/2244 and conversation/exterior1671 routes remain valid, or any intentional source-backed contract revision has explicit replacement evidence.
- Native/Wasm builds, workspace tests, lint, browser checks, repository safety/tracker and independent correctness/architecture review pass.

## Opt-in host integration (browser acceptance pending)

- Parent independently reproduced the source aggregate's 11 tests and all11,590
  public input actions with per-action restoration; debug/release reports are
  byte-identical (`local/map-research/pandora-aggregate-parent*`).
- `Preview::new_profile(rom, true)` now composes that exact compiler, source art,
  additional cameras, core scene key/control owner, 1024-bit inspection, typed
  carry projection and complete effective-room world patch set. No presentation
  drives the graph; diagnostic invocation/cue/local fields are read-only.
- Three new host tests cover initial capability/legacy separation, every11,590
  serialized host projection across restored states with the exact offline final
  snapshot, high flags/final owner and fallible camera projection. All nine host
  preview tests and strict workspace Clippy pass. Raw export is opt-in test-only,
  ignored at `local/map-research/pandora-preview-parent.json`.
- A dedicated ignored `serve_pandora_preview` test runs the same loopback handler
  with explicit owned-ROM/port environment variables. Parent diagnostic port8877
  is enabled for the browser qualifier; ordinary port8765 remains house-only.
- Independent review approved authoritative projection and unchanged transport.
  Two error-presentation findings were fixed red→green: hide stale dialogue on
  host error, and preserve that primary error over a previous art failure. The
  actual inline harness exercises both sequences; no fallback art is displayed.
- Both module registrations are explicit in main. Producer bridge/current pins
  and the continuous real-browser journey remain acceptance gates, not bypassed
  by passing the offline route.

## Notes

- Parent: [Open Pandora’s Box in the portable slice](open-pandora-portable-slice.md).
- Do not enable an unqualified placeholder sequence while source discovery is incomplete.

- Remaining integration split: [navigation/contact admission](qualify-pandora-navigation.md) and [fixed story continuation](port-pandora-story-state.md). These are required beyond the independently decoded visual resources; neither enables an unqualified live route.

- Parent reproduced current core (163 tests plus one doctest, all five private
  fixture suites) and Wasm after the authentic overhanging Town-exit correction.
  Latest house-only host also passed real input-only browser regressions: 511
  steps/512 visual checks, 2244/2245, and conversation/exterior 1671/1683. Evidence:
  `local/map-research/pandora-parent-overhang-core.txt` and
  `pandora-latest-house{511,2244,-browser}.json`. These preserve the old profile;
  they are not yet a Pandora journey witness. Producer source-gate revalidation
  is tracked separately; no legacy observer mismatch is hidden.

- Parent camera adapter has an explicit opt-in path for the eight additional maps
  and Town A, delegating directly to authenticated `SourceCamera` decoding and
  settled clamping. No duplicate geometry or in-flight camera-pan claim. Red→green
  tests cover all sectors, endpoint/clamp equality, source mutation rejection,
  house/A parity and unsupported-map rejection; three camera tests and workspace
  Clippy pass. Independently reviewed; existing host still uses the old path.

- Parent transport now reserves canonical command10 for native A/pot action, capability-gated Lift / throw (Z), separate from B/Interact/acknowledgements. Manual submission pauses and Resume advances delayed A/recovery; no automatic story advancement. TDD covers same-origin/two-byte parsing, segmented transport, repeat/held-input rejection, dialogue isolation and actual inline button dispatch. Live profile stays disabled pending the qualified aggregate compiler/route.
