# Treasure chests (and the tower Magirocks)

JP addresses unless marked. Evidence: source reads, the JP emulator on the
tower 1 chest (map `$103`), and the longplay video (30:35, 44:15, 57:15).

## 0. Correction: `$84:DD7E` is not a chest

The `00` spawn records with script `$84:DD7E` / descriptor `$82:F9E0`
(`docs/underworld-inventory.md` "chests 1..11") are the **Magirock pickups**
(the blue crystal on the floor, video 24:45, 40:30). They are sprites.
Real chests are **BG cells**: no actor, no descriptor. See section 5 for the
Magirock.

## 1. What a chest is

- A map cell whose low 9 bits are `$0F0` (closed). Open: `$0F1`. A second
  kind (any cell with attribute `($A001 byte >> 1) == 4`) opens to `$0F9`.
  The graphics are the map's own BG tiles (red box; open = lid up, dark inside).
- Native: the cell word in `$7E:A000+` (tower 1: `$7E:B298`, `$18F0` ->
  `$18F1`, the high bits are the collision attribute, chest is solid).
- Contents table: JP `$96:D10F`, EU `$99:D859` (data identical). Word per map,
  indexed by `$0480` (map x 2), points in the same bank to a list ended by `$FF`.
  1110 map slots, 132 chests.

Entry (`c` = column, `r` = row, cells of 16 px; `fl` = flags byte):

| `fl` | Size | Bytes 3.. |
|---|---|---|
| bit 7 clear, bit 6 clear | 7 | `item(16) open(16)` |
| bit 7 | 9 | `cond(16) item(16) open(16)`: no reaction unless flag `cond` is set |
| bit 6 | 11 | `cond(16) itemA(16) itemB(16) open(16)`: `itemB` if `cond` set, else `itemA` |

- `item`: bit 15 set = gems, amount = `item & $7FFF` in **BCD** (`$8030` = 30).
  0 = empty. Else an item ID (as the inventory's).
- `open`: opened flag = `$500 + open` (byte `$06C0 + flag/8`, bit `flag & 7`).
  On open the code stores `open + $8500` in `$09C9` and sets it with `$80:BBCD`.
- `fl` bits 3-4 (to `$7F:102C` of the worker): nonzero = fanfare item.

Chapter 1 chests:

| Map | Cell | Item | Flag |
|---|---|---|---|
| `$103` | (12,37) | `$10` S.Bulb | `$580` |
| `$108` | (35,32) | 30 gems | `$581` |
| `$109` | (12,32) | `$10` S.Bulb | `$582` |
| `$10F` | (29,23) | `$10` S.Bulb | `$583` |
| `$112` | (5,31) | `$59` Sleepless Seal, fanfare | `$584` |
| `$117` | (5,53) | 44 gems | `$586` |
| `$119` | (5,24), (10,24) | `$11` M.Bulb, `$1A` Life Potion | `$587`, `$588` |
| `$11A` | (13,8) | `$32` Crystal Thread, fanfare | `$589` |
| `$125` | (5,6) | `$4C` Starstone, fanfare | `$5FE` |

## 2. Opening

Trigger: A, through the interaction dispatcher `$87:923F` (by facing `$0956`),
then `$87:C7F1` (EU `$87:C7AE`), the same path as the wooden doors
(`crates/crysta-runtime/src/world/door.rs`). For `$0F0` the player must face
**Up**; sampled cell = (x/16, (y-24)/16) with entity y a multiple of 16
(same rule as the doors). The `$0F9` kind works for every facing (each facing
has its own sample). The list lookup must match (c,r), else nothing.

Flow, `$87:9444` (EU `$87:939E`), native code in the check worker:

1. `$097C |= $8000` (player held), Ark `$7F:1020 = $0300`. Pick the item
   (above). If it is an item and `$8D:96ED` says no room: go to 6.
2. Worker x/y to the chest cell. `$84:D628` (EU `$84:D60E`): the held item's
   icon (table `$A8:46A0`, gems another branch) in the `$0DF4` sprite (guess).
3. `COP CB` sets Ark's script: `$84:BF3E` (resource 3, pose `$39`, then `$3A`:
   both hands up), or `$84:BF45` when empty (no icon), or `$84:BF56` (by facing,
   poses `$18/$19/$1A`) for the `$0F9` kind.
4. `COP 36 4A` (sound `$4A`, port 3), `COP 44 0 0 $0F1` (or `$F9`), wait 4,
   `COP C1 $16` (22 frames). Then Ark script `$84:BF8F`.
