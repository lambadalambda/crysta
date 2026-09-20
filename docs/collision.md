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

That `12` and `14` each block from *opposing* directions rules out the obvious
rival reading, that they are one-way ledges or elevation edges.

### The doorway contact, and what it settled

Attribute `2` forms one-cell gaps in walls; map `$000F`'s grid shows `#2#`
twice along its row-12 wall. Walking the player into the gap at cell `(24,12)`
moved it from `y=192` to `y=336` **and changed the map from `$000F` to
`$0010`**, an edge the static exit graph independently predicts. So the
attribute admits movement. The transition belongs to the exit record rather
than the attribute: most attribute-`2` cells lie outside any exit rectangle.

That contact also pinned the horizontal offset, which the serpentine sweeps
could not. The press was refused at `x=402` and admitted at `x=399` against a
cell spanning `384..399`, so `dx` is `-2..=0`. Attribute `2` had been reported
offset-dependent precisely because `dx` in `5..=7` put the blocking cell one
column over; those offsets are now excluded and the ambiguity resolves to
walkable. This is exactly the differing sub-cell phase the earlier analysis
said was missing.

Attribute `16` sits in a one-cell alcove whose only open side is below. A
sustained press from that side was refused across seven attempts of 50 frames.

### Coverage across the slice

**47,091 of 47,616 cells (98.9%)** across all 24 Crysta maps now resolve. What
remains is `5` (8 cells), `21` (48, in `$0012`–`$0019`), `29` (65, in 17 maps)
and the town exterior's own `6`, `7`, `8` and `25` (404 cells, map `$000A`).

### Threshold sensitivity

A press is treated as refused after `STALL_FRAMES = 20` frames without integer
movement. That is not arbitrary: the longest mid-run stationary stretch after
which movement *resumed* is **2 frames**, a 10x margin, and the partition above
is unchanged for any threshold in `8..=40`. Attribute `2` leaves the solid set
only at `>= 60`, where the contact count collapses to 15.

## What is not established

- **The reference point is still a box.** `dx` is `-2..=0` from the doorway
  contact, but `dy` remains `-12..=-1` relative to the player word at
  `$7E:1000/$1002`: no vertical contact yet occurs at a differing sub-cell
  phase. The serpentine stalls all halt at `x % 16 == 8`, which is why they
  carried no sub-cell information. The search is bounded a priori to a
  sprite-extent window
  (`dx` in `-16..15`, `dy` in `-24..7`) to exclude mirror solutions; on this
  data that window is inert, and widening it to `+/-32` changes nothing.
- **Seven attributes remain undecoded**, 525 cells.
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

python3 -B tools/collision-qualification/test_derive.py   # 12 ROM-free controls
```

Each sweep takes well under a minute after the ~6,800-frame input-only boot.
`derive.py` prints `NO CONSISTENT OFFSET` rather than a best fit when samples
contradict a single-point model, refuses samples that span maps, and warns if
the consistent offsets touch the edge of the search window.
