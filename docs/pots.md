# Pots: native lift, carry and throw

[Issue](../meta/issues/throw-pots-freely.md). This generalizes the narrow
contract in [pandora-pots.md](pandora-pots.md). Addresses are Japanese
(JP). The European ROM (EU) runs the same code at the same address in
banks $80 and $84. The EU address is given where it differs. "Guess" marks
a claim without a native measurement or a complete source read.

Evidence: source reads (below) and native per-frame traces on both ROMs from
the cellar's ready state, Ark at (104,352) facing Left, with the full pot
row intact. The pot mechanics in all 21 fixtures agree between the ROMs.
Only the dialogue text timing, the fragment lifetime under a dialogue and
the map load frame after an exit differ.

## Where pots are

A cell can be lifted when its tile (`word & $1FF`) is `$FA`, `$FB` or
`$FC` (`$87:C872`). The layer's collision words (all `$18FA`/`$18FB`, type
`$0C`) in the slice:

| Maps (shared layer) | Pot cells (col,row) |
|---|---|
| `$0A` town, 64×80 | FB (55,26) (55,28) (55,29) (54,30); FA (50..52,33) (50,34) (51,34) |
| `$0B`–`$11`, `$20`, 32×64 | FA (3,21) (5,21) (10..12,37) (24,37); FB (4,21) (25,21) |
| `$12`–`$19`, 64×32 | FB (8,5) (43,21) (6,22) (7,22); FA (9,5) (44,21) (8,22) (52,22) (53,22) |
| `$1A`–`$1F`, 48×32 | FB (4,5) (11,6) |
| `$21`, `$41`–`$44`, world | none |

No `$FC` cells are in the slice. The maps in one group share one grid, so
a lifted cell stays lifted in all of them.

Held records: `$8D:B8A5` (same EU address) looks up `$047E` in the table at
JP `$96:DDBD` / EU `$99:E507`. It copies three 5-byte records to `$098A`
(FA), `$098F` (FB) and `$0994` (FC). The layout is a replacement tile word
(0 selects `$F8`), then a sprite index byte, then 2 unknown bytes.

| Maps | FA | FB | FC |
|---|---|---|---|
| `$0A`, `$25` | `$F8`, sprite 0 | `$F8`, sprite 1 | none |
| `$0B` `$0C` `$0D` `$0F` `$10` `$12` `$13` `$15` `$16` `$18` `$19` `$1B`–`$1D` `$1F`–`$21` | `$F8`, sprite 3 | `$F8`, sprite 4 | tile `$EF`, sprite 5 |

The maps `$0E`, `$11`, `$14`, `$17`, `$1A` and `$1E` have no entry. The
loader then leaves the previous map's records in place (source, not
measured).

## Recognizing a liftable cell

The A handler is `$87:9254` (EU `$87:91AE`). Its probe base is `$0966` = x
and `$0968` = y−8. The facing is `$0956` (0 Down, 1 Up, 2 Left, 3 Right).

| Facing | Probe point | Alignment (else A does nothing) |
|---|---|---|
| Down | (x, y+8) | y ≡ 0 mod 16 |
| Up | (x, y−24) | y ≡ 0 mod 16 |
| Left | (x−16, y−8) | x ≡ 8 mod 16 |
| Right | (x+16, y−8) | x ≡ 8 mod 16 |

The cross axis is free. For example, Up lifts cell (3,21) from any x in
48..63 at y=368, and Left lifts (5,21) from y=344..359 at x=104. These
were measured with pokes: y 369/370/384, x 103 and y 360 all fail.

The check order is: an actor in front first (`$87:C783`, talking wins),
then the cell (`$87:C7F1`, EU `$87:C7AE`). The lift (`$87:9683`, EU
`$87:95DD`) needs `$0988 == 0`, so only one pot at a time. `F0` (Up only),
`F3` and attribute type 4 are other interactions, not lifts.

| Situation | Result (native) |
|---|---|
| Standing, aligned, A | lift |
| Walking into the pot, A on a frame where the axis is aligned | lift. Walking blocks against the pot at the aligned coordinate. With the direction still held, the carry walk starts at once. |
| A while not aligned (for example x=105 on the way in) | nothing. The press is lost. |
| A during a dash (`$0980=$0010`), even at x=104 | nothing. After the bump (sound `$4F`) control is `$00A0` and A lifts. |
| A during the lift presentation (up to F+23) | ignored, not buffered |

