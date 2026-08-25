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

Planning and repository bootstrap. No playable implementation exists yet.

See:

- [Architecture](docs/ARCHITECTURE.md)
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

The Japanese release is the initial code and behavior reference. The European
release is the initial official English-localization reference. Supporting both
through one maintainable address/data model is planned, but the first
behavioral test lane should target only one executable revision.

## Planned repository layout

```text
crates/                 Rust workspace (planned)
  rom/                  ROM validation, normalization, and addressing
  assets/               Compression and data-format codecs
  core/                 Deterministic portable game simulation
  oracle/               Reference execution and differential testing
  renderer/             Deterministic command consumer/classic renderer
  audio/                Audio command model and compatibility backend
  desktop/              Native frontend
  web/                  WebAssembly frontend
disasm/                 Matching 65C816 reconstruction
docs/                   Architecture and reverse-engineering documentation
local/                  Ignored ROM-backed inputs and outputs
meta/                   Repository-local milestones and issues
tools/                  Developer-facing inspection and conversion tools
```

Directories are added only when their first implementation issue begins.

## Development model

Work is tracked in [`meta/issues.md`](meta/issues.md). Each issue has a focused
detail file with acceptance criteria. Follow red-green-refactor where practical;
reverse-engineering discoveries and documentation should instead be validated
with structural checks, round trips, known hashes, traces, or reproducible
experiments.

No project license has been selected yet. Until that issue is resolved, no
permission to copy, modify, or redistribute repository content is implied.
