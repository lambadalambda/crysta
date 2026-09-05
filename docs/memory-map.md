# Canonical memory map

The canonical memory map is the revision-bound source of names and metadata for
SNES memory used by the project. The committed artifact is
[`crates/memory-map/data/japan-v1.json`](../crates/memory-map/data/japan-v1.json),
with its Rust model and validation in the [`memory-map`](../crates/memory-map/)
crate.

This is a curated map, not a claim that every RAM location is understood.

## Identity and schema

Schema v1 binds each map to a supported ROM revision and the SHA-256 of its
normalized image. The current artifact is for revision `japan` and hash:

```text
f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548
```

The hash identifies the ROM revision to which the map applies; it is not a hash
of the JSON file. Loading the built-in map validates this declared identity.
Consumers that accept a ROM still use the `rom` crate to authenticate the ROM
bytes themselves.

Each symbol records:

- a stable ID and ca65 name;
- a typed address space (`hardware`, `direct_page`, `wram`, or `sram`),
  canonical address, and byte width;
- a semantic kind (`scalar`, `pointer`, `bitfield`, `buffer`, or `port`);
- access (`read_only`, `write_only`, or `read_write`) and lifetime
  (`hardware`, `interrupt`, `frame`, `map`, `session`, or `persistent`);
- explicit aliases, confidence (`imported_claim`, `static_corroborated`, or
  `trace_corroborated`), and qualified evidence records; and
- an optional ca65 assembly operand.

Unknown JSON fields are rejected. Validation checks the schema/revision/hash
binding, nonblank unique identifiers, ca65 names, nonzero in-range regions,
source locators, evidence references, and confidence/provenance compatibility.
External sources require an HTTP(S) URL and valid retrieval date; project
sources require a normalized repository-relative path. Imported evidence must
cite an external source, while project analysis, traces, documentation, and
tests must cite project sources.

Overlap checks use physical WRAM coordinates: direct page with `D=$0000`
overlaps the corresponding low WRAM bytes. Partial overlaps are rejected. An
exact overlap is accepted only when both symbols reciprocally identify each
other with `conflicting` aliases. Typed boundary/range disputes remain separate
`conflicting_claim` records and can be queried by symbol ID with
`lookup_conflicting_claims` without making the disputed range canonical.

SRAM regions, if added, must remain within one canonical HiROM window in
`$20-$3F:$6000-$7FFF`; they may not cross banks. This conservative validation
does not assert the cartridge's decoded SRAM extent or which windows physically
mirror one another.

## Canonical addresses and assembly operands

A symbol's `address` is its canonical 24-bit SNES bus address. For example,
`player_x` is `$7E:1000`, even when reconstructed code encodes the operand as
`$1000`. The optional `assembly_operand` is an instruction-encoding choice, not
a second canonical address.

Short operands rely on the execution context documented by the assembly:

- direct-page operands such as `$0036` assume `D=$0000`;
- absolute operands for canonical low WRAM use the mirror visible through the
  active DBR (the annotated reset/main-loop path establishes `DBR=$81`);
- bank-zero hardware addresses can be encoded with a 16-bit operand through an
  applicable hardware-register mirror, while explicit long accesses retain
  bank zero; and
- WRAM outside the low mirror, such as `$7F:8068`, remains a long operand.

Validation accepts a low-WRAM short operand only when the symbol's complete
region lies within `$7E:0000-$7E:1FFF`; a region that crosses the mirror boundary
is rejected. Hardware operands such as `$2121` are numerically the canonical
bank-zero address, so their execution-time DBR assumption remains an assembly
responsibility. Consumers use canonical addresses for lookup, comparison, and
state export.

## Evidence and unresolved claims

The public sources were imported as leads, not ground truth:

