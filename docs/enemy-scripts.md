# Chapter 1 enemy scripts

Research for running the chapter 1 enemy scripts in
`crates/crysta-runtime/src/actors.rs`. It lists every COP service and native
idiom that these scripts and their callbacks use, the engine around them
(scheduler, sleep, wake bits, movement, collision) and the blob's measured
timing in tower 1 (map `$101`).

Addresses are Japanese (JP) unless marked. "Guess" marks a claim without a
measurement or a complete source read. Notation as in [combat](combat.md):
`E+$xx` is the entity word at `$7E:1000+n*$40+$xx`, `7F:xxxx,X` is the
long-indexed field of the same entity.

Evidence: source reads (COP table `$80:83B2`, handlers in bank `$80`), a
script walker that follows COP branch operands, native branches and
registered callbacks (job folder `tmp/ep/explore.py`), and per-frame WRAM
records with `poseprobe` (`rec`) from the journey checkpoint
`first-tower-neutral-stable` (`tmp/tower/jp2/ctl.state` plus route lines
579-615 of `tools/tower-approach-qualification/tower-route.jsonl`; frame
65910, Ark at (112,607)). Pokes are marked where used.

## 1. EU addresses

| Area | JP | EU |
|---|---|---|
| COP table `$80:83B2` and all bank `$80` handlers named here | | same address (checked: table, `8E33`, `9AC7`, `AE38`, `AF22`, `B26E`, `B362`, `B38A`, `B474`, `B4B1`, `B530`, `C8E1`, `C967`, `D0CF`, `E9A2`, `ED75`, `F251`) |
| RNG `$86:8236` | | same |
| Bank `$85` combat scripts | `E03D`, `E0F9`, `E218`, `E27B`, `E2E9`, `E353`, `E3CF`, `E53F` | +`$98` (`E0D5`, `E191`, `E2B0`, `E313`, `E381`, `E3EB`, `E467`, `E5D7`) |
| Bank `$97` enemy scripts | `$97:B555`..`$97:CB03` | bank `$99`, about −`$3555` (see §8 per script) |
| Knight | `$95:EFA0`/`EFAB` | `$98:D4B6`/`D4C1` |
| Bank `$90` controllers | `938B`, `9BF7`, `9EEA` | `$90:95FA`, `9EB2`, `A201` |
| Shadowkeeper | `$93:D871` | `$99:9B59` (byte match 29/32, guess) |

EU script addresses come from byte matching (24-32 bytes, branch words
masked by score). The scripts use the same services in both ROMs; branch
targets move with the bank.

## 2. Engine around a script

### Scheduler (`$80:C8E1`, each frame, list from `$0DFA` through `E+$2E`)

For each entity, in list order:

1. If `7F:201E,X` ≠ 0 and its bit 15 (busy) is clear: wake dispatch
   (`$80:C967`, §5).
2. Skip the entity if `E+$06 & $0400`, or if `E+$04 & $0040` (hurt).
3. `DEC E+$0E`. While it stays ≥ 0 the script does not run. When it goes
   negative: `E+$0E = 0` and the script runs from `E+$0A`/`E+$0C` (RTL
   trick: push `+$0A − 1`, RTL).
4. Not hidden (`E+$04` bit 15 clear), or hidden with bit `$2000`: movement
   streams `$80:F251`, then position/collision `$80:D0CF`, then `$80:CB91`
   (player only). A hidden entity runs its script but does not move.

`E+$0E` is the script's sleep counter. `COP 8E`/`8F` and `COP C1` set it.
A value of n means the script runs again n+1 frames later; movement goes on
every frame.

### Visibility (`$80:E9A2`, draw pass, list from `$0DFC`)

`E+$04` bit 14 (`$4000`) = off screen. The draw pass sets it when the
sprite box (`E+$18`..`E+$1E`, from the composition) is outside the
256×224 screen at camera (`$081E`, `$0822`), and clears it otherwise.
Hidden entities (bit 15) also get it cleared. Measured: the blob at
(72,352) lost bit 14 at frame 66009 when Ark (at y 462) walked up; the
blob at (120,160) kept it the whole run.

### Pose step (`$80:ED75`, called by `COP 8E`, `8F`, `E4`, `E5`, `CD`)

