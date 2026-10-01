# Chapter 1 combat core

Research for spear combat in tower 1 (map `$101`, the first three enemies).
Addresses are Japanese (JP) unless marked. The EU column or the EU rule
below gives the European address. "Guess" marks a claim without a native
measurement or a complete source read.

Evidence: source reads, and native per-frame WRAM records (`poseprobe`
`rec`/`pose`, `fxprobe` `log` for OAM). The JP start is the journey checkpoint
`first-tower-neutral-stable` (map `$101`, (112,607), facing Up). The EU start
is `tmp/tower/eud/ctl.state` plus the JP route lines 579-615, which reach the
same place (EU frame 80427). Both runs give the same thrust timing, compositions
and damage formula. Two things were poked, and the text marks them: the
equipment (`$064A=$81`, `$0659=3`), because the route does not equip the spear,
and in some tests an enemy position or `$065D`/`$0690`.

Notation: `E+$xx` is the entity byte at `$1000+n*$40+$xx` (bank `$7E`).
`7F:0xxx,X`, `7F:1xxx,X` and `7F:2xxx,X` are the code's long-indexed fields.
For Ark (X=`$1000`), `7F:1022,X` is `$7F:2022`.

## 1. EU address rule

| Area | JP | EU |
|---|---|---|
| Player controllers and attack scripts (bank `$84`) | as in this doc | same address (checked: `$84:8000`, `9252`, `9458`, `A475`) |
| Death script `$84:DC59` | | `$84:DC20` |
| Combat, HUD and level code (bank `$85`, `$85:D281`..`$85:F5xx`) | as in this doc | **+`$98`** (checked: `D281`, `D93D`, `DE3A`, `DE8F`, `DF0B`, `DFF0`, `E03D`, `E14B`, `E2E9`, `E93C`, `EA99`, `EBA5`, `F319`, `F4BD`, `F63E`, `F835`) |
| Tables in bank `$8D` (level `BA61`, equipment `BC92`, profile pointers `BDFA`, profiles) | | **−`$137`** (`B92A`, `BB5B`, `BCC3`; blob profile `BEC0` → `BD89`) |
| `$8D:95DB` (add gems), `$80:B501` (COP D9) | | same |
| Ark resources 0/3/4/5 | `$A4:A1E4`, `$A6:9C70`, `$A5:A000`, `$A5:DCA6` | `$A6:A1E4`, `$A8:9C70`, `$A7:A000`, `$A7:DCA6` |

## 2. RAM

### Player stats block (`$064E`, Ark's profile: `7F:1022,X` of Ark = `$064E`)

| Address | Meaning | Start value (chapter 1) |
|---|---|---|
| `$064A` / `$064C` | equipped weapon / armor item ID (0 = none) | `$00` (the route does not equip) |
| `$0656` (byte) | level | 1 |
| `$0657` | max life = clamp(`$069C`+`$0698`, 1..999) | 28 |
| `$0659` | weapon power (`$8D:BC92`[(id−`$80`)*4] & `$3FF`) | spear `$81`: 3 |
| `$065B` | armor power | 0 |
| `$065D` | life | 28 |
| `$065F` | defense = `$06A2` + bonuses | 2 |
| `$0661`, `$0666` (bytes) | luck = `$069E` + `$069A` | 3 |
| `$0662` | attack = `$06A0` + bonuses | 3 |
| `$0664` | weapon attribute word (element) | 0 |
| `$066C` | status bits (`$0400` halves attack, `$0200` halves defense, `$0010` …) | 0 |
| `$066E` | current attack-type bits (set by each attack script) | |
| `$0690` (3 bytes, BCD) | EXP, cap 999999 | 0 |
| `$0694` (3 bytes, BCD) | gems, cap 99999 | 0 |
| `$069C`, `$069E`, `$06A0`, `$06A2` | base life, luck, attack, defense (from level table) | 28, 3, 3, 2 |
| `$04CE` | pending life regeneration (drained 1 per frame into `$065D`) | |
| `$0498` | number of living "counted" enemies in the room | 3 in `$101` |
| `$0600..$0607` | respawn after death: map, selector−1, x, y | `$0F`, 5, (384,144) |