## Lift timeline (A pressed in frame F)

| Frame | Event |
|---|---|
| F+1 | The cell word becomes `$00F8` (FA and FB). `$0988` = `$098A`/`$098F`. The pot object `$12C0` moves to Ark's (x,y) with script `$84:C363`/`C380`. Ark's script is `$84:BE9D`. Sound `$11`. |
| F+23 | Pot held idle `$84:C3D3`. Ark `$84:BEA1`. |
| F+24 | Carry idle `$84:B4FC`. The first frame where A (throw) and directions are accepted. |

## Carry

| Item | Native |
|---|---|
| Walk | 1,2,1,2 px per frame in all four facings. Holding for n frames moves ⌊3(n−1)/2⌋ px. There is one frame of latency, and the move continues one frame after release. `$0980=$0020` while moving, else 0. |
| Turn | A 1-frame tap turns in place |
| Dash (double tap) | Works while carrying: 3,2,2 px per frame (script `$84:B5A7`) |
| B | Jumps while holding (`$0999` = jump height, peak −28, about 24 frames) |
| Y, X | Nothing seen (no weapon in this state) |
| Put down | None. A always throws. |

## Throw

A with a pot, standing or walking. The throw uses the facing `$0956` of
the A frame. A direction pressed in the same frame as A is ignored (facing
Left, Up+A throws Left). A walk stops at once. It adds no momentum.

| Frame | Event |
|---|---|
| F+1 | Windup. The pot script per facing is `$84:C48E` Down, `C4A3` Up, `C4BA` Left, `C4CF` Right. Ark's script is `$84:B545`/`B558`/`B56F`. |
| F+19 | Release. The pot moves to the launch point (table below), `$0999 -= $20`, and it takes one step: this is the first sample. Flags `$0426`, script `$84:C721`. Sound `$12`. |
| F+19..F+37 | 19 samples at 3 px per frame (vx or vy = ±3) |
| F+33 | Ark is idle (`$84:A258`). With a direction held, the walk resumes at F+33. |
| F+37 | The height reaches 0 (landing). Sound `$13`, ±1 frame. |
| F+38 | Break. `$0988=0`. The pot slot goes back to `$84:C01F` at the break point. The fragment object `$84:C7B8` appears. |

| Facing | Launch | 1st sample | Landing sample (no hit) |
|---|---|---|---|
| Down | (x, y+10) | y+13 | y+67 |
| Up | (x, y−8) | y−11 | y−65 |
| Left | (x−10, y) | x−13 | x−67 |
| Right | (x+10, y) | x+13 | x+67 |

Height `$0999` for samples 1..19 (every facing, standing or walking):
−33 −34 −35 −35 −35 −35 −34 −33 −32 −30 −28 −26 −23 −20 −17 −13 −9 −5 0.
The source is a per-frame delta stream (`$84:C1F0`→`C23D`, pointer
7F:0012,X, count +$2A), set by `COP $AF $7B/$7C/$7D` (`$84:C49A..`).

Break point: landing → (x, y+8). Tile hit → (x, y+h+8), for example
(136,333,h −32) → (136,309). Sprite: one 16×16 OBJ with its top-left at
(x−8, y+h−16) in world space (OAM measurement). The flight frame is
`$A2:E631` in all facings.

Dash-throw (A while carry-dashing): release at F+1 with no windup, first
sample x+22, **6 px per frame**, the same height stream. Ark's control
returns at F+18 (dash state `$0010`). The source path is a guess (`$84:C649`, offset ±16).

Jump-throw (A in the air): release at F+1, height = jump height + stream
(first sample −59). Measured once, not modeled.

## What ends the flight

1. **Landing**. When the stream ends (sample 19, h=0), `$84:C78B` applies
   y += h+8. The landing tile at (x, y+h) selects the break: type `$13`/`$19`
   → `$84:C7BD` (sound `$15`, guess: splash; the town has type `$19`), `$14`
   → `$84:C7D9` (guess: pit), else `$84:C7A9` (normal break, sound `$13`).