Pose list = packet (`E+$10`, bank `E+$12`) [`7F:0008` = pose]. Record
`E+$20`: byte 0 = duration d → `E+$0E = d` (the record shows d+1 frames),
byte 1 = facing → `E+$14` (a mirrored entity reports Right(3) as Left(2)),
then the composition (sprite box into `E+$18..$1E`). `E+$20 += 1`. At the
end of the list (record word negative) or with a negative pose:
`E+$20 = 0`, and unless `E+$06 & $0040`, `E+$0E = 0` and both streams stop;
carry set. Otherwise carry clear.

### RNG (`$86:8236`, `COP 25`)

16 bytes r[0..15] at `$0408..$0417`. One step:

```
c = 0
for i = 15 down to 1: s = r[i] + r[i-1] + c; r[i-1] = s & $FF; c = s >> 8
then increment r[15]; on wrap increment r[14], and so on down to r[0]
```

Scripts read `LDA $0408` (word, r[0] | r[1]<<8) after `COP 25` and mask it.
Checked against the recording: two blob decisions on frame 66013 give 6 and
3 (`& 7`), as the blobs did. The drop roll in the death script
(`$85:E2F9`) and the crit roll use the same generator, so the sequence is
global, not per actor. The runtime's per-actor xorshift (`next_random`) is
a different generator.

## 3. COP services

Operand bytes follow the COP. "Word target" = a 16-bit address in the
script's bank. Status = how `actors.rs` handles it today:
**ok** (modelled), **partial**, **skip** (stepped over by the derived
length, which is wrong for branches), **freeze**, **missing** (no length
derivable or wrong behaviour).

### Waits, flow and timing

| COP | Handler | Operands | Semantics | Used by | Status |
|---|---|---|---|---|---|
| `00`/`01` | `8592`/`85B8` | long / – | call one level, return | Shadowkeeper | ok |
| `02`/`03` | `85DF`/`85F8` | count word / – | counted loop; `03` jumps back and yields | Cadet, show, Shadowkeeper | ok |
| `04` | `8613` | – | as `03` but jumps back in the same frame (no yield) | Shadowkeeper | missing (skip) |
| `05` | `862E` | flag word | wait for flag | Four Hiballs, ball wave, High Cadet, Three Cadets | ok |
| `06` | `864C` | long | long jump | controllers | ok |
| `07`/`08`/`48` | | flag word (+target) | write flag / branch / delete on flag | bosses | ok |
| `25` | `8E33` | – | one RNG step (§2) | nearly all | **missing** (skip: no step) |
| `59 n` | `9AC7` | byte n | if `E+$04` bit 14 (off screen): `E+$0A` = this COP, `E+$0E = n`, yield (retry after n+1 frames). Else go on | all enemies (first command) | **wrong** (always goes on) |
| `58 b t` | `9AB2` | byte, word target | wait until `$0648 == b`, then jump to t | Guardner | missing |
| `71 m1 m2 t` | `A016` | word, word, word target | jump to t if Ark is busy/dead/without `$0004`, or `$097E & m1`, or `$097C & m2`; else retry next frame | Cadet, Guardner | missing |
| `BC` | `AAA5` | – | `E+$0A` = next; go on (re-entry point for later RTLs) | flyers, Guardner, bosses | ok |
| `BD` | `AAB3` | – | `E+$0A` = next; yield 1 frame | controllers | ok |
| `BE` | `AAC1` | long, delay word | `E+$0A` = long, `E+$0E` = delay; yield | Guardner | missing |
| `BF`/`C0` | `AAE1`/`AAFB` | long | set `E+$0A/$0C`, `E+$0E = 0`; `BF` yields, `C0` goes on | bosses | ok |
| `C1 n` | `AB17` | word | `E+$0E = n`, `E+$0A` = next, yield (resume n+1 frames later) | flyer launcher, controllers | ok |
| `22` | `8CD8` | lo, hi, words | switch on `E+$26` | Shadowkeeper | ok |
| `5F` | `9BEA` | 4 word targets | jump by the player's facing `$0956` (0-3); ≥4: go on past 8 bytes | Cadet | missing |

### Poses and movement