`$85:F4BD` (JSL) recomputes `$0657 $0659 $065B $065F $0662 $0664 $0666`
from base stats and equipment. The menu's Equip runs it (`$85:B009` writes `$064A`).

### Entity fields used by combat

| Field | Meaning |
|---|---|
| `E+$00/$02` | x, y (feet) |
| `E+$04` | flags: `$0001` projectile, `$0008` guarding (Ark, R), `$0010` Ark: *not* attacking, `$0040` hurt, `$0080` dead, `$0200` can be hit (enemy, or `COP 65`), `$0400` player side (Ark), `$8000` hidden |
| `E+$06` | flags: `$0020` takes no damage, `$0010` script handles knockback itself |
| `E+$08` bit `$4000` | horizontal mirror |
| `E+$12`, `7F:000A,X` | bank and pointer of the current composition |
| `E+$14` | facing 0 Down, 1 Up, 2 Left, 3 Right |
| `E+$16` | layer: attacker and target must match (attacker < 0: any) |
| `7F:1020,X` | invulnerability: 0 = can be hit. Positive counts down, negative counts up by 1 per frame |
| `7F:1022,X` | profile pointer (bank `$8D`; Ark: `$064E`) |
| `7F:1024,X` | 1 when the enemy counts in `$0498` |
| `7F:102A,X` | enemy life |
| `7F:102C,X` | attack kind (Ark: 0..4, see §4) |
| `7F:201A,X` | last damage taken (BCD; the damage digits read it) |
| `7F:201E,X` | wake bits for the script: `$0200` my attack hit, `$0800` struck callback (`COP 65`), `$1000` knockback, `$4000` death, `$8000` busy |

### Enemy profile (25 bytes; `COP D9 n` sets `7F:1022,X = $8D:BDFA[n]`, life = profile+9)

Blob (tower 1, `$97:B55A`, profile 1 = `$8D:BEC0`):
`00 00 02 84 00 00 04 44 01 04 00 02 00 03 10 04 00 02 00 05 04 68 00 00 05`

| Offset | Meaning | Blob |
|---|---|---|
| +0, +4 | element resist / weak (vs weapon attribute) | 0, 0 |
| +2 | attack-type resistance: high nibble = types, low byte = 2-bit factor per type | `$8402` |
| +6 | attack-type weakness, same layout | `$4404` |
| +8 | level | 1 |
| +9 | max life | 4 |
| +`$0B` | EXP (BCD word) | 2 |
| +`$0D` & `$0FFF` | gem drop (BCD) | 3 |
| +`$0E` bits 4-7 | drop mask: drop if (random & mask) = 0 | 1 (one in two) |
| +`$0F`+5k, +`$10`+5k | attack of kind k (10 bits), +`$10` & `$78` = element | 4 |
| +`$11` | defense | 2 |
| +`$13` | luck | 5 |

Spawn records set it too (`$80:F94C`, descriptor byte 4). A spawn parameter
bit `$80` sets `7F:1024,X=1` and increments `$0498`.

## 3. Ark's attacks

Buttons (idle controllers `$84:88E8` Down, `8A0D` Up, `8ABC` Right, `8BA7`
Left): **A** attacks (after the interaction check `$87:923F`; only with
`$064A`≠0), **B** jumps, **R** guards, **X** uses the item, **Select** opens the menu.
Without a weapon, A does nothing in the open.

