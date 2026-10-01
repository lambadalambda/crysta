# Underworld (Chapter 1) inventory

Planning survey for making all of Chapter 1 playable: the five towers, their
enemies, the two small continents, the elder's "Hole" and the chapter change.
JP addresses (HiROM) unless marked. Sources: exit lists (`$81:8000 + 2*map`),
spawn lists (`$82:8000`, then `$83:8000`, `+ 2*map`; format in
`crates/assets/src/maps/actors.rs`), a byte scan of the scripts for `COP 07`
(set flag), `08` (branch on flag), `48` (delete if flag), `05` (wait for flag),
`14` (transfer), and the longplay video (`local/Terranigma (1995) ...webm`,
times are video minute:second). "Guess" marks a reading that is not verified
in the emulator. Python helpers used: `rom.py`, `graph.py`, `spawn.py`,
`agg.py`, `permap.py`, `copscan.py` in the job folder `tmp/uw/` (not in the
repo).

## 1. Map graph

Exit record: `x y w h dest(16) mode sel rawX(16) rawY(16)` (cells of 16 px).
Destination bit 15 = conditional list at `$81:dest` (10-byte entries
`flag dest mode sel x y`, end `FFFF`). Selector `$0D` = stairs up (arrive
facing up), `$0E` = stairs down, `$55`/`$66` = door south/north walk-in,
`$62` = tower door, `$77`/`$88` = west/east (guess, from the edges).

World map `$03` (`$81:8CC1`): Crysta `$0A`; towers `$100` (13,49),
`$107` (5,36), `$10E` (30,8), `$115` (46,11), `$11C` (55,41), all sel `$66`,
raw (248,992); `$12A` (41,43) raw (1144,464); `$12B` (43,29) raw (120,944);
cell (41,36) conditional `$F046`: flag `$74` set -> `$127` sel `$66` raw
(248,448).

