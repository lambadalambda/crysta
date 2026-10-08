# Tower 3 darts and Ark's statuses (the burn)

Source reads of the JP ROM, with EU checks where marked. Bank `$85` combat
code is EU +`$98` ([combat](combat.md) §1); the status table, the profiles
and the `$110` actor list were compared byte for byte. "Guess" marks a
claim without a native measurement or a complete source read. Notation as
in [combat](combat.md): `E+$xx` entity field, profile = `$7F:1022,X`.

## 1. The `$110` dart launchers

### 1.1 They are enemy records, not `FD` records

The `$110` actor list (`$82:8000[$110*2]` = `$82:8EF5`, from `$82:8EF7`)
holds, after the pedestals (`FD ... $90:FBB3`):

```
01 16 28 00 29 9a 90 d3 f4 82   block 1   $90:9A29, descriptor $82:F4D3
01 17 28 00 36 9a 90 00 00 00   block 2   (descriptor 00 00 00 = reuse)
01 18 28 00 43 9a 90 00 00 00   block 3
01 19 28 00 50 9a 90 00 00 00   block 4
01 04 12 00 2d fc 90 00 00 00   left launchers  $90:FC2D at x col 4,
01 04 17/18/1D/1E/1F ...         rows $12 $17 $18 $1D $1E $1F
01 2b 1d 00 12 fc 90 00 00 00   right launchers $90:FC12 at x col $2B,
01 2b 1a/17/14 ...               rows $1D $1A $17 $14
```