Pose records are resource lists (`$80:A24F` table, see
[ark-poses](ark-poses.md)). Each record is duration+1 frames, a facing byte
and a composition. **A composition starts with four boxes of 4 signed bytes
(dx, w, dy, h): +0 sprite, +4 attack box, +8 body box.** The box is
x0=x+dx, x1=x0+w, y0=y+dy, y1=y0+h. When mirrored (`E+$08 & $4000`):
x1=x−dx, x0=x1−w. Horizontal art faces Right. Left uses `COP B7` (mirror).

Ark's body box (standing, attacking): (−5,10,−16,14) or (−5,10,−15,14):
x−5..x+5, y−16..y−2.

### Attack table

| Attack | Input | Controller / Ark script | Kind (`7F:102C`), `$066E` | Pose lists (Down, Up, Right) |
|---|---|---|---|---|
| Thrust | A (standing or walking) | `$84:9252`/`92BE`/`932A`/`9396`; `$84:A475`/`A49C`/`A4C3`(Right)/`A4C7`(Left) | 0, `$0000` | res4 `$00`/`$01`/`$02`; horizontal into a wall: `$1B` |
| Rapid thrust | ≥2 more A presses, each 5-9 frames after a thrust start | `$84:9458`/`9491`/`94CA`/`9503`; `$84:A540`.. | 1, `$8400` | res5 `$00`/`$01`/`$02` (loop), end res4 `$03`/`$04`/`$05` |
| Jump attack | A during jump frames 8..18 | `$84:97E0`; `$84:A664`/`A67F`/`A69E`(R)/`A69A`(L) | 2, `$4400` | res4 `$06`/`$08`/`$0A`; landing `$07`/`$09`/`$0B` (no hits) |
| Dash attack | A while dashing | `$84:9549`/`9578`/`95A7`/`95D6`; `$84:A6F1`/`A717`/`A73D`/`A741` | 3, `$1400` | res4 `$0C`/`$0D`/`$0E` (stream `$27`-`$29`), then res3 `$36`-`$38` |
| Dash-jump attack | dash, B, then A in jump-loop frames 8..19 | `$84:9B33`; `$84:A7B7`/`A7DB`/`A7FF`/`A803` | 4, `$2400` | res4 `$18`/`$19`/`$1A` + spin loop `$0F`/`$11`/`$13`; after landing slide `$10`/`$12`/`$14` (kind 4, `$1400`) |
| Guard | hold R | `$84:9605`.. ; `$84:A5E4`.. | none | res4 `$15`/`$16`/`$17` loop; sets `E+$04 & $0008` |

Ark is an attacker only while `E+$04 & $0010` is clear. The controllers clear it
on the second frame of the action and set it again at its end.

### Thrust, measured (facing Up, JP and EU equal)

Frame P = the frame A is held (1 frame is enough).

| Frame | Script | Composition | Attack box (dx,w,dy,h) Up | Can hit |
|---|---|---|---|---|
| P+1 | `$84:A4BE` | `A36D` | (−8,16,−24,16) | no (flag `$0010` still set) |
| P+2..P+4 | | `A36D` | (−8,16,−24,16) | yes |
| P+5..P+8 | | `A436` | (−7,17,−40,25) | yes |
| P+9..P+10 | | `A3CE` | (−7,17,−42,24) | yes |
| P+11..P+12 | | `A402` | (−7,17,−40,24) | yes |
| P+13..P+16 | `A4BE`, then `A4C2` | `A39A` | (−8,16,−24,16) | yes |
| P+17 | `$84:A318` | standing `A597` | | no |

All records (frames each, then the attack box; facing byte 0 Down, 1 Up, 3 Right):

