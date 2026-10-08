# Tower 5 (`$11C`-`$123`) and the cape: mechanics, native code, runtime model

Research for making tower 5 playable, with the Crysta interlude before it.
Addresses are JP, then EU. Banks `$80`, `$84`, `$8D` and `$8F` have the same
addresses in both ROMs. "Guess" marks an unverified reading. Video times are
from the longplay in `local/`. Models: [tower four](tower-four.md),
[tower three](tower-three.md). Map list: [underworld
inventory](underworld-inventory.md) (corrections below). Enemies:
[enemy scripts](enemy-scripts.md). Chests: [chests](chests.md).

## 1. Maps and what happens

| Map | Video | What the player does | What happens |
|---|---|---|---|
| `$11A` | 57:18 | opens the chest at (13,8) | item `$32` Crystal Thread, flag `$589` (chest "11") |
| `$03` -> Crysta | 62:35-63:05 | walks home after "N. America" | |
| `$14` (Elle) | 63:20-64:00 | talks to Elle (needs `$101`-`$107`, `$589`) | "...That thread? It's beautiful. ... May I have that thread? I want to weave it into this cape." The thread leaves the inventory (`COP 55 $32`), flag `$29`. Then: "You must be tired. Go get some sleep at the Elder's." |
| `$0F` | 64:20-64:35 | steps on the bed | life refilled, sleep, flags `$2A` and `$D8`: night (reload of `$0F`) |
| `$14` | 65:20-66:50 | walks to Elle at night | Elle weaves under a lamp; "Crystal cape. I pray to you. And..." (flag `$30`). Talk: "Ark... What's the matter? Can't sleep?", a choice, more pages (flag `$2B`) |
| `$0F` | 67:00-67:35 | sleeps again | flag `$2C`, `$D8` cleared: morning |
| `$14` | 68:05-68:30 | talks to Elle | "Here, take this. Your cape as promised..." `COP 60 $BF`: "Ark obtained Elle's cape!" (fanfare `$34`), 480 frames later a last page, flag `$2D` |
| menu | 68:35-68:55 | equips the cape at the armor door | `$064C = $BF` ("DEF +6 A special cape woven by Elle.") |
| `$11C` | 69:50 | walks in | gate `$90:8F7E` param 7 (needs `$107`) |
| `$11D` | 69:56-70:12 | stands | "Immobilized!!" (6 orbs circle Ark), a white circle opens, the guardian sends one orb: "Elle's Cape reflected the orb!" (flag `$19B`); then the stairs (9,4) |
| `$11E`, `$11F`, `$120` | 70:14-70:55 | stairs up each time | 2 flyers; 2 Cadets ("Cadet cast a look!"); 2 Guardners. No events |
| `$121` | 70:56 | walks east into the light | exit (14,11) -> `$122` |
| `$122` | 71:02-71:08 | walks right | Ark recoils; "Guardian: This is your final challenge. Go on." (flag `$1A6`); exit (22,2) -> `$123` |
| `$123` | 71:12-71:22 | walks up a long dark corridor | light pools at the torch statues; at y < 272 the camera pans up 80 px to the boss, music 5 |
| `$123` | 71:23-74:50 | fights Shadowkeeper | 2 claws, a tail, shots; the body hides and shows; damage 1-2 a hit; "Defeated Shadowkeeper!!" |
| `$123` | 74:55-75:10 | reads | "You have done well to overcome your challenges. Your real journey begins today. You shall return to the Elder." -> `$106` (sel `$66`, raw (120,168)) |

Corrections to the inventory: the `$11D` guardian spawns **6** ring orbs
(`$90:A2FB`, `A315`, `A32F`, `A349`, `A363`, `A37D`), which circle Ark, and
one more orb (`$90:A1C6`) for the check. Shadowkeeper has two lives: stats
`$3C` (level 7, life 58), then life 100; its two claws have stats `$46`
(level 7, life 38). `$121`'s exit (9,2) 4x1 -> `$11C` sel `$16` raw
(224,656) is an outer ledge (guess: not in the video).

