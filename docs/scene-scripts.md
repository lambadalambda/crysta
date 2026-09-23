# Scene scripts: dialogue, choices and callbacks

How the runtime executes the script services that conversations use, read
from the Japanese ROM's handlers and checked against the retained house and
Pandora native journeys. Tracked in
[script dialogue and choices](../meta/issues/script-dialogue-choices.md).

## Execution model

Actor scripts are 65816 code with `COP` services mixed in. The scheduler
(`$80:C6F0`) enters each actor at its script pointer (`+$0A`/`+$0C`) when its
countdown (`+$0E`) runs out. A service either continues in the same frame or
yields, writing the pointer to resume at.

- **Interaction callbacks are subroutines, not threads.** The player
  controller's dispatcher (`$87:923F`, called from `$84:8DC5` on A) finds
  the faced actor, needs a registered callback (`$7F:0020+slot`, `COP 21`)
  and entity `+$06` bit `$0200` (or `$0100` with the player facing opposite),
  and calls the callback with X on the resident. The resident's own script
  is held meanwhile. `RTL` ends the callback; the resident then goes on from
  its own pointer, which `COP C0` may have redirected.
- **Blocking text and choices hold the world.** `COP 1F` and `COP 1A` loop
  inside the handler through nested frames that run only `+$04 & $0800`
  actors: the player and every resident stand still until the window is
  answered.
- **Cooperative text lets the world run.** `COP 20` advances the request once
  per turn of the owner's own script. The player is held by the input mask
  that `COP 2A $FF50` sets (A and L stay free) and `COP 29` clears.

The runtime mirrors this: `World::update` routes presses to the dialogue
first, holds everything while a script blocks, and runs the faced resident's
callback on a confirm press when nothing else owns the window.

## Services

| COP | Handler | Operands | Runtime behaviour |
|---|---|---|---|
| `1B` | `$80:8BEB` | text pointer | publishes a request; retries next frame while the window is busy |
| `1F` | `$80:8C4A` | — | blocks until the request is fully acknowledged |
| `20` | `$80:8C9A` | — | yields each frame until the request ends |
| `1A` | `$80:8B85` | catalog, table | blocks for the answer, then jumps through table[0 cancel, 1, 2] |
| `07` | `$80:8669` | flag word | sets (bit 15) or clears flag `word & $0FFF` |
| `21` | `$80:8CC8` | callback | registers the interaction callback (0 removes it) |
| `29` / `2A` | `$80:8FE6` / `8FF5` | mask | unlocks / locks pad buttons |
| `2F` | `$80:90C0` | mask, target | goes on while a mask button is held (`$0454`), else jumps |
| `0D` | `$80:87C2` | facing, 4 cell offsets, target | as `0F`, on the player inside a rectangle of cells around the actor (raw X, raw Y-8, inclusive) |
| `DF` | `$80:B827` | long script | retries while the player is in a forced action (`$097C & $0810`, the recoil), then takes the player's script; the host player stands |
| `14` | `$80:8A23` | map, mode, selector, x, y | queues a transfer; the world loads it at the frame's end at (x+8, y+16), keeping the pad mask |
| `ED` / `EE` | `$80:9F4C` / `9F93` | pose, dx, dy / speed | an eased move: `EE` adds its speed to a phase each frame and places the actor on the cosine table `$81:F462` between start and target, until the phase's bit 7 |
| `91` | `$80:A395` | — | plays the pose and ends the script's frame as an `RTL` does |
| `23` / `24` | `$80:8D1E` / `8D0B` | (pose,) target | faces a facing player, takes interaction, jumps; otherwise drops interaction |
| `C0` | `$80:AAFB` | long target | from a callback, redirects the resident's own script |
| `06` | `$80:864C` | long target | long jump |
| `BC` | `$80:AAA5` | — | stores the next command as the continuation `RTL` resumes at |
| `48` | `$80:96CB` | flag word | deletes the actor when the flag is set (bit 15) or clear |
| `54` | `$80:99EB` | item, target | gives the item (target taken when full; not modelled) |
| `3A` / `39` | `$80:929C` / `921F` | pose, vector, row / column | starts a scripted leg; the next `COP 8E` moves the actor at the class-0 stream, or skips the leg's loop at the target |
| `A7` | `$80:A876` | — | deletes the actor |
| `19` | `$80:8B35` | 14 bytes | writes the re-entry record `$0600..$060F`; stepped over |
| `05` | `$80:862E` | flag word | yields on itself until the flag is set (bit 15 clear) or clear (bit 15 set) |
| `3B` | `$80:9301` | — | marks the actor's cell occupied (`$80:BE8E`); a scripted leg lifts it (`$80:BF0E`) |
| `13` | `$80:89DA` | column, row, facing | places the actor (column ×16+8, row ×16), faces it (only left mirrors), lifts its mark |
| `49` | `$80:96E6` | map word | deletes the actor when the map matches (bit 15: when it does not) |
| `85` | `$80:A182` | count, pose | selects the pose; the next `COP 8F` plays its list that many times |
| `4B` | `$80:975B` | op, word | map-local counters at `$0640`: store, or BCD add capped at 9999; the bit-6 subtraction freezes |
| `BD` | `$80:AAB3` | — | yields one frame |
| `42` | `$80:9444` | dx, dy, tile, target | branches when the cell at the actor's cell plus (dx, dy) holds the tile |
| `44` | `$80:949B` | dx, dy, word | patches that cell: tile in bits 0-8 under its attribute (`(attr & $7F) << 9`), then waits `high >> 2` frames |
| `65` | `$80:9D25` | target, long return | the actor can be hit; a hit sends its script to the target, 16 frames' cooldown |
| `66` | `$80:9D5A` | — | returns to `COP 65`'s return address |
| `4A` | `$80:9713` | counter, word, target | branches when the `$0640` counter holds the word; counter bit 7: exceeds it, bit 6: is below it |
| `3D` / `3E` | `$80:9327` / `935D` | mode, dx, dy | marks / unmarks a further cell (mode 0: offsets from the actor) |
| `A2` | `$80:A71B` | long script, flags | spawns an actor running the script; flags bit 15 hides it |
| `31` / `32` / `33` | `$80:9107` / `913C` / `918F` | 1 / 1 / — | palette-fade helper; not drawn, `33` keeps only its three-frame tail |
| `37` / `6A` | `$80:91FC` / `9DD6` | 1 / 2 | sound, cosmetic helper; stepped over |
| `BA` / `D8` | `$80:AA6F` / `B4DF` | priority / art pointer | cosmetic here; stepped over |
| `38` / `76` / `D9` | `$80:9210` / `A127` / `B501` | 2 / 2 / 1 | music word, sound queue, hit profile; stepped over |