| List | Records |
|---|---|
| res4 `$00` thrust Down | 4 `A22A` (−8,16,−8,16), 4 `A31D` (−8,17,1,25), 2 `A299` (−8,19,0,25), 2 `A2DB` (−9,19,−1,27), 2 `A25E` (−8,16,−8,16) |
| res4 `$02` thrust Right | 4 `A46A` (4,15,−19,13), 4 `A595` (12,24,−22,24), 2 `A503` (11,24,−21,23), 2 `A54C` (12,24,−21,23), 2 `A4B3` (4,15,−18,13) |
| res4 `$1B` thrust Right, wall in front (probe x±12/13, y−8, `$84:A52D`) | 4 `BD4E` (−3,15,−16,4), 4 `BE79` (7,24,−19,20), 2 `BDE7`, 2 `BE30`, 2 `BD97` |
| res5 `$00`/`$01`/`$02` rapid | 8 × 2 frames, a fixed box: Down (−16,32,8,16), Up (−16,32,−40,16), Right (16,16,−24,32) |
| res4 `$03`/`$04`/`$05` rapid end | 3 frames (`A22A`/`A36D`/`A46A`) |
| res4 `$06` jump attack Down | 3 `A5E5` (−4,7,−16,6), then 2 each `A619 A662 A6B2 A6FB A619` (−8,16,−31,40) |
| res4 `$08` jump attack Up | 3 `A842` (−3,8,−21,5), then 2 each `A86F A8A3 A8EC A935 A86F` (−8,16,−42,40) |
| res4 `$0A` jump attack Right | 3 `AA4B` (1,10,−19,5), then 2 each (−12..−15,32,−32,32) |
| res4 `$0C`/`$0D`/`$0E` dash attack | 9 × 2 frames; Down ≈(−8,18,−11,24), Up ≈(−9,20,−45,25), Right ≈(8,24,−24,22) |
| res4 `$18`/`$19`/`$1A`, then `$0F`/`$11`/`$13` | 3 frames, then 1-frame spin records: Down (−16,32,−12,28), Up (−16,32,−40,24), Right (−1,20,−21,24) |
| res4 `$10`/`$12`/`$14` slide | 2,2,1,1 frames per record (Up), boxes Down (−8,16,−11,16), Up (−8,16,−31,16), Right (5,16,−16,16) |
| res4 `$15`/`$16`/`$17` guard | 2+1 frames per loop |

Other measured timings (JP, facing Up):

- **Chain**: a press 10 or more frames after the thrust start restarts the thrust at P+1. A press 1-4 frames after it is ignored and resets the press count (`$84:9402`: loop counter `7F:0002,X` 16→0; ≥13 ignored, 8..12 in-window, <8 restart).
- **Rapid**: presses spaced 5..9 frames apart give thrust, thrust, then rapid (`$84:A572`, 8 records × 2 frames, 17 frames a cycle with sound `$04`). It goes on while A keeps coming. Up cancels it (`$84:9471`, guess: the opposite direction).
- **Turn/walk out**: during a thrust a direction other than the facing (`$84:9282`) aborts it into walking after loop frame 5 (counter < 11).
- **Jump** (B at J, standing): crouch res3 list 3 (J+1..J+3), air lists 4 (8+4) and 5 (6+6), stand at J+28. Height is `$0970` (negative is up).
- **Jump attack** (B at J, A at J+10): res4 `$08` from J+11 to landing at J+28 (attacking), landing list `$09` 4 frames (not attacking), stand at J+32.
- **Dash attack** (dash Up, A at D): 18 frames of res4 `$0D` moving 3 px/frame (stream `$28`), attacking D+1..D+19. Then res3 `$37` 5+2 frames, and the dash goes on if Up is held.
- **Dash-jump attack** (B at D, A at D+10): `$19` 3 frames, then the spin until landing (D+34, 1 frame a record, 2 px/frame), then the slide for 16 frames (still attacking, 32 px). On landing Ark gets `7F:1020 = $1E` (30 frames of invulnerability).

## 4. Ark's hit on an enemy

### Hit scan (`$85:D281`, EU `D319`; every frame unless `$049A`≠0)

For each attacker with `E+$04 & $0400` and none of `$00D0`, against each
target with `$0200`, none of `$04E2`, the same layer `E+$16`, and `7F:1020,X`=0:

