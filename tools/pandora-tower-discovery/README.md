# Tower-approach discovery harness

[Owned issue](../../meta/issues/qualify-tower-approach-route.md). This is a
**discovery** harness for exploring past the accepted Pandora endpoint. It
qualifies nothing: there is no checker, no reference file and no pinned
evidence here. The accepted route and its strict checker remain
[`tools/pandora-qualification/`](../pandora-qualification/), and the directory is
named `-discovery` rather than `-qualification` to keep that distinction visible.

## Why it exists

The accepted route ends at `pandora-tour-control` in map `$41`, and every
exploration step beyond it previously meant replaying ~41,800 frames from boot.
This harness pays that prefix once and then holds the probe's stdin open, so
each additional command costs seconds instead of a quarter hour.

It keeps the accepted discipline: one empty-SRAM Session, real button input from
the menu onward, no warp, no memory patch, no save and **no state restore**. The
prefix is `tools/pandora-qualification/route.jsonl` verbatim minus its finish
command, so the session arrives at the accepted endpoint by the accepted path.

## Use

```sh
sh tools/pandora-tower-discovery/session.sh "local/Tenchi Souzou (Japan).sfc"
# ~15 minutes; wait until out.jsonl holds 383 checkpoints, then step:
sh tools/pandora-tower-discovery/step.sh probe-right 60 Right
sh tools/pandora-tower-discovery/step.sh probe-talk 6 A
python3 -B tools/pandora-tower-discovery/frame.py \
  local/pandora-tower-discovery/session/journey/probe-right.pixels \
  local/pandora-tower-discovery/probe-right.png --grid
sh tools/pandora-tower-discovery/stop.sh
python3 -B tools/pandora-tower-discovery/test_observe.py
python3 -O -B tools/pandora-tower-discovery/test_observe.py
python3 -B tools/pandora-tower-discovery/test_frame.py
python3 -O -B tools/pandora-tower-discovery/test_frame.py
```

Labels must be unique, ASCII alphanumeric or `-`, and frames must be 1..2000;
those are the probe's rules, not this harness's. Captures, framebuffers, PNGs and
session directories stay ignored under `local/`; `frame.py` refuses to write
anywhere else. The probe never exits on its own, because the holder keeps its
stdin open, so finish with `stop.sh` rather than leaving it resident.

`observe.py` exists because the probe's own JSONL only reports event flags below
`$200`, which hides `$243`/`$244`/`$292`. Any claim about tour completion or the
`$FE`/`$23` continuation has to read the full block.

`discovery-route.jsonl` is the retained itinerary behind every table below: 149
commands, 11,846 frames, appended after the accepted prefix. It is input only,
like `tools/pandora-qualification/discovery-route.jsonl`.

## What the first discovery session established

The session explored frames 41,788 → 53,634 from the accepted endpoint.
**The continuation did not fire.** The final flag set is exactly the documented
endpoint set — `$20,$22,$26,$27,$28,$2E,$FB,$243,$244,$292` — with no `$23`, no
`$FE` and no map change away from `$41`.

The hall splits into two pockets joined along the bottom. Bounds were measured by
walking into each edge with a held direction; `control` was observed at `160`
throughout, confirming input was actually being accepted (see the `Start` note).

Right pocket:

| Row | Leftmost | Rightmost | Blocked by |
|---|---:|---:|---|
| `y=204` | 120 | 232 | floor object left of 120 |
| `y=192` | 112 | 232 | floor object |
| `y=176` | 184 | 232 | central desk cluster |
| `y=157` | 200 | 232 | right bookshelf |
| `y=144` | 200 | 232 | right bookshelf |
| `y=112` | 232 | 232 | wall band above `y=112` |

Left pocket:

| Row | Leftmost | Rightmost |
|---|---:|---:|
| `y=192` | 40 | 164 |
| `y=176` | 40 | 120 |
| `y=157` | 40 | 72 |
| `y=144` | 40 | 72 |
| `y=125` | 40 | 40 |
| `y=112` | 40 | 40 |

A floor object occupying about `x=88..120` at `y≈205` blocks passage between the
pockets along the bottom row; it is what stopped rightward movement at `x=88` and
leftward movement at `x=120`. The pockets do connect one row higher, around
`y≈188`.

Nothing above `y≈112` is walkable. The three arches at `x≈64/128/192` each have
furniture directly below them, so the tour rooms `$42/$43/$44` are not reachable
on foot; this is consistent with the tour being forced scripted presentation.

Interactions tried with no effect on flags, map or script: `A` facing the floor
object from both sides, `A` facing the desk cluster, `A` facing the left
bookshelf from below and from the side, `A` facing up under each arch's
furniture, `B`, `X`, `Y`, holding `Left`+`B` into the floor object to push or
dash past it, and a 2,000-frame idle wait.

### Start opens an invisible input-swallowing state

Pressing `Start` in the hall produces no visible menu and no observable state
change, but it silently stops all player input: held directions no longer change
`position` or `facing`, and `control` stays `0` instead of rising to `160`. A
second `Start` releases it and movement resumes immediately.

This is worth knowing because it invalidates results quietly. The first sweep run
here was conducted inside that state and produced a clean set of "blocked
everywhere" readings that looked like map geometry and were nothing of the kind.
Any future sweep should assert that `control` reaches `160` while a direction is
held, rather than trusting position alone.

## What this does not establish

No conclusion about *where* the continuation triggers. Both pockets were swept at
six rows each, which is coarse: a position-polling predicate could sit on an
untested tile between rows. Equally, the trigger may not be positional at all.
Ruling that out needs either an exhaustive reachable-tile sweep or the event
bytecode work in
[Reverse the event script bytecode](../../meta/issues/reverse-event-bytecode.md),
which would read `$88AF3F/AF43`'s guard directly instead of guessing at it.

The coordinate convention in `frame.py` assumes the camera sits at the origin, as
it does throughout the hall. It does not hold for scrolling maps.
