# Map-ID lookup and loading-script projection

The static pipeline can now start with a **Japanese map ID**, rather than a known
packet offset. `assets::maps::scripts` follows a bounded subset of the loading
interpreter to produce typed commands, exact instruction bytes and resolved ROM
resource pointers. `map-inspector resolve-map` also decodes supported layer loads.

This is **not the gameplay event VM or a complete map loader**. It does not execute
audio/display effects, simulate resource caches, compose partial transfers, or
supply unspecified flags for conditional branches. It is a resource-loading
projection, with unsupported behavior rejected explicitly.

## Local command

```sh
mkdir -p local/maps
cargo run -p map-inspector -- resolve-map \
  'local/Tenchi Souzou (Japan).sfc' 128 > local/maps/cavern-program.json
cargo run -p map-inspector -- resolve-map \
  'local/Tenchi Souzou (Japan).sfc' 25 > local/maps/map-0025-program.json
```

IDs are **hexadecimal**, with an optional `0x` prefix. The CLI authenticates the
Japanese ROM, then returns before SRAM loading or emulator session creation.
It still links the native oracle dependency, as does `decode-layer`.

JSON schema version 1 includes:

- map ID, ROM revision/hash, initial runtime script address;
- the projected instruction path, including repeated instructions and returns;
- numeric and readable instruction addresses, exact raw bytes, readable operation
  text and resolved resource pointers;
- each supported layer load separately, with originating instruction, flags,
  normalized source range, dimensions in cells, decoded hash and raw cell words.

Readable operation strings are diagnostic text, not a replacement opcode ABI.
Raw bytes and addresses preserve provenance. Layers are separate source loads in
execution order—not a final merged map. The CLI accepts compressed first/second
layer flags `1..3`; alternate or disabled layer modes fail rather than being
silently interpreted as the same container. A failure emits no partial manifest.
All output is ROM-derived and must remain under ignored `local/`.

## Table provenance and bounds

The loader entry at `$86:9010` reads the map ID at `$7E:047E`, adds `$7E:0480`
(observed as twice the ID), and indexes a three-byte pointer table. Subscript
lookup at `$86:9035` uses a separate three-byte table.

| Japanese table | Normalized range (half-open) | Bounded indices |
| --- | --- | --- |
| Map entries | `$06959C..$06A28C` | `$0000..=$044F` (1,104 entries) |
| Subscript entries | `$06A28C..$06A505` | `$0000..=$00D2` (211 entries) |

The first range ends at the code-referenced subscript-table base. The second
ends at the code entry `$86:A505`; direct JSL references also identify that
boundary. These are physical table bounds, **not a claim that every entry is
usable**. For example, some entries contain the non-ROM pointer `$000004` and are
rejected. Both tables hold little-endian 24-bit runtime pointers.

The reader uses the workspace `rom::RuntimeRomAddress` type rather than duplicating
HiROM mirror validation. `assets` therefore now has a normal dependency on `rom`
(previously test-only). Parsing remains pure: authentication and filesystem I/O
are caller-owned.

## Packed resource pointers

Resource commands call `$86:90E7`. This pointer encoding is distinct from the
absolute three-byte pointers in the script tables. Under the qualified binary
arithmetic and 16-bit-index execution state:

```text
packed = little_endian_u24(operand)
within_bank = packed & $7FFF
bank_delta = (packed >> 15) & $FF
bank = (original_stream_bank + bank_delta) mod 256

if original_stream_bank < $C0 and bank < $C0:
    within_bank |= $8000
result = bank:within_bank
```

Packed bit 23 is ignored. The original **stream base bank** is used, not the bank
of some independently normalized source offset. Calls replace that base; returns
restore it. A base at or above `$C0` forces offset bit 15 clear even if bank
addition wraps below `$C0`. Incoming carry is cleared by the code. The apparently
special `$86:9135` path performs `CLC; ADC #$00`, not a guessed bank correction.

Examples exercised by the cavern path:

- base `$B3`, packed `$0B0000` → `$C9:0000` (dimension-prefixed layer);
- base `$D9`, packed `$09439E` → `$EB:439E` (attribute packet).

The static resolver rejects results outside ROM-backed windows and checks that
at least the resource's first byte exists in the supplied image. It does not
infer every resource's extent; the relevant content codec must validate that.
The CPU helper's extra overlapping fourth-byte read/store is not projected as a
resource field: each packed operand advances three bytes, which are retained.

## Supported command framing

Opcode addresses and widths were studied in the authenticated ROM and compared
with the local oracle traces. Only the canonical one-hot resource opcode bytes
below are accepted. The CPU's highest-set-bit dispatch also permits other bit
patterns; the static reader deliberately rejects those unqualified variants.

