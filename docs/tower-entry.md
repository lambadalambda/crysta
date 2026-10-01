# Tower 1 (`$100`): entry, intro, exit

Research for [enter-the-towers](../meta/issues/enter-the-towers.md). JP
addresses unless marked. Frames are oracle frame counts (`frames` after the
frame ran). "Guess" marks an unverified reading.

Native runs: JP from
`local/pandora-tower-discovery/departure/journey/tower-align-east-rest.state`
(Ark on `$03` at (216,880), frame 60376) with Up 50 frames. EU from the
`local/mode7` EU `arr0` state (`$03` arrival) plus the same legs as
`tower-route.jsonl` lines 565-575, which also end at (216,880) (frame 74793).
Brightness is INIDISP read from the save state (offset 207677).

## Controller `$90:8F28` (EU `$90:9104`, same bytes)

Scene record `$82:88C1` (both ROMs; table `$82:8000 + 2*map`) has three
entries: `$84:A129`, the guardian `$90:8BF0` (EU `$90:8D89`) and
`$90:8F09` (EU `$90:90E5`), whose script reaches `8F28`.

```
02 48 00 81          COP 48 $8100   delete self if flag $100 is set (first visit only)
02 07 00 81          COP 07 $8100   set flag $100
02 2A F0 FF          COP 2A $FFF0   lock every pad button ($045E)
02 DD 00 00 00 D0 00 00 80
02 DE 80
02 1B 4D 8F          COP 1B $8F4D   (EU 1B 29 91 = $90:9129)
02 1F                COP 1F
02 29 F0 FF          COP 29 $FFF0   unlock
02 A7                COP A7         delete self
6B                   RTL
```

The COP table `$80:83B2` is the same in both ROMs: `DD` → `$80:B735`,
`DE` → `$80:B7D6`.

COP DD (7 operand bytes here) sets up a camera move. It does not wait.
- byte 0 = 0: spawn a new camera actor (`$80:BC7C`, script `$87:C960`,
  which stores itself in `$0DEC`). Non-zero: reuse the existing one
  (`$80:B767`) and wait until its `+$26` is set.
- byte 1 = 0: offsets are relative to the focus `$0966/$0968` (measured:
  player (x, y-8)). Non-zero: absolute.
- bytes 2-3: signed start dx, dy, ×16 px (here 0, `$D0` = -768 px).
- bytes 4-5: signed target dx, dy, ×16 px (here 0, 0).
- byte 6: speed index (`×4 + [actor $7F:0022]`, pointer into `$7F:0010`).
  `$80` = 2 px a frame (measured).
- It saves the focus in `$7F:2004/2006`, then RTI, so `COP DE` runs in the
  same frame.

COP DE (1 operand byte, speed index `$80`). First pass (`+$24` = 0): it sets
the camera actor's target to the saved focus, sets the speed, increments
`+$24`, moves the script pointer back to the COP and yields. Each later frame
it yields until the actor sets `+$26` (arrived), then skips its operand and
goes on. So the camera starts 768 px above Ark (camera top 138, clamped
range 0..800) and scrolls down to him at 2 px a frame: 384 frames.

COP 1B `$90:8F4D` is Ark's two-page monologue (one request, two
pages):
- JP: 「アーク： これが じいさんの 言っていた 塔か・・・」 /
  「なんだか せすじが ぞくぞく するぜ。」
- EU: "Ark: So this is what gramps was calling a tower." /
  "Heh, I'm getting excited here."

COP 1F saves `$045E`, sets it to 0 while the window is open, and restores it
when the request ends. COP 29 then clears the lock in the same frame.

## Timeline (C = first frame of the controller)

| Event | JP | EU | Note |
|---|---|---|---|
| Exit selected on `$03` (y 816→814) | 60410 | 74827 | `$097C` = `$8000` |
| Fade out 14..0 | 60412-60426 | 74829-74843 | one step a frame |
| Map becomes `$100` (load, blank) | **60427** | **74844** | |
| Player placed at (256,1024), script `$84:A12E` | 60497 | 74899 | JP +70, EU +55 |
| C: controller runs, walk-in starts (`$84:BBA9`) | 60507 | 74907 | |
| Camera top 800→138 | C (60507) | C+1 | |
| Camera scrolls 140→800, 2 px/frame | 60511-60841 | 74911-75241 | then held at clamp |
| Fade in 1..15 | 60513-60526 | 74913-74926 | about one step a frame |
| Ark at (256,1007), walk-in ends | 60526 | 74926 | |
| Player script ordinary `$84:A258` (pad still locked) | 60528 | 74928 | |
| Title OBJ 「試練の塔 1」 / "Tower 1" | 60529-60667 | 74930-75072 | 5 / 6 large OBJ fly in and out |
| COP DE done; COP 1B, 1F | 60894 (C+387) | 75294 (C+387) | |
| Text ends, COP 29 unlocks (A mashed from C+386) | **60945** | **75380** | |

