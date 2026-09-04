# Milestones

Milestones are ordered capability gates, not calendar promises. An issue may
begin early when its dependencies are available, but a milestone is complete
only when all exit criteria are verified. Detailed requirements live in linked
issue files; the open issue index remains the authoritative work queue.

<a id="m0-safe-foundation"></a>
## M0 — Safe foundation

**Goal:** Create a licensed, testable Rust workspace that can consume verified local dumps without exposing copyrighted content.

### Exit criteria

- The project license, reconstructed-source publication boundary, and contribution policy are explicit.
- The supported ROM revision roles and compatibility identities are explicit.
- The Rust workspace passes formatting, lint, and test gates.
- Both known dumps are normalized and validated without modifying them.
- Oracle artifacts have committed/local/review-required classifications.
- Public CI is ROM-free and checks repository safety.

### Issues

None remaining; M0 is complete. Verification record:
[ROM-backed run](../docs/rom-verification.md).

<a id="m1-reference-oracle"></a>
## M1 — Reference oracle

**Goal:** Produce deterministic, inspectable reference executions from the original game.

### Exit criteria

- A documented emulator or harness can advance the reference game frame by frame.
- Inputs and snapshots replay deterministically.
- Selected machine and semantic state can be exported and compared.
- Short boot and early-game scenarios run reproducibly.

### Issues

None remaining; M1 is complete. The boot-to-name-entry and cursor scenarios
run for both JP and EU. The Pandora's Box / Crysta scenario is blocked by a
game-script desync (not an emulator defect) and is outside M1 scope.
Core swap: ares (ISC) replaced LakeSnes per
[ADR 0002](../docs/adr/0002-reference-emulator.md).

<a id="m2-matching-disassembly-foundation"></a>
## M2 — Matching disassembly foundation

**Goal:** Reconstruct enough of the 65C816 program to build exactly, navigate execution, and name shared state.

### Exit criteria

- The selected reference ROM can be reconstructed byte-for-byte from local input and source metadata.
- Boot, interrupts, and the main loop are labeled and explained.
- Canonical RAM/hardware symbols are shared by tools.
- Major code, data, and indirect-dispatch regions are classified.

### Remaining issues

- [Classify ROM code, data, and indirect dispatch](issues/classify-rom-code-data.md)

<a id="m3-content-and-script-pipeline"></a>
## M3 — Content and script pipeline

**Goal:** Decode the format foundations and content required by the opening vertical slice into lossless typed representations, then generate a local ROM-derived asset pack. Broader content coverage grows with later chapter milestones.

### Exit criteria

- Compression and major content formats have reproducible decoders.
- Round-trip-capable formats reproduce their source bytes.
- Event bytecode has a readable intermediate representation.
- A verified ROM can produce a versioned ignored asset pack locally.

### Issues

- [Implement and verify the compression codec](issues/compression-codec.md)
- [Decode map, metadata, and collision formats](issues/decode-map-collision-formats.md)
- [Decode graphics, palettes, sprites, and animation](issues/decode-graphics-animation.md)
- [Decode text and gameplay data tables](issues/decode-text-gameplay-data.md)
- [Reverse the event script bytecode](issues/reverse-event-bytecode.md)
- [Reverse the CPU-to-SPC audio protocol](issues/reverse-audio-protocol.md)
- [Build the reproducible local asset pack](issues/local-asset-pack.md)

<a id="m4-portable-vertical-slice"></a>
## M4 — Portable vertical slice

**Goal:** Run the opening Crysta and Pandora sequence through a deterministic Rust simulation validated against the reference.

### Exit criteria

- The portable core has versioned deterministic state and input.
- Movement, maps, collision, actors, combat primitives, and events work together.
- The opening vertical slice reaches the first tower transition.
- ROM validation, representative extraction, and a core replay pass an early WebAssembly feasibility check.
- Classic-mode state remains within documented equivalence expectations for its covered paths.

### Issues

- [Define the deterministic portable core model](issues/deterministic-core-model.md)
- [Validate extraction and the core boundary in WebAssembly](issues/validate-web-extraction-spike.md)
- [Port input, player movement, and animation](issues/port-player-input-movement.md)
- [Port map loading, transitions, and collision](issues/port-map-loading-collision.md)
- [Port the actor system and combat primitives](issues/port-actors-combat.md)
- [Implement the portable event runtime](issues/portable-event-runtime.md)
- [Complete the Crysta and Pandora vertical slice](issues/opening-vertical-slice.md)

<a id="m5-classic-presentation-and-chapter-1"></a>
## M5 — Classic presentation and Chapter 1

**Goal:** Deliver a faithful playable Chapter 1 with classic rendering, audio compatibility, menus, and saves.

### Exit criteria

- Classic rendering reproduces required SNES ordering and effects.
- Original music and sound can play through a compatible backend.
- Menus, inventory, and save/load work.
- Chapter 1 is playable end to end through the portable path.

### Issues

- [Implement the classic renderer](issues/classic-renderer.md)
- [Integrate a compatible SPC audio backend](issues/spc-audio-backend.md)
- [Port menus, inventory, configuration, and saves](issues/menus-inventory-save.md)
- [Complete the first tower and Chapter 1](issues/complete-chapter-one.md)

<a id="m6-full-classic-game"></a>
## M6 — Full classic game

**Goal:** Complete and qualify the full game in behaviorally faithful classic mode.

### Exit criteria

- All chapters, bosses, minigames, optional events, and the ending are reachable.
- A full completion-oriented replay suite passes.
- No supported classic path requires original 65C816 execution.

### Issues

- [Complete Chapters 2 and 3](issues/complete-chapters-two-three.md)
- [Complete Chapter 4 and the ending](issues/complete-chapter-four-ending.md)
- [Qualify the full classic-mode replay suite](issues/qualify-classic-mode.md)

<a id="m7-desktop-and-web-releases"></a>
## M7 — Desktop and web releases

**Goal:** Package the portable implementation for native desktops and browsers with user-supplied ROM extraction.

### Exit criteria

- A supported desktop release can validate a ROM, extract assets, play, and save.
- A browser build performs the same flow without uploading the ROM.
- Release artifacts remain ROM-free and reproducible.

### Issues

- [Ship the desktop frontend](issues/desktop-frontend.md)
- [Implement browser ROM and asset bootstrap](issues/browser-rom-bootstrap.md)
- [Ship the WebAssembly frontend and web platform services](issues/webassembly-frontend.md)

<a id="m8-enhancements-and-extensibility"></a>
## M8 — Enhancements and extensibility

**Goal:** Add opt-in improvements without weakening the qualified classic baseline.

### Exit criteria

- Enhanced presentation and controls are independently toggleable.
- Localization architecture supports additional user-provided versions or patches.
- Bug fixes and mods have explicit compatibility and provenance boundaries.

### Issues

- [Add enhanced and widescreen rendering](issues/enhanced-widescreen-rendering.md)
- [Add accessibility and control enhancements](issues/accessibility-control-enhancements.md)
- [Support additional localizations and opt-in fixes or mods](issues/localization-fixes-mods.md)

## Completion policy

Completing code is not enough to complete an issue or milestone. Acceptance
criteria must be verified, the issue must move from `issues.md` to
`issues_archive.md`, and any skipped ROM-backed verification must be recorded
as a blocker rather than silently accepted.