Type-`$01` records go through `$80:F51A`. A zero descriptor runs
`$80:FAF9`: the actor takes the previous actor's pose packet (`$7E:7000+$BF`)
and `$7F:1022 = $4A` (the previous descriptor's profile). So all ten
launchers share block 1's descriptor `$82:F4D3`:
`f2 fa b0 | 04 | 05 | ...`: pose packet `$B0:FAF2`, **profile 5**
(`$8D:BDFA[5]` = `$8D:C5CD`, EU `$8D:C496`, same bytes). `$80:F94C`
then sets life = profile+9 = 0. Byte `+$26` = 0, so they do not count in
`$0498`.

(For a real `FD` record, `$80:F5F9` sets no profile and the default list
table `$9A:D000`, which has only 4 lists; selector `$14` would be garbage.)

Header `14 | 20 42 | 00 00`: `7F:0008 = $14`, `E+$04 = $4220` (`$0200`
can be hit / attacker, `$4000`, `$0020`), `E+$06 = 0`. Script (right,
`$90:FC17`): `COP B1 08 00` (x += 8), `COP BC`, `COP D6 60 $FC23`
(|dx|, |dy| to Ark's probe <= `$60`), else `RTL` (retry next frame);
`COP 37 01` (sound 1), `COP 80 14`, `COP 8E` (wait for the list end),
`BRA` to the `COP BC`. The left one (`$90:FC2D`) has `COP B8` (mirror)
first, so its `B1 08` moves x by -8.

### 1.2 Attack box: list `$14` of packet `$B0:FAF2`

Decoded packet (1005 bytes), list `$14` (frames, attack box (dx,w,dy,h);
the body box is the same; sprite box (-8,16,-8,16)):

| Frames | Attack box |
|---|---|
| 2 | (-8, 8, -8, 8) |
| 2 | (-16, 16, -8, 8) |
| 2 | (-24, 24, -8, 8) |
| 2 | (-32, 32, -8, 8) |
| 2 | (-40, 40, -8, 8) |
| 64 | (-48, 48, -8, 8) |
| 6 each | (-40,40), (-32,32), (-24,24), (-16,16) (dy -8, h 8) |
| 96 | (-8, 8, -8, 8) |

One cycle is 194 frames. The dart is a growing box from the launcher's
x toward the room: right launchers (unmirrored) cover x-48..x, left ones
(mirrored: x1 = x-dx, x0 = x1-w) cover x..x+48; y-8..y in both. Positions:
x = col*16+8 (`$80:F5D7`) then +-8 from `B1`: right x = `$2B`*16+16 = 704,
left x = 4*16 = 64; y = row*16. The 8x8 box also exists in the idle record,
so touching a launcher hurts too. A new cycle starts at once if Ark is still
within `$60`.

The scan is `$85:D30C` (EU `D3A4`): attacker `+$04 & $0200`, none of
`$01D0` (the launcher passes), same layer `E+$16`, Ark `7F:1020 = 0`.

### 1.3 Damage

`$85:D648` (EU `D6E0`): box overlap (`$85:F78E`, `F835`); Ark's profile
`$064E` is non-zero, so: sound 7, then with X = the attacker:

- `$85:DE8F` (EU `DF27`): Y = profile + 5*kind (`7F:102C`, kind 0 here:
  guess, the slot is cleared at map load). eL = profile+8,
  atk = word at Y+`$0F` & `$3FF`, raw = ((eL+7)*atk) >> 3,
  `$16` = (Y+`$10`) & `$78` (element * 8).
- `$85:DF0B` (EU `DFA3`): def = Ark+`$0D` (`$065B` armor) +
  ((L+7) * Ark+`$11` (`$065F`, halved if `$066C & $0200`)) >> 2, then / 3.
  dmg = raw - def; if <= 0: (eL >> 2) + 1.
- `$85:D93D` (EU `D9D5`): for an enemy attacker with kind 0 it jumps to
  `DA87`: only Ark's profile `+$04|+$06` bits `$F800` without `$03FF` make
  damage 0; a damage of 0 becomes 1. The type checks (Y+`$11` against Ark's
  `$064E/$0650/$0652/$0654`) run only for kinds 1-4.
- `$85:DA9E` (EU `DB36`): status roll (§2.2). Element `$68` (13) is the
  enemy critical (§2.2).
- `$85:DE3A`: variance, cap 9999, digits `COP D5 00 36`, life.

Profile 5 (`00 00 00 84 00 00 00 14 04 00 00 00 00 00 00 08 00 00 00 00 08 68 00 00 00`):
level 4, life 0, kind 0 atk 8 element 0 luck 0; kind 1 atk 8 element 13.

Dart: raw = (11*8) >> 3 = **11**; damage = 11 - def. 8 means def = 3, e.g.
level 3, defense 4, no armor: (10*4)>>2 = 10, /3 = 3 (guess: Ark's stats at
41:22 are not known; the variance can also move 7 or 9 to 8). The runtime's
`combat::enemy_damage(profile, 0, stats, counter)` already gives this once
the launcher gets profile 5 from the shared descriptor.

### 1.4 Status

Kind 0 has element 0: no status and no crit. But the roll in `$85:DA9E`
still runs (chance max(4, 0+8-luck)/32, i.e. 1/8 for Ark luck >= 4); on
success it clears `$066C & $0060` (burn and sleep) and `$0684/$0686`. So a
dart hit can end a burn, it cannot start one.

## 2. Statuses and the burn

### 2.1 Which attacks carry a status

The element of kind k is `(profile+$10+5k) & $78`; the status table is
`$85:DB5F` (EU `$85:DBF7`, same bytes), 8 bytes per element e:
block mask, AND mask, OR bit (`$066C`), duration (to `$0670 + 2e`).

| e | blocked if `$066C &` | AND | sets | duration | runner (JP / EU) |
|---|---|---|---|---|---|
| 0 | - | - | - | 0 | none |
| 1 | `$F9E0` | - | `$8000` | 1200 | `$84:D079` |
| 2 | `$E1E0` | - | `$4000` | 0 | |
| 3 | `$E1E0` | - | `$2000` | 0 | |
| 4 | `$F9E0` | - | `$1000` | 0 | |
| 5 | `$F9E0` | - | `$0800` | 0 | |
| 6 | `$E9E0` | - | `$0400` attack /2 | 600 | |
| 7 | `$E9E0` | `$EFFF` | `$0200` defense /2 | 600 | |
| 8 | `$0100` | `$EF1F` | `$0100` | 180 | `COP DF $84:DB0F` / `$84:DAF0` |
| 9 | `$01E0` | `$EFFF` | `$0080` "IMMOBILIZED" | 180 | `$84:D94F` / `$84:D935` |
| 10 | `$0140` | `$EF5F` | `$0040` "ASLEEP" | 180 | `$84:DA1D` / `$84:DA03` |
| 11 | `$0020` | `$EE3F` | **`$0020` burn** | **120** | `$84:DBF1` / `$84:DBBE` |
| 12, 13 | - | - | - | 0 | 13 = critical, below |
| 14 | `$F9E0` | - | `$0010` | 900 (`$068C`) | |
| 15 | `$FFE0` | - | `$0008` | 600 (`$068E`) | |

(For e 14 and 15 the slot is `$068C`/`$068E`, but the runner counts
`$0010`/`$0008` in `$0688`/`$068A`: as read, unexplained.)

Profiles with a status (all on kind 1; kind 0 is element 0 in every
profile 1-73): burn (11) in profiles 2, 10, 54, 65, 68, 69; sleep (10) in 4,
28; critical (13) in 1, 3, 5, 7, 8, 9, 11, 12, 13, 14, 15, 17, 18, 22, 26,
31, 33, 38, 39, 42, 44, 45, 48, 55, 62, 64, 73.

Towers 1-5 ([underworld inventory](underworld-inventory.md)):

| Enemy | Profile | Kind 1 |
|---|---|---|
| yellow flyer (T1-T5) | 2 | **burn**, luck 99 |
| Guardner (T3-T5) | 4 | sleep, luck 5 |
| Hiball | 1 / `$16` | critical |
| Cadet / High Cadet | 3 | critical |
| knight | `$0E` | critical |
| blocks, launchers | 5 | critical |
| Shadowkeeper | `$3C` | none |

Kind 1 is what projectiles carry: the flyer's bullets are `COP A4`
children with kind 1 and profile 2 ([enemy scripts](enemy-scripts.md)).
So "Ark's toasted!" on T4 (52:55) comes from a yellow flyer's bullet (from
source; which flyer in the video: guess). The Guardner's kind-1 attack
(guess: its bolt) puts Ark to sleep.

### 2.2 The chance (`$85:DA9E`, EU `DB36`; the roll at `DAC3` / `DB5B`)

Only when the target is Ark. Y = attacker profile + 5*kind, X = `$064E`.

1. Armor: if `$064C` != 0 and `$8D:BD92[(id-$A0)*2] & bit(e)` -> no status.
2. c = (Y+`$13`) + 8 - `$0661` (Ark luck), 8-bit; if c is negative
   (bit 7) or < 4, c = 4. Status if (`$86:8236` RNG & `$1F`) < c.
   P = min(c, 32)/32. Flyer bullet: 99+8-luck is >= 32 for luck <= 75:
   always.
3. On success: `$066C &= $FF9F`, `$0684 = $0686 = 0` (any element, also 0).
   Then if `$066C & block` -> stop; else `$066C = ($066C & AND) | bit`,
   `$0670+2e = duration`. Because step 3 clears `$0020` first, a new burn
   hit restarts the burn.

Enemy critical (`DB23`): if damage != 0 and the element is 13: same c,
but (RNG & `$7F`) < c -> damage x2, digits `COP D5 00 3A`.

### 2.3 What the burn does

The runner `$84:D04A` (both ROMs; called from `$85:EFBB`) stops if Ark is
dead, `$049A` != 0, `$097C & $8000` or `$0010`, or `$0488` != 0. Then each
set bit of `$066C`, from `$8000` down, has a slot X = 2, 4, ... `$0020`
is slot `$0686` (`$84:D329`..`D366`):

- `$097C & $C4E5` -> `TRB $066C #$0020` (ends at once).
- slot > 0 (new): slot = -slot (-120), `$84:D3AC` (if `$04F6`: `JSL $84:D6DB`
  with `$C5CB`; guess: drops a carried object), `COP DF $84:DBF1` on Ark.
- each frame slot += 1; at 0: `TRB $066C #$0020`. The status lasts
  **120 frames**.

`COP DF` (`$80:B827`) puts Ark's script to the target (input locked: the
pad controller is gone), clears `+$04 & $80`, sets helper `$0DEE` to
`$84:B7E3`. If `$097C & $0810` (recoil) it does not switch but re-runs
itself next frame (guess for this native caller: the slot is already
negative, so the runner does not call it again).