1. `$85:F63E`: attacker box = composition +4. `$85:F78E`: target box = composition +8.
2. `$85:F835`: overlap, inclusive on all four edges. A negative right or bottom edge does not hit. Height is ignored.
3. Shield (`$85:D3FC`): a target with `$0008`, hit by a projectile (`$0001`). It is blocked if the target's +4 box overlaps and the facings match (`$85:F875`, table `$85:F8B6`).
4. `$85:D4C8`: target profile 0 → only "struck" (`7F:1020=$10`, wake `$0800`; doors, residents: the existing `strike()`). Otherwise the hit sound `$09` plays, the damage below is applied, and the attacker gets wake `$0200`.

### Damage, Ark → enemy (`$85:DBDF`, `DD4B`, `D93D`, `DA9E`, `DE3A`, `DFF0`)

```
L   = level ($0656)
raw = ((L + 7) * attack($0662)) >> 3 + weaponPower($0659); halve attack first if $066C & $0400
def = ((eL + 11) * eDef) / 2 / 6          (eL, eDef: profile +8, +$11; integer steps)
dmg = raw - def;  if dmg <= 0: dmg = (weaponPower >> 5) + 1
kind (7F:102C of Ark): q = dmg/4 + 1
   1 rapid: dmg - q;  2 jump: dmg + q;  3 dash: dmg + 2q;  4 dash-jump: dmg + dmg/2 + 2q
type ($066E & $FBFF vs profile +2 / +6): for the highest common type bit
   (bit 15 → field bits 0-1, 14 → 2-3, 13 → 4-5, 12 → 6-7):
   resist field 0 → immune (no damage), 1 → /2, 2-3 → /4
   weak field 1 → x1.5, 2-3 → x2
crit: if (random & $7F) < max(4, luck($0666) + 8 - eLuck): dmg *= 2 (other digit colour)
variance (frame counter $42): if bit 3 set: d = dmg/4 (bit 2 set) or dmg/8 (else), min 1;
   bit 8 of $42 set → dmg = max(1, dmg - d), else dmg + d
dmg = min(dmg, 9999); enemy life (7F:102A) -= dmg, min 0
```

Check (spear, level 1, blob): raw = 8·3>>3 + 3 = 6, def = 12·2/12 = 2 →
**4**. Measured: EU 4 (`$42`=`$3651`, no variance); JP 3, 3, 3 (`$42`=`$FB78`,
`$FB7C`: −1). The blob takes 1/4 from the rapid thrust and ×1.5 from the jump
attack (types in its profile; from source, not measured).

### After a hit