| COP | Handler | Operands | Semantics | Used by | Status |
|---|---|---|---|---|---|
| `80 p` | `A18C` | pose | `7F:0008 = p`, `E+$20 = 0`, stream restart pointers `7F:0014/16 = 0` (a running stream stops at the next list end), `E+$0A` = next | most | ok |
| `81 p` | `A1B9` | pose | as `80`, and start movement selector p (`$80:BBDB`) | blob, knight, Hiballs | ok (needs `E+$04 & 4` collision, §6) |
| `82 p s` | `A1DE` | pose, selector | as `81` with another selector | flyers, `B847`, show | missing (skip) |
| `85 c p` | `A182` | count, pose | `E+$22 = c`, pose p, no movement | blob, Hiballs | ok |
| `86 c p` | `A1AF` | count, pose | `E+$22 = c`, then `81 p` | `B635`, ball wave | missing (skip) |
| `87 c p s` | `A1D4` | count, pose, selector | `E+$22 = c`, then `82 p s` | `B847`, show | ok |
| `8E` | `A32F` | – | `ED75`; list not ended: yield (re-run at `E+$0A`); ended: go on | all | ok |
| `8F` | `A33E` | – | `ED75`; at list end `DEC E+$22`: not 0: restart list and streams (`7F:0010/12 = 0014/16`) in the same frame; 0: go on | blob, Hiballs | ok |
| `E4` | `B929` | – | `8E` with its own record countdown in `7F:200A` (script runs every frame) | knockback `$85:E03D` | missing |
| `E5` | `B953` | – | the same for `8F` | (not in these scripts) | – |
| `AF s` | `A965` | selector | start movement selector s only | Shadowkeeper | missing |
| `B0 n` | `A975` | byte (`FF`: word + bank) | `7F:0022 = $4000 + n<<12`, `7F:0024 = 0`; `FF`: `7F:0022 = 7F:0026 = word`, `7F:0024 = bank` | flyers, `B847`, show | partial (`FF` form not modelled) |
| `B1`/`B3` | `A9B9`/`A9EA` | word / 2 words | x += w (negated when mirrored) / and y | knight `EFA0`, Guardner, bosses | ok |
| `B6`/`B7` | `AA33`/`AA42` | – | clear / set mirror (`E+$08 & $4000`) | all | ok |
| `B8`/`B9` | `AA51`/`AA60` | – | toggle mirror / vertical flip | knight / Shadowkeeper | missing |
| `CC a p c d s` | `AE38` | 5 bytes | line move to (`7F:2004`, `7F:2006`), set by the script before. Pose p. Steps: N = max(|dx|,|dy|)/c (+1 when not 0) after halving dx, dy until both < 256; the whole vector is run (a+1) times (so a=0 stops at the target, a=4 runs 5 times as far). d: stored in `7F:200B` (meaning not traced). s ≠ `FF`: also start selector s | flyers, `B635`, `B847`, Cadet, Guardner | missing |
| `CD` | `AF22` | – | one step of the `CC` line per frame through `7F:0018/001A` (so walls stop it), pose stepped by `ED75`; when done `E+$0A` = next, yield | same | missing |
| `D0`/`D1` | `B136`/`B1D1` | | orbit | Cadet children, High Cadet | ok |
| `D8` | `B4DF` | long | art packet | flyers, death script | ok |

### Player tests (all against Ark's probe `$0966`,`$0968` = (x, y − 8))

