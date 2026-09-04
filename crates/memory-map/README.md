# Canonical memory-map crate

`memory-map` owns the typed, revision-bound memory symbols shared by the Rust
and assembly tooling. Its built-in schema-v1 artifact is
[`data/japan-v1.json`](data/japan-v1.json), bound to the normalized Japanese ROM
SHA-256:

```text
f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548
```

The ROM hash identifies the supported revision; it is not a digest of the JSON.
The crate validates the artifact's declared identity but does not load or hash a
ROM.

## Model

`MemoryMap` contains typed source, symbol, and conflicting-range records. Symbols
carry a canonical 24-bit address and width, address space, kind, access, lifetime,
aliases, confidence, evidence, and an optional ca65 operand. The modeled spaces
are hardware registers, direct page, WRAM, and SRAM. No canonical SRAM fields
are currently present because the available project evidence does not establish
the cartridge's decoded SRAM extent. Validation therefore accepts only
bank-local canonical HiROM windows at `$20-$3F:$6000-$7FFF` without asserting
which windows mirror the same physical storage.

All public Data Crystal and GameFAQs findings are `imported_claim` evidence
until project static analysis or traces corroborate them. Conflicting claims are
retained explicitly, including both meanings at `$7E:081E`. The Data Crystal
armor boundary at `$7F:8066` is stored as a typed conflicting range against the
separately selected `$7F:8068` imported lead; neither boundary is trace-proven.
Call `lookup_conflicting_claims("inventory_armor")` to query that dispute without
making the alternate range canonical.

See [Canonical memory map](../../docs/memory-map.md) for additional project
context. The crate's JSON artifact remains the machine-readable authority for
address conventions, source permalinks, confidence, and conflict records.

## Rust use

```rust,ignore
let map = memory_map::MemoryMap::built_in_japan()?;
let player_x = map.lookup_id("player_x").expect("known symbol");
let bytes = map.read_symbol(player_x, &wram)?;
let include = map.generate_ca65_include();
```

`read_symbol` accepts direct-page and WRAM symbols from a caller-provided 128 KiB
WRAM image. Direct page assumes `D=$0000`; canonical WRAM addresses are converted
to offsets in that image. Hardware and SRAM reads require a different consumer
and are rejected by this API.

Validation rejects unsupported schema/revision/hash identities, malformed or
mismatched source/evidence provenance, duplicate IDs or ca65 names, invalid
regions, confidence without matching evidence, and malformed conflict records.
Physical overlap checks treat `D=$0000` direct page as low WRAM and permit exact
overlap only through reciprocal `conflicting` aliases. SRAM ranges must fit one
canonical `$20-$3F:$6000-$7FFF` bank window. A short low-WRAM assembly operand is
accepted only when the complete symbol fits `$7E:0000-$7E:1FFF`. JSON records
reject unknown fields.

## Generated assembly include

`generate_ca65_include` emits symbols that define an `assembly_operand`, sorted
by ca65 name with stable hexadecimal formatting and revision/hash metadata. The
canonical address remains the identity used by Rust; the operand may be a
validated low-WRAM mirror used under the assembly's DBR assumptions.

The `disasm` reconstruction writes this deterministic content as
`memory-symbols.inc` inside its unique ignored `local/disasm/run-*` directory.
It is generated for each run and is not committed.

## Oracle consumers

The oracle exports selected direct-page/WRAM symbols at exact schema widths. Its
stable symbol-trace v1 digest uses domain `terranigma.symbol-trace\0` and a
canonical binary encoding that binds ROM/map identity, scenario ID, ordered
symbol IDs/addresses/widths, scenario labels, actual core frames, and values.
See [Canonical memory map](../../docs/memory-map.md) for the qualified local-SRAM
scenario and evidence limits. In particular, sampled event and inventory bytes
do not promote those broad ranges beyond `imported_claim`.

## Tests

Run the ROM-free schema and generator coverage with:

```sh
cargo test -p memory-map
```

The crate tests cover schema validation, physical overlap rules, WRAM reads,
conflicting claims, ROM-hash synchronization, and exact include generation.
Runtime samples elsewhere in the project are observations, not by themselves
proof of broad imported symbol names or extents.
