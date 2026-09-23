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

## Open

- In the native journey the Elder's `$D3`-ended arrival page closed while Up
  was held, without A. The acknowledge routine's input test is not traced;
  the runtime waits for A.
- Callbacks run on a faced-cell target, not the native rectangle probe.
- Choice catalogs other than 0 and 1 are refused.