| COP | Handler | Operands | Semantics | Used by | Status |
|---|---|---|---|---|---|
| `D6 d t` | `B474` | byte d, word t | jump to t if \|x − `$0966`\| ≤ d and \|y − `$0968`\| ≤ d; else go on | blob, knight, flyers | **missing** (skip: never taken) |
| `D7 h v` | `B4B1` | 2 word targets | dx = `$0966` − x, dy = `$0968` − y; \|dy\| ≥ \|dx\|: jump v, else jump h. Always jumps | blob, knight, Hiballs, Cadet | **missing** (skip runs into data) |
| `D3 w l e r` | `B362` | word, 3 word targets | dx = `$0966` − x. dx = 0: e. dx < 0: l if −dx−1 ≥ w, else e. dx > 0: r if dx−1 ≥ w, else e | same | **missing** |
| `D4 w u e d` | `B38A` | word, 3 word targets | as `D3` with dy = `$0968` + 8 − y (Ark's feet): u above, d below | same | **missing** |
| `D2 w l m t` | `B26E` | 3 bytes, word target | box in front of the actor by `E+$14`: Down [x−w,x+w]×[y,y+l], Up [x−w,x+w]×[y−l,y], Left [x−l,x]×[y−w,y+w], Right [x,x+l]×[y−w,y+w] (low edges clamped at 0, edges inclusive). Probe inside and m = `7F`/`FF`: jump t. m = 0: also needs (`E+$14` + `$0956`) & 3 = 1 (facing each other). m = 1: needs that sum even, or & 3 = 3 (not facing each other). Other m: never. Else go on | knight, `B635`, `B847`, Cadet, Guardner | missing |

### Spawns and deletion

All spawns copy `E+$00..$17`, `7F:0022/24/26` and `7F:1022` from the parent
(`$80:BCA4`), set `7F:001E` = parent and `7F:2020` = the child's script bank.

| COP | Handler | Operands | Semantics | Used by | Status |
|---|---|---|---|---|---|
| `A1 L` | `A6F1` | long | child after parent in the list | Guardner, Cadet, bosses | missing |
| `A2 L f` | `A71B` | long, `+$04` word | child with flags | controllers, Cadet | ok |
| `A0 L f` | `A693` | long, word | child at the head of the draw list | Shadowkeeper | missing |
| `A4 L dx dy f` | `A79D` | long, 2 words, word | child at (x ± dx, y + dy), dx negated when the parent is mirrored | flyers (bullets), Cadet (spells), show | missing |
| `E6`/`E7`/`E8`/`EA` | `B99A`/`B9A5`/`B9B0`/`B9C6` | as `A1`/`A2`/`A4`/`9A` | the same spawns, and the child joins the parent's group (`7F:102E` = group root) | flyer launcher, Guardner, High Cadet, show, Shadowkeeper | missing |
| `EB` | `B9D1` | – | a group root deletes every entity of its group (`$80:BD57`) | Guardner, High Cadet | missing |
| `A7` | `A876` | – | delete self | all | ok |

### Combat and callbacks

| COP | Handler | Operands | Semantics | Used by | Status |
|---|---|---|---|---|---|
| `D9 n` | `B501` | byte | `7F:1022` = profile `$8D:BDFA[n & $7F]`; without bit 7 also life `7F:102A` = profile+9 | flyer bullets, `B847`, Cadet | skip (no profile) |
| `DA w` | `B530` | word | `7F:1002 = 1004 = 1006 = 1008 = w` (handlers for wake bits 5-1, §5) | knight, `B635`, `B847`, Cadet | missing |
| `65`/`66` | `9D25`/`9D5A` | | hit target / return (doors) | – | ok |

Bosses only (not read in full): `5A bank addr idx n` (`9AEB`, copy n
colours to the palette buffer `$7F:0600 + 2*idx`), `63` (`9CA5`, 2 bytes,
switches the script to an effect in bank `$87`), `97` (`A453`, 8 bytes,
palette call `$8D:A910`, then `E+$0E` = last byte), `15` (`8A58`, Guardner:
loads a transfer record from `$8D:BA41` by `7F:101E`), `43`, `52`, `9A`,
`A8`, `E7`, `F5`.

## 4. Native idioms in the scripts

The scripts mix COP calls with plain 65816 code (16-bit A and X, X = the
entity, bank = the script bank).

| Idiom | Meaning | Where |
|---|---|---|
| `LDA $0408; AND #mask; BEQ; DEC; BEQ …` | random dispatch after `COP 25` | blob `B564`, all wanderers |
| `LDA $0408; BIT #$00A8; BEQ` | one in eight | `B64F` |
| `LDA $0408; AND #$3F; STA $000E,X` (long `9F 0E 00 00`) | random start delay | Four Hiballs `9396`, ball wave `9C06` (`& $1F << 3`) |
| `STA/DEC/CMP $0024,X`, `$0026,X` | script scratch words (counters, last excluded direction) | blob `B5EB`, `B601`, Hiballs |
| `LDA $0014,X` / `CMP $0014,X` | facing from the current pose record | `B65A`, `B6D5` |
| `LDY $0DEA; LDA $0016,Y; CMP $0016,X; BNE` | Ark on the same layer | flyers, Hiballs, knight, Cadet |
| `LDA $0966 / $0968; STA $7F:2004,X / 2006,X` (± offset) | set the `CC` target | flyers, Hiballs, Cadet |
| `LDA $0004,X; AND #$7FFF` / `ORA #$8000` | show / hide | flyers, Guardner, High Cadet |
| `LDA $0004,X; ORA #$0030` / `AND #$FFCF` | bits 4-5 of `+$04` (not attackable / not attacking, guess) | flyers |
| `LDA $0004,X; ORA #$0001` | becomes a projectile (shields block it) | `B635`, `B847` lunge |
| `LDA $0004,X; BIT #$4000; BEQ` | wait until off screen, then `COP A7` | flyer bullets `BAC0` |
| `LDA $0006,X; ORA #$0010` / `AND #$FFEF` | the script handles knockback itself (no knockback, §5) | flyer attack, Guardner, High Cadet |
| `LDA $0006,X; ORA #$0020` / `AND #$FFDF` | takes no damage | `B635` lunge, `B63D` |
| `LDA $0006,X; ORA #$4000` | (bullets, death script; meaning not traced) | bullets |
| `LDA #t; STA $7F:10xx,X` | register a wake callback (§5); `#0` clears it | knight, Hiballs, Cadet, flyers |
| `LDA #1; STA $7F:102C,X` | attack kind 1 for the hit formula | bullets |
| `STZ $0014,X` / `STA $0014,X` | bullet direction | bullets |
| `LDA $7F:001E,X; TAY; LDA $0000,Y …` | read the parent's position | flyer launcher, Guardner |
| `LDA #$97; STA $7F:2020,X; COP 06 …` | change the script bank for callbacks, then jump into bank `$97` | Four Hiballs, ball wave, Three Cadets |
| `JSR`/`RTS`, `JMP abs`, `BRA`, `RTL` | | all |

The native runner in `actors/native.rs` refuses all of these today (only a
few scratch words are allowed), so the blob freezes at `$97:B564`.

## 5. Wake bits and callbacks (`7F:201E,X`)

Set by the combat and collision code. The dispatch (`$80:C967`) runs at
the start of the entity's frame when bit 15 is clear. It takes the highest
set bit, **clears the whole word**, and points `E+$0A` (and `E+$0C` =
`7F:2020`, the script bank) at the callback. The script then runs from
there in the same frame (if `E+$0E` allows; the setters clear `E+$0E`).

| Bit | Raised by | Callback field | Default when the field is 0 |
|---|---|---|---|
| 15 `$8000` | knockback / death scripts | (busy: no dispatch) | |
| 14 `$4000` | death check `$85:E218` (life 0) | `7F:1012` (bank `7F:1014`) | `$85:E27B` death script |
| 13 `$2000` | death script, when `7F:1018` ≠ 0 | `7F:1018` (bank `7F:101A`) | |
| 12 `$1000` | Ark's hit (`$85:D578`), unless `E+$06 & $0010` | knockback `$85:E03D` (player side `$85:E0F9`) | |
| 11 `$0800` | after a hit that did not kill (`$85:E233`), or a hit on a profile-0 or immune target; only when `7F:1016` ≠ 0 | `7F:1016` (struck) | `$87:CA65` |
| 10 `$0400` | my projectile was blocked by a guard (`$85:D436`); only when `7F:100E` ≠ 0 | `7F:100E` | |
| 9 `$0200` | my attack box hit (`$85:D4EA`); only when `7F:1010` ≠ 0 | `7F:1010` | |
| 8, 7, 6 | the wall probe (`$80:E1B0`, `E1A2`, `E194`) for some tile classes, only when the field ≠ 0; that axis's stream stops | `7F:100C`, `7F:1000`, `7F:100A` | |
| 5 `$0020` | tile collision path `$80:D27D` (streams stop) | `7F:1002` (then cleared) | `$85:E53F` if `E+$04 & 2`, else a debug write |
| 4-1 | (not traced) | `7F:1002`, `1004`, `1006`, `1008` (`COP DA`) | |

Which tile classes raise bits 6-8 is not traced (guess: holes, ledges,
water). The knight and the Hiballs point them back at their own loop
start, so a bump restarts the decision.

### Knockback script (`$85:E03D`)

Sets `7F:0022` = `7F:0026` (the knockback movement base),
`7F:1020 = $FF38`, busy bit; `COP B6` (the mirror is cleared and stays
cleared); `COP 81 0/1/2` by the direction in `7F:0008` (0 pushed Down, 1
Up, 2 Left with `COP B7`, 3 Right); `COP E4` while a stream runs. Then: clear
busy, restore the base, death check `$85:E218`. Alive:
`7F:1020 = $FFF0`, **`7F:0008 = $FFFF`, `E+$22 = 1`**, then the enemy's
script resumes at its saved `E+$0A` (`7F:1026/1028`) in the same frame.
With the pose at `$FFFF` the interrupted `COP 8E`/`8F` ends at once, so the
script goes on with its next command.

### Death script (`$85:E27B`)

`E+$04 |= $80`, art packet `$A2:C000`, `E+$08 = $3000`, streams off; a
group root sends its group to `$85:E353`. With `7F:1018`: sound `$37`,
pose `$17`, wake bit 13. Else: sound `$0C` (`COP 37`), pose `$16`
(explosion), then the drop roll (`$86:8236` & profile mask), gem pose
`$0B`/`$0C`/`$0D` and script `$85:E3CF`, or `COP A7`. EXP, `$0498` and
level-up are in [combat](combat.md) §4.

## 6. Movement and collision

`COP 81`/`82`/`86`/`87`/`AF` start a selector s through `$80:BBDB`: X and Y
stream pointers at `7F:0022 + 4s` (in `$7F` RAM; the blob's base is `$4000`,
its descriptor's own resource; `COP B0 02` gives the common `$6000`).
`$80:F251` steps each axis per frame (`count`, `velocity` pairs; velocity
negated on X when mirrored, on Y with the vertical flip) into the pending
deltas `7F:0018`/`001A`. Then `$80:D0CF` applies them:

- `E+$04 & $0004` clear: position += deltas, no collision.
- `$0004` set, `$0002` clear (the blob, `$4204`): per axis, the box edge
  (`x + 7F:0028`, `y + 7F:002C`) is probed in the collision map
  (`$80:DDA0` right, `DA31` left, `D682` down, `D295` up). A blocked axis
  is clamped to the wall; the stream keeps running and the pose wait keeps
  its length. Measured: blob at x 40 (left wall) and at x 56 (y 328) held
  still for the rest of a 12-frame move. Blobs do not block each other or
  Ark.
- `$0004` and `$0002` set: a tile-attribute path (`$80:D1A9`, table at
  `$80:D1FB`) that can raise wake bit 5.

The probe classes and the box offsets per descriptor are not traced here
(see [collision](collision.md) for the player's probes).

## 7. Blob (`$97:B555`, script `$97:B55A`; EU `$99:8000`/`8005`)

Header `00 04 42 00 00`: `E+$04 = $4204` (off screen, hittable, wall
collision), `E+$06 = 0`. Profile 1 from the spawn record ([combat](combat.md)).

```
B55A  COP 59 08              sleep until on screen (never again later)
B55D  COP D6 20 →B5BF        Ark's probe within 32 px on both axes: near
B562  COP 25; r = $0408 & 7  wander:
        r=0: idle Down 2×30, move Down     r=1: idle Down 30, move Down
        r=2: idle Up 2×30, move Up         r=3: idle Up 30, move Up
        r=4: idle side 2×30, move Right    r=5: idle side 30, move Right
        r=6: mirror, idle 2×30, move Left  r=7: mirror, idle 30, move Left
      idle = COP 85 1E p; COP 8F (p = 3 Down, 4 Up, 5 side)
      move = COP 81 s; COP 8E (s = 6 Down, 7 Up, 8 Right/Left); then COP B6 (side)
      BRA B55D
B5BF  near: COP D7 → horizontal (|dx|>|dy|): COP D3 0: Ark left/same → $26=3, right → $26=2
                    vertical:   COP D4 0: Ark above/same → $26=1, below → $26=0
B5EE  COP 25; v = $0408 & 7; reroll while (v & 3) == $26
      v: 0 → r=0 path, 1 → r=2, 2 → r=4, 3 → r=6, 4 → r=1, 5 → r=3, 6 → r=5, 7 → r=7
```

So when Ark is near, the blob takes a random move but **never toward
Ark** (`$26` is the direction to Ark: 0 Down, 1 Up, 2 Right, 3 Left). It
does not chase. The `D6` test runs only between moves.

### Measured (JP)

| Event | Frames |
|---|---|
| Sleep | `E+$0E` counts 8..0; the script retries `COP 59` every 9 frames. On screen at 66009, first decision 66013 (latency up to 9 frames) |
| Idle `COP 85 1E p; COP 8F` | pose lists 3/4/5 are 1 frame; `E+$22` 30 → 1; the next command runs 30 frames after the `85` (66013 → 66043 → 66073) |
| Move `COP 81 s; COP 8E` | 3 records × 4 frames; `E+$0E` 3,2,1,0 per record; 1 px every frame from the `81` frame on; 12 px in 12 frames (66073-66084: x 72 → 60) |
| Next decision | the frame after the last move frame (66085), same frame as the new `COP 85` |
| Cycle | 30 or 60 idle + 12 move = 42 or 72 frames |
| Velocities | selector 6: y +1; 7: y −1; 8: x +1 (mirrored −1). Measured on blobs `10C0` and `1100` |
| Near (66541) | blob (56,328), probe (88,334): dx 32, dy 6 → horizontal, Ark right → `$26 = 2`; it chose v=0 (idle Down 60, move Down). Further near decisions (poked to (88,366), Ark above) never chose Up in 4 tries |
| Hit, survives (life poked to 20, hit 66312) | wake `$1000` on 66312; knockback 66313-66342 pushed Down: y +1×4, +2×4, +4×6, 0×2, −1×2, 0×2, +1×2, +2×2, +4×2, 0×4 (+47..48 px). 66343: the interrupted idle ends at once and the move runs, **unmirrored** (knockback cleared the mirror: it moved Right, x 88 → 100), decision at 66355 |
| Hit, dies (life 4, damage 4) | knockback 66313-66343, explosion pose `$16` 66344-66369 (13 records × 2), gem pose `$0B` from 66370 |

Positions per frame are in `tmp/ep/walk.bin`, `chase.bin`, `near.bin`,
`hit.bin`, `hit2.bin` (`rec.py` prints them).

## 8. Other chapter 1 enemy scripts

The header (5 bytes before the script) holds `E+$04` (bytes 1-2) and
`E+$06` (bytes 3-4).

| Script (JP → EU) | Who | Services beyond the blob's | Behaviour |
|---|---|---|---|
| `$97:B635` → `$99:80E0` (script `B63A`) | Hiball T2-T5 | `86`, `D2`, `DA`, `CC`/`CD`, `37`, callbacks `100A/100E/1016` | Wander: 7 in 8 a step (`86 18 p; 8F` turn when the facing changes, `81 s; 8E` move); 1 in 8 a patrol of 4 legs (`85 28 p; 8F`) with `D2 10 40 7F` after each. Ark in that box: face Ark (`D7`/`D3`/`D4`), projectile bit, callbacks → `B6AB`, squash 16 frames, no-damage bit, sound `$35`, `CC 04 p 03 00 FF` lunge (5× the vector to Ark, about 3 px a frame) |
| `$97:B847` → `$99:82F2` (`B84C`) | Hiball (stats `$16`, T4-T5) | `D9 16`, `B0 02`, `82`/`87` (common selectors `$70`/`$78`/`$79`), `DA`, `D2`, `CC`/`CD` | Random 1-2 steps in 4 directions, idle 24; same front-box lunge as `B635` |
| `$90:938B` → `$90:95FA` (`9390`) | Four Hiballs (T1 top), 12 Hiballs (T2) | `05`, `A2`, `C1`, `06` | Wait for flag 1; random delay (`$0408 & $3F` into `E+$0E`); spawn an appear effect, show, wait 31 frames, jump to `$97:B63D` |
| `$90:9BF7` → `$90:9EB2` (`9BFC`) | ball wave (`$112`) | `05`, `C1`, `37`, `86` | Wait for flag 3, 121 frames, random delay `($0408 & $1F) << 3`, appear (`86 04 0C`), set wall collision, jump to `$97:B63D` |
| `$97:B98D` → `$99:8438` (`B992`) | yellow flyer (T1-T2) | `B0 02`, `BC`, `59 0F`, `D6`, `80`, `82`, `CC`/`CD`, `A4`, `D9`, `D8`, `37` | Hidden; each frame (`BC`) sleeps off screen. Ark within 96: hidden random drift (`82 04 $7x`). Within 48: appear (sound `$37`), then chase Ark (`CC 00 04 01 18 FF`, 1 px a frame) until within 64; then attack: no knockback, 4 bullets (`A4`, kind 1, profile 2, fly until off screen), hide again |
| `$97:BBC2` → `$99:866D` (`BBC7`) | flyer variant (T3-T5, `$12B`) | as above, `E6`, `C1`, struck `1016` | As `B992`; the attack spawns a launcher child (`E6`) that fires bullets 11 frames apart from the parent's position (sound `$2B`) |
| `$95:EFAB` → `$98:D4C1` (`EFB0`); `$95:EFA0` → `$98:D4B6` (`B1 08 00` first) | knight (T1, `$12B`) | `D2`, `D7`/`D3`/`D4`, `B8`, `DA`, `D6 50`, callbacks `1016 → F073`, bits 1-8 → turn | Guards facing Ark (poses 3/4/5 with movement); recheck every list while Ark is within 80. Ark in the front box (16 wide, 32 long): thrust toward Ark (poses 6/7/8). After each surviving hit (struck) it thrusts. A wall/tile wake turns it to the other side |
| `$97:BD39` → `$99:87E4` (`BD3E`) | Cadet (T2-T5) | `80`, `D9`, `DA`, `CC`/`CD`, `D2 18 50 00`, `A4`, `A2`, `A1`, `5F`, `71`, `1B`/`1F`, `02`/`03`, struck `C331` | One random step or turn; pick a spot 64-72 px beside Ark (random side), line-move there (`CC 00 p 01 20`), face Ark, wait 16; Ark facing it in the box: cast (sound `$2E`, 3 poses, sound `$2C`, spell child `A4 $97:C2DD/C2F9/C315`). A branch with `71`/text spawns children (guess: the Cadet's message gag) |
| `$90:9EEA` → `$90:A201` (`9EEF`) | Three Cadets (`$118`) | `05`, `36`, `02`/`03`, `BD` | Wait for flag 1, appear, then the Cadet script `$97:BD41` |
| `$97:C345` → `$99:8DFB` (`C34A`) | Guardner (ghost) | `BC`, `59 1E`, `D2 60 A0 7F`, `A1`, `A2`, `E8`, `EB`, `71`, `58`, `BE`, `15`, `1B`/`1F` | Hidden until Ark is in a 96×160 box; appears; a group child (`E8 $97:C40F`) pulls Ark toward it (writes Ark's `7F:0018/001A` by ±1 a frame) and fires; on contact: text, then `COP 15` (guess: sends Ark back) |
| `$97:C688` → `$99:9134` (`C68D`) | High Cadet (`$113`) | `05`, `07`, `E8`, `EB`, `BC`, `C0`, `06 $85:E27B` | Controller: 3 copies (`E8`, life `$6000`). When a copy reports `$26 = 3` (guess: the real one was hit) the controller counts down 3 lives and deletes the group (`EB`); at 0: flag 2, death script `$85:E27B` |
| `$97:CB03` → `$99:95B4` (`CB08`) | tower 4 show boss (`$11B`) | `5A`, `82`/`87`, `A1`, `A4`, `E8`, `BB`, `B0`, text | Not traced in detail (ball chain) |
| `$93:D871` → `$99:9B59` (`D876`) | Shadowkeeper (`$123`) | `00`/`01`, `02`/`03`/`04`, `22`, `5A`, `63`, `76`, `A0`, `A2`, `AF`, `B3`, `B8`, `B9`, `BA`/`BB`, `E7`, `EA`, death callback `1012 → DB09` | Not traced in detail |

## 9. What the runtime needs, in order

For the blob in `$101` (items 1-4 make it walk as natively):

1. A global RNG at `$0408` with the `$86:8236` step, `COP 25`, and the
   native dispatch idioms of §4 (`LDA $0408`, `AND`/`BIT` masks, `DEC`/`BEQ`
   chains, `JMP`, the `E+$24`/`E+$26` scratch words). Today the blob
   freezes at `$97:B564`.
2. `COP D6`, `D7`, `D3`, `D4`. Today the derived length skips them, so the
   near branch never runs and `D7`/`D3`/`D4` would run into their tables.
3. `COP 59` and the visibility bit from the camera rectangle (§2). Today
   it goes on, so off-screen blobs move and draw from the RNG early.
4. Wall collision for `E+$04 & $0004` movers (§6). Today streams ignore
   walls.
5. Wake dispatch for bits 12/14 with the knockback and death scripts of §5,
   including the resume rule (interrupted wait ends, mirror cleared).
   [Combat](combat.md) has the hit scan and damage.

Then, for the other tower 1-2 enemies: `COP 82`, `86`, `B8`, `DA`, `D2`,
`D9` (real profile), callbacks `7F:1000`-`1016` and bits 1-11, `CC`/`CD`,
`A4`/`A1`/`E6` spawns with `COP A7` on leaving the screen, `B0 FF`. Bosses
and the Guardner add `04`, `58`, `5A`, `5F`, `63`, `71`, `97`, `A0`, `AF`,
`B9`, `BE`, `E4`, `E7`, `E8`, `EA`, `EB`, `15`.

## Open questions

- The meaning of `E+$04` bits 4-5 (`$0030`) and `E+$06` bit 14 on bullets.
- Which tile classes raise wake bits 6-8; the collision box offsets
  `7F:0028`/`002C` per descriptor.
- `COP CC` operand d (`7F:200B`).
- Who raises wake bits 1-4.
- The boss scripts (show, Shadowkeeper) and the Cadet's text branch.
- EU timing was not measured; the handlers are the same bytes.
