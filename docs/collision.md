# Movement collision: measured attribute semantics

Status: **the cell attribute is decoded exactly; its movement semantics are
qualified for four values in one map.** This is a bounded measurement, not a
complete passability specification, and not yet a trace of the admission
routine itself. See [the open issue](../meta/issues/qualify-collision-predicate.md).

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
- **The admission routine has not been traced.** Everything here is inferred
  from observed movement. Until the predicate is found and disassembled, a
  second input (facing, object bits, an actor plane, a per-map table) could
  refine any of it.
- **The layer is snapshotted once per map**, at entry. Given that bit 15 is
  written during play, in-run layer drift is not excluded by this method.
- **Nothing here is wired into the portable core yet.** `room-core` still uses
  its per-room qualified collision responses.

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
