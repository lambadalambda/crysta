# The Records screen

Research for the native save screen ("Records", `旅のきろく`) that the desk
in `$0F` opens ([saves](saves.md)). Checked on both ROMs in the reference
emulator: per-frame logs of OAM, CGRAM, the PPU registers (read from the
save state) and APU port writes, from the A press on the desk to the fade
back. Every VRAM and CGRAM range below was decoded from the ROM and compared
byte for byte with the capture. A renderer that uses only these rules
(mode 1, BG1 + BG3 + OBJ, the text and digit rules below) gives the captured
Japanese frame pixel for pixel. The European frame also matches, but the
emulator's PAL framebuffer is 2 pixels to the left and 28 lines down. The
same text and digit rules match slots that hold level 23, 12:34 and level
100, 111 hours.

Frames in this doc are counted from the A press (frame A). "End of A+n" is
the register value after that frame, which shows in the picture of A+n+1.

## Code

The routine `$87:8590` has the same address and logic in both ROMs. Only
the operands differ:

| What | Japanese | European |
|---|---|---|
| Screen routine | `$87:8590..89ED` | the same |
| Slot list (`JSR` with DB = `$92`) | `$87:CAA5` → `$87:CB4B` | `$87:CA62` → `$87:CB08` |
| Current game's level and time | `$87:CAB9` | `$87:CA76` |
| Two BCD digits into BG3 | `$85:C352` | `$85:C3EA` |
| Text request (`$85:910E`) | `$87:CD51` | `$87:CD0E` |
| Label request (`$85:869B`) | `$87:CD7B` | `$87:CD38` |
| Sound effect (to `$04B7`) | `$87:CDA5` | `$87:CD62` |
| Song table (jingle, room track) | `$96:F2A0` | `$99:F9EA` |
| Cursor actor, top and bottom rollers | `$87:842D`, `$87:84EC`, `$87:853E` | the same |

On entry, `$87:8590` clears flags `$FB` and `$FC` (`COP 07`). This happens
before the screen shows, so it also happens on a cancel. It then sets
`$04A0 |= $4040`, `$0868 |= $20` and `$048A |= $0200`. It saves the scroll
mirrors `$081E..` (32 bytes) to `$7E:4510` and `$085A..` (40 bytes) to
`$7E:4530`, and zeroes `$080E-$0814` and `$081E-$0824`. It copies
`$7F:0800-0802` to `0803-0805` and zeroes them. It saves CGRAM
(`$7F:0600` → `$7F:0400`, `$8D:A8EA`) and VRAM `$5000-$5FFF` (to `$7F:B000`,
`$8D:9223`). The exit (`$87:88C9`) restores all of this. The play clock
(`$062C` divider, `$062E` seconds) is paused only in the input loop, which
does `INC $062C` each frame. It runs during the fades, the setup and the
jingle wait.

## PPU

The screen writes these registers directly (`$87:85BD`). The others keep the
room's values, and the doc lists the values that the capture shows.

| Register | Value | Meaning |
|---|---|---|
| `$2105` | `$09` (room) | mode 1, BG3 priority |
| `$2107` | `$38` | BG1 map `$3800`, 32×32 |
| `$2108` | `$3C` | BG2 map `$3C00` (not shown) |
| `$2109`, `$210C` | room | BG3 map `$6800`, chars `$7000` |
| `$210B` | `$03` | BG1 chars `$3000`, BG2 `$0000` |
| `$2101` | `$02` (room) | OBJ 8×8/16×16, tiles `$4000`, second table `$5000` |
| `$212C`/`$212D` | `$15`/`$00` | main BG1, BG3, OBJ; no sub screen |
| `$2130`/`$2131` | `$80`/`$00` | no colour math (the clip mode needs a window, and none is on) |
| `$2125` | `$00` | no OBJ/colour window |
| Scroll | 0 | BG1 and BG3 at 0,0 |
| `$2100` | `$0F` | no fade in: the screen shows at full brightness |

Front to back: BG3 priority 1 (all text), OBJ 3 (title), BG1 1 (not used),
OBJ 2 (cursor, rollers), BG1 0 (the scroll). The backdrop is colour 0,
`$18A5`. It fills rows 0–8, where BG1 is empty.

## VRAM and CGRAM

