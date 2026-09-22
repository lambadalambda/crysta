# Native Crysta return arrivals: selected evidence

`evidence.json` describes two **measured completed-frame loaded → free-player
profiles** from genuine Japanese-ROM input-only boot sessions. This is not a
map-loader/departure timing model, a general scheduler, or qualification of these
selectors under every story/controller state. A host may load synchronously and
start this bounded profile at cursor zero; it must not mistake native departure
or loader duration for part of that profile.

All numbers in JSON are decimal. Frames are completed native frames. Both recipes
run after the existing 6800-frame empty-SRAM bootstrap in the collision trace
probe. Inputs are real fixed held buttons; neither captures nor recipes use
warps, memory patches, mode/data rewrites, save states or restoration. Captures,
ROM, actor words and decoded layers stay ignored/local. Only selected metadata,
source record operand fields, hashes and input commands are published here.

## Exact profiles

| Edge / selector | Initialized | Ownership | Forced script | First final XY | Recovery | Free |
|---|---:|---:|---:|---:|---:|---:|
| `$1E → $0A` / 5 | 12914 | 12928 | 12929 | 12948 | 12949 | 12950 |
| `$19 → $17` / 14 | 14498 | 14507 | 14507 | **14573** | 14575 | 14576 |

- Selector5: **36 advances, 37 samples including cursor 0**; initialized
  `(792,752)`, settled `(792,769)`.
- Selector14: **78 advances, 79 samples including cursor 0**; initialized
  `(442,345)`, settled `(456,368)`. First endpoint XY is frame **14573**, not
  the frame14574 script change. Position arrival is not control release.
- `positions` is a sparse sequence of `[elapsed,x,y]`. Hold each position until
  the next listed elapsed cursor; **do not interpolate**. These change points
  retain native nonuniform pacing and repeated positions exactly.
- `state_changes` similarly carries selected map/control/player flags,
  player/controller scripts, controller flags/slot, special flags and facing
  forward. Together with `positions`, this reproduces the selected observation
  at **every** frame in the inclusive window, not just endpoints.
- `phases` uses elapsed cursors. `ownership` is the first sampled `$097C & $8000`
  assertion; `forced` is first `$84BB70`/`$84BD7E`; `recovery` clears ownership
  with player `$84A303`; `free` is the first `$84A258` resume. The initialized
  interval before that ownership bit is not permission for ordinary walking.
- The controller-slot address can change during initialization. In particular,
  selector5's pre-ownership slot observations must not be interpreted as a
  portable stable controller identity.

## Source records and queue stages

| Edge | Exit record | Full rectangle operands | Mode | Raw XY | Adjusted queued XY |
|---|---|---|---:|---|---|
| `$1E → $0A` | `$818F9B` (ROM `$018F9B`) | `(39,12,1,4)` | 0 | `(784,752)` | `(784,736)` |
| `$19 → $17` | `$818F42` (ROM `$018F42`) | `(58,20,1,1)` | 0 | `(448,352)` | `(434,329)` |

JSON retains all decoded operands: rectangle, destination, mode, selector and
raw destination coordinates. Queue observations pin the raw/adjusted values at
12815/12816 and 14429/14430 respectively. They are context, **not** cursors in the
loaded-to-free profile. Initialized positions differ again from both queue stages.

Independent source confirmation supplied by the parent: the exit-scan gate at
**`$8D87A0`**, `LDA $097C; BIT #$0010; BEQ $87AA`, has **clear = permits scanning,
set = suppresses scanning**. `$879F` is not an instruction boundary. Do not
confuse this `$0010` gate with sampled arrival ownership `$8000`. The extractor
checks exit operands against the pinned ROM; the gate annotation records that
separate source finding, not an end-of-frame observation of the transient gate.

## Hostile-input control

`return19-hostile-route.jsonl` preserves the entire baseline prefix and total
frame count, replacing only completed frames **14498–14574** with **Left+A**,
then returning to neutral on **14575**. Selected loaded-to-free observations
match baseline exactly. `$0454` demonstrates input delivery (including `$0280`
on frame14511), despite no motion takeover or interaction-script dispatch.

The captures are **not byte-identical during ownership**. The following detailed
word/slot and post-free comparisons were inspected separately; the verifier
recomputes loaded-to-free selected differences and top-level ancillary field
names, not these more detailed prose observations. `$0454` differs through
14575, player ancillary word `+$28` differs at14520, and actor slot `$1100` word
`+$0E` differs on 11 frames within14551–14571. Metadata publishes only differing
field names/frame numbers, not raw actor word values or layers. These scratch
word semantics remain unclassified. All observations match from14576 onward
apart from command labels; later ordinary walking also matches. This establishes
bounded observed rejection, not universal rejection of every interaction.

`return1e-hostile-route.jsonl` likewise preserves its baseline prefix and total
frame count, holding **Left+A on 12914–12948**, then neutral from **12949**.
All selected loaded-to-free observations match baseline. The only observed
non-label/non-held differences are `$0454` in `service_words` on12934–12949;
player/controller/actor words and layers match throughout. All observations
match from12950 through the final ordinary-walking checkpoint13163, apart from
labels. Both hostile captures and exact recipes are pinned in `evidence.json`.
The phase names and elapsed cursor convention are identical for both edges.

## Reproduction

Python 3, standard library only; tests require no ROM or captures:

```sh
python3 -m unittest discover -s tools/arrival-qualification -p 'test_*.py'
```

Use caller-chosen paths; no absolute developer paths are embedded in metadata:

```sh
python3 tools/arrival-qualification/extract.py verify \
  --rom 'local/Tenchi Souzou (Japan).sfc' \
  --capture1e "$CAPTURE_1E" \
  --capture19 "$CAPTURE_19" \
  --hostile19 "$HOSTILE_19" \
  --hostile1e "$HOSTILE_1E"
```

Replace `verify` with `extract --evidence local/arrival-evidence.json` to generate
an independent candidate file. Verification re-extracts and compares the complete
metadata, including raw capture and recipe SHA-256 hashes, exact held-input frame
coverage, source operands, sparse position/state change points and hostile
comparisons. A missing/duplicate frame, non-arrival window row, wrong initialized
or free boundary, source hash mismatch, or changed selected observation fails.
The raw hash also detects changes outside the selected profile; only selected
fields are interpreted semantically. Recipes resolve relative to this directory.

Original local evidence locations (pass these as arguments, not source defaults):
- `$1E`: parent `local/arrival-qualification/live1e/capture.jsonl`.
- `$19`: discovery worktree `local/arrival19/recapture.jsonl`.
- Hostile `$19`: discovery worktree `local/arrival19/hostile-left-a/capture.jsonl`.
- Hostile `$1E`: parent `local/arrival-qualification/hostile1e/capture.jsonl`.

The original `$19` recipe deliberately retains discovery detours. Do not trim it
without a new native replay: its inputs, timing, actor scheduling and capture
hash are part of this selected evidence. Runtime/core behavior and tests are
owned separately by the parent.