| Tower | Map | Exits (dest/sel) | Role |
|---|---|---|---|
| 1 | `$100` | `$03`/55, `$101`/62, `$103`/62, `$104`/62 | outside stairs, face gate |
| 1 | `$101` | `$100`/55, `$102`/0D | floor 1 |
| 1 | `$102` | `$101`/0E, `$103`/0D | floor 2 |
| 1 | `$103` | `$102`/0E, `$100`/15 (outer ledge) | floor 3 |
| 1 | `$104` | `$100`/15 (outer ledge), `$105`/0D | floor 4 |
| 1 | `$105` | `$104`/0E | top: guardian, Hiball fight, door |
| all | `$106` | none (script `COP 14` in, script out) | light-pillar room before the flyover |
| 2 | `$107` | `$03`/55, `$108`/62 | outside |
| 2 | `$108` | `$107`/55, `$109`/0D x2 | floor 1 (jewel statues) |
| 2 | `$109` | `$108`/0E x2, `$10A`/0D | floor 2 (floor switches) |
| 2 | `$10A` | `$109`/0E, `$10B`/0D | floor 3 (4 switches) |
| 2 | `$10B` | `$10A`/0E, `$10C`/0D | floor 4 (statue room, 12 Hiballs) |
| 2 | `$10C` | `$10B`/0E | top |
| 3 | `$10E` | `$03`/55, `$10F`/62 | outside |
| 3 | `$10F` | `$10E`/55, `$110`/0D, `$114`/0E, fall -> `$114` (flag `$1F`, mode 4) | floor 1 |
| 3 | `$110` | `$10F`/0E, `$111`/0D | floor 2 (moving blocks, bridges) |
| 3 | `$111` | `$110`/0E, `$112`/66 | floor 3 |
| 3 | `$112` | `$111`/55, `$113`/66 | floor 4 (8 ball spawner, 2 bridges) |
| 3 | `$113` | `$112`/55 | top: High Cadet |
| 3 | `$114` | `$10F`/0D | basement under the hole (chest 8) |
| 4 | `$115` | `$03`/55, `$118`/62 | outside |
| 4 | `$116` | `$117`/0D x2 | floor 1 (under the holes) |
| 4 | `$117` | `$116`/0E x2, `$118`/0D x2, 4 holes -> `$116` (flag `$1F`) | floor 2 |
| 4 | `$118` | `$117`/0E x2, `$119`/0D x2, `$115`/55, hole -> `$117` | entry floor (Three Cadets) |
| 4 | `$119` | `$118`/0E x2, `$11A`/0D, fall -> `$118` | floor 3 (tightrope) |
| 4 | `$11A` | `$119`/0E; script `COP 14` -> `$11B` | top |
| 4 | `$11B` | script `COP 14` -> `$11A` | boss arena ("the show") |
| 5 | `$11C` | `$03`/55, `$11D`/62 (+ shares `$11D`'s two records: the list has no `$FF` between) | outside |
| 5 | `$11D` | `$11C`/55, `$11E`/0D | orb guardian (needs Elle's cape) |
| 5 | `$11E` | `$11D`/0E, `$11F`/0D | Hiball-like pair |
| 5 | `$11F` | `$11E`/0E, `$120`/0D | Cadets |
| 5 | `$120` | `$11F`/0E, `$121`/0D | Guardners |
| 5 | `$121` | `$120`/0E, `$11C`/16, `$122`/88 | junction |
| 5 | `$122` | `$121`/77, `$123`/66 | guardian "final challenge" |
| 5 | `$123` | `$122`/55 | Shadowkeeper |
| - | `$12A` | `$03`/55 | Mu (boulders); script out to `$03` |
| - | `$12B` | `$03`/55 | Polynesia (knights, water); script out to `$03` |
| - | `$127` | `$03` x4 (edges), `$201`/00 (7x2 at (15,15)) | the Hole; `$201` is Chapter 2 |

No list exists for `$106`/`$10D`/`$11B`/`$124`/`$126` in `$81` (null), except
that `$106` and `$11B` have spawn lists. Count: 38 maps (6+1 shared, 6, 7, 7,
8, plus `$12A`, `$12B`, `$127`).

Script transfers (`COP 14 map mode sel x y`):
- Tower tops (`$90:9438`, `9A1C`, `9E21`, `9FED`, `A3DE`) -> `$106` sel `$66`
  raw (120,168).
- After the flyover, `$87:F7A4..F7E8` (picked by `COP 0A $3C..$40`): set
  `$101`/`$103`/`$105`/`$107`/`$109`, then `$03` mode 1 sel `$10` at
  T1 (208,816), T2 (80,608), T3 (480,160), T4 (736,208), T5 (880,688).
- `$11A` -> `$11B` (`$90:A000`, sel `$10`, raw (128,112)); `$11B` -> `$11A`
  (`$90:A058`, sel `$20`, raw (376,552)) after flag `$01`, sets `$11D`.
- `$12A`/`$12B` door `$90:A454`: sets `$120`+`$40D` -> `$03` sel `$55`
  (688,480) (the `$12B` return point), or `$11F`+`$40E` -> `$03` (656,704)
  (the `$12A` return point). `$40D`/`$40E` are world-map patch flags
  (guess: they raise the small continent on `$03`).
- Elder `$88:8FBE` (map `$0B`): after flag `$109` (tower 5 done) sets `$74`
  (opens the Hole) and `$402` (`$D8:0000` world patch).

## 2. Spawns per map

`FD $84:A129` (player start) is in every list and is left out.
`FB $87:98BF`/`$87:98E8` = palette/char animation actors (params `$09..$1E`).
`FB $97:B4AA` = the tower-outside BG2 darkening HDMA (`docs/tower-entry.md`).
Classes: **E** enemy, **E\*** ambush enemy ("Guardner appeared!", 16-byte
record `C0`), **B** boss/scripted fight, **G** guardian NPC, **O** object,
**C** controller (`FE`/`FD`, no body).

| Map | Spawns |
|---|---|
| `$100`,`$107`,`$10E`,`$115`,`$11C` | G face gate (`$90:8BF0` T1; `$90:8F7E` param 1/3/5/7 T2-5), 2 O statues `$90:8F09`/`8F15`, C intro `$90:8F23` (T1 only) |
| `$101` | C `$90:9059` (flags `$280-283`), E Hiball `$97:B555` x3 |
| `$102` | E `$97:B98D` x5 (one `FF`: drop script `$90:90B0` p1), E `$97:B555` x4, chest 1 |
| `$103` | E knight `$95:EFAB` x2, E `$97:B555` x4 (one `FF`: `$90:91CF` p2), chest 2 |
| `$104` | E knight `$95:EFAB` x3, E `$97:B555` x6, chest 3 |
| `$105` | G `$90:9232` (hooded, del. `$114`), O door `$90:93DB` (del. `$101`, sets `$114`, -> `$106`), E Hiball `$90:938B` x4 ("Four Hiballs appeared!") |
| `$106` | C `$90:8AB9`, O `$90:8AD9` (Ark on the light pedestal, pose `$FA:7D1B`) |
| `$108` | E `$97:B98D` x3 (one `FF` `$90:94E6` p3), E `$97:B555` x7, O jewel statues `$90:9585`/`95F4` (flags `$284`/`$285`), chest 4 |
| `$109` | E `$97:B98D` x2 (one `FF` `$90:9530` p4), E `$97:B555` x2, E Cadet `$97:BD39` x2, C switches `$90:9716`/`9740` (flags `$286`/`$287`), C `$90:985D` (waits `$286`+`$287`, sets `$28C`/`$28D`), chest 5 |
| `$10A` | C switches `$90:976A..97E8` x4 (`$288-28B`), C `$90:98CF` (waits all 4, sets `$28E`/`$28F`), E `$97:B635` x4, E `$97:B98D` x2, E `$97:BD39` x2, O `$90:9663` x2 |
| `$10B` | O `$90:9672`/`96DC`, C `$90:991E` (waits `$02`,`$03`, sets `$290`/`$291`), E `$90:938B` x12, chest 6 |
| `$10C` | G `$90:9965` (del. `$117`), O door `$90:99DF` (del. `$103`), O `$90:9812` (hittable, `COP 65`) |
| `$10F` | E `$97:B635` x4, E `$97:BBC2` x3, O traps `$90:9C3C/9C46/9C6D` x8 (columns), O `$90:9C94` x8 (row) |
| `$110` | E `$97:B635` x5, E\* Guardner x1, C `$90:FBB3` x8 (tile patch, bridge cells), O `$90:9A29..9A50` (4 blocks), O `$90:FC2D` x6 / `$90:FC12` x4 (wall launchers, guess) |
| `$111` | C `$90:9AC6` (flag `$111`), E Cadet x3, E `$97:BBC2` x2, E\* x2, O `$90:FC5B`/`FC4A` x2 each |
| `$112` | C `$90:9BA4` (sets `$112`), C `$90:FBB3` x2, E `$97:B635` x4, E `$90:9BF7` x8 (ball wave), E `$97:BBC2` x2, E Cadet x2, chest 7 |
| `$113` | C `$90:9CE0` (sets `$11A`, music), O door `$90:9DEC` (del. `$105`, waits `$11A`), B High Cadet `$97:C688` |
| `$114` | C `$90:FA4E` (fall landing, guess), chest 8 |
| `$116` | C `$90:FA4E` x4, E\* x2, E `$97:BBC2` x3, E `$97:B847` x10 |
| `$117` | C `$90:FA4E`, E\* x3, E `$97:B847` x2, E `$97:BBC2` x2, E Cadet x5, chest 9 |
| `$118` | C `$90:FA4E`, G `$90:9E2E` (del. `$11C`), B Three Cadets `$90:9EEA` x3, C `$90:9F33` (sets `$11C`), E Cadet x6 (one `FF` `$90:A065` p5) |
| `$119` | E `$97:B847` x2, E\* x2, E `$97:BBC2` x4, E Cadet x1, chest 10 |
| `$11A` | E `$97:B847` x2, E\* x1, O `$97:B434`, O door `$90:9FB6` (del. `$107`, needs `$11D`), chest 11 |
| `$11B` | B show `$97:CB03` (ball chain), C `$90:A04B` (sets `$11D`, back to `$11A`) |
| `$11D` | G orb `$90:A0B5` (del. `$19B`, spawns 5 orb actors `$90:A2FB..A37D`) |
| `$11E` | E `$97:BBC2` x2 |
| `$11F` | E Cadet x2 |
| `$120` | E\* Guardner x2 |
| `$121` | animation actors only |
| `$122` | G `$93:D7BA` (del. `$1A6`, text, sets `$1A6`) |
| `$123` | C intro `$8F:8000` (del. `$109`), B Shadowkeeper `$93:D871`, C `$90:A3A9` (del. `$109`, -> `$106`) |
| `$127` | G elder `$90:8000` (desc `$83:ED7F`, house class) |
| `$12A` | `FA` on `$11E`/`$1AF`; O boulders `$90:AF61` x6 (desc `$82:ECA9`), O door `$90:A454`, 4 bodiless `$90:A66F..A722` (desc bank `$A7`, not parsed) |
| `$12B` | `FA` on `$11E`/`$1AF`; E knight `$95:EFA0` x2, E `$97:BBC2` x4, E `$97:B98D` x3, O door `$90:A454`, same 4 bodiless actors |

Chests: `00` record, script `$84:DD7E`, desc `$82:F9E0`, record byte 3 =
chest number 1..11; opened flag = `$900 + n` (`$84:DD83`). Contents table not
traced. Seen in the video: 30 gems (T2, 30:35), Crystal Thread (T4, 57:15),
Magirock x2 (24:45, 40:30; guess: from the `FF` drop scripts, which set
`$10B`/`$113`/`$116`/`$118`/`$11B` and show text).

### Enemy and boss types

Descriptor byte 4 indexes `$8D:BDFA` (25-byte stat records); `+8` = level
(guess), `+9` word = HP (`$80:F968` copies it to `$7F:102A`; damage at
`$85:E00A`). Every descriptor here has `d3 = $04` (mode `$0004`). Palette
form `d8`: `$91` (table palette, 4-byte field) for the small enemies,
`$9E` knight, `$40` objects and guardians (as tower 1's statues), `$00`
(plain pointer) for the hooded guardian and Shadowkeeper. The elder in
`$127` is class 0, `d8 = $81` (house form).

| Pose packet | Name (video) | Stats idx: level/HP | Scripts | Maps |
|---|---|---|---|---|
| `$CB:627D` | Hiball (red ball) | `$01`: 1/4; `$16`: 3/11 | `$97:B555`, `B635`, `B847`, `$90:938B`, `9BF7`, boss `$97:CB03` | all towers |
| `$C9:62DE` | not named (yellow flyer, guess) | `$02`: 2/9 | `$97:B98D`, `$97:BBC2` | T1, T2, T3, T4, T5, `$12B` |
| `$C4:7378` | armored knight (name not seen) | `$0E`: 4/23 | `$95:EFAB`, `$95:EFA0` | T1, `$12B` |
| `$C9:1DB4` | Cadet / High Cadet (wizard) | `$03`: 4/20 | `$97:BD39`, `$90:9812`, `$97:C688`, `$90:9EEA` | T2-T5 |
| `$CA:715A` | Guardner (ghost) | `$04`: 4/9 | `$97:C345` (`C0` record, param 1-3) | T3-T5 |
| `$F5:5A90` | Shadowkeeper | `$3C`: 7/58 | `$93:D871` | `$123` |
| `$F7:4B29` | hooded guardian | (`$0A`, NPC) | `$90:9232`, `9965`, `9E2E`, `A0B5`, `$93:D7BA` | tops, `$118`, `$11D`, `$122` |
| `$F7:0000` | face gate + knight statues | - | `$90:8BF0`, `8F7E`, `8F09`, `8F15` | tower outsides |
| `$F9:0000` | top-floor door to the light | - | `$90:93DB`, `99DF`, `9DEC`, `9FB6`, `A454` | tops, `$12A`/`$12B` |
| `$B0:FAF2` | statues, blocks, traps | `$05` (HP 0) | `$90:9585..FC5B`, `$97:B434` | T2, T3 |
| `$CD:133A` | chest; boulder (`$12A`) | - / `$12`: 12/27 | `$84:DD7E`; `$90:AF61` | all |

Totals: 5 small enemy graphics in 7 stat variants and 14 enemy scripts;
5 scripted fights (Four Hiballs, High Cadet, Three Cadets, the T4 "show",
Shadowkeeper), of which 2 are true bosses (T4 show, Shadowkeeper).

## 3. Story flow

| Step | Flags | Video |
|---|---|---|
| Elder sends Ark out ("Five towers await you") | `$26`, `$28`, `$21`, `$3B`, `$296` (elder `$88:8E4B` branches) | 17:00-18:15 |
| T1 entry: pan, "So this is what gramps was calling a tower." | `$100` | 19:00 |
| T1 face gate: "Reach the uppermost floor and gain the power to control the world." | `$115` (`$90:8C75`) | 19:20-19:55 |
| T1 inside; level 2 at 22:55, Magirock 24:45, outer ledge 24:10 | | 20:05-26:30 |
| T1 top: guardian, "Four Hiballs appeared!", door | `$114` | 26:35-27:45 |
| Light pillar (`$106`), earth, spiral, snow flyover, map "Eurasia" | `$101` | 27:50-29:25 |
| Crysta souls regain form (cut-in), back on `$03` | | 29:30-29:45 |
| T2 entry (gate checks `$101`) | | 30:00 |
| T2: jewel statues, gold switches, statue room | `$284-291` | 30:10-36:10 |
| T2 top: guardian "Huhuhuhu", light, desert flyover, "South America" | `$117`, `$103` | 36:15-38:35 |
| T3 entry (checks `$103`) | | 39:20 |
| T3: moving blocks, light beams, wall arrows, holes, Guardners, Cadets | `$111`, `$112` | 39:30-45:45 |
| T3 top: "High Cadet appeared!", light, ocean, moon rise, "Africa" | `$11A`, `$105` | 45:50-48:45 |
| T4 entry (checks `$105`) | | 49:40 |
| T4 `$118`: hooded guardian, "Three Cadets" | `$11C` | 49:50-50:15 |
| T4: holes ("take bold chances"), burners ("Ark's toasted!"), tightrope, Crystal Thread | `$1F` (fall) | 50:20-58:05 |
| T4 top -> `$11B` "I have prepared a most entertaining show", ball chain | `$11D` | 58:10-60:15 |
| Light, mountain flyover, "N. America" | `$107` | 60:20-62:20 |
| Crysta: Elle weaves the cape, night talk, sleep at the elder's | (Crysta flags, not traced) | 63:00-68:40 |
| T5 entry (checks `$107`) | | 69:50 |
| T5 `$11D`: "Elle's Cape reflected the orb!" | `$19B` | 70:00-70:15 |
| T5 floors; `$122` "This is your final challenge" | `$1A6` | 70:20-71:05 |
| `$123` Shadowkeeper fight, "Defeated Shadowkeeper!!" | | 71:25-74:50 |
| "Your real journey begins today. You shall return to the Elder." Light, Ayers-rock flyover, "Australia" | `$109` | 74:55-77:05 |
| `$12B` Polynesia (knights, door), "Polynesia was resurrected" | `$120`, `$40D` | 77:50-78:55 |
| `$12A` Mu (boulders, holes), "Mu was resurrected" | `$11F`, `$40E` | 79:15-79:45 |
| Elder: "only the land is revived... we shall go outside" | `$74`, `$402` | 80:40-81:05 |
| `$127` the Hole: story, choice "unfinished business? Yes/No" (Yes: back to Crysta, Elle's farewell 84:00) | | 81:35-86:35 |
| Vortex, "Chapter 2 Resurrection of the World", `$201` | | 86:40-87:15 |

Gate rule (towers 2-5, `$90:8F7E`): the door is sealed by tile patches
(`COP 3F`) unless flag `$100 + param` (the previous tower's done flag) is
set. It also compares `$0954` with 3 and tests flag `$196` (meaning not
known). The box rooms `$41-$44` are not part of this chapter; the pause menu
(the box with doors) is used in it (20:15, 44:05, 63:45, 68:40).

## 4. Mechanics seen in the towers

| Mechanic | Where | Evidence |
|---|---|---|
| Spear attack: thrust; dash thrust; jump; damage numbers over the target | everywhere | 20:40, 23:20, 43:10 |
| HUD: level, equipped item, LIFE cur/max, gem count | everywhere | 20:05 "28/28", gems 0 -> 570 |
| Level up box "Ark level n! Life up x! Strength/Defense/Luck up 1!" (levels 2-7) | after kills | 22:55, 27:40, 40:10, 51:00, 57:00, 78:05 |
| HP regeneration over time (Crystal Spear, below crystal blue) | everywhere | 26:00 3/33 -> 6/33 |
| Enemy drops: gems, purple orbs (guess: heal/Magirock), enemy name banners | everywhere | 21:10, 26:55 |
| Chests (11) and items (gems, Magirock, Crystal Thread); menu use (S.Bulb, Life Potion raises max life 59 -> 64) | | 30:35, 44:15, 57:15, 63:50 |
| Hint texts on pickups/steps | T1-T4 | 21:55, 23:45, 30:40, 31:40, 51:15 |
| Outer ledges (exit outside the tower and back) | T1 | 24:10 |
| Floor switches and jewel statues, doors opened by flag sets | T2 | 31:40-35:50 |
| Moving blocks, retracting bridges (`COP 44` tile patches), wall launchers, light-beam traps | T3 | 40:00-42:10 |
| Holes: fall to the floor below (flag `$1F`, mode-4 exit) | T3, T4 | `$10F`, `$117`-`$119` |
| Fire burners, spike walls, tightrope | T4 | 52:55, 54:05 |
| Ambush ghosts (Guardner appears near Ark) | T3-T5 | 42:05, 55:30 |
| Scripted fights with a banner, room locks until done | tops, `$118`, `$11B`, `$123` | |
| Orb reflection by Elle's cape (item check) | `$11D` | 70:10 |
| Tower outside: pan, title OBJ, gate guardian | all 5 | `docs/tower-entry.md` |
| Resurrection: light pillar room, earth/spiral/Mode 7 flyover, parchment map, souls cut-in | after each top | 27:50, 36:45, 46:50, 60:20, 75:10 |
| Chapter title and move to `$201` | `$127` | 86:55 |

## Open

- Enemy behaviours per script (14 scripts) and the stat record fields past HP.
- Chest contents table; the `FF` drop scripts' items and texts.
- `$0954` and flag `$196` in the gate test.
- The `$106` cutscene chain (flyover, map, souls) and how `COP 0A $3C..$40` picks the tower.
- Exact gating of `$12A`/`$12B` (the `FA` conditions on `$11E`/`$1AF`).
- Crysta flags between T4 and T5 (Elle, Crystal Thread, the cape).
