# Saves

Research for [menus, inventory and saves](../meta/issues/menus-inventory-save.md),
verified by saving natively on both ROMs in the reference emulator, dumping
SRAM, decoding it and loading it back through the title screen.

## Where the player saves

The slice's only save point is the object on Ark's desk in the bedroom `$0F`:
spawn record `$83:8D4F` (European `$83:8D57`), `FD` at (472,160), a hidden
parent (`$88:D60D`, European `$88:E40F`) with a visible child at (0,-16). Its
talk callback `$88:D63B` runs the save screen (`COP 00 $87:8590`): a 15-frame
fade, "Records" (`旅のきろく`), slots 1–3 ("No Data"), each with name, level
and play time. A saves at once ("1 saved", jingle `$35`, the room's track
again, a 15-frame fade in); B cancels. The save clears flags `$FB` and `$FC`.

The title's Start opens "Restart" (`旅の再開`): slots 1–3, New Game, Copy
Data, Erase Data, the cursor on the last slot saved (SRAM `$1FFE`). A on a
valid slot loads it; on an empty one it starts a new game.

## SRAM

8 KiB at `$30:6000–7FFF` (header `$FFD8` = 3).

| Offset | Contents |
|---|---|
| `$0100 + n·$500` | slot n (0–2), primary |
| slot + `$000–1FF` | WRAM `$7E:0600–07FF` |
| slot + `$200–4F9` | WRAM `$7F:8000–82F9` |
| slot + `$4FA`, `$4FC` | checksum: sum word, xor word |
| `$1100 + n·$500` | backup copy (`$000–4FD`) |
| `$1FFE` | last slot saved (`$0496`) |

In the slot: `$00` map, `$02` facing, `$04`/`$06` position (entity − (8,16)),
`$08–0F` a second resume record (`COP 19`), `$10–1B` the name (`D4`-ended,
five characters: Japanese kana shifts, European letters), `$2C` frame
divider, `$2E` play seconds (u32, 60 frames a second on both ROMs, so the
European clock runs slow), `$56` level, `$94`/`$96` money (BCD), `$A4` text
speed, `$C0…` the event flags, `$1ED` Prime Blue, `$200–2FF` the item slots.
Unknown bytes pass through.

Checksum (`$8D:A867`): over the slot's `$27D` little-endian words, sum and
xor both from `$5236`, the sum wrapping at 16 bits. Save `$8D:A6FB`, load
`$8D:A764` (carry set on failure), copy `$8D:A7A5`, verify `$8D:A82E`. File
select (`$87:CB50`) restores a bad primary from a good backup; both bad shows
"No Data". Erase (`$87:83F7`) breaks both checksums: `$FFFE` over the
primary's map word, `$FEFF` over the backup's.

## Loading

The block copies back; if flag `$20` is set (`$87:8184`) the game loads the
saved map with the player at the saved position + (8,16), facing + 1 (from
the desk: (472,176) facing up); if clear, the prologue. The map's own track
plays.

## New game

Rebuilt from the ROM alone by the steps below, the block equals the native
one on both ROMs byte for byte (captured at `$87:8164`, the name confirmed,
before any script runs; European Start at the name entry keeps "Ark").

1. Choosing New Game (or A on an empty slot, `$87:820E`) calls `$87:CCA7`
   (European `$87:CC64`): zero `$7E:0600–07FF` and `$7F:8000–82FF`, then
   `JSL $86:B93A` → `$86:B900`: word pairs (address, value) at `$86:B93F`
   (both ROMs), ended by a negative address, stored with the caller's bank; a second list
   at `$86:B9C9` (bank `$7E`) is empty. In the slot: `$34–3F` the button
   map (`$86:8000` reads it), `$56` level 1, `$57`/`$5D`/`$9C` 28 (max HP, HP, base; guess),
   `$9E`/`$A0` 3, `$A2` 2, `$A4` 1, `$A6` 7, `$B4` `$4000`, `$1EF` 1.
   Outside it: `$0832–0850`, `$0402`/`$0406` (a `PHB MVN PLB RTS` stub, guess), `$003C`,
   `$0056–005C`, `$049E`. `$86:B8D6` (power-on) runs the same list after
   clearing all WRAM.
