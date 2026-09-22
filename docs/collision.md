# Movement collision: measured attribute semantics

Status: **an opt-in directional resolver candidate now matches bounded native
slope trajectories in all four directions; not a complete passability
specification.** The table at `$80:E85C` is decoded, but `COP CA` does not decide
ordinary walking. Production remains at 19/24 maps until the remaining
qualification gates pass. See [the directional issue](../meta/issues/qualify-crysta-directional-collision.md).

## The attribute field, and the bit that is not attribute

The qualified loader initialization recorded in [static maps](static-maps.md)
writes each runtime cell as

```text
initialized_word = index | ((attributes[index] & $7F) << 9)
```

so at initialization the attribute is bits 9..15. Two caveats matter, and both
come from this repository's own prior work:

- The loader's `& $7F` discards **bit 7 of the source table byte**, so at most
  seven bits are ever recoverable, in any map.
- [Static maps](static-maps.md) records **word bit 15 being set later during
  play** ("Bit 15 set later"), with its writer and meaning unclaimed. That bit
  is attribute bit 6. On a runtime layer it cannot be distinguished from a
  source attribute of `n | $40`.

So `MapCell::attribute()` inverts *initialization*, and `base_attribute()`
(`(raw >> 9) & $3F`) is what a runtime layer actually supports. The difference
is not academic: in the measured map exactly two cells have the dynamic bit
set, `$9ce8` and `$9845`. Under a seven-bit reading they become "attributes"
76 and 78 — apparently uncovered values — when they are really 14 and 12, one
of them the most common wall word in the room. Keying passability on the
six-bit base keeps them classified.

**The community overlay code is an equivalent encoding, not a rival one.**
`(raw >> 8) & $FE` is exactly `2 * attribute` for every 16-bit word: the index
reaches the upper byte only through bit 8, and the mask clears that bit. The
published labels were never the problem; what was missing is any mapping from
the code to admitted/refused movement. [The map format record](maps.md)
previously presented the mask itself as the open question.

## Measured semantics

Evidence is two independent input-only sweeps of map `$000F`, the fresh-start
house room, through `tools/collision-qualification/probe.rs`: one horizontal
serpentine and one vertical serpentine. Session discipline matches every
accepted route — one empty-SRAM Session, real buttons from boot, no warp, no
memory patch, no save and no state restore. The player is walked to every
sample; positions are never assigned.

```text
3224 frames, 1075 distinct positions covering 66 cells,
17 distinct sustained contacts (23 total)
192 consistent offsets: dx in -8..7, dy in -12..-1
walkable: [0, 22] for 192/192 offsets
solid:    [12, 14] for 156/192 offsets
          [2, 12, 14] for 36/192 offsets
          UNRESOLVED (offset-dependent): [2]
```

That is the sweeps' own verdict, and it stands: serpentine walking alone does
not resolve attribute `2`. The doorway contact below does, by excluding the
offsets that made it ambiguous.

`derive.py` solves for a collision reference offset rather than assuming one,
keeping every offset where no attribute both stops a sustained press and is
stood on.

| Base attribute | Observed | Decoded as |
| --- | --- | --- |
| `0` | collision point stands on it, ~60 cells | `Walkable` |
| `22` | collision point stands on it, 3–8 cells | `Walkable` |
| `2` | **walked through**, carrying the player between maps | `Walkable` |
| `12` | stops a sustained press from Up, Down and Right | `Solid` |
| `14` | stops a sustained press from all four directions | `Solid` |
| `16` | refuses a sustained press from its only open side | `Solid` |
| `25` | solid under every consistent offset of the town sweeps | `Solid` |

That `12` and `14` each block from *opposing* directions rules out the obvious
rival reading, that they are one-way ledges or elevation edges.

### The doorway contact, and what it settled

