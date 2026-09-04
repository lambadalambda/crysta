# Matching Disassembly Toolchain

Status: decided 2026-08-27. ADR for
[the matching-build issue](../../meta/issues/matching-disassembly-build.md).

## Context

M2 needs an incremental 65C816 reconstruction that preserves opaque local ROM
ranges, fixes known code and data at exact addresses, and proves byte identity.
The workflow must remain optional and must not make normal Rust builds depend on
a cartridge dump or assembler.

## Decision

Use the zlib-licensed cc65 toolchain: `ca65` for 65816 assembly and `ld65` for a
flat, linker-placed image. The committed linker map assigns normalized offsets
`$000000..$3FFFFF` to the canonical HiROM addresses
`$C0:0000..$FF:FFFF`. Bounded `.incbin` slices preserve unknown ranges while
assembled source replaces understood ranges.

Run reconstruction through the explicit `disasm` binary rather than a Cargo
build script. It validates a user-provided dump through the `rom` crate,
requires the Japanese reference revision, keeps all derived products under
ignored `local/disasm/`, and performs the final byte comparison. Ordinary
workspace commands remain ROM-free.

Operational setup, syntax conventions, version policy, assertions, and artifact
handling are documented in the [crate README](../../crates/disasm/README.md).

## Alternatives

- **Asar** has convenient SNES patching features, but patching an existing image
  is a poorer fit than constructing a fresh linked output, and its LGPL license
  would require a separately recorded tooling exception.
- **WLA-DX** supports 65816 sections and linking but adds no required capability
  over cc65 and is GPL-licensed.
- **A custom assembler** would maximize control but duplicate mature encoding
  and linking machinery before producing disassembly value.

## Consequences

- Exact placement is explicit and reviewable in source and linker assertions.
- Developers need cc65 only when running reconstruction locally.
- Tool-version changes are qualified by ROM-free comparator tests and a local
  full-ROM match rather than accepted silently.
- Opaque `.incbin` directives remain metadata-only in Git; the referenced bytes
  and every reconstructed binary stay local.
