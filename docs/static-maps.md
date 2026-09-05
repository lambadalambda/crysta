# Static map layers and loader qualification

The Japanese portal cavern (map **`$0128`**) now has a ROM-only layer decoder and
a trace-qualified attribute initialization path. Static output matches the
emulator **byte-for-byte at two loader stages**. This is one qualified map, not
a general map-script interpreter, full map renderer, or collision engine.

The [loaded-map viewer](maps.md) still displays runtime captures. The distinction
between compressed source words, initialized words, and gameplay changes matters.

## Reproduce

From the repository root:

```sh
# Decode directly from the authenticated ROM; no save or emulator boot needed.
mkdir -p local/maps
cargo run -p map-inspector -- decode-layer \
  'local/Tenchi Souzou (Japan).sfc' 0x90000 > local/maps/cavern-static.json

# Authenticate the pinned SRAM and compare static data at actual loader stops.
cargo run -p map-inspector -- qualify-loader \
  'local/Tenchi Souzou (Japan).sfc' 'local/saves/Terranigma.srm' \
  > local/maps/cavern-loader.json
```

`decode-layer` accepts a **hexadecimal normalized/headerless offset**, with an
optional `0x` prefix. It emits source range/hash, dimensions in cells, decoded
hash and raw row-major words as JSON. Its `source_sha256` includes the two
dimension bytes; the compressed-layer hash in the table below covers only the
packet. It does not infer a map ID from an offset
or apply an arbitrary attribute table. It runs no oracle session and needs no
SRAM, though the native binary still links/builds the oracle dependency.

`qualify-loader` is deliberately fixed to the existing Japanese ROM and pinned
three-slot save documented in [maps.md](maps.md). It emits source extents, hashes,
per-stop instruction counts/digests and sampled pointers, plus final differences.
Command success verifies intermediate bytes and final checkpoint identity; it
reports final cell differences without enforcing the pinned single-cell result.
The integration test additionally enforces that exact difference and the hashes.
It writes no files itself. Redirect ROM-derived output only under ignored
`local/`; do not commit or redistribute captures or decoded content.

## Static container

| Field | Size | Qualified interpretation |
| --- | ---: | --- |
| Width | 1 byte | Number of 256-pixel pages; multiply by 16 for cell width |
| Height | 1 byte | Number of 256-pixel pages; multiply by 16 for cell height |
| Compressed packet | Variable | Existing zero-header [compression codec](compression.md) |

Packet output is `width_in_cells * height_in_cells * 2` bytes, row-major u16
little-endian cells. The cavern has page dimensions `(5, 2)`, hence 80×32 cells.
All 2,560 source words in this fixture are at most `$01FF`.

`assets::maps::StaticLayer::from_rom(image, offset)` is pure and bounded. Its
caller authenticates the ROM and establishes source-pointer provenance. It:

- rejects offsets at or beyond 4 MiB, missing headers, zero dimensions and layers
  exceeding the supported `$4000`-byte runtime capacity;
- confines the source container to one 64 KiB bank (cross-bank behavior remains
  unsupported even when a larger input slice exists);
- bounds decompression to the dimensions and rejects mismatched output lengths;
- preserves the exact original container, including ignored compression bits,
  separately from decoded cells, without canonical recompression;
- offers checked coordinate lookup, source range/bytes, and lossless layer bytes.

The parser does not recognize map identity from bytes, resolve scripts or tables,
or prove a malformed-but-structural container is used by the game.

## Source provenance and actual loader

The earlier candidate at `$80:F690` was **not** the loading path used here. The
cavern path writes the high dimension bytes at `$86:9538` / `$86:953E`, with
8-bit A and X=0, after reading the source dimension bytes at `$86:8ADA` onward.
The failed earlier probe is retained as history, not evidence against this path.

At map loading, `$7E:047E = $0128` and `$7E:0480 = $0250`. Code at `$86:9010`
adds those values and reads a three-byte entry from `$86:959C + 3 * map_id`.
For the cavern this is normalized **`$069914`**, producing script pointer
**`$B3:89D3`**. The trace later reaches the shared layer command with script base
`$B3:89BF`; the pointer conversion routine at `$86:90E7` resolves its encoded
pointer to **`$C9:0000`** in direct-page `$66..$68`.

These are observed pointer provenance, not a new general static script resolver.
Script control flow and packed relative-pointer variants remain separate work.
Static extraction currently takes the qualified normalized offset explicitly.

### Qualified ROM extents

All ranges below are normalized/headerless and half-open:

| Resource | Range | Compressed bytes | Decoded bytes |
| --- | --- | ---: | ---: |
| Layer container, including dimensions | `$090000..$0905F8` | 1,528 total | 5,120 |
| Layer packet only | `$090002..$0905F8` | 1,526 | 5,120 |
| Metatile attribute packet | `$2B439E..$2B4462` | 196 | 512 |

