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
sh tools/pandora-tower-discovery/sweep.sh probe 8      # serpentine floor sweep
sh tools/pandora-tower-discovery/stop.sh
```

Breakpoint sessions use the same scripts with a different probe. `trace.sh`
refuses to run against a walking session, because the walking probe panics on a
trace command and that costs a 15-minute prefix:

```sh
export DISCOVERY_DIR=local/pandora-tower-discovery/trace
PROBE=trace sh tools/pandora-tower-discovery/session.sh "local/Tenchi Souzou (Japan).sfc"
sh tools/pandora-tower-discovery/trace.sh 80A395          # target, frames, instructions
sh tools/pandora-tower-discovery/stop.sh
```

The probe also takes `{"profile":{"from":"...","to":"..."},"frames":N}` on stdin,
which reports which addresses in a half-open range executed, and how often.

```sh
for t in test_observe test_frame test_report_trace; do
  python3 -B tools/pandora-tower-discovery/$t.py
  python3 -O -B tools/pandora-tower-discovery/$t.py
done
```

Labels must be unique, ASCII alphanumeric or `-`, and frames must be 1..2000;
those are the probe's rules, not this harness's. Captures, framebuffers, PNGs and
session directories stay ignored under `local/`; `frame.py` refuses to write
anywhere else. The probe never exits on its own, because the holder keeps its
stdin open, so finish with `stop.sh` rather than leaving it resident.

`observe.py` exists because the probe's own JSONL only reports event flags below
`$200`, which hides `$243`/`$244`/`$292`. Any claim about tour completion or the
`$FE`/`$23` continuation has to read the full block.

`discovery-route.jsonl` is the retained itinerary behind the walking tables
below: 149 commands, 11,846 frames, appended after the accepted prefix. It is
input only, like `tools/pandora-qualification/discovery-route.jsonl`. Every
session also retains its own `journey/route.jsonl`; the trace commands behind the
breakpoint table are listed with that table.

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
`sweep.sh` therefore stops the sweep when `control` does not reach `160` under a
held direction, rather than printing readings nobody checks, and it observes the
vertical row step too, since that is where the admission bug below hides.

## What the breakpoint session established

`trace-probe.rs` replays the same prefix but breakpoints script addresses with
`Session::trace_until_pc` instead of walking. It drops the artifact writes and
the accepted probe's per-frame `wram_image` reads, keeping `save_state` at the
same point in each command. Only `save_state` synchronizes the core
(`serialize(true)`, see `docs/sprite-hardware.md`); the framebuffer, VRAM, CGRAM
and OAM accessors are passive copies. Equivalence is therefore empirical, not
structural, which is why the endpoint was checked against the accepted capture
before anything else was trusted: frame 41,788, map `$41`, `(120,192)`, facing 1,
control 0, script `$84A258`, and the documented flag set exactly. It matches.

| Target | Result |
|---|---|
| `$88AF3F` continuation flag write | `InstructionLimit`: **not reached** |
| `$8087C2` COP0D predicate helper | `InstructionLimit`: **not reached**; nothing polls a position |
| `$88AE64` Pandora controller | `InstructionLimit`: **not reached**; that script has ended |
| `$89D2E5` guide actor script | `TargetReached`, every frame, from actor dispatch `$80C745` |

Reproduce with a `PROBE=trace` session and, in order:
`trace.sh 88AF3F`, `trace.sh 8087C2`, `trace.sh 88AE64`, `trace.sh 89D2E5`,
then `trace.sh 80890C` and `trace.sh 80A395`.

Each negative above is one budget, not two: the probe's 2,000,000-instruction cap
binds first, at roughly 13,900 instructions per frame, so every `InstructionLimit`
stop covers about **144 frames — 2.4 seconds of idle game time** at the endpoint.
They are not claims that the address never executes.

So the continuation is not a position poll that was missed, and the controller
that ran the box sequence and the tour is no longer running at all. The live
entity is the guide actor in slot `$1040`.

Profiling `$89D200..$89D800` (half-open) over two frames hits only three
addresses:
`$89D2AC` and `$89D49B`, which are both `RTL` (idle actor stubs), and `$89D2E5`,
which is `COP $91`. The guide is parked on that COP, re-entering it every frame.

The dispatch chain, each step verified against a live trace:

```
guide actor $1040 -> $89D2E5  COP $91
  native_cop_handler $808378: AND #$00FF / ASL / JMP ($83B2,X)   ; all 8 bits kept
  -> table entry $0084D4 -> handler $80:A395
       BB           TYX                 ; X = actor slot
       22 75 ED 80  JSL $80ED75         ; predicate
       B0 FA        BCS $80A396         ; loop on the JSL while carry set
       68 68 6B     PLA / PLA / RTL
```

The trace confirms every link: stopping at `$80A395` gives `A=X=0x0122` (exactly
`2 * $91`), `Y=0x1040` (the guide's slot), and a return address of `$89D2E7` on
the stack, reached through the dispatcher's indirect jump.

**The gate is therefore `$80ED75`, evaluated for actor slot `$1040`.** Its first
instructions read the actor's bank byte at `slot+$12` and a long-indexed word at
`$7F0008,X`. Reading that as "wait until this actor's motion completes" is an
inference, not a verified claim; confirming it is the next step and needs the
handler decoded properly rather than eyeballed.

### The ROM map's COP table bound is too small

`docs/rom-map.md` documents COP selectors `$00..$7C` as 125 pointers at
normalized `$0083B2..$0084AB`, treating the following word `$109A` as proof of
the table's end. But the dispatcher's `AND #$00FF` only clears the high byte of a
16-bit load: all eight selector bits survive into the index. The game
demonstrably issues `COP $91`, resolving through `$0084D4` to a working handler
at `$80:A395`. Across `$00..$EF`, `$7D`/`$7E`/`$7F` are the *only* entries that
are not bank-`$80` pointers, and they are exactly what stopped the documented
contiguous scan. Entry `$00` points to `$8592`, so the table cannot exceed 240
entries before colliding with its own first handler. The 125-entry bound is a
scan artefact, not the real extent. Tracked as
[Correct the COP service table bound](../../meta/issues/correct-cop-table-bound.md).

## What this does not establish

What `$80ED75` actually tests, and therefore how to satisfy it. That is the open
question, and it is now a bounded one: a single predicate, called with a known
actor slot, rather than an unbounded search of a room.

The sweep result is a negative, not a proof. A later pass covered the right
pocket at 13-16px rows (`208,192,179,166,153,144,131,118`) and the left pocket at
13-19px rows (`192,176,157,144,125,112`), with `control` verified at 160 on every
pass. Against
32px-scale trigger boxes — the box-opening gate's own `raw_bounds` span 32px —
that is adequate coverage, but it remains sampling.

Movement needs roughly 7 frames of held input before it is admitted, so row
steps below that silently move nothing. An earlier sweep used 6-frame steps and
produced a uniform stuck reading for that reason alone. See
[input admission](../../docs/input-admission.md).

The coordinate convention in `frame.py` assumes the camera sits at the origin, as
it does throughout the hall. It does not hold for scrolling maps.