Attribute `2` forms one-cell gaps in walls; map `$000F`'s grid shows `#2#`
twice along its row-12 wall. Walking the player into the gap at cell `(24,12)`
moved it from `y=192` to `y=336` **and changed the map from `$000F` to
`$0010`**, an edge the static exit graph independently predicts. So the
attribute admits movement. The transition belongs to the exit record rather
than the attribute: most attribute-`2` cells lie outside any exit rectangle.

That contact also resolved attribute `2`, though **not** in the way first
recorded here. The original claim was that the press was "refused at `x=402`
and admitted at `x=399`", narrowing `dx` to `-2..=0`. The frame trace refutes
it: at `x=399` with Down held, `y` does not change for eight frames while the
game slides `x` to `392`, and vertical motion only begins at `392`. The player
was never admitted at `399`; it was moved. That slide is the same doorway snap
recorded in
[the door-entry issue](../meta/issues/decode-door-entry-trigger.md).

What actually excludes `dx` in `5..=7` is occupancy: the player stands on
attribute-`2` cells around `(392, 198..225)`, while the `(379,192)` down-stall
only calls attribute `2` solid under `dx >= 5`. Re-running `derive.py` on these
inputs reports `dx` in `-2..=4`, not `-2..=0`.

Attribute `16` sits in a one-cell alcove whose only open side is below. A
sustained press from that side was refused across seven attempts of 50 frames.

### Coverage across the slice

**47,320 of 47,616 cells (99.4%)** across all 24 Crysta maps now resolve. What
remains is `5` (8 cells), `21` (48, in `$0012`–`$0019`), `29` (65, in 17 maps)
and the town's `6`, `7`, `8` (175 cells, map `$000A` only).

### The town, and what walking it needed

Reaching map `$000A` runs the qualified route that talks to the room B
resident, opens the gate and steps outside. Two serpentine sweeps there, 10,178
frames over 4,514 positions with 30 sustained contacts, derive `{0, 22}`
walkable and `{12, 14, 25}` solid under **all 112 consistent offsets** —
independently reproducing the house's partition in a different map, and adding
`25`. That attribute forms a band running the full height of the town, and the
player never crosses it.

The town also forced two corrections the house never exposed:

- **Actors stop players too.** A resident standing in front of Ark produces
  exactly the same sustained stall as a wall. `derive.py` now discards a
  contact with a live actor within 24px in the pressed direction. The slot
  table includes Ark's own shadow at the player's exact position, which must be
  ignored or it disqualifies every contact ever measured.
- **A layer read mid-transition is not the map.** The probe originally dumped
  on first sight of a new map id, which for `$000A` caught it before its
  attribute pass: 5,120 cells of attribute zero, every wall apparently
  walkable. It now waits for input to be admitted. The corrected runtime layer
  matches the static ROM decode exactly, which is a cross-check the earlier
  capture would have failed silently. `derive.py` reported
  `NO CONSISTENT OFFSET` on the bad data rather than fitting it.

### Threshold sensitivity

A press is treated as refused after `STALL_FRAMES = 20` frames without integer
movement. That is not arbitrary: the longest mid-run stationary stretch after
which movement *resumed* is **2 frames**, a 10x margin, and the partition above
is unchanged for any threshold in `8..=40`. Attribute `2` leaves the solid set
only at `>= 60`, where the contact count collapses to 15.

## What is not established

- **The reference is settled, and it is not the point these sweeps measure.**
  `$80:940D` computes the collision sample as `(x - 8, y - 16)` — the same
  corner the exit probe uses — and `room-core` reads a 16x16 box from it. So
  the player occupies a box, not a point, and the `dx` in `-2..=4` / `dy` in
  `-12..=-8` this derivation reports are simply the range a single-point model
  cannot distinguish; `dx = -8` is not even inside it. Treat these ranges as
  what the sweeps constrain, not as the game's model. The attribute partition
  they establish is unaffected, because it does not depend on which offset
  inside the range is correct. The serpentine stalls all halt at
  `x % 16 == 8`, which is why they carried no sub-cell information. The search
  is bounded a priori to a sprite-extent window
  (`dx` in `-16..15`, `dy` in `-24..7`) to exclude mirror solutions; on this
  data that window is inert, and widening it to `+/-32` changes nothing.