Inline native code that writes only the display -- PPU registers, their
shadows `$0468..$046B`, the actor's scratch `$7F:201C` and the helper flag
`$7E:46E6` -- is stepped over; anything else still freezes the script.
Code that tests the player's animation (`LDA $7F:2016,X` / `$7F:0008,X`
through `$0DEA`, as the blue door's push test `$88:AB4C` does) takes its
mismatch branch: the runtime's Ark only stands and walks. Short runs that
only use script scratch words (`$0440`, `$04BC..$04C3`: `STZ`, `STA`,
`LDA`, `INC`, `DEC`, `CMP` and branches) execute, as the tour's guide and
controller take turns through `$04BC`; the words outlive map loads. Tile patches
survive a move between maps that share the first layer (`B`..`$11`, `$20`),
because `$86:9145` does not reload it.

Leg vectors come from the common resource: `$60`/`$68`/`$69` step half a
pixel a frame, `$70`/`$78`/`$79` one, `$80`/`$88`/`$89` two; up adds one to
the vector. Legs need only the common `$6000` movement base and the nine
audited streams. `COP 0F` compares its selector with the player's facing
unless it is `$7F`. Loading a map clears the local flags 0..31 and the
counters (`$8D:8AED`); items carry over.

Scripts also hide and show their actor with inline native code on entity
`+$04` bit 15 (`LDA $0004,X; ORA #$8000` / `AND #$7FFF; STA $0004,X`, about
160 and 265 sites). The draw list (`$80:EB68`) skips such an actor and its
animation and movement stop; its script runs on. The runtime recognises
exactly these two sequences: a hidden resident is neither drawn nor
occupying its cell.

Text pages end in `$D5` (A goes on), `$D3` (A closes) or `$D4` (the request
returns at once and the page stays up for a following choice). A or L
acknowledges; B does not. A choice starts on option 1; Up/Down follow the
catalog's neighbour links, A or L confirms, B cancels with result 0.

## Verified conversations

- **Elder, room B** (`$83:8B96`): his own script shows arrival text with
  `COP 1F` after a 48-frame `COP C1`. Talking runs `$88:8EDE`: one request,
  then `COP 07 $8026`, then choice 0. Either answer returns through `COP C0`
  into his own script, which shows three follow-up pages with `COP 20`.
- **Weaver, map `$13`** (`$83:8EBE`): choice 1 after one request; option 2 or
  cancel sets local flag 1 and grants nothing; option 1 sets `$28` and
  re-registers the callback. The retry path asks the same choice.