SHA-256:

| Content | SHA-256 |
| --- | --- |
| Compressed layer packet | `0c17b274896c4994ed883d1601540ad3f26f8d151a0f2f442e952ca4a9895b4d` |
| Decoded raw layer | `6a7495daacd32b54f0b6caf22bde1b873fa444455c5a7c39c854adfa230015fb` |
| Compressed attribute packet | `0f6ed48c40d50d47cab61044a53f24e613d40d31fe9c66aead55d796433fe8c3` |
| Decoded attribute table | `8adcb94d19af67b8995b84c442005cadbcef176df3811ff79a111dc590cb3914` |
| Initialized layer after attribute lookup | `f1b717b31a416aab274c8f62ca53b9b585df7a60a9799c836caeeb9f949b550b` |

### Reproducible trace stops

Replay labels `0..1112` run normally with the documented Start/A inputs. A is
released before tracing. Each stop has an 80-frame / 2,000,000-instruction bound.
The target entry is included **before** its instruction executes. A frame number
at a mid-frame stop is the number of completed frames, not a replay label.
All ten stops have direct page `$0000`.

| Target PC | Completed frames | Observation |
| --- | ---: | --- |
| `$86:86ED` | 1119 | Map loader entered with ID `$0128` |
| `$86:902B` | 1119 | Initial script pointer `$B3:89D3` resolved |
| `$86:8A40` | 1135 | Attribute packet `$EB:439E` → scratch `$7E:5000` |
| `$86:8A44` | 1136 | Scratch bytes equal the statically decoded 512-byte table |
| `$86:8A68` | 1136 | Copy to `$7F:0000` matches that table |
| `$86:8ADA` | 1136 | Dimension-prefixed layer source `$C9:0000` |
| `$86:8B66` | 1136 | Packet `$C9:0002` → `$7E:A000`, dimensions 1280×512 pixels |
| `$86:8B6A` | 1138 | All 5,120 freshly decoded runtime bytes equal static output |
| `$86:9332` | 1178 | Attribute initialization begins; table still matches ROM output |
| `$86:936C` | 1181 | All 5,120 initialized bytes equal the pure attribute transform |

The attribute copy helper uses self-modified code in low WRAM (including an MVN
at `$86:0403`, a WRAM mirror), so decoding every traced PC against ROM bytes would
be wrong. The qualification compares copied memory and pointer state instead.
Raw research traces and scratch snapshots remain local.

## Attribute initialization versus gameplay

The loop at `$86:9351..$86:936A` reads each cell, masks to nine bits, uses that
index into the byte table at `$7F:0000`, masks the table byte to seven bits, and
replaces the upper cell bits:

```text
index = raw_word & $01FF
initialized_word = index | ((attributes[index] & $7F) << 9)
```

`StaticLayer::attributed_cells(&[u8; 512])` reproduces that transformation without
mutating the original static layer. This qualifies the low-nine-bit **attribute
index** and initialization arithmetic. It does not yet qualify how metatile
indices map to terrain graphics or what attribute values mean for movement.

Applying the attribute table changes **656** of 2,560 raw source words. Continuing
to the existing frame-1601 gameplay checkpoint leaves only **one** difference
from the initialized layer:

| Cell index | Coordinate | Initialized | Runtime | Difference |
| ---: | --- | --- | --- | --- |
| 448 | (48, 5) | `$0007` | `$8007` | Bit 15 set later |

The runtime layer hash remains
`c3c7af3a0ef3c6c53e641b058ccaccad8a9b5ea42dca79c28e41c41ecaa450c5`.
Thus the earlier 657 raw-vs-runtime differences separate into 656 attribute
initializations and one subsequent change. The writer and gameplay meaning of
that final bit are **not** claimed here; it must not be baked into static assets.

## Validation and remaining scope

```sh
cargo test -p assets --test static_maps --test local_static_maps
cargo test -p map-inspector --test local_loader --test local_static
```

Synthetic tests use red→green development for parsing and attribute lookup,
then cover non-square indexing, exact output capacity, malformed offsets,
headers/dimensions/packets, truncation, bank boundaries and preservation of a
noncanonical encoding. Local static tests authenticate the Japanese ROM and pin
source extent/content. The fresh-process loader test exercises real execution,
asserts equality at intermediate stages, and pins final hashes/differences.
Missing local inputs cause explicit skips, not fallback fixtures.

No new third-party implementation was used. Evidence comes from the owned ROM,
the existing compression implementation and the vendored oracle. Static opcode
research used the vendored ares opcode metadata; no decoded instruction dumps or
extracted map bytes were committed.

The [parent map issue](../meta/issues/decode-map-collision-formats.md) remains
open: general map-script/pointer resolution, graphics/metatile reconstruction,
representative indoor/outdoor/dungeon/world maps, placements/regions/exits,
dynamic changes and trace-qualified collision behavior are still required.