- **Six attributes remain undecoded**, 296 cells.
  `qualified_passability()` returns `None` for them: an uncovered attribute is
  unknown, not walkable. The exterior's four are the ones that matter for
  walking the town, and reaching map `$000A` needs the `$0026` progression
  gate.
- **There is no single binary cell-admission predicate established here.**
  The traced `COP CA` branch and its table are described below. The actual
  `$80:D107` motion resolver also reads direction, the sample pair, old-edge
  slope types and controller flags. Table zero/nonzero cannot replace it.
- **The layer is snapshotted once per map**, at entry. Given that bit 15 is
  written during play, in-run layer drift is not excluded by this method.
- **The new probe-table decoder is not wired into ordinary movement.**
  `room-core` uses its qualified O/S/P responses and bounded aliases for 5/25/29;
  the Crysta builder installs those aliases. Unknown types still fail closed.

## Reproduce

Requires the locally owned headerless Japanese ROM, SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
No ROM, capture or layer dump is committed.

```sh
sh tools/collision-qualification/build.sh
P=local/collision-qualification/probe/target/release/collision-probe
ROM='local/Tenchi Souzou (Japan).sfc'
D=local/collision-qualification

python3 - <<'ROUTE' > /tmp/horizontal.jsonl
import json
c = []
for i in range(14):
    d = 'Right' if i % 2 == 0 else 'Left'
    c.append({'label': f'row{i:02d}-{d.lower()}', 'frames': 110, 'buttons': [d]})
    c.append({'label': f'row{i:02d}-down', 'frames': 14, 'buttons': ['Down']})
c.append({'finish': True})
print('\n'.join(json.dumps(x) for x in c))
ROUTE

python3 - <<'ROUTE' > /tmp/vertical.jsonl
import json
c = []
for i in range(12):
    d = 'Down' if i % 2 == 0 else 'Up'
    c.append({'label': f'col{i:02d}-{d.lower()}', 'frames': 110, 'buttons': [d]})
    c.append({'label': f'col{i:02d}-right', 'frames': 14, 'buttons': ['Right']})
c.append({'finish': True})
print('\n'.join(json.dumps(x) for x in c))
ROUTE

"$P" "$ROM" "$D/horizontal" < /tmp/horizontal.jsonl > "$D/horizontal.jsonl"
"$P" "$ROM" "$D/vertical"   < /tmp/vertical.jsonl   > "$D/vertical.jsonl"
python3 tools/collision-qualification/derive.py \
  "$D/horizontal/layer-000f.json" "$D/horizontal.jsonl" "$D/vertical.jsonl"
```

The town needs the qualified route that talks to the room B resident, opens the
gate and steps outside, as a prefix:

```sh
python3 - <<'ROUTE' > /tmp/town.jsonl
import json
prefix = [l.strip() for l in open('tools/house-conversation-qualification/route.jsonl')
          if l.strip() and '"finish"' not in l]
c = [json.loads(l) for l in prefix]
for i in range(18):
    d = 'Right' if i % 2 == 0 else 'Left'
    c.append({'label': f'h{i:02d}-{d.lower()}', 'frames': 150, 'buttons': [d]})
    c.append({'label': f'h{i:02d}-up', 'frames': 18, 'buttons': ['Up']})
for i in range(14):
    d = 'Down' if i % 2 == 0 else 'Up'
    c.append({'label': f'v{i:02d}-{d.lower()}', 'frames': 150, 'buttons': [d]})
    c.append({'label': f'v{i:02d}-right', 'frames': 18, 'buttons': ['Right']})
c.append({'finish': True})
print('\n'.join(json.dumps(x) for x in c))
ROUTE

"$P" "$ROM" "$D/town" < /tmp/town.jsonl > "$D/town.jsonl"
python3 tools/collision-qualification/derive.py \
  "$D/town/layer-000a.json" "$D/town.jsonl"

python3 -B tools/collision-qualification/test_derive.py   # 16 ROM-free controls
```