| VRAM (words) | Contents | Source (JP / EU) | Decoder |
|---|---|---|---|
| `$3000-$37FF` | BG1 chars, 128 tiles 4bpp | LZ `$AB:8ABA` / `$AD:9543` (`$1000` bytes, same data) | `compression::decode`, `graphics::decode_tiles_4bpp` |
| `$3800-$3BFF` | BG1 map, 32×32 | raw `$E7:198F` / `$E9:198F`, `$800` bytes (same data) | little-endian entries |
| `$5000-$5FFF` | OBJ tiles `$100-$1FF`: rollers, cursor | LZ `$C8:0890` / `$C8:777E` (`$2000` bytes) | as BG1 chars |
| `$4B00-$4BDF`, `$4C00-$4CDF` | OBJ tiles `$B0-$BD`, `$C0-$CD`: the title glyphs (16×16, 2 tiles a glyph per row) | label engine, dialogue font | `labels::label_glyphs` |
| `$6800-$6BFF` | BG3 map | cleared (`$8D:A889`), then text and digits | rules below |
| `$7000-$72FF` | BG3 chars `$00-$5F` | the room's: LZ `$A9:9000` / `$AB:9000` | `ShopArt::panel` |
| `$7300-` | BG3 chars `$60-`: text glyph tiles | text engine, dialogue font | rules below |

The BG1 map uses palettes 2 and 3 with priority 0. Rows 0–8 are tile 0
(blank). The European OBJ packet differs only in tiles `$180-$185` and
`$190-$195`, which this screen does not use.

| CGRAM | Contents | Source (JP / EU) |
|---|---|---|
| `$00` | backdrop `$18A5` | word 0 of `$CC:7096` (copied from `$20`) |
| `$01-$1F` | BG3 palettes 0–7 (room) | `$B2:8B78` / `$B4:90DB` (`ShopArt::panel_palette`) |
| `$20-$7F` | BG palettes 2–7 | raw `$CC:7096` / `$CE:7096`, `$C0` bytes (same data) |
| `$80-$AF` | OBJ palettes 0–2 (room, not used) | – |
| `$B0-$FF` | OBJ palettes 3–7 | raw `$CD:0953` / `$CF:0953`, `$A0` bytes (same data) |
| `$B3`, `$C3` | replaced by colour `$20` | – (no sprite here uses colour 3) |

The BG3 palettes that the screen uses: 0 is the text (1 white `$7FFF`, 2
edge `$10C6`), 2 is the LEVEL and TIME labels, 3 is the digits' top halves,
and 4 is their bottom halves. OBJ palette 4 is the title and the cursor, and
OBJ palette 6 is the rollers.

## Sprites

The actors are spawned by `COP A2`. Each one sets the animation table
`$B0:BD4F` (European `$B2:C159`, same bytes) with `COP D8` and its tile
base with `COP B0 02`: OAM tile = `$100` + the component's tile. An
animation record is (duration, facing, composition offset from the table),
the same shape as the house sequences. Each composition is one frame, so
nothing animates. Decode it with `sprites::SpriteFrame::decode` at the
offset. A piece is at actor − anchor + piece offset, and the OAM y is one
less than the screen line.

| Actor | Animation | Composition | Actor position | On screen |
|---|---|---|---|---|
| Top roller | 0 | `+$42` (`$B0:BD91`), 45 pieces, anchor (120,24) | (128,80) | x 8–247, lines 56–79 |
| Bottom roller | 0 | the same | (128,228) | lines 204–227 (clipped at 223) |
| Cursor | 1 | `+$18E` (`$B0:BEDD`), 1 piece, anchor (0,16) | (24, `$87:84D4`[slot]) | 16×16 tile `$12A`, palette 4, priority 2, at x 24, line 88 + 16·slot |

The cursor y table `$87:84D4` holds 104, 120, 136, … (the title's menu uses
the later entries). The rollers are OBJ priority 2 and palette 6: 16×16
tiles `$100-$10E` and `$120-$126` on top, and an 8×8 row of `$128`, `$129`,
`$138`… and `$139` under it.

The title comes from the label engine (`$85:869B`). The script is
`C2 64 18 00 4B 10` (x 100, y 24, VRAM `$4B00`), `C8 00` (all at once),
`C7 FF`, `C6 10` (OBJ palette 4), then the glyphs, then `D4`. Glyph i is a
16×16 sprite at x 100 + 12i, OAM y 24 (lines 25–40), priority 3, colour 3
cleared. The glyphs are at `$92:CC26` in the Japanese ROM (`81 8F 53 41 65
42`, 旅のきろく, 5 glyphs) and at `$92:E2CA` in the European ROM (`32 45 43
4F 52 44 53`, Records, 7 glyphs). The script starts at `$92:CC1A` /
`$92:E2BE`. The exit clears the labels with `$92:CC2D` / `$92:E2D2` (`C0 C7
00 D4`).