So, with A mashed, map load to pad unlock is **518 frames JP**
(60427→60945) and **536 frames EU** (74844→75380). The text part depends
on A: with A at 60947 and 61148 (the qualified route) JP unlocks at 61150
(trace: `$90:8F46` runs in frame 61150). Without the text it would be
C+387: JP 467, EU 450 frames after the map change.

On later visits `COP 48` deletes the controller, so there is no pan or text
(guess: walk-in and fade only).

## Arrival with selector `$66` (question 3)

Raw (248,992) plus the usual (8,16) is (256,1008). Native placement is
(256,1024) (selector 6 offset, guess (0,+16)). The walk-in is player script
`$84:BBA9`, `$097C` = `$8000`, pad locked. Per frame (JP, EU is the same at
EU C):

| Frame | y |
|---|---|
| 60497-60506 | 1024 (placed, script `$84:A12E`) |
| 60507-60509 | 1023 |
| 60510-60511 | 1022 |
| 60512 … 60526 | 1021 … 1007, one px a frame |
| 60527 | 1007, script `$84:A318` |
| 60528 on | 1007, ordinary script `$84:A258` |

x stays 256.

## Exit back to `$03` (question 2)

Exit list `$100` = `$81:C2E2` (table `$81:8000 + 2*map`, both ROMs):

| Record | Cells (x,y) w×h | Dest | Sel | Raw |
|---|---|---|---|---|
| `$81:C2E2` | (0,63) 32×2 | `$03` | `$55` | (208,816) |
| `$81:C2EE` | (15,54) 2×2 | `$101` tower door | `$62` | (120,608) |
| `$81:C2FA` | (14,34) 1×3 | `$103` | `$62` | (200,704) |
| `$81:C306` | (15,26) 2×3 | `$104` | `$62` | (200,656) |

`$103` and `$104` exit back to `$100` at raw (224,608) and (240,480)
(selector `$15`), so these are higher doors on the tower outside (guess:
ledges you reach from inside). `$101` comes back at raw (248,896), `$55`.

Leaving natively (JP from the unlocked state at 61448, Down held):
- Ordinary walking at 1.5 px a frame: 1007, 1008, 1010, 1011 … 1022, 1023
  (61461), 1025 (61462). The stairs are walls at x 216 and 296 (y 1007).
- No special edge rule. At 61462 (y = 1025, origin y-16 = 1009, row 63)
  the exit is taken: pending map `$047C` = 3. The scan sees the position
  after this frame's movement (y 1023 did not match).
- Walk-out: script `$84:B975` (the selector-5 doorway controller),
  `$097C` = `$8000`, one px a frame down, 1026 … 1042 (61463-61479), whatever
  the pad does. Fade 15→0 over 61465-61479.
- 61480: map `$03`. Ark placed at (216,816) at 61554 (raw + (8,16) +
  selector 5 (0,-16)), script `$84:DEE5`, then `$84:E337` walks him down:
  817 (61557-61571), 818 (61572-61573), then one px a frame to 832 at 61587.
  Fade in 2..15 over 61575-61588. Control (`$84:DF12`) at 61588 at
  **(216,832)**.
- EU: the same positions; exit at 75979, map `$03` at 75997, placed 76060,
  (216,832) at 76089, control at 76090. EU world-map scripts are `$84:DEAA`,
  `$84:E2FC`, `$84:DED7` (JP − `$3B`).

## BG2 of `$100` (question 4)

TM = `$17` (BG1, BG2, BG3, OBJ), mode 1. BG2: map `$3C00`, chars `$0000`,
one 32×32 screen, scroll (0,0) at every sampled frame. It is a fixed backdrop:
night sky with clouds over a dark forest (rows 0-27; rows 28-31 are empty).
It does not scroll with the camera. It shows wherever BG1 is transparent:
about 70% of the screen during the pan (camera 270) and 28% at the stairs
(camera 800), part of which the statue OBJ cover. CGADSUB = `$82`
(subtract, on BG2 only). The top lines are black and the sky gets lighter
down to about line 100 (guess: a per-line HDMA fixed colour that is
subtracted from BG2). So BG2 is needed for the pan; at the stairs it is
mostly hidden.

## Open

- Source of the BG2 tiles and of the per-line darkening (HDMA channel and
  table).
- The writer of the title OBJ (guess: the map load or bank-`$82` scene).
- The speed table behind index `$80` (only 2 px a frame is measured).
- Repeat visits (flag `$100` set) are not run natively.