`derive.py` reads raw probe output directly: it keeps only frames where the
control word says ordinary walking is admitted, and only those belonging to the
layer's own map. Both filters are load bearing. Without the control filter the
town sweeps report `NO CONSISTENT OFFSET`, because the scripted arrival walks
the player *through* the solid door cell, which is the same unsoundness class
as the stall and layer-dump defects above.

Each sweep takes well under a minute after the ~6,800-frame input-only boot;
the town run also pays for its route prefix. `derive.py` prints
`NO CONSISTENT OFFSET` rather than a best fit when samples contradict a
single-point model, refuses samples that span maps, and warns if the consistent
offsets touch the edge of the search window.

## Traced table: a controller-action gate, not walking admission

`tools/collision-qualification/admission-route.jsonl` reproduces two Right-held,
Right-facing frames in map `$000F`, ordinary control `$00A0` throughout each
witness. Each starts from the same empty-SRAM boot route; positions are reached
by pad input, never assigned. These are not identical complete machine states:
the route walks from the open floor to the wall.

| Witness | Completed frames | Position before → after | `$80:ADAD` successor |
| --- | --- | --- | --- |
| `right-admitted` | 6850 → 6851 | `(331,112)` → `(332,112)` | `$80:ADAF` (zero) |
| `right-refused` | 6975 → 6976 | `(472,112)` → `(472,112)` | `$80:ADB7` (nonzero) |

The names describe observed displacement, not causation. Both traces execute
`$84:8E6C`, the `COP CA` handler `$80:AD39`, ordinary stream selection at
`$84:8E76`, and motion application at `$80:D107`. Fresh replay reproduces the
retained instruction-trace hashes:

- admitted: `a966f3a739bb2ef6021ef96eeb6317a13a07dbd5e2a21754af085a5a53f325c7`
- refused: `3d9faf531790c3ee40550796f7a4d7e7a6b2b085248e59abf6d7d0fc4c30a41d`

### Source contract and two corrections

At `$84:8E6C`, `COP CA $8E76` skips `COP 2B $4100,$90F3` when its probe is
nonzero, then **continues COP61**. `$84:90F3` is the Right accelerated-action
entry already identified in [input admission](input-admission.md). Thus the
probe gates that branch opportunity; it does not skip the ordinary move.
Linear CPU disassembly must not interpret COP operands as instructions.

In the traced ordinary branch (`$0868 & $0080 == 0`), `$80:AD52` computes the
right probe from player X plus the signed X offset and width, and Y plus its
offset. The retained mirror words are `(-8,16,-16,16)`, so the probe is at
`(x+8,y-16)`. `$8D:8C7E` maps the coordinates into the runtime layer;
`$8D:8D3D` advances by the byte row stride at `$087E`, with layer wrapping.
The handler checks one row when aligned and two when the 16px height straddles
rows. Map-edge wrapping and the alternate `$0868` branch are not decoded here.

At `$80:AD95`, the high byte of the cell word is read from `$7E:A001,X`:

1. If bit 7 is set, substitute **6 for the high byte**.
2. Shift right once, then mask with `$1F`.
3. Read the byte at `$80:E85C + index`; nonzero branches at `$80:ADAD`.

The dynamic override therefore selects **entry 3, not entry 6**. The unfinished
decoder had applied that substitution after the shift; a red → green control
with different entries 3 and 6 now catches it. Word bit 14 does not reach this
lookup. This does not redefine the loader's seven-bit attribute field.

`assets::maps::collision::ProbeTable` decodes only the table and CA's lookup:

