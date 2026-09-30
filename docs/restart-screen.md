# The Restart screen

Research for the native file select ("Restart", `旅の再開`) that Start on the
title opens ([saves](saves.md)). Checked on both ROMs in the reference
emulator, in the same way as the [Records screen](records-screen.md): per-frame
logs of OAM, CGRAM, the PPU registers and APU port writes, and captures of
VRAM, CGRAM, OAM, WRAM and SRAM. Every VRAM and CGRAM range below was decoded
from the ROM and compared byte for byte with the capture. A renderer that uses
only the rules below (the Records renderer plus the BG1 map and the HDMA split
of this doc) gives each reference capture in `local/restart/` pixel for pixel
on both ROMs. The Japanese picture is at (0, +8) in the 256×240 capture, and
the European picture is at (−2, +28). Line 0 is not compared, and the
colours go through the emulator's curve (the `unlike` rule of
`crates/crysta-app/tests/local_records.rs`).

Frame S is the frame on which Start (or A) is held first. "End of S+n" is the
state after that frame, which shows in the picture of S+n+1.

## Code

The screen is map `$04`. The map loader loads its art, and one actor
(`$87:8000`, both ROMs) runs the screen. The logic is the same in both ROMs.
Only the operands differ:

| What | Japanese | European |
|---|---|---|
| Screen actor | `$87:8000` | the same |
| Spawn of the actor (guess: map `$04`'s stream) | `$83:8928` `FD … 00 80 87` | the same |
| Main loop; A handler | `$87:81BE`; `$87:820E` | the same |
| Copy; Erase | `$87:825D`; `$87:8342` | the same |
| Back to the main page (redraw) | `$87:8242` | the same |
| Page + slot list (`JSR`) | `$87:CAA5` → `$87:CB4B` | `$87:CA62` → `$87:CB08` |
| Text request | `$87:CD51` | `$87:CD0E` |
| Sound effect (to `$04B7`) | `$87:CDA5` | `$87:CD62` |
| New game clear | `$87:CCA7` | `$87:CC64` |
| Input merge (`$045A`/`$045C`) | `$87:8D3F` | `$87:8D2F` |
| Cursor actor, copy cursor | `$87:842D`, `$87:8489` | the same |
| Top, bottom roller | `$87:8FC6`, `$87:900A` | `$87:8F20`, `$87:8F64` |
| HDMA actor, tables | `$87:904E`: `$87:9076` / `$87:907D` | `$87:8FA8`: `$87:8FD0` / `$87:8FD7` |
| SRAM load, copy, verify | `$8D:A764`, `$8D:A7A5`, `$8D:A82E` | the same |

The actor's COP services, as read from the handlers (`$80:83B2` table):
`COP 00 addr24` calls (one level), `COP 01` returns, `COP 06 addr24` jumps,
`COP BC` sets the resume point here, `COP BD` yields one frame,
`COP BE addr24 n` resumes at addr after n frames, `COP BF addr24` resumes at
addr on the next frame, `COP C1 n` waits n frames, `COP 02 n … COP 03` is a
counted loop with one frame per turn, `COP 07`/`COP 08` set/test a flag.
`COP 48 flag` deletes the actor on the flag state (guess).

The setup (`$87:8000`):

1. Clear flags `$FB`, `$FC`, `$FF`. Save CGRAM colour `$20` (`$7F:0640`) and
   set it to 0. Yield.
2. 18 frames of forced blank (`COP 02 $12`, `$2100 = $80`).
3. LZ `$AB:8ABA` / `$AD:9543` → `$7E:5000` → VRAM `$0000` (`$1000` bytes).
   Raw `$B4:8000` / `$B6:8000` → VRAM `$1000` (`$4000` bytes).
4. Put the saved colour into CGRAM `$20`, `$B3`, `$C3`, `$00` and `$0B`.
5. Animation table (`COP D8`), the title label (`COP 6C $92:CBEF`), then the
   page and the slots (`$87:CAA5` with `Y = $CB21`).
6. `$04C8` (cursor) = the word at SRAM `$7FFE`, after the slot list masked it.
7. Spawn the rollers, the cursor and the HDMA actor. Call the fade in
   (`$86:E46C`). Call the main loop (`$87:81BE`).
8. On return: if `$0610` (the name) is not 0, load the game (`$87:8160`). If
   not, start the name entry (`$87:80DB`).

## PPU

| Register | Value | Meaning |
|---|---|---|
| `$2105` | `$09` | mode 1, BG3 priority |
| `$2107` | `$38` | BG1 map `$3800`, 32×32 |
| `$2108` | `$3C` | BG2 map `$3C00` (not shown) |
| `$2109`, `$210C` | `$68`, `$07` | BG3 map `$6800`, chars `$7000` |
| `$210B` | `$00` | BG1 chars `$0000` (Records: `$3000`) |
| `$2101` | `$02` | OBJ 8×8/16×16, tiles `$4000`, second table `$5000` |
| `$212C`/`$212D` | HDMA, see below | |
| `$2131` | `$00` | no colour math |
| Scroll | 0 | BG1, BG3 and the camera (`$080E`/`$0812`) at 0,0 |

HDMA (`$8D:92C3`, mode 1 into `$212C`/`$212D`, table in bank `$87`, set by the
HDMA actor, each frame (guess)):

| Lines | `$212C`/`$212D` | Shows |
|---|---|---|
| 0–77 | `$10`/`$00` | OBJ only: the title and the top roller over the backdrop |
| 78– | `$15`/`$00` | BG1, BG3, OBJ |

With flag `$FB` set (name entry), the second row is `$11`/`$04` (table
`$87:907D` / `$87:8FD7`).

Front to back (the same as Records): BG3 priority 1 (text, stats, border),
OBJ 3 (title), BG1 1 (not used), OBJ 2 (cursor, rollers), BG1 0 (the scroll).
The backdrop is colour 0, `$18A5`.

## VRAM and CGRAM

| VRAM (words) | Contents | Source (JP / EU) | Decoder |
|---|---|---|---|
| `$0000-$07FF` | BG1 chars, 128 tiles 4bpp | LZ `$AB:8ABA` / `$AD:9543` (Records' BG1 chars) | `compression::decode`, `decode_tiles_4bpp` |
| `$1000-$2FFF` | not used here (guess: name entry) | raw `$B4:8000` / `$B6:8000`, `$4000` bytes | – |
| `$3800-$3BFF` | BG1 map | map `$04` layer and metatiles, below | – |
| `$4000-$47FF` | OBJ tiles `$00-$7F`, not used here | LZ `$A9:F02F` / `$AB:F02F`, first `$1000` bytes | `compression::decode` |
| `$4B00-$4B7F`, `$4C00-$4C7F` (EU `…DF`) | OBJ `$B0-$B7` (EU `$BD`), `$C0-…`: title glyphs | label engine | `labels::label_glyphs` |
| `$5000-$5FFF` | OBJ `$100-$1FF`: rollers, cursor | LZ `$C8:0890` / `$C8:777E` (Records' OBJ tiles) | as BG1 chars |
| `$6800-$6BFF` | BG3 map | border (`$85:E716`), text, stats | rules below |
| `$7000-$72FF` | BG3 chars `$00-$5F` | LZ `$A9:9000` / `$AB:9000` (`ShopArt::panel`) | as Records |
| `$7300-` | BG3 chars `$60-`: text glyphs | text engine | as Records |

`$3000-$37FF`, `$6000-$67FF` and `$7800-$7FFF` keep the title's data. The
screen does not show them.

The map `$04` loading script (JP `$B3:8002`, EU `$B5:8002`, from
`scripts::resolve_map`) names the other resources:

| Script bytes | Resource (JP / EU) | Destination |
|---|---|---|
| `02 00 00 91 A1 09`, `08 FC 1B 00` | audio `$C6:2191` / `$C8:2191`, song `$1B` | APU |
| `80 00 10 00 …` | LZ `$C8:0890` / `$C8:777E` | VRAM `$5000` |
| `40 00 60 20 …` | `$CC:7096` / `$CE:7096`, `$60` colours | CGRAM `$20` |
| `40 00 50 B0 …` | `$CD:0953` / `$CF:0953`, `$50` colours | CGRAM `$B0` |
| `20 00 20 00 01 …` | metatiles, LZ `$CA:5513` / `$CC:5513`, `$800` bytes | `$7E:2000` |
| `10 01 …` | layer `$CD:16C7` / `$CF:16F8` (`StaticLayer`: `01 03`, 16×48 cells) | `$7E:A000` |
| subscript `$29`: `80 00 10 00 …` | LZ `$A9:F02F` / `$AB:F02F`, `$1000` bytes | VRAM `$4000` |
| `40 00 20 90 …` | `$B2:8B38` / `$B4:909B` | CGRAM `$90` |
| `80 00 08 00 …` | LZ `$A9:9000` / `$AB:9000`, `$800` bytes | VRAM `$7000` |
| `40 00 20 00 …` | `$B2:8B78` / `$B4:90DB` | CGRAM `$00` |

BG1 map: with the camera at 0,0, the entry at tile row r, column c
(r, c < 32) is word `(r mod 2)·2 + (c mod 2)` of metatile
`layer[r/2][c/2] & $1FF`, 8 bytes a metatile. This reproduces all 1024
entries on both ROMs. Palettes 2 and 3, priority 0, some H-flipped; tiles
0–`$71`. Unlike in Records, rows 0–9 are not blank, but the HDMA hides them.

| CGRAM | Contents | Source (JP / EU) |
|---|---|---|
| `$00` | backdrop `$18A5` | colour `$20` (step 4) |
| `$01-$1F` | BG3 palettes 0–7 | `$B2:8B78` / `$B4:90DB` (`ShopArt::panel_palette`) |
| `$0B` | `$18A5` (border colour) | colour `$20` (step 4) |
| `$20-$7F` | BG palettes 2–7 | `$CC:7096` / `$CE:7096` |
| `$80-$8F` | 0 | – |
| `$90-$AF` | OBJ palettes 1–2 (not used) | `$B2:8B38` / `$B4:909B`; `$91`/`$92` = `$7FFF`/`$2D7D` (source not traced) |
| `$B0-$FF` | OBJ palettes 3–7 | `$CD:0953` / `$CF:0953` |
| `$B3`, `$C3` | `$18A5` | colour `$20` (step 4) |
| `$BE`, `$CE` | `$08DF`, `$7FFF` (not used; source not traced) | – |

The colours on screen: BG3 0–2 and `$0B`, BG `$25-$37`, OBJ palette 4
(title, cursor), OBJ palette 6 (rollers).

BG3 border (`$85:E716`, the room loader, unless `$048A & $4800`): entry
`$2820` (tile `$20`, palette 2, priority 1) on columns 0 and 31 of rows 0–27
and on all of row 28. Below line 78 it shows as colour `$0B` = the backdrop
at x 0–7 and 248–255. BG1 is empty there too.

## Sprites

The same as Records. There are 45-piece rollers at actor (128,80) and
(128,228), OBJ palette 6, priority 2. They are at camera + (128,80/228)
(`$080E`/`$0812`, which is 0 here; Records uses `$081E`/`$0822`). The cursor
is one 16×16 piece, tile `$12A`, palette 4, priority 2. The frames are the
Records' (`$B0:BD4F` / `$B2:C159` +`$42`, +`$18E`, `RecordsArt::roller`,
`cursor`).

Cursor (`$87:84BD`): actor at `$87:84D4`[`$04C8`]. The screen line is the
actor y − 16, and the OAM y is one less.

| `$04C8` | Entry | Cursor x, line |
|---|---|---|
| 0 | slot 1 | 24, 88 |
| 1 | slot 2 | 24, 104 |
| 2 | slot 3 | 24, 120 |
| 3 | New Game | 24, 136 |
| 4 | Copy Data | 24, 152 |
| 5 | Erase Data | 24, 168 |

OAM order: title glyphs, cursor, bottom roller, top roller. The Copy cursor
(`$87:8489`) is the same sprite, and it follows `$04C8`.

Title (label engine, like Records): script `C2 64 18 00 4B 10` (x 100, y 24,
VRAM `$4B00`), `C8 00`, `C7 FF`, `C6 10` (OBJ palette 4), glyphs, `D4`.
Glyph i is a 16×16 sprite at x 100 + 12i, OAM y 24, priority 3.

| | Japanese | European |
|---|---|---|
| Script | `$92:CBEF` | `$92:E293` |
| Glyphs | `$92:CBFB`: `81 8F 53 82 9C 80 E6` (旅の再開, 4) | `$92:E29F`: `32 45 53 54 41 52 54` (Restart, 7) |
| Name entry title | `$92:CC03` (x 72, y 8) | `$92:E2A7` |
| Clear labels | `$92:CC2D` | `$92:E2D2` |

`label_glyphs` gives the captured tiles on both ROMs (0 unlike pixels).

## Text

The same engine, area and rules as Records (`C2 03 0B 1A 0E`: x0 24, y0 88,
line n at 88 + 16n). `HouseDialogue::decode_requests` with the page and then
the three slot requests gives the captured BG3 text pixels exactly. This is
true for blank SRAM and for the 2008 SRAM, on both ROMs.

| Script | Japanese | European | Content |
|---|---|---|---|
| Page | `$92:CB21` | `$92:E1BE` | `C4 00 C2 03 0B 1A 0E C8 00 C7 FF`, then lines 0–5: ` 1`, ` 2`, ` 3`, ` はじめから` / ` New Game`, ` きろくをうつす` / ` Copy Data`, ` きろくをけす` / ` Erase Data` |
| Filled slot n | `$92:CBA0` + 12n | `$92:E244` + 12n | the name at x 56, line n (Records') |
| Empty slot n | `$92:CBC4` + 12n | `$92:E268` + 12n | `NO DATA` / `No Data` at x 56 |
| Clear | `$92:CB9B` | `$92:E23F` | before a redraw |
| Close | `$92:CB9D` | `$92:E241` | on A (load or new game) |

All lines start with one space on both ROMs, so the text starts at x 36.

The slot list is Records' `$87:CB4B`. It is unchanged: it repairs a bad
primary from its backup, copies a good primary over its backup, draws the
name or "No Data", and draws the level and time (rows 11, 13, 15; the
Records' LEVEL/TIME/digit rule, `records::stats_tiles`). At the end it sets
SRAM `$7FFE` to 0 if no slot is good, or else masks it to 2 bits. There is no
current-game line. Verified: slots with level 1/7/7 and 0:18/1:21/1:43, both
ROMs.

The Japanese ROM shows the European 2008 names (`21 52 4B D1 D4`) as `Aねち`.
The checksums pass, and the slots load on both ROMs.

## Entry timing

From the Start press on the title (S):

| End of frame | Japanese | European |
|---|---|---|
| S+7 … S+49 | brightness 14 … 0, one step every 3 frames (title) | the same |
| S+52 | forced blank | the same |
| S+57 | APU `$F0`, port 1 `$01` (sample bank upload) | S+56 |
| S+58 | map `$04` loading (mode 1) | S+57 |
| | `$F0`/`$05` at S+109, song upload, `$F4` (play) at S+132 | `$F0`/`$05` at S+111, `$F4` at S+131 |
| S+154 … S+169 | blank off, brightness 1 … 14, all black (colour 0 and `$20` are 0; no layers) | S+150 … S+165 |
| S+170 | forced blank; `$87:8000` setup | S+166 |
| S+208 | HDMA on | S+206 |
| S+209 | blank off, brightness 0 | S+207 |
| S+213 … S+269 | brightness 1 … 15, one step every 4 frames | S+211 … S+267 |
| S+270 | the loop reads the input first | S+268 (guess: not run) |

The text is typed under the forced blank and shows all at once. The music is
song `$1B` of map `$04` (`$04B2 = $1B`). The difference between JP and EU
comes from the SPC upload handshake. A button held during the fade counts
as a press on the first loop frame (`$0454` holds it).

## Input

The main loop (`$87:81BE`) reads `$0454` once a frame (the press, then after
16 frames, then every 5; verified) and takes the first match:

| Button | Effect |
|---|---|
| A (`$0080`) | the entry, below |
| Up (`$0800`) | if `$04C8` > 0: − 1, sound `$22` |
| Down (`$0400`) | if `$04C8` < 5: + 1, sound `$22`; at 5: nothing, no sound |

There is no wrap. B, Start and the others do nothing. `$04C8` changes in the
press frame, and the sprite moves in the next frame (as in Records).

The cursor starts at SRAM `$7FFE` & 3, after the slot list. With blank SRAM
(all `$00` or all `$FF`; the same picture) that is 0. The 2008 SRAM has 2
(slot 3). `$7FFE = $0107` with good slots gives 3, so the cursor starts on
New Game. It never starts at 4 or 5. Unlike Records, the start is not
`$0496`.

A on each entry (`$87:820E`):

| `$04C8` | Effect |
|---|---|
| 0–2 | `$0496` = slot; `$8D:A764` loads it (carry clear); if that fails, `$87:CCA7` (new game). Sound `$23`, close text. |
| 3 | `$87:CCA7` (new game). Sound `$23`, close text. `$0496` does not change. |
| 4 | Copy |
| 5 | Erase |

Then the actor tests `$0610`: a loaded game has a name, so it loads. A
new game has 0, so it goes to the name entry.

## Loading a slot

A at frame A on a good slot (verified with the 2008 slot 1, JP and EU):

| End of frame | Japanese | European |
|---|---|---|
| A | `$0496` = slot, block loaded, `$85:F4BD` (stats), APU port 0 `$F7` (`$F8` if `$06B4 & $2000`; guess: sound mode), map request | the same |
| A+1 | sound `$23` on port 3 | the same |
| A+4 … A+17 | brightness 14 … 1, one step a frame; text and sprites stay | the same |
| A+18 | brightness 0, forced blank | the same |
| A+19 | `$047E` = saved map (`$0F`) | the same |
| | `$F0`/`$05` A+43, `$FF` A+55, upload A+57–79, `$F4` A+81 | A+39, A+51, A+53–72, A+74 |
| A+98 | blank off | A+89 |
| A+100 … A+114 | brightness 1 … 15, one a frame | A+91 … A+105 |

`$87:8160`: `$047C` = `$0600` (map), `$0490` = `$0602` + 1, `$0492`/`$0494`
= `$0604`/`$0606`, `STZ $049C`, `$44` = `$062C`. Only if flag `$20` is set.
If the flag is clear, `COP 14 $0129 …` (the prologue map; guess). The map's own track plays
(`$04B2` = 5 for `$0F`). The result matches [saves](saves.md) "Loading":
player at (472,176), `$0956` = 1, and WRAM `$0600-$07FF` and
`$7F:8000-$82F9` equal to the slot, except the clock (`$062C`, `$062E`),
which ticks.

Differences from `World::resume`:

- The native load sets `$0496` = slot. A later desk save puts its cursor
  there. The host must keep this.
- `$85:F4BD` recomputes the stats on load.
- A slot without flag `$20` goes to the prologue. `World::resume` returns
  `None` for it.
- The transition: 15-frame fade out, about 80 dark frames, 15-frame fade in.

## New game

A on New Game at N (verified JP, EU), or on an empty slot (`$0496` = slot):

| End of frame | New Game | Empty slot |
|---|---|---|
| N | `$87:CCA7`, sound `$23` (port 3 at N+1), close text | the same, after a failed `$8D:A764` |
| N+7 … N+63 | brightness 14 … 0, one step every 4 frames (`$86:E488`) | N+8 … N+64 |
| N+64 | forced blank: name entry setup (`$87:80DB`: flag `$FB`, colour math `$82`/`$05`, BG3 map `$DE:6DBF` / `$E0:6DBF` → `$6800`, chars `$1000`, "NAME ENTRY" label) | N+65 |
| N+77 | blank off | N+78 |
| N+81 … N+137 | brightness 1 … 15, one step every 4 frames | N+82 … N+138 |

The song continues (no APU command). The HDMA uses the flag `$FB` table. The
name entry itself is in [saves](saves.md) and a later issue.

## Copy Data

There is no prompt text. The only feedback is the cursors and the sounds.
Flag `$FC` means "source chosen". `$04C6` holds the source.

| Stage | Input | Effect |
|---|---|---|
| Enter (A on 4 at C) | – | `$04C8` = 0 (at C; sprite C+1), sound `$23`; loop from C+1 |
| Source | Up/Down | 0–2, no wrap, sound `$22` (as the main loop) |
| | A | sound `$21`, set `$FC`, `$04C6` = `$04C8`. The source is not checked. The main cursor stays and blinks (hidden P+2, shown P+3, …). A second cursor (`$87:8489`) shows from P+1 and follows `$04C8`. |
| | B | sound `$24` → main page (redraw) |
| Destination | Up/Down | the second cursor, 0–2, sound `$22` |
| | A | `$8D:A82E` on the destination: good (not empty) → sound `$1B`, stay. Empty → sound `$23`, `$8D:A7A5` (source << 8 \| destination). If the source is bad → sound `$1B`, stay. Else clear `$FC` → main page. |
| | B | sound `$24`, clear `$FC` (the second cursor goes), `$04C8` = source → Source stage |

`$8D:A7A5` verifies the source. It copies primary `$000-$4F9` through
`$7E:5000-$54F9` to the destination primary, and writes the source's sums to
`+$4FA`/`+$4FC`. It then copies the destination primary `$000-$4FD` to its
backup. `$7FFE` does not change. Verified on both ROMs (slot 1 → slot 3):
110 bytes change, and slot 3 primary and backup `$000-$4FD` = slot 1.

## Erase Data

| Stage | Input | Effect |
|---|---|---|
| Enter (A on 5) | – | `$04C8` = 0, sound `$23` |
| Select | Up/Down | 0–2, sound `$22` |
| | A | `$8D:A82E`: empty → sound `$1B`, stay (code; not run). Good → sound `$21`, set `$FC`, wait 2 frames (`COP C1 2`). The cursor blinks (hidden P+2, shown P+3, …). |
| | B | sound `$24` → main page |
| Confirm | A | sound `$23`, erase, clear `$FC` → main page |
| | B | sound `$24`, clear `$FC` → Select (the cursor stays) |

The erase writes `$FFFE` to the primary's word 0 and `$FEFF` to the backup's
word 0. Then the redraw's slot list finds the primary bad and copies the
backup over it (`$000-$4FD`). So in the end both copies start `FF FE`.
Nothing else changes. `$7FFE` stays, masked, or goes to 0 when no slot is
good. Verified on both ROMs (slot 2 of the 2008 SRAM: 4 bytes).

## Back to the main page

This is `$87:8242`, after a copy, an erase, or B in Copy/Erase. It
clears the text (`$92:CB9B`) and types the page and the slots again, several
frames long (the typing rule is still open, as in Records). `$04C8` =
`$7FFE` (masked). The main loop resumes 2 frames later (`COP BE $87:81BE, 2`).
The observed frames from the confirming press E (erase of slot 2, 2008 SRAM):

| End of frame | Japanese | European |
|---|---|---|
| E+1 | `$FC` clear, cursor steady | the same |
| E+2 | all text and stats cleared | the same |
| E+4, 5, 6 | ` 1`, ` 2`, ` 3` | the same |
| E+7, 9, 11 | New Game, Copy Data, Erase Data | the same |
| E+12 | slot 1 (name, level, time) | E+12 |
| E+16, E+18 | slot 2 (No Data), slot 3 | E+15, E+17 |
| E+20 | cursor to `$7FFE` | E+19 |

A copy takes 2 frames more (E+4 clear, cursor at E+22). B in Copy
takes the same time as an erase.

## Japanese and European

The logic, the positions and most timings are the same. The differences:

- Addresses: the tables above. The data is not all at bank + 2: the OBJ
  tiles stay in bank `$C8` (`$777E`), and the text is in bank `$92` at other
  addresses.
- The title is 7 glyphs, not 4, at the same x 100. The page texts differ.
  All lines have one leading space on both ROMs.
- The audio upload is shorter on EU. The screen shows 2 frames sooner, and
  a loaded map 9 frames sooner. The redraw's slot typing is 1–2 frames
  faster.

## Reference captures (`local/restart/`, ignored)

256×240 frames, `.png` and `.rgb` (raw RGB, as `local/records/`).
Blank SRAM: `blank.srm` (all zero). 2008 SRAM: `local/saves/Terranigma.srm`.
Copy SRAM: `copy.srm` (the 2008 SRAM with slots 2–3 zeroed, `$1FFE` = 0).

| File (`jp-`/`eu-`) | Scene |
|---|---|
| `blank` | blank SRAM, S+600 (JP) / S+400 (EU), cursor slot 1 |
| `2008` | the 2008 SRAM, cursor slot 3 |
| `copy-source` | copy SRAM, Copy entered, cursor slot 1 |
| `copy-dest` | source slot 1 chosen, second cursor on slot 3; both cursors shown |
| `copy-done` | after the copy: slots 1 and 3 filled, cursor slot 1 |
| `erase-confirm` | 2008 SRAM, slot 2 chosen, blinking cursor shown |
| `erase-done` | after the erase: slot 2 No Data, cursor slot 3 |

A picture shows the OAM of the end of the frame before it, which matters for
the blinking cursor.

## Open

- The typing rule of the text engine (the redraw timeline above), as in
  Records.
- The purpose of VRAM `$1000-$2FFF` (`$B4:8000`) and OBJ `$4000-$47FF` here
  (guess: the name entry and map `$04`'s shared sheet). The source of CGRAM
  `$91`/`$92`/`$BE`/`$CE`. None of them show on this screen.
- APU `$F7`/`$F8` on load (guess: the sound mode), and the prologue branch
  for a slot without flag `$20` (not run).
- Why A on an empty slot fades one frame later than New Game (guess: the
  failed load's checksum pass).
- `COP 48` semantics and the spawn record at `$83:8928` are read from the
  code, not traced.