2. **Tile hit**, in the projectile mover `$80:D1A9` (object flags +4 bits
   `$04` and `$02`). After each step there is one probe at the pot's x, with
   no half width:

   | Moving | Probe y |
   |---|---|
   | Left or Right | y−8 |
   | Up | y+2 |
   | Down | y−16 |

   Type = `(word >> 9) & $1F`. The table `$80:D1FB[2·type + obj.$16]` uses
   pot `$16` = 0. **Blocking types: `$05`, `$08`–`$0B`, `$0D`–`$0F`, `$17`,
   `$18`, `$1A`–`$1F`.** It passes `$00`–`$04`, `$06`, `$07`, `$0C` (pots,
   table), `$10`–`$16` and `$19`. The height is ignored. On a hit the
   velocity becomes 0 and 7F:201E |= `$20` (`pcoll` in the fixtures). The
   hit sample is the last. The next frame breaks via `$84:C775`, with no
   landing-tile check. In C only `$0E` (walls) and `$05` (the door at
   (11,21)) block. The pot flies over the ground pots and the table.
3. **Actors do not stop it.** The hit scan is `$85:D281` (EU `$85:D319`).
   The attacker has flag `$0400` and none of `$00D0` (a flying pot, not a
   carried one). The target has `$0200` (set by `COP $65`, `$80:9D25`) and
   none of `$04E2`, the same `+$16`, and 7F:1020 = 0. The boxes come from
   the sprite frames (attacker +4, target +8: dx, w, dy, h). The pot, the
   residents and the door all use x±8, y±8. The overlap is inclusive
   (`$85:F835`, EU `$85:F8CD`) and the height is ignored. A struck target
   gets 7F:1020 = `$10` (16 frames immune) and its `COP $65` callback runs.
   The pot flies on. In direct C: the resident (56,384) is hittable, says
   "ouch", and a dialogue freezes the world about 10 frames after the
   landing. The door actor (184,352) is hittable and adds 1 to BCD `$0640`
   on the next frame. The residents (152,368), (184,416), (216,368) and
   Ark are not hittable, so the pot passes through them.
4. **Other pots**: the ground pots are type `$0C` cells, so the pot passes
   over them. Only one pot is airborne, because `$0988` stays set until
   the break.
5. **The room edge**: every room has a wall ring. Nothing else was seen.

The door lane matches pandora-pots.md. Up from (184,368), the door is
struck at the first sample (184,357). `$0640` changes at (184,354). The
wall stops the pot at 348. A Right throw at y=368 also strikes the door,
from x≥168.

Fragment: the object is at the break point with frames `$A2:E2BD`, `E2EA`,
`E31E` and `E352` (3 frames each), then `E386`. The object stays allocated
(it was seen 360 frames later). `E386` shows nothing on screen (guess:
empty frame).

## Other cases