2. The intro's `COP 07 $80FB` (`$87:80EC`, both) sets flag `$FB` (`$DF` = 8).
3. The name entry (`$87:89EF`) writes typed codes straight to `$0610+`
   (`$04BE` index, 5 characters). Japanese: `D0` opens a katakana run, `D1`
   closes it; Start appends `D1` if open, then `D4`. Start with nothing
   typed writes the default by immediates: `$87:8C99` `D0 3B 73 42 D1 D4`,
   European `$87:8C8E` `21 52 4B D1 D4` ("Ark"; the `D1` is a leftover).
4. `$87:CCCE` (European `$87:CC8B`), called at `$87:8156`: word pairs at
   `$87:CD09` (European `$87:CCC6`), the same 17 on both: `$0600` `$0F`,
   `$0604`/`$0606` (296,96), `$0608` 3, `$060C`/`$060E` `$0210`, the
   clock `$062C–0631` 0, and `$047E`, `$0952–0956`, `$048C–048F` outside.
   The bank-`$7E` list after it (`$87:CD4F`, European `$87:CD0C`) is empty.
5. `$85:F4BD` (European `$85:F555`) recomputes stats from the bases, no
   equipment: `$57` = `$9C` + `$98` (1–999), `$5D` ≤ it, `$61` = `$66` =
   `$9E` + `$9A` (`$98`/`$9A` bonuses, 0 in a new game; guess),
   `$62` = `$A0`, `$5F` = `$A2`.

Nonzero result: `$00` `0F`, `$04–0F` `28 01 60 00 03 00 00 00 10 02 10 02`,
the name, `$34` 80, `$37` 80, `$38` 40, `$3B` 40, `$3C` 20, `$3E` 10, `$56`
01, `$57` 1C, `$5D` 1C, `$5F` 02, `$61` 03, `$62` 03, `$66` 03, `$9C` 1C,
`$9E` 03, `$A0` 03, `$A2` 02, `$A4` 01, `$A6` 07, `$B5` 40, `$DF` 08, `$1EF`
01. `$7F:8000–82F9` all zero.

From there to the desk (both ROMs, same bytes but the clock):

| Slot | Change | Writer |
|---|---|---|
| `$2E5` | `$7F:80E5` bit 1: map `$0129` seen (bit per map `$100+`, guess) | `$8D:A022` |
| `$236` | item `$7A` ×1 (fixed slot) | `$8D:9653` in the prologue |
| `$268` | item `$A0` ×1 (first armour slot) | `$8D:9653` |
| `$02–0F` | `05`, (384,144), `$0F`, `00`, `$0005`, `$0180` | `COP 19` `$80:8B35`, bedroom |
| `$C4` | flag `$20` (intro done) | `$88:9791` (European `$88:9BBB`) |
| `$DF` | flag `$FB` cleared | `$87:8590`, the save screen opening |
| `$2C–31` | the clock | `$86:8164` |

## Play clock

`$86:8164` (both): `DEC $062C`; below 0 it reloads `$3B` and increments the
u32 `$062E`. 60 ticks a second on both ROMs; the European clock gains one
second per 1.2 s. It runs from the frame gate `$86:8000` (`$86:809D`), so it
ticks once per game frame that waits there: walking, text boxes and fades
tick (bedroom: 890 of 890 frames, European 2200 of 2200). Frames spent in
loaders that do not call the gate do not count (the opening's loads lost
111 of 4701 frames, the room's reload after the save screen 28).

Paused by an `INC $062C` after the gate: the Records screen's input wait
(`$87:877D`) and the waits at `$80:824A–828A` (Start pause, guess) and
`$80:82A7`/`$80:831C` (`$7F:0CB4` countdown, guess). A pause that starts on
a wrap adds one second, then holds `$3C`.

After a new game `$87:CCCE` zeroes `$062C–0631`; the gate's tick in the same
frame gives `$3B` and 1 s. The clock also runs on the title and the name
entry, but that time is cleared. On a save, 1,048,576 s or more becomes
`$58000` (`$87:87DE`, guess: a display cap).