## 2. Native mechanics

### The cape interlude (Crysta)

Elle's record on `$14` (Elle's house, `$83:8F2D` / `$83:8F35`, cell (37,7),
spawned unless flag `$196`): script `$88:BC7B` / `$88:C852`. `COP 48 $0027`
(gone before the weaver's `$27`). `COP 09` takes chained flag words (`$1xxx`
XOR, `$2xxx` AND, `$4xxx` OR, the last word without those bits, then a
target; `$80:8695`). With `$101` XOR `$2C` (after tower 1, before the
morning): she stands at (34,7) and weaves (pose 4 loop, sound `$4C`).

- Day, `$2A` clear: callback `$88:BD82` / `C959`. `COP 09 $2101 $2103 $2105
  $2107 $0589`: towers 1-4 done and the thread chest open -> `$BDA2`: text,
  `COP 55 $32` (`$80:9A5E`, `$8D:96A0`: remove item `$32`, guess), flag
  `$29`. Else a hint.
- Night (`$2A` set): `COP 63 00 20` (guess: the lamp light). Unless `$30`:
  pad locked, three weaving loops, text `$88:C162`, flag `$30`. Callback
  `$88:BD58` / `C92F`: `$30` and not `$2B` -> `COP C0 $BCF5`: music `$0E`,
  flag `$2B`, 180 frames, text, choice 1 (`COP 1A 01`), a page per answer.
- Morning (`$2C`): the default position; `$BD58` with `$2C` and not `$2D`:
  `COP 56 $BF $BE04` (no room: a text), `TSB $048A #$0100`, text,
  `COP 60 $BF $01A4 $34`, `COP C0 $BD36`: 480 frames, text, `TRB $048A`,
  flag `$2D`.

Flag `$24`: the first talk ever. The bed (`$0F`, controller `$88:8ABA` /
`$88:8BD3`, `COP 0C`, cells (18,6)-(19,7), guess): `LDA $0657; STA
$065D` (life to max), then `$29` and not `$2A` -> `$8B08`/`8C21`: flags `$2A`, `$D8`;
`$2B` and not `$2C` -> `$8B33`/`8C4C`: flag `$2C`, `$D8` cleared. Both: Ark
lies down (`COP DF`, `COP 89 50 0F 00 05`), music `$17`, 120 frames, fade
(`COP 6B $8BD7`, `COP 6E`), 300 frames, `COP 14` -> `$0F` mode 1 raw
(328,112). Night: the controller `$88:803D` on every Crysta map but
`$0E`/`$1F`-`$21` spawns `$88:8000` (both ROMs) when `$2A` XOR `$2C`: colour
math (`COP 76`) and a fixed colour `$7F:0800..0802 = $3F,$52,$80` (the
blue tint).

### The orb check (`$11D`)

Guardian `$90:A0BA` / `$90:A3E7` (deleted by `$19B`): x - 8, `COP 2A $EF40`
(pad locked; Start, A, L and R stay free), window and colour math
(`COP 76`: `$2123`, `$2127`, `$212C`, `$212D`, `$2130`, `$2131`), 30
frames, the 6 ring orbs (`COP A2`, flags `$1010`: `COP D9 03`, `COP D0`
orbit around `$0DEA`, Ark), sound `$37`, text "Immobilized!!" (`COP 1F`),
`COP 64 01` (an effect actor at `$87:A26B`: the circle window, radius
`$0474`, guess). 44 frames: `INC $0474`, and every other frame (`$0042 &
1`) a lighter backdrop (`$7F:0600 = n * $0421`, n up to `$1F`). 30 frames
blink (`+$04` bit 15), poses 7 and `$0A`, the orb (`COP A4 $A1C6`, dy -16,
flags `$8202`), then pose 4 until local flag 1.