- [Data Crystal, Terranigma RAM map revision 53316](https://datacrystal.tcrf.net/w/index.php?title=Terranigma/RAM_map&oldid=53316)
- [GameFAQs, Terranigma SRAM/WRAM/ROM Data Guide](https://gamefaqs.gamespot.com/snes/588784-terranigma/faqs/78151)

`current_map`, `player_x`, and `player_y` are trace-corroborated. The broad
`event_flags` and inventory item, key-item, weapon, armor, and magic ranges
remain `imported_claim`. Runtime samples corroborate selected contents and
record starts only; they do not establish every byte, event-bit meaning, record
layout, or full range extent.

Two kinds of conflict are preserved explicitly:

- Data Crystal gives two incompatible meanings for `$7E:081E-$7E:081F`:
  `map_x_scroll` and `crystal_blue_camera_y_claim`. Both remain imported symbols
  at the same exact range with reciprocal `conflicting` aliases; neither meaning
  is resolved.
- The selected imported armor range starts at `$7F:8068` and cites GameFAQs.
  A typed `conflicting_claim` records Data Crystal's `$7F:8066` start and its
  inferred extent through `$7F:807F`. The local trace samples records at
  `$7F:8068`, but neither competing boundary is proved over the other.

Schema v1 supports SRAM metadata, but the current artifact contains no canonical
SRAM symbols. Available evidence is not sufficient to claim field names,
extents, or physical mirrors.

## Stable symbol trace v1

The oracle's symbol trace is separate from the general frame export. Every
checkpoint stores both a scenario-local label and the actual core frame counter,
so scenario steps are not confused with emulator time.

The v1 digest is SHA-256 over a canonical binary encoding, independent of JSON
or serde. It begins with the domain separator
`terranigma.symbol-trace\0`, then binds:

1. the format version;
2. the normalized runtime ROM SHA-256, memory-map schema version, and map ROM
   SHA-256;
3. the length-prefixed scenario ID;
4. selected symbols in order, including each length-prefixed ID, canonical
   address, and width; and
5. checkpoints in order, including scenario label, actual core frame, and every
   length-prefixed raw symbol value.

All numeric fields, lengths, and counts are unsigned 32-bit little-endian values;
SHA-256 values are their raw 32 bytes and strings are exact UTF-8 bytes. Thus a
digest change identifies a change in identity, selected definitions, timing, or
sampled values rather than a serializer choice.

## Qualified local-SRAM trace

The qualified ROM-backed scenario uses the Japanese ROM and an optional local
save obtained externally from
[FantasyAnime's Terranigma save page](https://fantasyanime.com/legacy/terran_saves.htm):
**Game Save #1, default slot 3** (`Chapter 2, World Resurrection`). The required 8 KiB
SRAM SHA-256 is:

```text
709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055
```

The save is not bundled or downloaded by the project. Contributors place their
own copy at `local/saves/Terranigma.srm`; the integration test skips unless both
it and the Japanese ROM are present. The source record and hash qualify the
local input used by the scenario, but do not elevate unrelated FantasyAnime,
Data Crystal, or GameFAQs meanings to project-verified facts.

The historical scenario ID `qualified-sram-slot-1-movement` is retained only
to preserve its published v1 digest. It actually loads default **slot 3**;
[the Crysta replay](opening-doorway.md) qualifies real slot 1 using two Up taps.
The trace selects `current_map`, `player_x`, `player_y`,
`event_flags`, `inventory_items`, `inventory_weapons`, and `inventory_armor`.

| Scenario label | Actual core frame | Controlled assertions |
| ---: | ---: | --- |
| 1600 | 1601 | Map is `$0128`; player X/Y are `776/112`; event bytes include nonzero `EF 7F A1 70`; item records begin `10 05 1A 01 13 02 11 01`, weapon records `80 01 81 01`, and armor records `A1 01 A0 01` (ID/quantity pairs). |
| 1799 | 1800 | Before controlled Right input takes effect, player X/Y remain `776/112`. |
| 1840 | 1841 | After Right input, player X/Y are `834/128`; the sampled event block and item block remain byte-stable from label 1600. |

The stable symbol-trace v1 digest is:

```text
6d2c6756ff22e9c41a8287f649252599a493422b54d684e886b64e9b12741595
```

The digest binds all selected values at all three checkpoints, while committed
assertions intentionally make only the controlled claims above. Raw SRAM and
trace data remain local and ephemeral.

## Consumers and generated output

The Rust crate embeds, deserializes, and validates the JSON, then provides typed
ID/address lookup, conflict queries, and direct-page/WRAM reads. The oracle uses
those APIs to export requested symbols in schema-defined widths and order.

`Session::new_with_sram` accepts exactly 8 KiB of caller-provided SRAM. Length is
validated before claiming the process-global ares session, and valid bytes are
copied into the core before the emulated system powers on. `Session::new`
preserves the existing behavior by supplying zero-filled 8 KiB SRAM.

The disassembly generates deterministic `memory-symbols.inc` content. Names are
sorted lexicographically and its header includes schema version, revision, and
ROM hash. `disasm reconstruct` writes it into a unique ignored
`local/disasm/run-*` directory and still requires the reconstructed image to be
byte-for-byte identical to the normalized Japanese ROM. No generated include is
committed.

See [`crates/memory-map/README.md`](../crates/memory-map/README.md) for crate APIs
and focused tests, and
[`crates/disasm/README.md`](../crates/disasm/README.md) for reconstruction setup.
