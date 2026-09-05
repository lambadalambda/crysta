# Qualified Crysta doorway

The owned Japanese ROM and pinned three-slot SRAM now have an **input-only,
repeatable bedroom `$000F` → room `$0010` doorway replay**. This is neither a
forced warp nor a new-game qualification. It connects a decoded exit record to
native trigger code, a controller actor and the destination loader. It does
**not** establish a complete event VM or general collision implementation.

## Reproduce

```sh
cargo run -p map-inspector -- qualify-opening \
  'local/Tenchi Souzou (Japan).sfc' local/saves/Terranigma.srm
cargo run -p map-inspector -- trace-opening \
  'local/Tenchi Souzou (Japan).sfc' local/saves/Terranigma.srm
cargo test -p map-inspector --test local_opening
```

The commands authenticate the Japanese reference and SRAM SHA-256
`709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
Each runs in a fresh process; no RAM patches, forcewarps or snapshot restores.
Normal qualification checks seven full-WRAM reference hashes and emits player,
camera, flags, layer and framebuffer hashes. The integration test compares two
complete normal-run results, then checks a separate instruction-stop run.
`trace-opening` is diagnostic original-CPU execution, not portable simulation.
Raw traces, screenshots and extracted assets remain under ignored `local/`.

### Save-slot correction

The supplied save **defaults to slot 3**, Chapter 2 World Resurrection. The older
Start/A-only cavern replay loads that slot, not slot 1. Two Up taps select the
actual slot 1, Level 1 / Chapter 1 / Crystal Blue. Earlier slot-1 labels attached
to the cavern did not change its measured map `$0128` or hashes.

The name-entry input experiments do not establish a current ares/game-script
stall or an emulator defect. They establish visible name entry, not a qualified
new-game confirmation path. Historical LakeSnes upload investigations are
separate. This genuine saved-game route avoids that unresolved input path.

### Inputs and checkpoints

Half-open ranges; all other buttons released. Input label F is applied **before**
`run_frame`; checkpoint N is after N calls, thus label N−1.

| Button | Labels |
| --- | --- |
| Start | `[400,408)` |
| Up | `[900,908)` and `[950,958)` |
| A | `[1100,1112)` |
| Left | `[1601,1657)` |
| Down | `[1657,1682)` |

| Completed frame | Map | Player (pixels) | Meaning |
| ---: | --- | --- | --- |
| 1601 | `$000F` | 472,176 | Saved bedroom, neutral |
| 1657 | `$000F` | 393,176 | Approach centered doorway |
| 1681 | `$000F` | 392,209 | Trigger has matched |
| 1682 | `$000F` | 392,210 | Down released for subsequent frames |
| 1697 | `$000F` | 392,225 | Autonomous departure |
| 1698 | `$0010` | 392,226 | Map switch; loader underway |
| 1703 | `$0010` | 392,336 | Destination player position populated |
| 1751 / 1801 | `$0010` | 392,353 | Arrival settled |

The seven hash-pinned frames are 1601, 1681, 1697, 1698, 1703, 1751 and 1801;
1657 and 1682 above are supplementary local observations.

The intermediate clearing to `(0,0)` is not a playable spawn. Both runtime maps
are 32×64 cells (512×1024 pixels). Camera changes from `(256,0)` to `(256,256)`.
The canonical 64-byte event block `$7E:06C0` is unchanged at the checked stages,
SHA-256 `bf7d61f4953777ab96d024578f0a147a6700c9a75085c45454f9aec69e801fcb`.
This does not mean every transient runtime flag is unchanged.

## Exit data and geometry

`assets::maps::exits::ExitList` reads little-endian pointers at CPU `$81:8000`
(normalized `$018000`), into the same ROM bank. Pointer zero means no source.
The supported map-ID prefix is `$0000..=$044F`, inherited conservatively from
the loading table, **not** a claim about the full exit-table count. The earliest
observed data pointer `$88AC` leaves an unqualified table tail. Readers reject
invalid pointers, truncation, bank crossing and more than 256 records.

Each 12-byte record contains tile X/Y/width/height (four bytes), destination
(u16), mode (u8), selector (u8), and destination pixel X/Y (two u16s). `$FF` at
the next record boundary ends the list. Destination bit 15 selects an
unimplemented conditional table; it is not silently masked into a map ID.
Records, lists and their exact source extents/bytes are retained.

| Source map | List extent, normalized | Records | First record |
| --- | --- | ---: | --- |
| `$000F` | `$018E3C..$018E55` | 2 | `(24,12,1,2)` → `$0010`, mode 0, selector 5, raw `(384,336)` |
| `$0010` | `$018E55..$018E7A` | 3 | `(24,20,1,1)` → `$000F`, mode 0, selector 6, raw `(384,176)` |

Selection at `$8D:8797..8838` first scans coarse rectangles using byte-wrapping
subtraction of `origin >> 4`. It chooses the **first** coarse match, then tests
word-wrapping pixel deltas against `dimension*16−15`, exclusive. Fine failure
returns no exit; it does not resume scanning. Player bounding origin here is
`position + (-8,-16)` with 16×16 extents. At `(392,209)`, origin `(384,193)`
selects record `$81:8E3C`. The pure selector models geometry only; runtime gates
(such as `$097C & $10`) and controller effects are outside it.

Synthetic tests cover framing, budgets, raw/conditional fields and wrapping
geometry. Optional owned-ROM tests pin source hashes and these two lists.

## Evidenced execution path

The actor scripts are **native 65C816 code interleaved with COP services**.
COP handlers advance saved return addresses over inline operands. Treating the
whole stream as ordinary instructions, or as a conventional standalone bytecode
VM, is incorrect. Low-WRAM self-modified MVN execution also cannot be decoded
against ROM merely by its banked PC.

The separate trace command verifies eight stopped-instruction checkpoints. A
stopped frame count can precede the next completed-frame snapshot.

1. `$8D:883E`, frame 1680: X=`$8E3C`, DB=`$81`, origin `(384,193)`.
2. `$8D:884C`, frame 1680: Y=`$0010`, immediately before storing pending map
   `$047C`; current map remains `$000F`.
3. `$8D:888D`, frame 1680: pending map, mode, selector and raw coordinates are
   populated. `(selector−1)*3 = 12` selects controller `$84:B94D` through
   `$8D:895B`.
4. `$8D:88DF`, frame 1681: before arrival-position adjustment. Effective arrival
   selector is the high nibble if nonzero, otherwise the low nibble; selector 5
   indexes signed `(0,−16)` at `$8D:8985`, producing queued `(384,320)`.
5. `$8D:8720`, frame 1697: before pending→current map store; previous map is
   `$000F`, Y=`$0010`, player `(392,226)`, resume `$84:B975`.
6. `$86:902B`, frame 1698: current map `$0010`, pending cleared, destination
   loader entry agrees with the ROM map table. Entry-only lookup deliberately
   does not claim all conditional resource branches have been resolved.
7. `$80:F3F1`, frame 1702: actor loader, player entity cleared.
8. `$84:A12E`, frame 1711: player initialization script enters with `(392,336)`;
   the entity's spawn data existed before this script first ran.

The departure controller schedules player path `$84:B96E` via COP `$CB`.
That path uses COP `$B6`, `$84` (animation/movement/graphics operands) and `$8E`
(animation wait), looping with resume `$84:B975`. Departure continues after
input release. These service identifications are bounded research, not a
production CPU or actor-script interpreter. Arrival scheduling, free movement,
collision responses and playable semantics are tracked separately in the
[portable room slice](../meta/issues/portable-room-slice.md).