| Opcode | Total bytes | Packed pointer starts at byte index | Recorded family |
| --- | ---: | ---: | --- |
| `$80` | 9 | 4 | Graphics |
| `$40` | 7 | 4 | Palette |
| `$20` | 8 | 5 | Metatiles / attributes |
| `$10` | 5 | 2 | Layer |
| `$04` | 5 if byte 4 is zero, otherwise 6 | 1 | Background tilemap |
| `$02` | 6 | 3 | Audio |
| `$01` | 7 | 4 | Sprite resource |

Byte index zero is the opcode. Non-pointer fields are retained in the exact raw
instruction rather than assigned unproved gameplay meanings. Resource commands
are recorded even when the actual loader might skip a cached transfer.

### Control flow

| Command | Total bytes | Projected behavior |
| --- | ---: | --- |
| `$00` | 1 | Return from a call; otherwise consume pending stream or finish |
| `$08 F9` | 4 | Call a subscript; operand bit 15 enables flagged unwind |
| `$08 FF` | 4 | Jump without a new frame, or return if current call is flagged |
| `$08 F8` | 2 | Return from a call; at root, continue |
| `$08 FA` | 4 | Replace pending subscript; zero cancels it |
| `$08 FD` | 6 | Jump through the subscript table when an event flag matches |
| `$08 FE` | 4 | Return from flagged call; otherwise skip its opaque word |
| `$08 FC` | 4 | Record audio selection; do not execute the global audio scan |
| `$08 00` | 4 | Record display configuration; do not execute hardware changes |

F9 saves the caller's stream base, cursor and flag state. The actual byte index is
`(operand * 3) & $7FFF`. Bit 14 variants are rejected, so the supported index can
be extracted by clearing bit 15; treating bit 14 as just another flag would be
wrong. Unflagged FF uses a direct subscript index within the bounded table.
Flagged FF returns **without** looking up its operand, even if that operand would
be an invalid table index.

### `$08 FD`: the event-flag branch

This is the only state-dependent instruction the projection evaluates, and the
only one whose result depends on anything outside the ROM. It is six bytes,
`08 FD <condition> <target>`, and `$86:907D` implements it as:

```text
LDA [$62],Y : BMI set_path
AND #$7FFF : JSL $80BBC7 : BCS skip : BRA take   ; clear sense
set_path: AND #$7FFF : JSL $80BBC7 : BCS take    ; set sense
skip: INY x4 : RTS                  ; skip both operand words
take: INY x2 : JSR $902C : RTS      ; jump through the subscript table
```

`$80:BBC7` wraps `$80:BBA6`, the event-flag test: flag `n` is bit `n & 7` of
byte `(n & $0FFF) >> 3` counting from `$7E:06C0`, selected through the mask
table at `$80:BBD3`. So the flag index is `condition & $0FFF`, and condition
bit 15 selects the sense — set means branch-if-set, clear means branch-if-clear.
Bits 12..14 reach neither, because `$86:907D` clears bit 15 and `$80:BBA6`
masks to `$0FFF`.

`resolve_map` evaluates this against a fresh game, where no flag is set;
`resolve_map_with_events` takes an explicit bitmap. A script containing no
`$FD` resolves identically for any bitmap.

Flags past the end of a supplied bitmap are **refused**, not read as clear:
answering a branch with no evidence would pick a loading path outright. Map
`$0176` is the only entry that branches on a flag above 511 (663, byte `$52`),
which the 64-byte event block recorded in `docs/opening-doorway.md` does not
reach. `EventFlags::AllClear` covers every flag by construction.

`AllClear` is not the measured new-game state: a new game sets flags 32 and
251 (`docs/new-game-bootstrap.md`). No branch in the table references either,
so the two agree on 1,103 of 1,104 maps — the exception being `$0176` above,
where a faithful 64-byte bitmap is refused and `AllClear` resolves.

Supporting this one instruction took the Crysta slice from **3 of 24 maps
resolving to 24 of 24**, and the whole table from **557 of 1,104 to 900**. Its
absence was the single reason map loading still needed a per-map allowlist
there. The 30 flags branched on span 35..=663.

Pending FA state is global to the loading path, not scoped to a call. A nested
`$00` returns before considering it. At root END, a nonzero pending index is
loaded and cleared; that stream can schedule another one. Root F8 is not END.

