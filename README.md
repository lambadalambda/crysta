# Terranigma

A ROM-free reverse-engineering and portable reimplementation project for
*Terranigma* / *Tenchi Souzou*.

The long-term goal is a deterministic, maintainable game core that can run on
native platforms and the web while preserving an optional behaviorally faithful
classic mode. The original SNES program remains the reference oracle; portable
code and platform frontends are developed independently of raw SNES hardware
registers.

> [!IMPORTANT]
> This repository does not contain ROMs or extracted game assets. Contributors
> must provide their own legally obtained cartridge dump. Do not commit ROMs,
> dialogue, graphics, music, maps, or generated asset packs.

## Status

The safe repository foundation (M0), deterministic reference oracle (M1), and
matching-disassembly foundation (M2) are complete. The Japanese reference can
be reconstructed byte-for-byte; boot, interrupts, the main loop, canonical
memory, sparse ROM regions, and known indirect dispatch are versioned and
inspectable. Work is now on the content and script pipeline (M3). A bounded
compression decoder and deterministic encoder reproduce qualified JP/EU graphics
and map packets byte-for-byte. A local loaded-map inspector now shows qualified
portal-cavern captures and raw tile/collision structure. The cavern layer can
also be decoded directly from ROM, with attribute initialization matched against
loader execution. Broader maps and collision behavior qualification remain.

No playable portable implementation exists yet.

See:

- [Architecture](docs/ARCHITECTURE.md)
- [Canonical memory map](docs/memory-map.md)
- [ROM code, data, and dispatch map](docs/rom-map.md)
- [Boot, interrupts, and main loop](docs/boot-main-loop.md)
- [Compression packets](docs/compression.md)
- [Local map viewer and runtime layout](docs/maps.md)
- [Static map layers and loader qualification](docs/static-maps.md)
- [Milestones](meta/milestones.md)
- [Open issues](meta/issues.md)
- [Contributing](CONTRIBUTING.md)

## Project goals

- Document and reconstruct the original 65C816 program.
- Build deterministic reference traces from known-good cartridge dumps.
- Decode game data and scripts with round-trip-tested tools.
- Reimplement gameplay as a portable Rust core.
- Support desktop and browser frontends without embedding copyrighted content.
- Preserve a strict classic mode while making enhancements opt-in.

## Non-goals

- Distributing ROMs or extracted copyrighted assets.
- Requiring a complete line-by-line Rust translation before producing useful
  vertical slices.
- Making raw SNES registers the permanent portable-engine API.
- Mixing behavioral fixes into the classic reference mode.

## Reference dumps

Tooling will verify input before reading it. Initial known-good references are:

| Version | Normalization | CRC32 | SHA-256 |
| --- | --- | --- | --- |
| Tenchi Souzou (Japan) | Headerless, 4 MiB | `3CC7FDF4` | `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548` |
| Terranigma (Europe, English) | Strip a 512-byte copier header when present | `974523FF` | `93ba50d853e98e1ca227a2ca72389c0e3ac18d6b50c946b3f618c16c2d3edd38` |

**The Japanese release is the sole behavior reference** for reference traces,
symbol maps, and differential tests through M6. The European release is used
only as the official English-localization source until cross-version behavior
support is defined in M8. See
[ADR 0001](docs/adr/0001-version-support-model.md) for the full
version-support model.

## Planned repository layout

```text
crates/                 Rust workspace
  rom/                  ROM validation, normalization, and addressing
  oracle/               Reference execution and differential testing
  disasm/               Matching 65C816 reconstruction tooling
  memory-map/           Typed revision-bound memory symbols
  assets/               Bounded compression and content codecs
  map-inspector/        Local runtime map captures and browser viewer
  core/                 Deterministic portable game simulation (planned)
  renderer/             Deterministic command consumer/classic renderer (planned)
  audio/                Audio command model and compatibility backend (planned)
  desktop/              Native frontend (planned)
  web/                  WebAssembly frontend (planned)
docs/                   Architecture and reverse-engineering documentation
local/                  Ignored ROM-backed inputs and outputs
meta/                   Repository-local milestones and issues
tools/                  Developer-facing inspection and conversion tools
```

Directories are added only when their first implementation issue begins.

## License

Original project code is licensed under the [MIT License](LICENSE).

MIT does not cover ROM-derived content. That content remains the property of
its rights holders and stays out of version control. The publication policy in
[Contributing](CONTRIBUTING.md) defines what reconstructed source and derived
artifacts may be committed.

## Development model

Work is tracked in [`meta/issues.md`](meta/issues.md). Each issue has a focused
detail file with acceptance criteria. Follow red-green-refactor where practical;
reverse-engineering discoveries and documentation should instead be validated
with structural checks, round trips, known hashes, traces, or reproducible
experiments.
