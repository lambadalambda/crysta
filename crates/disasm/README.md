# Matching disassembly tool

This crate reconstructs the Japanese reference ROM from ca65 source and opaque
ranges, then rejects any output that is not byte-identical to the validated
input. It is an explicitly invoked local tool: ordinary workspace builds,
tests, lints, and documentation do not require a ROM or assembler.

## Toolchain decision

The project uses [cc65](https://cc65.github.io/) (`ca65` and `ld65`) because it
provides native 65816 assembly, relocatable objects, exact linker placement,
`.incbin` slices, link-time assertions, and Linux/macOS/Windows support. cc65's
zlib license is compatible with the repository dependency policy. Asar is more
SNES-specific but is primarily a patcher and LGPL-licensed; WLA-DX provides a
linker but offers no needed advantage and is GPL-licensed.

The currently qualified local tools report `ca65 V2.18 - N/A` and
`ld65 V2.18 - N/A`. A different version is acceptable only when the ROM-free
tests and a local byte-matching reconstruction pass. Install cc65 with the
platform package manager (`apt install cc65` or `brew install cc65`) or use an
upstream Windows/source distribution. Tools are resolved in this order:

1. `TERRANIGMA_CC65_DIR`;
2. the ignored `tools/cc65/` directory;
3. `PATH`.

If tool binaries are redistributed, their upstream license and provenance must
be included. Local binaries remain ignored by default.

## Running

Provide a legally obtained Japanese dump, with or without a 512-byte copier
header:

```sh
cargo run -p disasm -- reconstruct 'local/Tenchi Souzou (Japan).sfc'
```

The command:

1. loads and authenticates the input with the `rom` crate;
2. rejects every revision except the Japanese behavior reference;
3. creates a fresh run directory under `local/disasm/` and writes the normalized
   input there as `rom-clean.bin`;
4. validates the canonical Japanese memory map and generates
   `memory-symbols.inc` in that ignored run directory;
5. assembles `asm/*.s` in lexical order and links with `linker.cfg`;
6. writes objects, listings, a map, and `rom-built.bin` in that same ignored run
   directory;
7. compares every output byte and prints the known Japanese SHA-256 on success.

Each invocation uses a new directory, so failed or concurrent runs cannot
replace a prior result or alias the source dump. The source dump is read and
validated before the run directory is created; it is never modified.

## Addressing and placement

Normalized file offsets cover `$000000..$3FFFFF`. The linker maps that range to
the canonical contiguous HiROM mirror `$C0:0000..$FF:FFFF`; therefore file
offset `$008000` is CPU address `$C0:8000`. `linker.cfg`, not scattered `.org`
directives, owns placement and requires one filled 4 MiB ROM region.

Opaque regions use bounded `.incbin "rom-clean.bin", offset, length` directives.
As routines become understood, those slices shrink and assembled instructions
or structured data occupy exactly the released bytes. Link-time address and
size assertions guard important boundaries; the final byte comparison remains
authoritative.

`memory-symbols.inc` is generated deterministically from the revision/hash-bound
[canonical memory map](../../docs/memory-map.md); no generated copy is committed.
The map stores canonical 24-bit addresses. A low-WRAM short operand is valid
only when the complete symbol fits the `$7E:0000-$7E:1FFF` mirror; direct-page
operands assume `D=$0000`. Hardware operands such as `$2121` are numerically
the canonical bank-zero address, while using that 16-bit spelling still relies
on the DBR mirror documented at the assembly entry point. Symbols outside the
supported short forms retain long operands.

Assembly conventions:

- select the 65816 CPU explicitly;
- state accumulator/index widths with `.a8`/`.a16` and `.i8`/`.i16` at entry
  points and after `REP`/`SEP`; these directives describe encoding and emit no
  instructions;
- do not enable `.smart` globally because it cannot follow all control flow;
- use `z:`, `a:`, or `f:` address-size overrides when exact opcode encoding
  depends on direct-page, absolute, or long addressing;
- record emulation/native mode, M/X, DBR, PBR, and direct-page assumptions for
  externally reachable routines;
- use linker assertions for fixed labels and region sizes.

The currently annotated reset, native interrupt, frame-gate, and top-level
control flow is indexed in
[Boot, interrupts, and main loop](../../docs/boot-main-loop.md).

A mismatch reports the first normalized file offset, its canonical SNES
address, and the built and expected bytes. Full ROMs, normalized copies,
objects containing opaque bytes, maps/listings, and linked outputs are all
local-only artifacts and must never be committed.