Burn script `$84:DBF1` (EU `$84:DBBE`):

```
Ark +$04 |= $0010          (Ark's attacks do not hit)
7F:1020 = $3C              (60 frames invulnerable)
COP A0 $84:DC2F            text child: if $0DC2 = 0, COP 1B $84:DC40 (EU $84:DC0D, "...TOASTED")
COP B6                     mirror off
COP 84 04 00 05; COP 8E    resource 5 list 4: 4,4,4,3,3,2,2,5,5,5,5,5,5,5 = 57 frames
COP 84 05 00 05; COP 8E    resource 5 list 5: 1 frame (`$A5:EB60`), then held
COP 02 3E 00               up to 62 frames: leave early when $066C & $0020 = 0
COP CB 01 $84:87C1         pad control back
```

No life change in the runner or the script: **the burn does no damage**.
It locks input for 57 + 1 + up to 62 frames, which ends with the 120-frame
status (the two run in parallel from the same frame). It cannot be shortened
below the two poses (about 58 frames).

The burn ends by: the counter (120 frames); `$097C & $C4E5`; a later
successful status roll from any enemy hit (§2.2 step 3); a new status
of element 8 or 10 (sleep), whose AND masks `$EF1F`/`$EF5F` clear
`$0020`; a map load (`$8D:8BF4` -> `$85:DFA8`, EU `$85:E040`:
`TRB $066C #$17F8`, slots `$067C-$068A` = 0, and life 0 -> 1).

While `$0020` is set, other code tests `$066C & $00E0` (push and
interaction tests in [tower two](tower-two.md)): these refuse.

Text: JP `$84:DC40` (child `$84:DC2F`), EU `$84:DC0D` (child `$84:DBFC`):
EU bytes `c4 00 c1 d2 00 76 53 cf "TOASTED" 6d c5 64 d7` (Ark's name and
"'s" are control/dictionary codes; guess for their decoding).

Sleep for comparison (`$84:DA1D` / EU `$84:DA03`): same start, text JP
`$84:DA73` / EU `$84:DA59` ("ASLEEP"), pose 7 of resource 5 looped with
`COP E4` while `$0040`, then pose 8; 180 frames; the runner (`$84:D2F3`)
adds 8 to the counter each frame with `$0454 & $80` (guess: a button
press), so mashing wakes Ark sooner.

## 3. Open

- A native measurement of a dart hit (Ark's stats, frame of the hit) and of
  a flyer-bullet burn (the order of the recoil and `COP DF`).
- `7F:102C` = 0 for the launchers (guess).
- `$097C & $C4E5` meanings.