- zero entries: `{0,1,2,17,19,20,22,23,24,30}`;
- entries 6 and 7: `$06` and `$07`;
- every other entry: `$0F`.

All six previously undecided attributes have nonzero probe entries. **That
is not a verdict that all six are solid.** The resolver's `$80:E849` helper
also reads this table, without CA's dynamic-bit substitution.

### Why the remaining work is directional collision

The actual motion applier is `$80:D107`. Its special-player dispatch has four
32-word tables per direction: first sample, then second sample with O/P/S first.
The first-table addresses are Up `$D542`, Down `$D8E8`, Left `$DC60`, Right
`$DFDC`, all in bank `$80`; successive pair tables are `$40` bytes apart.
The owned-ROM test checks these useful constraints:

| Type | Source finding | What it does not establish |
| --- | --- | --- |
| 5 | Same targets as partial16 in all 16 tables | All action hooks/modes or map scopes |
| 6/7 | Dedicated slope handlers; old-edge diversion precedes new-edge override | Full four-direction, neighbour-dependent slope behavior |
| 8 | Up first target `$D506` tests `$097C` bit2; Down first target shares Open | A global Open/Solid classification |
| 21 | Same targets as solid12 in all 16 tables | All action hooks/modes |
| 29 | Same as Open in 15 tables; Right S-first differs | A global Open alias |

Right S-first at `$80:E09C` sends 29 to `$DEFC` (block without nudge), whereas
0 goes to `$DEE6` (positive-nudge test). Existing narrow Up stair aliases remain
valid; broadening 29 to Open would erase a real source distinction.

### New slope discovery reaches the missing terrain

`slope_route.py Right|Left` reuses the house conversation prefix through
`exterior-walk-down`, then holds Down for 76 more frames, releases for 12, and
holds the selected horizontal direction for 40. It avoids the old exact-target
`goto` experiment, which exhausted/oscillated and repeatedly missed its targets.
Discovery was validated by native replay rather than a guessed physics test.

Both runs reach `(504,929)` and climb opposite sides of the town's slope:
Right finishes at `(560,912)`, Left at `(448,912)`. The traced frame is
11979 → 11980, control `$00A0` before and after:

| Input | Before → after | Slope path reached | Trace SHA-256 |
| --- | --- | --- | --- |
| Right | `(509,923)` → `(510,922)` | `$80:DF74 → DFA2`, type6 | `37e7691d1449dc859a5bb077b5e14068efa9c5f913916a0ffeaccd6f6c17376b` |
| Left | `(499,923)` → `(498,922)` | `$80:DBF8 → DC26`, type7 | `acade0fdbc7d884516ce397c92c6031e5f5955b801e5f8eb515334feb231a9ce` |

Separate movement-probe and trace-probe replays agree on every frame's map,
position, facing, held buttons and control, including the prefix. Neutral/setup
frames also contain control zero; they are not reclassified as walking evidence.
The entry layer places type6 at `(32,57)` and type7 at `(30,57)`. The *leading
edge* contacts them while the top-left collision point remains on type0: a
point-only occupancy census cannot establish slope coverage.

These are discovery witnesses, not a qualified portable slope implementation.
The five missing maps remain missing; no unknown cells were made passable.

### Reproduce the new evidence

