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