| Item | Native |
|---|---|
| Damage digits | Entity `$87:A1BF` at (target x, target y − (`E+$1A`+8)), with BCD digits from `7F:201A,X`. It moves 1 px down per frame for 8 frames, up for 24, stays 16, then is deleted (48 frames). `COP D5 00 38` normal, `3A` critical, `36` damage to Ark. |
| Enemy invulnerability | `7F:1020 = $FFF0` on the hit frame. The knockback handler `$85:E03D` sets `$FF38` and counts it up for 34 frames, then `$FFF0` again (16 frames). It can be hit again **50 frames** after the hit. |
| Knockback direction | `$85:F8D1`: from attacker − target, the dominant axis. 0 = pushed Down, 1 = Up, 2 = Left, 3 = Right. |
| Blob knockback (Ark below it) | y: −4 for 6 frames, −2 for 4, −1 for 4, 0 for 4, then −2 for 4, −1 for 4 (the blob's `COP 81 01`), 48 px in 34 frames |
| Death | Life 0 sets `E+$04 |= $0080`. At the end of the knockback, `$85:E218`/`E23E`: `$0498` −1 (if counted), EXP += profile+`$0B` (BCD, `$85:F319`), then the explosion `$85:E2E9` (13 records × 2 frames) |
| Drop (`$85:E2E9`) | If (random & mask) = 0 and the cell is valid (`$85:E4A3`): a gem entity at the death point. Icon `7F:0008` `$0B` (<10 gems), `$0C` (<100), `$0D`. It appears for 16 frames, then can be picked up for 240 frames, then blinks for 32 frames (hidden every other frame) and goes. |
| Pickup (`$85:E482`) | Ark's probe (`$0966`,`$0968`) = (x, y−8) inside [x−8, x+8) × [y−16, y) of the gem: gems += amount (BCD, `$8D:95DB`), sound `$47`, gone. |

Measured kill (JP, blob life 1, poked to (120,150), Ark at (120,182) facing Up):
A at 66024, hit 66029, death check 66064 (`$0498` 3→2, EXP 0→2),
explosion 66065-66091, gem 66092. Pickup (walking Up into it) at 66155: gems 0→3.

## 5. Enemy hit on Ark

Scan `$85:D30C` (EU `D3A4`): enemy (`$0200`, none of `$01D0`) attack box (+4) against
Ark (`$0400`, none of `$0262`) body box (+8), same layer, Ark `7F:1020=0`.
**`$85:F856`: no hit while Ark is 16 px or more above the ground (`−$0970 ≥ 16`).**
Guard (`$85:D45B`): Ark with `$0008`, hit by a projectile (`$0001`) from the front: blocked (sound 9), the projectile bounces.

```
eL = enemy level; atk = profile[$0F + 5*kind] (10 bits)
raw = ((eL + 7) * atk) >> 3
def = armorPower($065B) + ((L + 7) * defense($065F)) / 4 / 3   (defense halved if $066C & $0200)
dmg = raw - def;  if dmg <= 0: dmg = (eL >> 2) + 1
then type/element vs armor ($064E/$0650), armor status checks, the same variance; life $065D -= dmg (min 0)
```

Blob → Ark: 8·4>>3 = 4, def = 8·2/12 = 1 → 3. Measured 2, 2 (variance −1).

| Item | Native (JP measured: hits at 65907 and 65950) |
|---|---|
| Sound / digits | sound `$07`, digits `COP D5 00 36` |
| Knockback | Helper `$0DEE` runs `$84:8000`; Ark `$84:80B2` (pushed Down, list res0 `$0D`, stream `$37`), `80BD` (Up, `$0C`/`$36`), `80CC` (Left, `$0E`/`$38`), `80C8` (Right, mirrored). About 10 px in the first 10 frames, then still. 26 frames, input locked. Ark then faces the attacker. |
| Invulnerability | `7F:1020 = $FF38` at the hit (+1 per frame), `$0010` when the knockback ends (−1 per frame). It can be hit again **43 frames** after the hit. |
| Blink | During the knockback only, Ark's sprites are hidden on every second frame (hit+2, +4, …). They are not hidden during the 16 frames after it. |
| Low life | life ≤ max/4: sound `$1C` every 128 frames (`$85:E14B`) |
| Regeneration | `$85:E14B`: if `$07EF`≠0 (set by `$93:D5BE`; guess: tower maps) and the weapon is `$81`, then +1 life every 256 frames (`$42 & $FF` = 0, via `$04CE`, sound `$29`). Measured: 24→25. Weapon `$9C`: every 64 frames. |
| Game over | `$065D` = 0 (`$85:E1BB`): Ark `E+$04 |= $0080`. Helper `$84:DC59`: collapse pose res0 `$0F` (110 frames), music `$3B`, a timed message (アークは だんだん 意識が遠…, about 500 frames), then life = max and transfer to `$0600..$0606` (map `$0F`, Ark's house). Measured hit 65901 → map `$0F` at 66608. |

## 6. Level up

`$85:EA99` (each frame, while Ark is idle): if EXP ≥ the next level's threshold, the
helper runs `$85:F35B`. The table `$8D:BA61` (EU `$8D:B92A`) has 11 bytes per level: EXP (3 BCD bytes),
life, attack, defense (words), luck (byte), 1 byte unknown. Level 1-12:

| Lv | EXP | Life | Atk | Def | Luck |
|---|---|---|---|---|---|
| 1 | 0 | 28 | 3 | 2 | 3 |
| 2 | 38 | 33 | 4 | 3 | 4 |
| 3 | 120 | 39 | 4 | 4 | 5 |
| 4 | 250 | 47 | 5 | 5 | 6 |
| 5 | 430 | 52 | 6 | 5 | 7 |
| 6 | 675 | 59 | 6 | 6 | 8 |
| 7 | 995 | 65 | 7 | 7 | 9 |
| 8 | 1573 | 72 | 8 | 7 | 10 |
| 9 | 2293 | 79 | 10 | 8 | 11 |
| 10 | 3138 | 86 | 11 | 10 | 12 |
| 11 | 4118 | 95 | 13 | 11 | 13 |
| 12 | 5233 | 102 | 15 | 14 | 14 |

Measured (JP, EXP poked to 36, then a kill), L = the EXP frame:

| Frame | Event |
|---|---|
| L | `$097C |= $8000` (the world stops). Ark `7F:1020 = $C8`. |
| L+10 | Ark victory pose `$84:A8C8` |
| L+80 | level +1, message `$92:8000` (アークは レベルが 2 になった！) |
| L+160 / +239 / +319 / +399 | base life (+ the same to current life), attack, defense, luck each += table delta, each with its own message (`$92:8020`, `803E`, `805B`, `8078`). A zero delta is skipped. |
| L+479 | `$85:F4BD` recompute, message `$92:8095` |
| L+516 | control back. Ark `7F:1020` = −59 (counts up, about 59 frames immune) |

Each message closes by itself after about 80 frames. A was not pressed. Guess: A closes them sooner.

## 7. HUD (towers, layout `$06BE`=0)

BG3 tilemap: the WRAM buffer `$7F:D000` (32×32 words) is uploaded to VRAM word
`$6800` when the update flag `$7F:0CB4` is set. `$85:E93C` (EU `$85:E9D4`) runs each frame.
It redraws only values that differ from their caches `$7F:0CA6` (life), `0CA8` (max),
`0CB2` (level) and `0CAA/0CAC` (gems).

| Item | Cells (col,row) | Format |
|---|---|---|
| "LEVEL" "ITEM" labels | row 1 from col 4 | palette 2 |
| level | (4..5, 2..3) | 2 digits, right-aligned |
| "LIFE" label | (24..26, 1) | |
| life | (22..24, 2..3) | 3 digits, right-aligned, then "/" at col 25 |
| max life | (26..28, 2..3) | left-aligned |
| gem icon | (24..25, 25..26) | tiles `$40 $41 / $50 $51`, palette 3 |
| gems | row 25..26 from col 26 | 5 BCD digits without leading zeros. 1-3 digits end at col 28. |
| frame | cols 0 and 31 | tile `$2820` |

Each digit d is 8×16: top word `$2C21+d` (tile `$21+d`, palette 3,
priority), bottom word (top & `$E3FF`) | `$1010` (tile `$31+d`, palette 4). "/" is tile
`$2F`/`$3F`. Tiles come from the HUD sheet (menu.md: `$A9:F02F`, guess: the same sheet).
Screenshot: `LEVEL 1`, `LIFE 28/28`, gem `0`.

## Open questions

- The first-press A path: the A mask in the controllers is `$0081` and R is `$0011`. Bit 0 is not a real pad bit (guess: key configuration).
- The meaning of profile +0/+4 (weapon element) and the armor attribute path were not measured.
- Enemy knockback distance is per enemy (`COP 81 n` in its own script). Only the blob was measured.
- The gate in `$101` opens when `$0498` reaches 0 (guess: a map script polls it; not measured).
- The jump attack, the dash attacks and the guard were measured without an enemy (timing only). Their hits are from source.
