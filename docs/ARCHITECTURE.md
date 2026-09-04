# Architecture

## Strategy

The project has two cooperating lanes:

1. **Reference lane** — an annotated, reproducible understanding of the
   original ROM, plus deterministic execution traces from a reference emulator.
2. **Portable lane** — a high-level Rust implementation tested against the
   reference lane but not permanently coupled to SNES registers or CPU state.

The assembly is the specification and oracle. It is not necessary to finish a
complete line-by-line translation before implementing a tested vertical slice.

```text
User-provided ROM
  ├── validation and normalization
  ├── matching disassembly
  ├── data/script extraction
  └── reference execution
             │
             │ snapshots, traces, and differential tests
             ▼
      deterministic Rust core
             │
       semantic commands
      ┌──────┼─────────┐
      ▼      ▼         ▼
  renderer  audio  save/platform services
      │      │         │
      └──────┼─────────┘
             ▼
       desktop / web
```

## Reference lane

### ROM normalization

The Japanese normalized image is the behavior reference; see
[ADR 0001](adr/0001-version-support-model.md). All offsets are defined
against a headerless image. Input handling must detect
and remove a 512-byte copier header without changing the source file, then
verify the normalized image against a known hash.

The repository stores hashes and metadata, never ROM bytes.

### Matching disassembly

The disassembly should initially permit opaque ranges sourced from a local ROM.
Known routines and data replace those ranges progressively. A reconstruction
build is considered matching only when its normalized output is byte-identical
to the selected reference ROM. The cc65 toolchain, HiROM placement model, and
explicit local workflow are defined in
[ADR 0003](adr/0003-matching-disassembly-toolchain.md). The first labeled
[boot, interrupt, frame-gate, and main-loop flow](boot-main-loop.md) establishes
these conventions against an ares instruction trace.

The disassembly should capture:

- 65C816 M/X width state;
- bank and direct-page assumptions;
- reset, NMI, IRQ, BRK, and COP entry points;
- code/data boundaries and indirect dispatch;
- named WRAM, SRAM, VRAM, CGRAM, and OAM structures;
- cross-version correspondences where evidence supports them.

### Canonical memory map

The [canonical memory map](memory-map.md) is a schema-versioned, ROM-revision-
and hash-bound source of typed direct-page, WRAM, SRAM, and hardware metadata.
Canonical 24-bit addresses remain distinct from assembly operands that rely on
`D=$0000` or a documented DBR mirror. Public maps are evidence leads rather than
ground truth, and unresolved conflicts remain explicit.

The `memory-map` crate validates the committed Japanese map and serves both
Rust/oracle lookup and deterministic ca65 include generation. Validation treats
`D=$0000` direct page and low WRAM as the same physical bytes, hardens evidence
provenance and short operands, constrains prospective SRAM symbols to bank-local
HiROM windows, and exposes boundary disputes as typed conflict queries.
Reconstruction writes its include only to an ignored per-run directory and
remains subject to the byte-identical ROM comparison. The schema supports SRAM,
but no canonical SRAM fields are claimed until sufficient evidence exists.

### ROM classification map

The [ROM map](rom-map.md) is a separate schema-versioned, revision/hash-bound
source of sparse code, typed data, entry-state, and indirect-dispatch metadata.
Its typed address model distinguishes normalized file offsets, canonical HiROM
addresses, and runtime mirrors. Unknown gaps stay unclassified, partially
understood function starts do not imply guessed code extents, and conflicting
claims remain outside canonical lookup.

The `disasm` crate validates and exposes the committed Japanese artifact,
resolves ROM-backed pointer tables against authenticated local input, generates
runtime/canonical ca65 constants, and supports explicit-namespace inspection.
Reconstruction consumes the generated include and still requires a byte-exact
output match. The initial map classifies every direct boot/interrupt/main-loop
target, all 125 valid COP selectors, and the trace-proved opening target of the
mutable top-level state dispatch.

### Oracle

A reference emulator or compatible execution harness should support deterministic
frame stepping, recorded controller input, snapshots, and state export. The
portable implementation will be compared against selected semantic state each
frame. Full-memory comparisons are useful during discovery but may later need
documented exclusions for irrelevant transient bytes.

