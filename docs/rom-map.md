# ROM code, data, and dispatch map

The matching-disassembly tooling owns a sparse, versioned classification map for
the Japanese behavior-reference ROM. The committed source is
[`crates/disasm/data/rom-map/japan-v1.json`](../crates/disasm/data/rom-map/japan-v1.json),
and its Rust model and validators live in
[`crates/disasm/src/rom_map.rs`](../crates/disasm/src/rom_map.rs). The artifact
contains metadata only, never ROM bytes.

## Identity and address spaces

Schema version 1 is bound to revision `japan` and normalized-image SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
Every region and entry records both a normalized offset and its unique canonical
HiROM address. Entries additionally record the runtime mirror at which control
is known to arrive.

The `rom` crate provides distinct checked types for these namespaces:

- `NormalizedOffset`: `$000000..$3FFFFF` in the headerless image;
- `CanonicalRomAddress`: `$C0:0000..$FF:FFFF`;
- `RuntimeRomAddress`: full ROM windows in banks `$40..$7D` and `$C0..$FF`,
  plus `$8000..$FFFF` in banks `$00..$3F` and `$80..$BF`.

Banks `$7E/$7F`, lower halves of partial banks, and values wider than 24 bits
are rejected. Explicit types prevent a number such as `$808000` from silently
being interpreted as either a file offset or CPU address.

## Classification policy

The map is intentionally sparse. Absence means **unclassified**, not code.
Known code ranges and typed data ranges are non-overlapping; data kinds include
raw bytes, vectors, function-pointer tables, callback records, and script-entry
tables. A known function start may own a one-byte code region when its complete
extent has not been proved. This marks a valid decode entry without guessing
that the gap before the next start is executable.

Entry points have stable IDs, semantic kinds, canonical/runtime addresses,
confidence, and evidence. Optional 65C816 decode state records E, M, X, DBR,
and D independently. Only entries with every component known are exposed as
decoder seeds. Conflicting external claims have a separate typed collection and
do not participate in canonical region lookup.

Each claim cites a repository source and provenance. Public maps are leads, not
ground truth: the public Data Crystal Terranigma ROM map was empty when this map
was established, so no callback-record or script-entry table was invented.

## Known indirect dispatch

Schema version 1 represents ROM tables and pointers read from mutable memory.
ROM table layouts declare an entry count, stride, pointer offset, and one of
three encodings: bank-local little-endian 16-bit, runtime 24-bit, or normalized
24-bit. Validation requires each decoded pointer to match the exact start of its
declared entry.

The current Japanese map proves:

- **COP services:** selectors `$00..$7C` index 125 bank-$80 pointers at
  normalized `$0083B2..$0084AB`. This is the maximal contiguous valid run; the
  following word is `$109A`, which is not in a bank-`$80` ROM-backed HiROM
  window.
- **Top-level state:** `$80:805A` executes `JMP ($049E)`. The reset trace stops
  at that instruction and proves low-WRAM pointer `$7E:049E` contains `$805D`,
  then traces the jump to runtime `$80:805D`. Mutable dispatch is modeled but
  deliberately cannot be resolved from ROM bytes alone.

The optional local integration test authenticates a user-provided Japanese ROM,
resolves all 125 COP pointers, checks the table boundary, and verifies the
mutable-dispatch metadata. It skips when the ROM is absent.

## Inspection

Address queries require an explicit namespace:

```sh
cargo run -p disasm -- inspect-rom offset:008000
cargo run -p disasm -- inspect-rom canonical:C08000
cargo run -p disasm -- inspect-rom runtime:80:8000
```

All three examples identify normalized `$008000`, region `boot_main`, and entry
`reset`. A valid but unknown location reports `region=unclassified` and
`entry=none`; an out-of-range or unmapped address is rejected.

Rust consumers can use `RomMap::built_in_japan()`, typed address lookups,
`decoder_seeds()`, and `resolve_dispatch()`. Structural validation rejects
unknown JSON fields, address disagreement, overlaps, invalid evidence, unsafe
decode states, malformed table extents, and targets that are not exact entries.
`validate_rom()` additionally checks the authenticated image revision and
SHA-256.

## Reconstruction integration

`reconstruct` authenticates the local ROM, validates this map, resolves the COP
table against the actual image, and writes a deterministic `rom-map.inc` beside
`memory-symbols.inc` in the ignored per-run output directory. Named entries
produce distinct ca65 constants such as:

```text
NativeNmiHandlerCanonical = $C5F98F
NativeNmiHandlerRuntime = $85F98F
```

The assembly consumes runtime constants from that generated include instead of
duplicating addresses by hand. The final linker output is still compared with
every byte of the normalized source ROM; map generation cannot weaken the
byte-identical reconstruction gate.