| Case | Native |
|---|---|
| Stepping on an exit while carrying (C's bottom exit) | The pot script becomes `$84:C5CB` (drop, stream `$6A`). Sound `$12`, no break. The map changes and `$0988` clears after the load. The cell stays consumed. |
| Other drop entries (source only) | `$84:C7F3` picks `C5D3`/`C5CB`/`C5E5`/`C5E9` for transitions by `$097C & $0300`. `C5FB` covers 4 facings, ±16, streams `$8B`–`$8D`. `C649` covers ±16 with the throw streams (from `$84:9C22`, `9C68`, `9F5B`; guess: damage or knockback). |
| Throwing onto a resident | See "What ends the flight", item 3 |

## Fixtures (`local/pots/`, git-ignored)

`local/pots/{jp,eu}/<name>.csv` hold one row per emulated frame, starting
at the first recorded frame. `recipes.json` has each fixture's `prefix`
(probe commands, not recorded) and its `action`. The inputs are honest
carry-walks from the ready state, with no pokes. The ready states are in
`local/pots/states/`: JP is the journey checkpoint `pot-first-ready`; EU
is from boot (1800 neutral, 10 Start, 150 neutral, then rows 1–186 of
`crates/oracle/tests/fixtures/eu-pandora-tour.inputs`,
`tools/eu_to_pot.jsonl`). The tools are copied to `local/pots/tools/`
(potprobe with per-frame WRAM recording; `make.py <jp|eu>` regenerates).
They point at the scratch copies, so update the paths before you use them.

The columns hold the state after the frame ran:

| Column | Meaning |
|---|---|
| `i`, `frame` | row index, emulator frame (ROM-specific) |
| `input` | buttons held this frame (`+`-joined). A in row i takes effect at i+1. |
| `px`,`py`,`facing`,`control`,`pscript` | `$1000`, `$1002`, `$0956`, `$0980`, Ark script (`$100A` 24-bit) |
| `held` | `$0988` |
| `phase` | from the `$12C0` script: `parked`, `lift`, `held`, `windup`, `flight`, `break` |
| `potx`,`poty`,`height` | `$12C0`, `$12C2` (ground point), `$0999` (signed) |
| `pscr`,`pframe`,`pflags`,`pcoll` | pot script, sprite frame (bank:ptr), +4 flags, 7F:201E,X (`0020` = tile hit) |
| `fragx`,`fragy` | fragment object position (script `$84:C7A9..C82F`) |
| `door640` | `$0640` |
| `hit` | slots whose 7F:1020,X became `$10` this frame (the actor struck) |
| `sound` | APU port-3 values written with port 2 = 0 this frame (hex) |

| Fixture | Start | Result (both ROMs) |
|---|---|---|
| `lift-standing-left` | (104,352) L | lift (5,21), sound `$11` at i=1 |
| `lift-walking-left` | walk R 10, L, L+A at x=104 | lift at i=21, carry continues Left |
| `throw-left-standing-open` | (104,352) | lands (37,352) |
| `throw-up-standing-wall-adjacent` | (104,352) U | hit at 332 after 4 samples |
| `throw-up-standing-miss-lane` | (136,368) U | hit at 333 after 9 samples |
| `throw-up-standing-lands` | (136,400) U | lands 335 |
| `throw-up-standing-door-lane` | (184,368) U | door struck at i=19, hit at 348 |
| `throw-down-standing-over-table` | (104,352) D | over the table, lands 419 |
| `throw-down-standing-wall` | (136,400) D | hit at 464 (h −5) |
| `throw-right-standing-wall-row-above` | (104,352) R | hit at x=144 (row 21 wall via y−8) |
| `throw-right-standing-door` | (104,368) R | passes (152,368), door struck, lands 171 |
| `throw-right-standing-wall` | (168,400) R | hit at 226 |
| `throw-left-standing-resident` | (136,400) L | strikes resident at x=72, lands 69, dialogue |
| `throw-{down,right,up,left}-walking` | (136,352)/(104,368)/(136,400)/(168,400) | A in the 7th walking frame, same flight |
| `throw-right-dashing` | (104,368) | 6 px per frame, door struck, hit at 224 |
| `throw-left-jumping` | (104,352) | jump-throw, hit at 31 (h −25) |
| `exit-while-carrying` | (120,432) D | drop at exit, map `$0D` |

## Component consequences

- Lift: any cataloged FA/FB (FC) cell by the facing and alignment table,
  standing or walking, never dashing. Replace the three recorded poses.
- Throw: all four facings from any position, standing or walking. Use the
  launch and step tables, the 19-value height stream and the tile-hit
  probe with the blocking-type set. Actor strikes use the box overlap and
  do not stop the pot. Recovery is F+33 (the component's 32 counts from
  F+1).
- `crates/crysta-runtime/src/world/pots.rs` swaps two names. Port-3 `$12`
  is the release (throw) sound, not a break. `$13` is the break for both
  landing and hit, not a door hit.

## Open questions

- The dash-throw and jump-throw source paths and speeds (measured once).
- The landing-tile breaks for `$13`, `$14` and `$19` (the town has `$19`)
  have no native measurement.
- The frame of the `$13` sound jitters by one frame (`throw-down-standing-over-table`
  is one frame later than the other landings).
- The end of the fragment object's lifetime. Whether `E386` is empty.
- The stale held records in maps `$0E`, `$11`, `$14`, `$17`, `$1A` and `$1E`.
- `$04F6` switches the probe to `F3` only and forces the exit drop.
  Its meaning is unknown.