```sh
sh tools/collision-qualification/build.sh
ROM='local/Tenchi Souzou (Japan).sfc'
D=local/collision-qualification
P="$D/probe/target/release"
"$P/trace" "$ROM" "$D/admission-review" \
  < tools/collision-qualification/admission-route.jsonl > "$D/admission-review.jsonl"
python3 -B tools/collision-qualification/check_trace.py \
  "$D/admission-review.jsonl" "$D/admission-review"

for direction in Right Left; do
  python3 -B tools/collision-qualification/slope_route.py "$direction" > "$D/slope-$direction-route.jsonl"
  "$P/collision-probe" "$ROM" "$D/slope-$direction" \
    < "$D/slope-$direction-route.jsonl" > "$D/slope-$direction.jsonl"
  python3 -B tools/collision-qualification/slope_route.py "$direction" --trace > "$D/slope-$direction-trace-route.jsonl"
  "$P/trace" "$ROM" "$D/slope-$direction-trace" \
    < "$D/slope-$direction-trace-route.jsonl" > "$D/slope-$direction-trace.jsonl"
done

cargo test -p assets --test local_collision
python3 -B -m unittest discover -s tools/collision-qualification -p 'test_*.py'
python3 -B -O -m unittest discover -s tools/collision-qualification -p 'test_*.py'
```

`check_admission.py ROM RUN.jsonl ...` is a **diagnostic comparison**, not a
walking acceptance gate. It authenticates/normalizes the JP ROM, requires the
matching layer for every sampled map and live-actor evidence, and refuses stale
zero-attribute layers or missing coverage. It accepts both current actor schemas
and historical coordinate pairs. Stalls cannot bridge frame/control/map gaps.
Exit status is 0 for no projection disagreements, 1 for disagreements, 2 for
invalid evidence. A zero result still does not qualify final movement.

On the retained `ext3.jsonl`, the diagnostic reports 33 disagreements: 21
occupancy frames on dynamic-bit cells in town, 11 on type14 in map C, and one
map-B sustained contact whose sampled cells have zero entries. These are not
silently excused: entry-layer drift, controller/actor effects and sampling
geometry remain unverified inputs. Missing layers in the old `town.jsonl` and
missing actor evidence in early captures now fail instead of looking green.

## Opt-in directional candidate and live-frame replays

`Room::with_passive_directional_collision()` adds a pure, CPU/ROM-free
translation of the special-player branches for 6/7, source-equivalent passive
5/21 geometry, and 29's directional exception. **Neither the default core nor
production `crysta-runtime` enables it.** Callers assert fixed `(-8,-16)`
offsets, 16×16 bounds and inactive action hooks (`$0980 & $0050 == 0`). Type8
still fails closed; no global alias bypasses its extra state input.

The implementation preserves old-edge slope diversion before the bit15
substitution, raw (not overridden) slope-neighbor probes, exact positive-edge
remainders and alignment, and the source's asymmetric neighbor checks. It
retains bounds, sample halos, atomic errors and existing scoped material rules.
Synthetic controls cover all ordered O/S/P pairs and passive 5/21 substitutions
at every subpixel position, all directions and 1/2px attempts (83,968 comparisons),
as well as slopes, masks, pair order, neighbor stride, thresholds, flags and
snapshots. These are source/implementation controls, not substitute native data.

### What the new captures prove

The trace probe accepts `"motion": true` on a bounded frame command. From one
empty-SRAM, input-only session it stops immediately before the first `$80:D107`,
requires player X/slot `$1000`, unchanged pre-collision XY and the same frame,
and reads the **live layer and attempted velocities**. It then finishes exactly
one frame. Incomplete traces or map changes abort the capture. No warps, memory
patches, state restoration or per-frame player resets are involved.

`crysta-runtime/tests/local_collision.rs` pins six complete JSONL hashes and
replays **1,972 consecutive motion frames** across six independent sessions.
Each starts after twelve contiguous, settled neutral frames. Every motion frame
checks map, special state, player flags, passive controls, fixed bounds, attempted
velocity and final XY, with an independent snapshot-restored walking history.
No motion frame is excluded. The initial TDD failure was Right frame11976,
`UnsupportedType(6)`; the candidate makes the entire windows pass.