`$08 FD` depends on game flags via `$80:BBC7` and is evaluated against a
caller-supplied bitmap; see below. Other unknown control variants fail. Note
that the dispatch chain at `$86:8C17` has no `CMP #$FE`, so hardware treats
every byte outside the chain the way it treats `$FE`; this projection refuses
them instead, which is stricter than hardware rather than a claim about it. FC's global audio-list scan and display effects
are outside this projection; their nested operations are not included in its
instruction path. This is why the result is not advertised as a general event
interpreter or an exact full-machine side-effect trace.

## Safety and losslessness

`resolve_map(image, id, Limits)` rejects malformed pointers, truncated table or
instruction bytes, unsupported commands, and unqualified bank-crossing streams.
The cursor may end an instruction exactly at a bank boundary, but cannot start a
subsequent instruction across it. Original runtime mirrors remain distinguishable
from normalized offsets.

Default limits are 4,096 instructions and 32 active calls. Callers may select up
to 65,536 instructions and 64 calls; zero instruction budget is invalid, while
zero call depth explicitly prohibits calls. These limits bound loops, recursion,
work and allocation; there is no unbounded terminator search.

Each `Instruction` retains its exact bytes and address. Tests compare them with
the corresponding source slice. This is lossless instruction/source retention,
not an assembler or a promise that all scripts can already be relocated or
round-tripped through a semantic encoding.

## Qualified cases

Five map IDs resolve through the tables and decode eight layer packets. The
fixture test pins entry pointers, instruction counts, layer extents, dimensions
and decoded hashes. Only the two cases explicitly marked below also have fresh
loader equality checks. Unvisited IDs are not assigned guessed room names.

| Map ID | Initial script | Projected instructions | Layer ranges (normalized, half-open) | Dimensions (cells) |
| --- | --- | ---: | --- | --- |
| `$0004`, save-selection presentation | `$B3:8002` | 16 | `$0D16C7..$0D1725` | 16×48 |
| `$0024`, static-only | `$B3:8037` | 16 | `$0B0C9D..$0B0FAD`; `$0A7E71..$0A7FDA` | 32×64; 16×16 |
| `$0025`, static-only | `$98:82B0` | 16 | `$29BAF5..$29C8A1`; `$328A1E..$328A69` | 64×80; 16×16 |
| `$0128`, portal cavern | `$B3:89D3` | 15 | `$090000..$0905F8` | 80×32 |
| `$0263`, static-only | `$D9:1182` | 11 | `$2B3860..$2B397D`; `$2B67F2..$2B6816` | 16×32; 16×32 |

See [local fixture hashes](../crates/assets/tests/local_map_scripts.rs). This
coverage does **not** establish representative indoor/outdoor/dungeon/world-map
behavior. In particular, `$0004` is a menu scene, not another gameplay room.

### Runtime checks from the same qualified SRAM replay

`qualify-loader` now resolves its layer and cavern attribute sources through map
IDs. Fixed expected PCs/pointers/hashes remain independent qualification checks,
not the extraction mechanism. It first verifies map `$0004` during the normal
Start-to-save-selection sequence, then continues the unchanged A-button replay
to the cavern. Both run in one process-global oracle session.

For the menu, it stops at `$86:902B` (frame 449) to compare the resolved entry,
`$86:8B66` (frame 529) to compare packet pointer/dimensions/destination, and
`$86:8B6A` (frame 530) to compare all
**1,536 decoded bytes**. The layer SHA-256 is
`3d28667ce54abe8e5d5327adac2cddcf87e5ce49c45b137e60f67f6a82d14ac5`.
No menu attribute-initialization or gameplay claim is made.

The existing cavern qualification still compares all 5,120 raw and initialized
bytes at separate loader stages, then pins the sole later cell difference in its
integration test. Its original ten-stop report is retained, with an additive
`menu` report. See [static map qualification](static-maps.md) for those stages.
The extra stops do not simulate transitions or alter controller input.

## Tests and remaining work

```sh
cargo test -p assets --test map_scripts --test local_map_scripts
cargo test -p map-inspector --test local_scripts --test local_loader
```

Synthetic tests cover all resource framings, control continuation, nested flags,
deferred state, pointer-bank arithmetic/restoration, table/stream bounds and
budgets. The source-first-byte bounds test was demonstrated failing before its
fix. Local tests authenticate inputs and skip explicitly when absent. Raw script
bytes, prototype scans and oracle traces stay under ignored `local/`; no new
third-party implementation was used.

Remaining: game-flag-dependent loading, alternate layer modes and partial/cached
composition, general gameplay event bytecode, additional visited rooms and actual
transitions, metatile graphics, and behavior-qualified collision rules. The
[parent map issue](../meta/issues/decode-map-collision-formats.md) and
[event-bytecode issue](../meta/issues/reverse-event-bytecode.md) remain open.