- **Elle's wake-up, map `$0F`** (`$83:8D36`, a `$00` record, descriptor
  `$83:F881`): on a fresh game (`$FB` only) she gives items `$7A` and `$A0`,
  locks the pad, waits 120 frames, shows three requests (five pages, the
  first two ending `$D5 $D4`), sets `$20`, walks four legs to the door
  (unlocking the pad 320 frames after `$20`, as natively at 5903 → 6223) and
  deletes herself.

- **The friends at the blue door, map `$0C`** (`$83:8C1E`, `$88:9A6F`),
  after the weaver: `COP 13` moves the friend to (184,416), `$27` is set,
  the pad locks, `COP 85 28 01` holds 40 frames, then three requests (six
  pages) with a 64-pixel walk left between the second and third, and choice
  1. Option 1: two pages, `$2E`, the walk back, the pad unlocked -- as the
  native journey (16749 -> 18416).

- **The box, map `$21`** (`$83:928F`, `$88:ACFA`, controller `FE` `$88:AD89`):
  the controller asks for help on entry. The box registers a contact
  callback natively (`LDA #$AD69; STA $7F:1010,X`). Walking into it runs the
  callback a frame later -- local 1, contact disarmed (`+$04 & ~$0200`), cell
  marked -- and recoils Ark away: ten pixels, sixteen frames at rest, one
  pixel. The controller's warning sets local 2; then `COP 0D`'s gate (raw X
  120..152, Y 368..400) takes the next approach, `COP DF` waits out the
  recoil, `$22` is set and `COP 14` reloads `$21` at (136,368). The native
  route's presses reproduce each step (26805 contact, 26832 rest, the
  opening at (136,368)).
- **The tour inside the box, `$41`..`$44`**: the reloaded `$21`'s controller
  (`$88:AE64`) shows four requests and transfers to `$41`; there the
  controller `$89:D3B1` and the guide `$89:D2AD` take turns through `$04BC`
  (eased moves, eleven requests), transferring 41 -> 44 -> 42 -> 43 -> 41,
  setting `$243` on the last and `$244` after the final request. Ark then
  walks as natively (`pandora-left-rest`). The tour maps load through the
  Pandora compile (`first_background`). The graphics controller `$89:D253`
  stays frozen: its loads are the compile's.
- **The blue door, map `$0C`** (`$83:8C32`): each hit counts in `$0640`;
  the first patches the upper cell and shows one page; the second patches
  both cells to the open stairs, marks them, sets `$292`, and the friends'
  reaction runs through locals 2..9 (a spawned fade child sets 4 and 6)
  before the door deletes itself and control returns. The screen fades are
  not drawn, so the reaction is shorter than natively.

## Flag-gated geometry

D's hidden gate `$83:8CC8` (`$88:A9B4`: `COP 48 $8026`, `COP 3B`, `COP BC`,
`RTL`) stamps the house exit cell `(7,44)` while `$26` is clear and deletes
itself once it is set. It is load-time only, as natively: setting `$26`
while D is loaded does not lift the stamp. The runtime blocks the player on
the cells visible actors mark (`COP 3B`/`3D`), as natively, and on the cells
of bodies that have walked or whose script froze before it could mark. A
standing body whose script marks nothing, as C's blue door, leaves its cell
alone. The host doorway action refuses a blocked cell, and one whose exit
cell holds a closed door (collision type 5, C's blue door on its stairs).
Before the Elder the player reaches the house (`$0B`–`$11`) but for E,
below the blue door; after him the slice is as before.

Spawn lists also hold `FE` records: script-only controllers at (8,0) with
their script at bytes 2..4 (`docs/house-scene.md`); the runtime runs them.
The `FD` record with `$84:A129` is the player's own and is skipped. Stairs
(selector 14) settle at the raw anchor plus (8,16).

Every map load also applies the flag-gated patch table `$96:CD9D`
(`$8D:8FB4`): each flag from `$280` owns one primary entry (a tile at a
cell, or a block copy within the grid) plus continuations, applied when the
flag is set and the entry names the map. `$292`'s two entries reopen C's
stairs whenever the shared layer is decoded again, as natively on the
return from the box. Second-layer entries are not applied.

## Open

- In the native journey the Elder's `$D3`-ended arrival page closed while Up
  was held, without A. The acknowledge routine's input test is not traced;
  the runtime waits for A.
- Callbacks run on a faced-cell target, not the native rectangle probe.
- Choice catalogs other than 0 and 1 are refused.