| Capture | Frames | Player slope-adjustment handlers observed (bank `$80`) |
| --- | ---: | --- |
| `motion-Right` | 40 | `DFA2` (Right6) |
| `motion-Left` | 40 | `DC26` (Left7) |
| `motion-Right-vertical` | 236 | `D8A6` (Down6), `DFA2` |
| `motion-Left-vertical` | 236 | `D82E` (Down7), `DC26` |
| `motion-tree-east` | 714 | `D447` (Up6), `DBD0` (Left6), `DC26` |
| `motion-tree-bottom` | 706 | `D4C4` (Up7), `DBD0`, `DF4C` (Right7) |

The suite requires all eight handlers. Recorded PCs span the remainder of the
frame, so coverage uses **only the prefix from `$D109` through the first `$D197`**;
later NPC calls cannot supply missing player coverage. `$D107` was already
captured at the entry breakpoint and is not repeated by the resumed trace.
Missing start/end markers fail, with ROM-free mutation controls.

This establishes frame-wide movement equality **conditioned on each native live
layer**, not an independently simulated world or isolated resolver-output equality.
The `after` XY is frame-end, not a second breakpoint at resolver return. Snapshot
checks exercise portable walking continuation with that externally supplied room,
not native snapshots or world-state authentication. Observing each slope handler
also does not exhaust every native neighbor/pair/alignment branch.

The existing authenticated house suites additionally advance an independent
candidate history: **1,985 trajectory transitions plus 238 passive-material
transitions**, including three P/S nudges, all agree without additional exclusions.
These inherit the earlier fixtures' admission/source assumptions; they do not
newly measure every candidate precondition. Total committed native/regression
comparisons: **4,195**.

A separate diagnostic of retained map-$41 discovery data matched its first
**880 cardinal/neutral frames (41789–42668)**, initialized once at `(120,192)`;
the snapshot replay also agreed. This stops before the A command, uses the
initial checkpoint grid, lacks native velocities/per-frame layer observations,
and is not a pinned full-envelope test. Type15 was not reclassified. The wider
map-$41 envelope remains a qualification gate, not a completed acceptance claim.

### Reproduce the six pinned motion captures

Capture implementation/input recipes: tooling commit `6300ec3`, using the
workspace oracle and owned normalized JP ROM (SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`).
Complete output pins live in the replay test; raw outputs remain ignored.

```sh
sh tools/collision-qualification/build.sh
ROM='local/Tenchi Souzou (Japan).sfc'
D=local/collision-qualification
for direction in Right Left; do
  python3 tools/collision-qualification/slope_route.py "$direction" --motion \
    > "$D/motion-$direction-route.jsonl"
  python3 tools/collision-qualification/slope_route.py "$direction" --motion --vertical \
    > "$D/motion-$direction-vertical-route.jsonl"
done
python3 tools/collision-qualification/slope_route.py TreeEast > "$D/motion-tree-east-route.jsonl"
python3 tools/collision-qualification/slope_route.py TreeBottom > "$D/motion-tree-bottom-route.jsonl"
for name in motion-Right motion-Left motion-Right-vertical motion-Left-vertical motion-tree-east motion-tree-bottom; do
  "$D/probe/target/release/trace" "$ROM" "$D/$name" \
    < "$D/$name-route.jsonl" > "$D/$name.jsonl"
done
CRYSTA_COLLISION_FIXTURES="$PWD/$D" cargo test -p crysta-runtime --test local_collision
ROOM_CORE_FIXTURES="$PWD/local/movement" \
  cargo test -p room-core --test local_trajectories
ROOM_CORE_MATERIAL_FIXTURES="$PWD/local/house-materials" \
  cargo test -p room-core --test local_materials
```

### Remaining production gate

The candidate is deliberately opt-in. Native branch/mutation qualification is
still incomplete, broad passive 5/21/29 admission needs native route evidence,
type8 needs its state input, and the map-$41 envelope needs a reproducible gate.
Only then should the free-roam builder/occupancy rebuild opt in. The existing
reachability search still merges by cell and drops errors; it is not evidence
for a new 24-map claim. Actual successful routes must retain and check every
step, transitions and round trips. **19/24 remains the production result.**