## Text

All text is on BG3. The dialogue engine (`$85:910E`) types it with the
dialogue font into the `$7F:D000` map buffer, with tiles from `$60` up. It
is 16×16 glyphs, 2bpp, colours 1 and 2 on palette 0, priority set. `C4 00`
clears colour 3. The codes are the dialogue engine's
([house dialogue](house-dialogue.md), [European text](european-text.md)).

- `C2 03 0B 1A 0E` opens the area at tile column 3 and row 11: x0 = 24,
  y0 = 88. Line n is at y 88 + 16n, and there are 7 lines. No frame is drawn.
- Glyph k of a run is at x0 + 12k. Pixel-exact rule: a glyph writes its
  columns 0–11 whole, zeros included, over what is there. Columns 12–15 are
  written only for a glyph at an even index in its run. The next glyph
  overwrites them, so only the last glyph of an even run keeps them. `CF`
  (new line), `C2` and `C3` start a new run. `CF` also clears katakana mode.
- `C3 xx yy` moves to line yy, x0 + 4·xx (xx counts map bytes, 2 per tile).
- `D2 n` calls table `$92:C447` (European `$92:C5CD`). Entry 0 is WRAM
  `$0610`, the current name. Entry `$0D` is `$061C`, the slot's name, which
  `$87:CB4B` copies from SRAM slot + `$10..$1B`. Names end with `D4`. The
  Japanese names use `D0`/`D1` kana switches.
- `CD addr` prints the word at addr in decimal, without leading zeros. The
  glyph is digit + `$73` (JP) or + `$63` (EU).

| Script | Japanese | European | Content |
|---|---|---|---|
| Page | `$92:CB4D` | `$92:E1F3` | ` 1`, ` 2`, ` 3` on lines 0–2; JP `  現在の` / EU ` Playing` on line 4; JP `  をきろくしますか？` / EU ` Save this?` on line 6 |
| Filled slot n | `$92:CBA0` + 12n | `$92:E244` + 12n | `C3 08 0n D2 0D …`: the name at x 56, line n |
| Empty slot n | `$92:CBC4` + 12n | `$92:E268` + 12n | `C3 08 0n`, JP `NO DATA` / EU `No Data` at x 56 |
| Current game | `$92:CBE8` | `$92:E28C` | `C3 08 05 D2 00`: the name at x 56, y 168 |
| Saved page | `$92:CB77` | `$92:E21E` | lines 0–2 as above; line 5 JP `  ` + `CD $04C6` + `へきろくしました` / EU ` ` + `CD $04C6` + ` saved` |
| Clear | `$92:CB9B` (`D7 D4`) | `$92:E23F` | before the saved page |
| Close | `$92:CB9D` (`C7 00 D4`) | `$92:E241` | on exit |

So the Japanese header lines start at x 48 (two spaces) and the European ones
at x 36 (one space). In the saved line, the digit is at x 48 in JP and at x
36 in EU.

`$87:CB4B` checks each slot in turn. If the primary is good, it copies it
over the backup. If the primary is bad, it copies the backup over it and
checks again. If the check still fails, the slot shows "No Data" (empty
script). If it passes, the slot shows the name (filled script) and the
level and time. After the loop, SRAM `$7FFE` is set to 0 if no slot is good,
or else masked to 2 bits. The cursor starts on WRAM `$0496`, the slot last
saved or loaded in this session, not on SRAM `$7FFE`.

### Level and time

Code writes these into the BG3 map buffer, not through the text engine.
Take R = 11 + 2·slot for a slot (from SRAM `+$56` and `+$2E`), or R = 21 for
the current game (`$0656`, `$062E`). Row R is the top and R+1 the bottom.

| Field | Columns (x) | Tiles |
|---|---|---|
| LEVEL label | 15–16 (120–135), row R+1 only | `$283D`, `$283E` (palette 2) |
| Level | 17–18 (136–151) | tens, ones |
| TIME label | 20–21 (160–175), row R+1 only | `$280E`, `$280F` |
| Hours | 22–23 (176–191) | tens, ones |
| Colon | 24 (192) | `$2C49` over `$3059` |
| Minutes | 25–26 (200–215) | tens, ones |

