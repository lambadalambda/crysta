# The underworld's end (`$12A`, `$12B`, `$127`): native code, runtime model

Research for the end of Chapter 1: tower 5's resurrection, the two small
continents, the elder's flag `$74`, the Hole and the change to Chapter 2.
Addresses are JP, then EU. "Guess" marks an unverified reading. Video
times are from the longplay in `local/`. Models: [tower four](tower-four.md),
[light room](light-room.md). Map list: [underworld
inventory](underworld-inventory.md) (corrections below).

## 1. Maps and what happens

| Map | Video | What the player does | What happens |
|---|---|---|---|
| `$123` | 74:50-75:05 | beats the Shadowkeeper | "You have done well to overcome your challenges." "Your real journey begins today. You shall return to the Elder." -> `$106` |
| `$106`, `$07`, `$40` | 75:10-77:20 | (watches) | orb, Earth, spiral, Ayers-rock flyover, parchment "On this day, Australia ..." (76:45-77:05), souls (6 figures), flag `$109` |
| `$03` | 77:25 | lands at raw (880,688) | |
| `$12B` Polynesia | 77:45-78:45 | kills 2 knights, 4 flyers, 3 small flyers; goes to the door at the top (78:40) | door: black, parchment "On this day, Polynesia was resurrected." (78:50), flags `$120`, `$40D`, -> `$03` raw (688,480) |
| `$12A` Mu | 79:15-79:45 | walks up past 6 rolling boulders (79:18-79:30), not killed; door (79:34) | parchment "On this day, Mu was resurrected." (79:45), flags `$11F`, `$40E`, -> `$03` raw (656,704) |
| `$0B` | 80:40-81:20 | talks to the elder | "You witnessed the resurrection ..." "However, it is only the land that is revived. Life remains unborn." "... We shall go outside." "Outside, in -> direction, there is a huge hole. Come with me."; flags `$74`, `$402`; he walks out |
| `$03` -> `$127` | 81:30-81:35 | walks onto cell (41,36) | conditional exit (flag `$74`), area title "Hole" |
| `$127` | 81:40-82:25 | talks to the elder at the rim | the story of the surface, then "Have you any unfinished business? / Yes, wait! / No, let's go." |
| `$127` | 82:30-83:35 | No, then Yes | No (without `$247`): "Are you really sure? Shouldn't you bid farewell ..." (83:10); Yes: a short text, Ark leaves |
| `$13` | 83:55-84:45 | walks right into Elle's room | "Elle: Don't! Please ..." "I'm sorry...Ark." "... I have to send you off with a smile" "But I can't." "I will wait until that day." flag `$247` |
| `$127` | 85:10-86:35 | talks again, No | advice ("The Crystal Spear ... effective only under crystal blue", "Do not be stingy with items", "... never shall you quail"); the rim opens; Ark jumps into the Hole (86:35) |
| `$201` | 86:38-86:48 | (watches) | grey vortex (Mode 7) |
| `$129` | 86:50-87:05 | (watches) | "Chapter 2 / Resurrection of the World" on black |
| `$128` | 87:10-87:25 | (Chapter 2) | "Ark slowly opened his eyes to a barren waste" |

Corrections to the inventory: the `$12A`/`$12B` door sets `$11F`/`$40E`
on `$12A` (Mu) and `$120`/`$40D` on `$12B` (Polynesia), and the flags
come only after the reload (§2). The boulders of `$12A` are obstacles, not
targets. The `$127` choice is not "Yes: back to Crysta": both answers are
callbacks of the elder (§2); Ark leaves on foot.

## 2. Native mechanics

### Tower 5's end and the 5th resurrection