Stable symbol trace v1 records scenario labels separately from actual core
frames and hashes a canonical, domain-separated binary encoding of ROM/map,
scenario, symbol-definition, timing, and value data. For qualified local-save
scenarios, `Session::new_with_sram` validates exactly 8 KiB before claiming the
process-global session and supplies it before power-on; `Session::new` continues
to use zero-filled SRAM. ROMs, external saves, and raw traces remain local, and
sampled contents do not establish every meaning or extent in an imported map.

## Portable lane

### Deterministic core

The intended conceptual API keeps immutable content separate from mutable,
serializable simulation state:

```rust,ignore
pub fn tick(
    data: &GameData,
    state: &mut GameState,
    input: FrameInput,
) -> FrameOutput;
```

`GameData` is an immutable, versioned view of the local asset pack. Snapshots
identify its compatible schema and source revision instead of serializing the
content itself. `GameState` owns authoritative gameplay data. `FrameInput`
contains controller state plus any deterministic host-service responses;
non-frame lifecycle operations such as loading a save may also use explicit
APIs. `FrameOutput` contains semantic render, audio, save, and platform
requests. The core must not poll a host clock, filesystem, GPU, audio device,
or browser API.

Authoritative simulation uses explicit fixed-width integer and wrapping
semantics. Floating-point math is reserved for presentation unless equivalence
has been demonstrated.

### Hardware migration boundary

Raw hardware behavior may exist in a temporary compatibility runtime, but new
portable systems use semantic boundaries:

| Original SNES operation | Portable boundary |
| --- | --- |
| Controller register read | `InputState` |
| OAM writes | Ordered sprite render commands |
| Tilemap/VRAM DMA | Tilemap or resource updates |
| CGRAM writes | Palette updates |
| NMI-driven update | Deterministic frame/tick boundary |
| SRAM reads and writes | `SaveStore` |
| APU port writes | Audio commands or compatibility backend |
| WRAM globals | Typed `GameState` fields |

An emulator or static recompilation runtime is a useful bridge, not the desired
public engine API.

### Content pipeline

Codecs should preserve source structure and support byte-exact round trips when
the format permits. Decoded representations should remain lossless and typed;
PNG, JSON, or other convenient exports are views, not necessarily canonical
sources.

Event bytecode is treated as a first-class language with a disassembler,
readable intermediate representation, and eventually an assembler or compiler.

### Rendering

Simulation emits platform-neutral render commands whose schema stays close to
the core boundary. The `renderer` crate is a deterministic command consumer
capable of headless framebuffer output and classic regression testing; it does
not own windows or a GPU device. Desktop and web frontends handle GPU upload,
scaling, shaders, and presentation.

Two presentation policies are planned:

- **Classic:** reproduce SNES ordering, palettes, viewport, and effects.
- **Enhanced:** optional widescreen, higher resolution, shaders, expanded UI,
  or interpolation without mutating classic simulation behavior.

### Audio

Accurate SPC700/S-DSP execution is the initial low-risk route. Replacing the
original audio driver or sequence engine is optional later work. Audio timing
must not drive authoritative gameplay.

### Saves and snapshots

Classic SRAM compatibility is preferred where practical. Portable snapshots
should be versioned separately from in-game saves and include enough state for
replay and differential debugging.

## Platform frontends

Desktop and web frontends adapt host services to the core. The web target uses
`wasm32-unknown-unknown`, browser file selection for ROM-derived assets,
IndexedDB or an equivalent storage adapter, browser input APIs, and WebAudio.
No core feature may require threads or blocking filesystem access without a
portable fallback.

## Validation hierarchy

Use the narrowest reliable test for each discovery:

1. Unit tests for arithmetic and pure transformations.
2. Byte-exact codec round trips.
3. Known hash and address-map fixtures.
4. Routine-level differential tests where execution can be isolated.
5. Frame-level state comparisons from recorded input.
6. Framebuffer and audio hashes for presentation regressions.
7. End-to-end chapter and completion replays.

ROM-backed tests are local and optional. CI tests must remain ROM-free.

## Enhancements

Classic mode remains the regression baseline. Enhancements are explicit feature
flags with documented effects. They may change rendering or controls freely;
changes to authoritative simulation require separate enhanced-state tests and
must not contaminate classic comparisons.

## Open architectural decisions

The milestone backlog intentionally leaves these choices open until measured:

- whether static recompilation is used as a temporary bridge;
- rendering and audio backend libraries.

Record consequential choices as short architecture decision records rather
than silently encoding them in implementation details.