Digit d is `$2C21`+d (palette 3) over `$3031`+d (palette 4): the shop's
`ShopArt::TOP`/`BOTTOM` digits. Each number is the tens and ones of its BCD
form (`$86:81E0`), so level 100 shows "0". A tens digit of 0 is left blank
for the level and the hours, but not for the minutes. The time is seconds
(u32) / 3600 : (seconds mod 3600) / 60 (`$86:8178`), with seconds dropped.
With 100 hours or more it shows 99:59. "No Data" slots have no level or
time.

## Timing

Entry (both ROMs):

| End of frame | What |
|---|---|
| A, A+1 | brightness 15 (the desk callback starts) |
| A+2 … A+15 | brightness 14 … 1, one step a frame |
| A+16 | forced blank; setup runs under it |
| A+46 (JP), A+47 (EU) | `$2100 = $0F`: the whole screen shows at once |

From the next frame, the loop (`$87:8772`) reads `$0454` once a frame. That
is the new presses plus the auto-repeat: the press, then after 16 frames,
then every 5 frames. The loop takes the first match in this order:

- B (`$8000`): cancel.
- A (`$0080`): save.
- Up (`$0800`): if the slot is above 0, slot − 1 and sound `$22`.
- Down (`$0400`): if the slot is below 2, slot + 1 and sound `$22`.

There is no wrap and no sound at either end. The sound goes out on APU
port 3 on the next frame, and the cursor sprite moves on the next frame.

Cancel (B at frame S): `$92:CC2D` labels, `$92:CB9D` text. End of S+2 is
forced blank and the restore runs. At end of S+5 the blank is off at
brightness 0, and at S+6 the OAM is empty. S+7 … S+21 is brightness 1 … 15
(`$8D:A403`). The room's music never stops.

Save (A at frame S, `$87:87D4`):

1. `$0496` = slot, `$04C6` = slot + 1. If `$0630 & $FFF0` (the time is
   `$100000` s or more), the time is set to `$0005_8000` s.
2. `$8D:A6FB` writes `$0600` = `$047E` (map), `$0602` = `$0956` (facing),
   `$0604`/`$0606` = `$0952`/`$0954` (position), and SRAM `$7FFE` = slot.
   It then copies the block, adds the checksums and makes the backup copy.
   It does not change the name or the time. Flags `$FB`/`$FC` were already
   cleared on entry.
3. `$92:CB9B` clears the page, and `$92:CB77` plus the slot list type the
   saved page. The engine does not type it all at once: it takes several
   frames, one line or a few glyphs a frame (the rule is not decoded). JP:
   the old text goes at S+2; ` 1`, ` 2`, ` 3` come at S+4, S+5 and S+6;
   the message at S+11; slots 1, 2, 3 at S+12, S+16, S+19. EU: the message
   at S+10, then the slots at S+11, S+14, S+16.
4. Jingle `$35`: port 0 command `$F0` (JP S+19, EU S+16), then `$FF`, then
   the upload of song `$35` from the song table, then `$F4` (play) at JP
   S+58 / EU S+49.
5. It waits 180 frames (`LDA #$B4`), then plays the room track
   (`$04B2` + 1) again the same way: `$F0` at JP S+238 / EU S+229, `$F4` at
   S+284 / S+271. The upload times come from the SPC handshake.
6. It clears `$04B4`/`$04B6`/`$04B8`, then does the cancel's exit: forced
   blank at JP S+289 / EU S+276, then brightness 1 … 15 over 15 frames (JP
   S+294–S+308).

## Japanese and European

The logic, the positions and the timing are the same, apart from these:

- Addresses: the table above, and for the data, not bank + 2. BG1 chars go
  from `$AB:8ABA` to `$AD:9543`, and the OBJ tiles stay in bank `$C8` at
  `$777E`. The text is in bank `$92` in both ROMs, at other addresses. None
  of these are in `layout::europe` yet.
- The text: the title is 7 glyphs, not 5, and starts at the same x 100
  (not centred again). The header lines have one space, not two. The `CD`
  digits are base `$63`, not `$73`.
- The setup takes one frame more (screen at end of A+47), and the save's
  music commands come a few frames sooner.
- The OBJ packet differs in 12 tiles that are not used here.

Open: the typing rule of the text engine (only step 3 above shows it, since
the entry setup is typed under forced blank); what `C2` (`$85:9782`) would
draw as a frame and why nothing shows. The first European entry with all
slots empty was not run: the one-frame difference comes from runs with slot
1 filled, and the Japanese run shows no difference between the empty and
the filled case.
