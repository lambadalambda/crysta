# Shops

Research notes for the Crysta shops (`$1D`, `$1E`), from the disassembly.
No native route visits a shop yet, so none of this is checked against a run.

## Opening

Both maps have an `FE` record whose script `$92:CC8A` scans the 9-byte
records at `$96:C6DC` for the current map (`$047E`). Each match spawns a
child (`COP A1`, `$92:CCCD`) on the record's cell, which counts the stock
into `+$24` and registers the talk callback `COP 21 $CD70`. A map without a
match runs `COP A7` (`$92:CC81`).

The callback `$92:CD70` sets `$0DE8` to the shop type and bit 8 of `$048A`.
`$92:D098` finds the first item for sale; with none it shows "sold out"
(`$92:A1ED`) and ends. Else it shows the greeting `$92:A2A0`, the help
`$92:A355`, spawns the display actor `$92:D190`, and loops in its own code
(`$92:CDD2..CEFB`, a frame a pass through `JSL $80:80DF`).

There is no sell. The keys:

- Left and Right: the next or previous item, wrapping; an owned unique item
  is skipped (`$92:CF61`).
- Up and Down: the count, 1 to 9, wrapping; not for unique items (`$92:CFC2`).
- L: the item's description (table `$92:8E3A`) and the suffix `$92:A4FB`.
- A: buy (`$92:CE1D`). B: leave (`$92:CEC8`, text `$92:8095`).

Ark holds the chosen item over his head: `JSL $84:D6DB` sets the pose
(`$C3B7` hold, `$BFDF` release), `$84:D628` loads its tiles to VRAM `$46A0`,
`$84:C278` its palette (`$B1:DA31`, `$AF:E43B`) to `$7F:07F0`; `$84:C29F`
darkens it when the item cannot be bought.

## Data

A record: map word, flag word (0 always, else event flag N set, `$80:BBA6`),
stock pointer (bank `$96`), column, row, shop type. The position is
(column × 16 + 8, row × 16 + 16). The table ends at a map word with bit 15
set; the scan goes on after a match.

| Record | Map | Flag | Stock | Cell | Type |
|---|---|---|---|---|---|
| `$C6DC` | `$1E` | – | `$96:C903` | 39,5 | 0 |
| `$C6E5` | `$1E` | `$D8` | `$96:C918` | 39,5 | 0 |
| `$C85F` | `$1D` | – | `$96:CB75` | 23,21 | 3 |

A stock entry is 4 bytes: item, price (BCD word), unique flag; `$FF` ends
the list. Type 3 (Prime Blue) also costs Prime Blue: `$92:D57D`, a BCD word
per item.

Item names: pointers at `$92:8179` (item × 2); a name is `C9 <width>
<glyphs> D4`. The shop texts index 9-entry tables by `$0DE8` through the
`CE` text control (`CE D0 0D 79 83` prints the name of item `$0DD0`).

| Text | Type 0 | Type 3 |
|---|---|---|
| Sold out | `A210` | `A236` |
| Greeting | `A2C5` | `A2FD` |
| Help | `A372` | `A3F7` |
| Not enough money | `A55F` | `A5B7` |
| Too many | `A67B` | `A6CD` |
| Confirm | `A782` | `A7CD` |
| Thanks | `A887` | `A8A1` |
| Inventory full | `A92C` | `A99A` |
| Not enough Prime Blue | – | `A8EE` |

The confirm is `COP 1A` catalog `$0A`, table `$CE37`: Buy goes to `CE3D`,
Stop and cancel to `CF11`. Refusals, in the order `$92:D120` tests them:
3 no free slot, 2 not enough Prime Blue, 0 not enough money, 1 owned with
the count reaching 10. After a refusal the help shows again. Sounds: port 3
`$22` when the item or count changes, `$47` on buying. After a purchase Ark
holds the item up for 60 frames (`COP CB $84:B4BF`, then `$84:A2E9`).

Not decoded: the name and price window the display actor draws (`COP 6C`,
`$92:A1D8`, `$92:A1E1`); which child answers when flag `$D8` spawns two on
one cell.

## Money and items

- Money: `$0694` (4 BCD digits) and `$0696` (the fifth), up to 99,999;
  added at `$8D:95DB`, taken at `$8D:95FF`.
- Prime Blue: `$07ED`, BCD (`$8D:95A8`, `$8D:95C0`). The add compares the
  BCD sum with `$03E8` as binary, so 400 or more becomes the raw `$03E7`.
- Inventory: `$7F:8000`, (item, count) pairs, 9 of each at most, placed by
  `$8D:9732`: `$01..$0F` one fixed slot each (`$8D:9790`), `$10..$79` 27
  shared slots at `$00..$35`, `$7A..$7F` fixed, `$80..$9B` 12 slots at
  `$48..$5F`, `$A0..$BB` 12 at `$68..$7F`. Helpers: `$8D:9628` count,
  `$8D:9653` add, `$8D:96A0` remove, `$8D:96ED` room.
- A new game starts with no money, no Prime Blue and no items (`$87:CCA7`).