The top `$123` has no door. Its controller `FE` JP `$90:A3A9` / EU
`$90:A6CD` (deleted with `$109`) waits for `$0498 = 0` (no enemies), then
`TRB $066C` with `#$FE00` (clears Ark's statuses, guess), locks the pad,
music 1, `COP 00 $97:B41B` (call, guess: the "Defeated Shadowkeeper!!"
banner), 60 frames, two texts, `COP 14` -> `$106` sel `$66` raw (120,168).
After that the chain is the one of towers 1-4:
- `$106`'s orb picks index 4 from `$0482 = $123` (`$04CC`).
- `$07` plays the T5 scenes (flyover: jump table `$86:BF98`, entry 4) and
  the text JP `$92:C83B` / EU `$92:DEC9`.
- The souls map is `$40` (spawns `$83:94E2`): the same controller
  `$87:F6FA` (`FE` param 4), but 6 figures with their own descriptors
  (`$83:EBE4`, `EC3E`, `EC7E`, `ED37`).
- `COP 0A $40` -> `$87:F7E8`: flag `$109`, `$03` mode 1 sel `$10` raw
  (880,688).

So the only difference is in `$123`. `$109` then unlocks the elder's
branch and the continents' doors.

### The continents' door (`$12A`, `$12B`)

One script for both maps: JP `$90:A454` / EU `$97:BD55`, descriptor
`$82:F61B` (`$12A`) / `$82:F606` (`$12B`). Spawn lists: `FA $11E` jumps
past the player start and the enemies to the door. `FA $1AF` (a later
chapter, guess) jumps to the end of the list.

Without `$11E` (JP offsets):
1. On `$12B` it is deleted with `$120`, on `$12A` with `$11F`.
2. `COP 3B`, then each frame `LDA $0498; BEQ` (the boulders do not count:
   the door works with all 6 alive in the runtime and in the video, guess).
3. `COP 0D` (Ark in front) -> `$A498`. Without `$109`: text JP `$90:A623`
   and back.
4. With `$109`: lock (`COP 2A $FFF0`), door poses `$17`, `$18`, 30 frames,
   flag `$11E`, then `LDA $047E; STA $047C` (JP `$90:A4B4`, EU `$97:BDB5`).
   This asks for the current map again: a reload.

With `$11E` (`$A4BD`, after the reload):
1. Clear `$11E`, hide, force blank (`COP 76 00 80`), `$048A &= $7FFF`,
   `JSL $8D:A889`.
2. Decompress `$E4:413F` to `$7E:5000` (`$86:83BE`), DMA to VRAM `$4000`
   (`$86:E834`). Palette `$EB:3C92` to `7F:0700` (`$86:E894`).
3. `TM = $14`, `OBSEL = $62`, colour math, `7F:0600 = $08A6`, music 2.
4. Flag `$001`. The 4 bodiless actors JP `$90:A66F/A6A8/A6E5/A722` (EU
   `$97:BF91/BFCA/C007/C044`) wait for it (`COP 05`). Then they follow the
   camera at +(80,72), (176,72), (80,168), (176,168) and draw frames 20-23
   (`JSL $80:ED75`): the parchment as 4 large OBJs (guess).
5. 120 frames. Text JP `$90:A5CA` (Mu) / `$90:A5F5` (Polynesia), with its
   button wait. 60 frames.
6. `$12A`: `$11F`, `$40E` -> `$03` mode 0 sel `$55` raw (656,704). `$12B`:
   `$120`, `$40D` -> raw (688,480).

`$40D`/`$40E` (and `$402`) are world-map patch flags (guess: the raised
land on `$03`).

### The elder and flag `$74`

On `$0B` the elder's talk callback JP `$88:8EDE` checks `$109` first ->
`$88:8FBE` (EU `$88:9195`): text, flags `$74` and `$402`, the callback
removed, the pad locked, his script set to the walk out (`COP C0
$88:8ECE`). With `$74` his record is deleted on the next load
(`COP 48 $8074`). On `$03` cell (41,36) the conditional list `$81:F046`
gives `$127` sel `$66` raw (248,448) when `$74` is set.

### The Hole `$127` and its choice

The elder (spawn (18,11), script `$90:8005` in both ROMs, descriptor
`$83:ED7F` / `$83:ED27`):
- With `$2AF` (set at `$97:8E88`, a later chapter, guess): hidden. He waits
  for Ark falling in (`+$04 & 4`), then takes him, sets `$198` and shows a
  text. Not Chapter 1.
- Else: `COP 3B`, callback `$805F`: story (JP `$90:80A4`, EU same), then
  `COP 1A` (catalog JP `$00`, EU `$02`), jump table `$806A`: cancel and
  option 1 -> `$809D`, option 2 -> `$8070`.

| Answer | Code | Result |
|---|---|---|
| Yes / cancel | `$809D` | text JP `$8274` / EU `$82C6`, end |
| No, `$247` clear | `$8070` -> `$8096` | text JP `$83BF` / EU `$8466` ("bid farewell"), end |
| No, `$247` set | `$8070` | text JP `$8297` / EU `$82F8` (advice), then `COP 3F $88` at (16..20,13) |

`COP 3F $88`: absolute cells, attribute 8 (guess: the lip of tower 4, §2
there) in row 13. Rows 13-19 of the Hole are attribute `$0C` and block
Ark. From a lip cell `$80:CF50` starts the jump (`$84:9ECC`, pose `$13`,
move selector `$27`, sound `$10`, guess). Ark lands in the pit, in the
exit (15,15) 7x2 -> `$201` mode 0 sel 0 raw (112,112).

`$247` is Elle's farewell on `$13`: actor at (42,8) on the record list,
script from JP `$96:B9F5` / EU `$96:C496`, deleted without `$74`. It waits
for Ark in its zone (`COP 0D`) with Right held (`COP 2F $0100`). Then
text; with `$247` it stops there. Else: lock, pan (`COP DC`), 240 frames,
music `$0E`, text, flag `$247` (JP `$96:BA3D`, EU `$96:C4DE`).

### The chapter change

`$201` (spawn `00` (7,8), script JP `$90:8436` / EU `$90:84FF`): `COP DF`
takes Ark, native Mode 7 setup (BGMODE 7, decompression, `COP A1` children
`$86:F983`, `$90:85BA`): the vortex. At the end (the branch at `$90:8661` / EU `$90:872A`):
- `$1AF` set -> `$127` raw (272,160) (the way back, later chapters);
- else -> `$129` mode 1.

`$129` (`FE` JP `$90:86E2` / EU `$90:87AB`) is the chapter title card,
picked by flags: `$20` clear -> Chapter 1 (-> `$0F`); `$186` -> `$1F9`;
`$06F` -> flag `$070`, -> `$7E`. Else (Chapter 2):
- flag `$06F`;
- text JP `$90:87D7` / EU `$90:88A2` ("Chapter 2 / Resurrection of the
  World");
- 720 frames, `STZ $07EF`;
- `COP 19 $128 ...` (guess: the restart point);
- `COP 14` -> `$128` mode 0 sel `$21` raw (768,240).

`$128` is Chapter 2's first map (controller `$90:8844` / EU `$90:8935`).

## 3. Scripts

| Script | JP | EU |
|---|---|---|
| `$123` end controller (`$0498`, `TRB $066C`, -> `$106`) | `$90:A3A9` | `$90:A6CD` |
| souls controller / T5 exit (`$109`, `$03`) | `$87:F6FA` / `$87:F7E8` | `$87:F64E` / `$87:F73C` (guess: + same offset) |
| continents' door | `$90:A454` | `$97:BD55` |
| door reload (`$047E` -> `$047C`) / parchment block | `$90:A4B4` / `$90:A4CE` | `$97:BDB5` / `$97:BDCF` |
| parchment quarters (wait `$001`) | `$90:A66F..A722` | `$97:BF91..C044` |
| boulder (`COP 99` child, delete) / child | `$90:AF61` / `$90:FC6E` | `$97:C93C` / `$90:F9F0` |
| elder `$0B` / `$109` branch (`$74`, `$402`) | `$88:8E4B` / `$88:8FBE` | `$88:9022` / `$88:9195` |
| elder `$127` / talk callback | `$90:8000` / `$90:805F` | same |
| Elle's farewell (`$247`) | `$96:B9F0` | `$96:C491` |
| vortex `$201` | `$90:8436` | `$90:84FF` |
| chapter title `$129` | `$90:86E2` | `$90:87AB` |

## 4. Runtime state (throwaway runs)

`World::enter_with_events` with `$100-$109`, `$19B`, `$1A6`.

| Map | Result (JP / EU) |
|---|---|
| `$123` | nothing works: intro `$8F:828F`, Shadowkeeper without a body, freezes at `$93:E767` / `$99:AA4F`; controller freezes at `$90:A3B4` / `$90:A6D8` (the native run has `TRB $066C`) |
| `$106` (previous map `$123`, poked) | the resurrection runs: flag `$109`, `$03` at (888,704), about 1690 frames. The 5th end needs no change in `resurrection.rs` |
| `$03` | the exits to `$12A` and `$12B` work. Cell (41,36) does nothing with `$74`: the conditional list `$81:F046` is not evaluated for a walk-in exit |
| `$12B` | enemies have bodies; flyer burst freezes at `$97:BCD4` / `$99:877F` (as in tower 4). Enemies dead: the door sets `$11E`, then freezes at `$90:A4B4` / `$97:BDB5` (`$047E` read not allowed; the world only takes a pending map `$0007`) |
| `$12A` | boulders run, no freeze; door as on `$12B` |
| `$12A`/`$12B` with `$11E` | door freezes at `$90:A4CE` / `$97:BDCF` (PPU, DMA, decompression) |
| door without `$109` | text, nothing set (works) |
| `$0B` | works: 5 pages, `$74`, `$402`, the elder walks out |
| `$13` | the farewell starts, then freezes at the text (`$96:BA13` / `$96:C4B4`): text command `$DF` (JP `$96:BA50`, EU `$96:C4F1`) is refused |
| `$127` | JP: the callback freezes at `$90:805F`: label call `$13` in the story (`$90:817D`; only `06`, `08`, `25` are allowed). EU: story (13 pages), choice and the three answers run. The `COP 3F $88` cells become solid stamps (attribute 8 is not in the walker's set), so Ark cannot go into the Hole |
| `$201`, `$129`, `$128` | refused: "outside the Crysta slice" (`crysta_runtime::admitted`). `$128` already has a static background (`cavern_loads`) |

## 5. Proposed runtime model

1. **The continents' door.** Let native runs read `$047E` (the current
   map). A pending map equal to the current map reloads it. Then a Rust
   scene keyed on the door's reload with `$11E` (as `resurrection.rs`):
   clear `$11E`, black screen (later the parchment), the ROM text with its
   button wait, the flags and the transfer per map (§2). This makes both
   continents playable.
2. **The way to the Hole.** Evaluate conditional exit lists for walk-in
   exits (`$81:F046`: flag word, destination, mode, selector, x, y). The
   falls already do it for their lists.
3. **Text.** JP label call `$13` (`$92:C5E7` table, as the other three).
   Text command `$DF` (both ROMs; guess: a speaker colour). Without them
   the JP Hole talk and Elle's farewell (so `$247`) freeze.
4. **The jump into the Hole.** Attribute 8 from `COP 3F` as walkable.
   Stepping down onto it plays a fixed jump (pose `$13`, guess: 2-3 cells
   down, measure in the video at 86:35). Then the exit test, which finds
   `$201` at (15,15) 7x2. A first cut: Down on a lip cell -> transfer.
5. **The chapter change.** Admit `$201` and `$129` as scenes, not rooms.
   `$201`: a dark placeholder for the vortex (about 600 frames), then the
   `$1AF` branch. `$129`: flag `$06F`, the title text from ROM, 720
   frames, then the end of the slice (a "chapter end" state for the host).
   `$128` is Chapter 2.
6. **Tower 5's end.** In `$123`'s controller: `TRB $066C` (or ignore the
   status bits) and `COP 00 $97:B41B`. It depends on the Shadowkeeper
   ([underworld playable](../meta/issues/underworld-playable.md)).
7. **Smaller.** The world patches `$402`/`$40D`/`$40E` on `$03`; the
   `$12B` flyer burst (tower 4 §5.5); the native vortex and parchment
   art; the souls map `$40` (already skipped).

## Open

- `$0498` and the boulders: do they count natively (the video says no).
- The jump's distance and the meaning of attribute `$0C`.
- `COP 19` operands in `$129`; `COP 00 $97:B41B` in `$123`.
- The `$DF` text command.
- The EU texts of the door and the elder (pointers only from the scripts).