5. By `$09C7`:
   - gems: `$09CB = amount`, text JP `$92:810E` / EU `$92:8131`
     ("Ark obtained / 30 gems!", `D8 CB 89` prints `$09CB`), `COP 1F`, then
     `$8D:95DB` adds gems (BCD), `COP 37 47` (the gem sound).
   - 0: text JP `$92:8164` / EU `$92:818F` ("Darn! Empty!!!").
   - item, no fanfare: text JP `$92:812C` / EU `$92:8151` ("Ark obtained /
     *name*!", `CE C7 09` = name of item `$09C7`), `COP 1F`, then `$8D:9653`
     adds it.
   - item, fanfare: text JP `$92:8147` / EU `$92:816F` (ends `D4`: window
     stays), `$048A |= $0100`, `COP 30 34` (track `$34`), `COP 33`,
     `COP C1 $168` (360 frames), `COP 32 FF` (map music back), `COP 33`, text
     JP `$92:8162` / EU `$92:818D` (`D7`: close), `$048A &= ~$0100`, add.
   Then set flag `$500+open`, `$09C7 = 0`, item sprite ends (`$84:BFDF`),
   Ark script `$84:A2E9` (stand, by facing), `$097C` cleared.
6. No room: open (`$0F1`) the same way, text JP `$92:8096` / EU `$92:80B0`
   ("I'm overloaded...") or, with `$04F8` set (already 9), JP `$92:80DD` /
   EU `$92:8115` ("I have enough *X*s."), then patch back to `$0F0`, no flag.

Measured (JP, `$103`, frames): A seen 61861; 61862 `$09C7 = $10`,
`$09C9 = $8580`, `$097C = $8000`, Ark in the lift pose with the icon over his
head; window at about 61894; text "アークは 不思議な球根を 手に入れた！";
A at 61984; 61986 flag (`$0770 = 01`), `$7F:8000 = 10 01`, `$097C = 0`.
A second A on the open chest does nothing. Video: the fanfare text
(Crystal Thread, 57:15) stays about 8 s, as the 360-frame wait says.

## 3. On map load

`$8D:912B` (both ROMs, called from `$8D:8C10` in the loader) walks the map's
list; for each entry whose flag `$500+open` is set it patches the cell:
`$0F0` -> `$0F1`, `$0F1` stays, anything else -> `$0F9` (`$8D:91E4`: cell
word = tile | `$7F:0000+tile` attribute << 9). So an opened chest is drawn
open from the first frame; no sprite.

## 4. Runtime: services

Present: `COP 44` tile patch (`patches`), text (`COP 1C`/`1F`), sounds
(`COP 36`/`37`), music (`COP 30`/`32`/`33`), `Inventory::has_room`/`add`
(`$8D:96ED`/`9653`), `add_money` (convert the BCD amount), event flags, the
door's facing/sample rule. `COP 60 GRANT_ITEM` is **not** used by chests.

Missing: the chest table decoder; the `$0F0`/`$0F9` branch in the A
interaction (before the thrust, as `open_door`); the native `$87:9444` flow
as a timed model (like `door.rs`); the load patch `$8D:912B`; Ark's lift
poses via `COP CB` (resource 3 `$39`/`$3A`); the held-item icon
(`$84:D628`, `$A8:46A0`); the `$04F8` "enough" distinction of `$8D:96ED`.

## 5. Magirock (`00` records, `$84:DD7E`, EU `$84:DD38`)

- Script code at `+5` (`$84:DD83`): flag `$900 + param` set -> `COP A7`
  (deleted; a taken Magirock is simply absent on load). Else `COP D8`,
  pose `$11` (`COP 80 11`), `COP 3B` (solid), `COP 21 $DDC8` (talk target),
  loop `COP 80 11; COP 8E`.
- Talk (`$84:DDC8`, `COP 71` check): Ark script by facing (`COP 5F`,
  `COP DF $84:BF67/73/7F/83`, lift poses `$18/$19/$1A`), wait 14
  (`COP C1 0E`), `COP BA 30`, the Magirock moves to (Ark x, Ark y-18),
  `COP DF $84:BF96`, wait 4, text JP `$84:DEC9` / EU `$84:DE83` ("Ark
  obtained / Magirock!"), `COP 1F`, `$07ED += 1` (BCD; the runtime's
  Prime Blue word), flag `$900+n`, `COP CB 01 $84:87C1`, `COP A7`.
- Descriptor JP `$82:F9E0` / EU `$82:F96D` is 11 bytes: packet `$CD:133A`
  (EU `$CF:136B`), `d3 = 04`, `d4 = 00`, then three `$FFFF` words. In
  `$80:FA4E` a `$FFFF` field is **absent** and takes 2 bytes
  (`FB81`, `FBE8`, `FCD4`->`FD8C`): no second packet, no palette, no
  graphics upload. The pose uses OBJ tiles and palette already loaded by
  another actor of the map. Which one (and the relocation's `C9/CB` left
  from it) is not traced. The bytes after the 11 are the next descriptor.
- Missing in the runtime: `COP 21`, `71`, `5F`, `CB`, `3C`.