Orb `$90:A1C6` / `$90:A4F3`: 10 frames, shown, `+$06 |= $4000`, Ark
`+$06 |= $20` (`LDY $0DEA; ORA; STA $0006,Y`), a trail child `$A2C7` /
`$A5EB` (copies `LDA $002C,X; TAY` x/y, after-images, gone with local 1).
Check `$90:A1F4` / `$90:A521`: `LDA $064C; CMP #$00BF` (the **equipped**
armor is Elle's cape; having it is not enough).

- Cape: `+$04 |= $10`, line move to Ark's probe (`COP CC 00 09 04`), sound
  9, text child `$A26E` / `$A59B` ("Elle's Cape reflected the orb!",
  `COP 20`), line move to (`$0408 & $FF`, 1), flags `$19B` and local 1,
  delete. The guardian: 30 frames blink, hidden, 22 frames `$0474 -= 2`,
  `STZ $0474`, `COP 29 $EF40`, delete.
- No cape: `7F:1010 = $A2B1` (callback when the orb hits Ark), line move to
  Ark, delete. Callback `$90:A2B1` / `$90:A5D5`: 8 frames Ark `y += 4`
  (`LDY $0DEA; LDA $0002,Y; ADC #4; STA $0002,Y`). Ark arrives at y 191;
  32 px down puts him on the exit (7,13) 2x2 -> `$11C` (guess: this is how
  the tower throws him out; the guardian waits on, the pad stays locked
  until the load).

### `$122` and the `$123` intro

`$122` guardian `$93:D7BF` / `$99:9A9C` (deleted by `$1A6`): at (382,108),
waits until Ark's x >= `$129` (`LDY $0DEA; LDA $0000,Y`), `$04A4 = 1`,
`COP DF $93:D84E` / `$99:9B36` (Ark: `COP B6`, `COP 84 19 03 01`, sound,
`COP 84 0B 1D 00`, 15 frames, `COP 82 01 00`, `STZ $04A4`), shown, poses 3
and 4, waits `$04A4 = 0`, Ark back to `$84:87C1`, text, flag `$1A6`.

`$123` intro `$8F:8005` (deleted by `$109`): `JSR $8289` writes VRAM
directly (`$2116`, 1024 words `$1E00` to `$2118`: a BG3 clear, guess),
`COP AA $816D` (an HDMA child as the light room's), `COP AA $8182` (each
frame: the darkness table at `$7E:5000`/`5100` from the camera y `$0822` and
the lit torches `$04A4`, `COP 4E`), `COP A2 $8151` (mosaic `$2106` flicker,
5 frames). Then each frame: `$04A4` bits from `$0822` (table `$8F:809A`).
Ark y < `$110`: Ark held at y `$110` (`COP CB 01 $84:87C1`), pad locked,
`$0DEC` = itself (camera target), from Ark's place up 1 px a frame to y
`$C0`, `$04A4 = $7F`, local flag 1, music 5, 300 frames, pad free, delete.

Controller `$90:A3AE` / `$90:A6D2` (deleted by `$109`): waits `$0498 = 0`;
`TRB $066C #$FE00` (Ark's statuses off), pad locked, music 1, `COP 00
$97:B41B` (120 frames, then up to 600 frames for `$04FA`), 60 frames, two
texts, `COP 14` -> `$106`. Flag `$109` comes after the flyover (`$87:F7E8`).

### Shadowkeeper (`$123`)

All of `$93:D876`-`$E8xx` moves to bank `$99`, `- $3D18`, in EU. The parts:

- Body `$93:D876` / `$99:9B5E`: `+$06 |= $10`, x - 8, `JSR $E74C`
  (backdrop `$7F:0600`, `COP 76`, `$04AA = 1`), `COP 63 01`, two `COP 5A`
  palettes, then `COP EA $E025` (claw holder), `COP A1 $D990` (keeps Ark's
  y in camera y `$0812` + `$60`..`$E0`, written through Y), `COP E7 $D902`
  (head: `COP E7 $E153`, `COP E7 $DCC6`). Waits local 1, 71 frames, phase
  word `$04A6` = 1, 2, 256 frames, `$0DEC` = body (camera), 180 frames.
- Main loop `$D9B4` / `$9C9C`: `+$26 & 3 = 3` (both claws dead) ->
  `$DA4D`. On a lit torch row (`JSR $E6BC`: y near table `$93:E147`, bit in
  `$04A4`) -> phase 7. |Ark y - y| >= 64 (`$095A`): 1 in 8 phase 3 (a
  charge, selector 9, 112 frames, if y < `$390`), else phase 8 (claws open,
  `COP A2` x3 shots `$E3F9`/`$E3FE`/`$E403`, 4 times). Near: phases 5, 6
  by the claw still alive and Ark's side (`$0958`).
- Claws `$E242`, `$E275` / `$A52A`, `$A55D` (`COP E7` from the holder,
  flags `$0230`): `COP D9 $46`, death callbacks `$E178`/`$E193` / `$A460`/
  `$A47B`: root `+$26 |= 2`/`1` (`LDA $7F:102E,X; TAY; ORA; STA $0026,Y`),
  holder `+$16`/`+$14 = 0`, debris children, hidden.
- Tail `$DCC6` / `$9FAE`: 11 segments (`COP E7` x11), phases by `COP 22`
  on `$04A6`; it walks the actor list (`LDA $002E,Y; TAY` 11 times) to hide,
  show, place the segments and to set their `+$0A`/`+$0C`/`+$0E` (a jump
  into another actor's script). Sting `$DEF7` (phase `$10`): line moves to
  Ark, sparks (`COP 99`), `$04A8` as the done flag.
- Stage 2 `$DA4D` / `$9D35`: `INC $0498`, death callback `$DB09`, phases
  `$0B`-`$0F`. First death `$DB09` / `$9DF1`: explosion (`COP A0`), life
  100 (`7F:102A`), callback `$DB37`. Last death `$DB37` / `$9E1F`: phase
  `$11`, pad locked, `INC $049A`, flash, `STZ $0498`, hidden, child `$DB79`
  (60 frames, `INC $04FA`), `COP 06 $85:E27B` (the death script).
- Torches: a child at a torch row toggles its bit (`EOR $04A4`) and patches
  the tiles (`COP 46`, `$E4B5`). Which part puts them out: guess, the tail.

### Shadowkeeper natively (JP trace, 2026-10-08)

Reached by the warp (`docs/native-warp.md`): flags `$101`-`$107`, `$19B`,
`$1A6` (bitmap `$06C0`), `$122`, then north; Ark arrives on `$123` at
(128,976). A bot fought with Ark's life kept full and the spear `$81`
(attack 30, power 20, defense 2): the lives are native, the hit counts hold
for those stats only. Phases 3 (charge) and 7 (torch) did not happen.

- Intro: per camera band one torch bit in `$04A4` (`$8F:809A`). At Ark y
  < 272 (frame 61130) he is held at 272, the pad locked (`$FF50`), `$0DEC`
  = the intro actor, which climbs to y 192: the camera 159 -> 80 in 124
  frames. At 61254 `$04A4 = $7F`, flag `$001`, music 5; the head (`$D916`)
  opens a circular window (`COP 63 01`): `$0474` +1 a frame while wisps
  (`$8F:80B5`, `COP 9A`, random flips) spawn every 3 frames, then -2 a
  frame with wisps `$8F:80B2`. 61548: phase 9, the camera on the body; the
  tail shows, hides at phase `$A`. Pad free at 61752.
- Entities: body `$1080` (stats `$3C`, life 58, then 100), head `$1440`,
  claw holder `$1100`, claws `$14C0`/`$1500` (stats `$46`, life 38), tail
  controller `$1480` and 11 segments, Ark clamp `$13C0`, end controller
  `$10C0`; all at (128,192), the offsets in their compositions.
- Stage 1: the body is immune (`+$04 & $30` in the hit scan's `$04E2`
  mask). Far (|dy| >= 64): phase 8, 4 volleys of 3 shots 9-18 frames apart
  after a 6-frame flash; about 2.7 px a frame, the side ones drifting out;
  20 damage. 1 in 8 phase 3. Near: phase 5 two-claw slam (`COP 6F` quake,
  rocks `$E35F`); phases 6 and `$D` one claw's swipe and a flame (`$E3B1`),
  12-16 damage. A dead claw sets root `+$26` bit 1 or 2, clears the
  holder's `+$14`/`+$16`, leaves debris (62046, 62190; 2 hits each).
- Stage 2 (62205, `INC $0498`): phase `$B` the tail above the body, `$C`
  the tail's head swings; `$04A8` clear -> phase `$E`, the body hittable,
  advancing (camera 80 -> 60), single volleys. Phase `$10` the sting:
  sparks `$DFB7`, sound `$2D`, a line onto Ark, the impact `$DFF8`. Life 58
  in 2 hits; `$DB09`: explosion (`COP A0 $DE12`), life 100 (3 hits).
- Last death (63304): phase `$11`, pad lock, `INC $049A` (hit scan off),
  the tail's segments explode every 3 frames; 63364 the body hidden,
  `$0498 = 0`; 63425 `$04FA = 1`; `COP 06 $85:E27B`.
- After: the level-up pages; `$90:A3AE`: music 1, 120 frames
  (`$97:B41B`), then its first text `$90:A3EB`, the windowless banner
  "Defeated Shadowkeeper!!" (63995); the guardian's text; `COP 14` -> `$106`.
- Without the stand-in the runtime freezes at once: the intro at
  `$8F:828F` (the VRAM fill loop), the body at `$93:E767` (`STA $04AA`).
  Missing: spawns `9A`, `A0`, `EA`; services `04`, `5A`, `63`, `6F`, `A8`,
  `AF`, `E4`, and the intro's HDMA `4E`/`8A`/`93`/`AA`; writes to `$04A6`,
  `$04A8`, `$04AA`, `$049A`, `$0DEC`, `INC`/`STZ $0498`, `EOR $04A4`; reads
  of `$0812`/`$0822`/`$081E`, `$0958`/`$095A`; the `+$2E` list walk;
  writes into other actors' `+$0A`/`+$0C`/`+$0E`, x/y, `+$04`,
  `+$14`/`+$16`; the death callback `$7F:1012`; `JSL $86:81B0`.

## 3. Scripts

| Script | JP | EU |
|---|---|---|
| Elle (`$14`, record `$83:8F2D` / `8F35`) | `$88:BC7B` | `$88:C852` |
| Elle: thread / take it (`COP 55 $32`) | `$88:BD82` / `BDA2` | `$88:C959` / `C979` |
| Elle: talk / night talk / cape (`COP 60 $BF`) | `$88:BD58` / `BCF5` / `BDD2` | `$88:C92F` / `C8CC` / `C9A9` |
| bed `$0F` (night `$2A` / dawn `$2C`) | `$88:8ABA` (`8B08` / `8B33`) | `$88:8BD3` (`8C21` / `8C4C`) |
| night controller / tint | `$88:803D` / `8000` | same |
| `$11D` guardian / orb / cape test | `$90:A0BA` / `A1C6` / `A1F4` | `$90:A3E7` / `A4F3` / `A521` |
| `$11D` push / reflect text / trail | `$90:A2B1` / `A26E` / `A2C7` | `$90:A5D5` / `A59B` / `A5EB` |
| `$11D` ring orbs (6) | `$90:A2FB`..`A37D` | `$90:A61F`..`A6A1` |
| `$122` guardian / Ark's recoil | `$93:D7BF` / `D84E` | `$99:9A9C` / `9B36` |
| `$123` intro / darkness / flicker | `$8F:8005` / `8182` / `8151` | same |
| `$123` controller | `$90:A3AE` | `$90:A6D2` |
| Shadowkeeper body / main loop | `$93:D876` / `D9B4` | `$99:9B5E` / `9C9C` |
| claw holder / claws / claw deaths | `$93:E025` / `E242`, `E275` / `E178`, `E193` | `$99:A30D` / `A52A`, `A55D` / `A460`, `A47B` |
| head / tail / sting / Ark clamp | `$93:D902` / `DCC6` / `DEF7` / `D990` | `$99:9BEA` / `9FAE` / `A1DF` / `9C78` |
| stage 2 / first death / last death | `$93:DA4D` / `DB09` / `DB37` | `$99:9D35` / `9DF1` / `9E1F` |
| flyer / Cadet / Guardner | as tower 4 | as tower 4 |

Shadowkeeper's services: `00`/`01`, `02`/`03`/`04`, `22`, `46`, `5A`, `63`,
`76`, `99`, `9A`, `A0`, `A1`, `A2`, `A8`, `AF`, `B3`, `B8`, `BA`/`BB`, `CC`/
`CD`, `D8`, `D9`, `E4`, `E7`, `EA`. Native, beyond tower 4's: `$04A4`,
`$04A6`, `$04A8`, `$04AA`, `$0474`, `$049A`, `$04FA`, `INC`/`STZ $0498`;
`$0DEC` (camera target); `$0812`, `$0958`/`$095A`; Ark's y written through
Y; the list walk `$002E,Y`; writes to other actors' `+$04`, `+$08`
(`AND #$CFFF`), `+$0A`/`+$0C`/`+$0E`, x/y, `+$14`/`+$16`, `+$26`; local
`JSR`s (`$E6xx`-`$E8xx`, a sine helper with `JSL $86:81B0`).

## 4. Runtime state today (throwaway runs)

`World::enter_with_events` with `$100`-`$107`; Crysta with `$20`-`$28`,
`$589`; Ark placed at each actor for 300 frames, life refilled,
`hit_spawned` every 30 frames.

| Map | Result (JP / EU) |
|---|---|
| `$14` | Elle stands. JP: each talk freezes at the first text: `$88:BDA8` (thread), `BCCE` (night), `BDEB` (cape). EU: `$29` is set, the night scene sets `$30` and `$2B`, then `$88:C8FC` freezes; the cape is given (`$BF` in the items), then `$88:C911` freezes, so `$2D` never comes. Cause: `HouseDialogue` refuses the pages: JP `E4 02` label calls, `C0`, `CB` (word, guess: a jump into another text); EU `C0`, `CB` |
| `$0F` | the bed freezes at `$88:8ADE` / `$88:8BF7` (`LDA $0657; STA $065D`): no sleep, no `$2A`/`$2C` |
| Crysta at night | `$88:8000` freezes at `$88:8010` (both; `$7F:0800` writes): no tint |
| `$11C` | the gate opens with `$107`; Ark walks into `$11D` |
| `$11D` | the guardian freezes at `$90:A120` / `$90:A44D` (`INC $0474`) after "Immobilized!!", with the pad locked: Ark is stuck. No body (descriptor `$82:F35B` / `$82:F2E8`: "unsupported house palette descriptor", the plain-pointer form `d8 = $00`) |
| `$11E` | flyer burst `$97:BCD4` / `$99:877F` (as tower 4); flyers fight |
| `$11F`, `$120` | Cadets and Guardners run (paralysis, sleep, vacuum). The second Guardner on `$120` has no body: a 16-byte record that reuses the descriptor ("missing descriptor reuse predecessor") |
| `$121` | nothing |
| `$122` | the guardian freezes at `$93:D7D1` / `$99:9AAE` (`LDY $0DEA; LDA $0000,Y; CMP`); no body (as `$11D`) |
| `$123` | intro `$8F:828F` (VRAM writes), body `$93:E767` / `$99:AA4F` (`STA $04AA`), controller `$90:A3B4` / `$90:A6D8` (`$0498` is 0 because the body has no body: `TRB $066C` refused). Shadowkeeper: no body (`$82:F385` / `$82:F312`, plain-pointer palette); Ark walks through it |

Not reached: the orb check, the `$122` recoil (`COP 84` on Ark), every part
of Shadowkeeper after `$E767`.

### After the runtime work

The interlude plays on both ROMs: the text commands (`$CB`, `$DF`, any
label), `COP 55`/`56`, the bed's life (`$0657`, `$065D`) and the night
colour; Elle's cape is worn at once (a stand-in for the menu's armor
door). `$11D` lets the cape through (flag `$19B`) and throws Ark back to
`$11C` without it. Open: `$122`, `$123` and Shadowkeeper, the bodies of
the plain-pointer descriptors, the second `$120` Guardner.

## 5. Proposed runtime model (blockers first)

1. **The `$11D` check.** Without it the tower cannot be entered.
   - Accept `$0474` (and `$0042`, the frame counter, read only) as scratch
     words, and `STA $7F:0600` as a backdrop write to the display, so the
     guardian runs on. `COP 64 01`: the circle window, cosmetic first.
   - Equipped armor: keep `$064C` in the world (`equip_armor`, saved in the
     slot as `$064A` is) and let a run read it (`LDA $064C; CMP #`). The
     armor door of the menu is not ported; until then a host action or a
     test hook equips. Without the cape equipped the native path pushes Ark
     onto the exit: keep the `7F:1010` hit callback and Ark's y pokes.
   - The orb: `COP A4`, `COP CC`/`CD` to Ark's probe, Ark `+$06` writes,
     the trail's `LDA $002C,X` (the same idiom as the flyer burst).
2. **The interlude.** Text first: teach `HouseDialogue` the `C0` and `CB`
   commands and JP `E4 02` (and the other label indices these pages use);
   this unblocks all three Elle scenes in both ROMs. Then the bed: life to
   max as a world action for `LDA $0657; STA $065D`, `COP 89` on Ark (lie
   down), `COP 6B`/`6E` (fade, guess), so `$2A`/`$2C` and the reload come.
   `COP 55` (remove item) and `COP 56` (room test) are checked in EU
   already. The night tint (`$7F:0800..0802`, fixed colour) is cosmetic.
3. **Shadowkeeper's body.** The plain-pointer palette form (`d8 = $00`) in
   the descriptor decoder; it also gives the `$11D` and `$122` guardians
   bodies. With it the body counts in `$0498` and the controller's end
   works.
4. **The `$123` intro.** Step over `JSR $8289` (VRAM fill, cosmetic) and
   accept `$04A4` and `$0DEC`; the camera pan is a Rust camera target.
   The darkness HDMA (`$8F:8182`) can come later (draw all lit first).
5. **Shadowkeeper's fight.** Natively it needs the phase words, the list
   walk and writes into other actors' script pointers, which no other
   script uses. Proposal: a Rust fight keyed on `$93:D876` / `$99:9B5E`, as
   the show of tower 4 could be: a body (stats `$3C`), two claws (stats
   `$46`, hit callbacks), at both claws dead the body takes hits; at 0 an
   explosion, life 100; at 0 again the end (`$0498 = 0`, `INC $04FA`, the
   death script). Attacks first as a reduced set: charge (phase 3), three
   shots (phase 8), tail sting (phase `$10`). Torches and darkness after.
   The Ark clamp `$D990` (Ark's y kept on screen) is a small native poke.
6. **`$122`.** Accept `LDY $0DEA; LDA $0000,Y` (Ark's x); the recoil
   (`COP 84` on Ark) as a fixed pose sequence, or skip it to `STZ $04A4`.
7. **Smaller.** The second `$120` Guardner (16-byte reuse); the flyer burst
   (`$97:BCD4`); the end wait `$04FA` (now scratch for the EU show).
