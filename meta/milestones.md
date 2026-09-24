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
[ROM-backed run](../docs/rom-verification.md). The formatting half of the
quality-gate criterion regressed through rustfmt drift and has been repaired:
`cargo fmt --all -- --check` passes with no excluded files, the last holdout
being a pinned producer source that was reformatted under an evidenced repin.
Lint, test, docs, safety and tracker gates pass. One maintainability follow-up
from that repin's review is tracked below; it changes no pin.

- [Consolidate the qualification stage template](issues/consolidate-stage-template.md)

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
run for both JP and EU. The earlier claimed ares game-script desync was not
qualified. A fresh Japanese input-only journey now reaches the house conversation
M4 evidence gate, not a demonstrated emulator or game-script failure.
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

### Issues

The Japanese reconstruction matches all 4,194,304 bytes, and the map
verification is recorded in
[ROM code, data, and dispatch map](../docs/rom-map.md). M2's capabilities are
complete; one published bound in that map was later found to contradict observed
execution and is tracked below.

- [Correct the COP service table bound](issues/correct-cop-table-bound.md)

<a id="m3-content-and-script-pipeline"></a>
## M3 — Content and script pipeline

**Goal:** Decode the format foundations and content required by the opening vertical slice into lossless typed representations, then generate a local ROM-derived asset pack. Broader content coverage grows with later chapter milestones.

### Exit criteria

- Compression and major content formats have reproducible decoders.
- Round-trip-capable formats reproduce their source bytes.
- Event bytecode has a readable intermediate representation.
- A verified ROM can produce a versioned ignored asset pack locally.

The compression codec is qualified against representative JP/EU graphics and
map packets, including byte-exact re-encoding; see the
[format and verification record](../docs/compression.md). A first
[local loaded-map inspector](../docs/maps.md) now visualizes qualified cavern
checkpoints and raw structure. The [static cavern layer and attribute lookup](../docs/static-maps.md)
now match actual loader output. A bounded [map-ID loading-script projection](../docs/map-scripts.md)
resolves five tested IDs into eight layers, with menu and cavern loader equality.
A [ROM-only full-map viewer](../docs/static-graphics.md) now draws the cavern
from decoded graphics/palettes/metatiles with runtime resource and patch equality.
State-dependent loading, broader gameplay coverage and collision behavior
qualification remain open.

### Issues

- [Decode map, metadata, and collision formats](issues/decode-map-collision-formats.md)
- [Trace and qualify the movement collision predicate](issues/qualify-collision-predicate.md)
- [Trace the player's movement admission routine](issues/trace-movement-admission-routine.md)
- [Qualify the directional resolver for Crysta's remaining collision types](issues/qualify-crysta-directional-collision.md)
- [Qualify native horizontal first8 dispatch](issues/qualify-horizontal-first8.md)
- [Qualify native horizontal Partial/8 and Solid/8 pairs](issues/qualify-horizontal-type8-pairs.md)
- [Qualify native slope-mediated horizontal type8 contacts](issues/qualify-slope-mediated-type8.md)
- [Decode the door-entry trigger for one-cell exits](issues/decode-door-entry-trigger.md)
- [Decode the actor script VM](issues/decode-actor-script-vm.md)
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

Native evidence now reaches the [first-tower interior entrance](issues/qualify-tower-approach-route.md)
with stable two-axis control. Portable integration remains open; this native
route needs acquisition/frozen return but no equipment selection or combat.

### Issues

- [Define the deterministic portable core model](issues/deterministic-core-model.md)
- [Port input, player movement, and animation](issues/port-player-input-movement.md)
- [Port map loading, transitions, and collision](issues/port-map-loading-collision.md)
- [Port the actor system and combat primitives](issues/port-actors-combat.md)
- [Implement the portable event runtime](issues/portable-event-runtime.md)
- [Complete the Crysta and Pandora vertical slice](issues/opening-vertical-slice.md)
- [Make the Crysta slice fully playable](issues/playable-crysta-slice.md)
- [Play the Crysta slice free-roam in a native app](issues/free-roam-crysta-app.md)
- [Promote the Crysta room builder into a library](issues/crysta-room-library.md)
- [Walk between Crysta maps through the real exit geometry](issues/crysta-map-transitions.md)
- [Place Crysta residents and let the player talk to them](issues/crysta-resident-interaction.md)
- [Native macOS window, renderer and gamepad for the Crysta slice](issues/crysta-native-shell.md)
- [Solid residents, the bedroom start, and resident animation](issues/crysta-solid-animated-residents.md)
- [Walking residents: execute the ordinary loop](issues/crysta-walking-residents.md)
- [Play the Crysta story from the wake-up scene to the world map](issues/play-crysta-story.md)
- [Run scripted movement, entry scenes and map transfers](issues/scripted-movement-scenes.md)
- [Play the frozen return and the Elder's mission](issues/frozen-return-mission.md)
- [Leave through the south gate onto the world map](issues/south-gate-world-map.md)
- [Play the slice's music and sound effects](issues/crysta-music-and-sounds.md)
- [Draw Yomi instead of a placeholder](issues/yomi-sprite.md)
- [Open the shops in Crysta](issues/crysta-shops.md)
- [Decode the shop stock](issues/shop-data.md)
- [Decode the shop texts' indexed controls](issues/shop-text.md)
- [Keep money, Prime Blue and the inventory](issues/money-and-inventory.md)
- [Run the shop's talk callback](issues/shop-state-machine.md)
- [Draw the shop display](issues/shop-display.md)

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
