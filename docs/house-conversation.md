# Room B conversation → first exterior: source contract

Scope: Japanese ROM SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
This is semantic progression qualification, not a native scheduler, general event
VM, dialogue/font decoder, or exterior graphics/collision implementation.
[Tracked issue](../meta/issues/qualify-house-conversation-progression.md).
RE/source evidence substitutes for initial red tests. Only source metadata,
hashes, tooling and input commands belong in Git; generated evidence stays local.

## Core contract (source backed)

**A flat list of pages followed by one completion flag is insufficient.**
The fresh request grants the flag before a subsequent choice and follow-up.
Use stable resident identity `$838B96`, not its native allocation slot `$1040`.
Its callback is `$888EDE` (the write at `$888F08` is not the callback entry).

| Phase | Source request / control | Effect |
|---|---|---|
| B entry | `$888E6E → $888FDA`, wait `$888E72` | No `$0026` grant; do not confuse arrival text with interaction |
| First interaction | `$888F02 → $888FF0`; wait `$888F06` | After this request returns, `$888F08 COP07 $8026` sets global `$0026` |
| First choice | `$888F0C COP1A`, catalog 0 | Two options; cancel result 0, option results 1/2 |
| First option 1 | `$888F1D →` resident continuation `$888E86 → $8890D9` | Follow-up text, no additional global event write |
| First option 2 or cancel | `$888F17 → $888E96 → $88905A` | Alternate follow-up, no additional global event write |
| Repeat | `$888F23 → $889156`; wait `$888F27`; choice `$888F29`, catalog 1 | `$0026` already set |
| Repeat option 1 | `$888F34 → $88918C` | Follow-up, no additional global event write |
| Repeat option 2 or cancel | `$888F3B → $8891D6` | Alternate follow-up, no additional global event write |

The callback first tests `$0109,$003B,$0296,$0021,$0028`, then `$0026`.
Only the fresh-house state (`$0020,$00FB`, later plus `$0026`) is qualified here;
other story branches must not accidentally fall through to this transcript.
`$003B` targets the immediately following check rather than changing this path.
Repeat selection at admission is safe for this bounded state. Page/choice data
must come from the separately owned dialogue decoder, not copied Japanese text.

`COP1B` publishes a banked request; `COP1F` executes/waits for it to finish.
`COP1A` calls `$859F28`, then dispatches an inline result table. It is a real
choice, **not an animation/facing selector**. The first choice table at `$888F11`
is `[cancel:$888F17, option1:$888F1D, option2:$888F17]`; the repeat table at
`$888F2E` is `[$888F3B,$888F34,$888F3B]`.

### Admission and controls

- Native interaction registration at `$848A1E` is held **A** (`$0080`), tested by
  `$80906F..90AC`; not exclusively a rising edge. The dialog/UI can impose its
  own deterministic input admission without reproducing CPU frame counts.
- `$87923F` refuses while `$0DC2 != 0`, then dispatches by facing `$0956`.
  Actor targeting `$87C783..C7F0` precedes tile fallback. It walks the list rooted
  at `$0DFA`, following entity `+$2E`, taking the first rectangle hit among actors
  with `+$04 & $0100`. Callback lives at extended `+$20`, installed by `COP21`.
- Candidate rectangles use extended offsets `+$28/+2C` and extents `+$2A/+2E`;
  unsigned 16-bit differences are accepted **inclusively** (`<= extent`). The
  ordinary resident witness has origin `(120,112)`, offsets `(-8,-16)`, extents
  `(16,16)`: rectangle **X112..128, Y96..112**.
- Forward sample points are 8 and 16 pixels along facing from Ark's position.
  Up at `(120,128)` hits the lower rectangle boundary; Down there misses. The
  first actor-hit path additionally checks callback/interaction-policy bits
  `entity+$06` (`$0200` unrestricted, otherwise `$0100` with opposite facing),
  and optional gate fields at `$7F:2028+slot` / `$7F:2026+slot` before dispatch.
  Here “extended +offset” means `$7F:0000+slot+offset`; ordinary entity fields
  are `$7E:0000+slot+offset`.
  The fresh idle resident witnesses `$0200`, not an opposite-facing restriction.
- Native tile type `$2000` can extend the **far** sample another 32 pixels along
  facing, after ordinary actor/tile probes fail. This is a counter interaction
  rule, not permission to replace targeting with a radial distance check.
- Choice handler `$859F46..9FE6`: Up/Down/Left/Right follow catalog neighbor links;
  A or L (`$00A0`) confirms; B (`$8000`) cancels, returning result 0. The two
  required catalogs link Up/Down between options, with no Left/Right neighbor.
  Text-page A/L acknowledgements are separate from B cancellation; native B/X
  pulses did not advance the witnessed follow-up text.

### D gate: load-time membership, not a live event subscription

Hidden gate **`$838CC8`**, header `$88A9AF`, origin `(120,720)`:

1. `$88A9B4 COP48 $8026`: reject/delete if global `$0026` is already set.
2. `$88A9B8 COP3B`: otherwise stamp occupancy at cell **1415** (`7,44`),
   raw `$0592 → $8592`, independent of the visible wandering resident.
3. `$88A9BA COPBC`: save continuation `$88A9BC`, a bare RTL.

`COPBC` handler `$80AAA5..AAB2` stores the continuation and returns; later actor
invocations return immediately. There is **no retest, COP3C clear, or unlink**.
Changing `$0026` while D is already loaded does not remove this stamp. Reloading
D reconstructs occupancy and rejects the actor when the flag is set. A normal
B-conversation→D journey necessarily loads D after the grant, so the gate is
absent. This already-loaded counterfactual is source-qualified, not a RAM-poked
acceptance experiment. Existing pre-conversation closed-gate native evidence is
in [house backgrounds](house-backgrounds.md) and [scene census](house-scene.md).

### Outgoing endpoint

The first exterior is **map `$000A`**, not an unverified name guess. D's direct
exit `$818DFE` has rectangle `(7,44,1,4)`, destination A, mode0, selector5, raw
position `(496,752)`. A's selected player record is `$8389B8`; its source record
position is **not** the exit's queued/settled spawn. Existing shared exit logic
uses bounding origin `Ark+(-8,-16)` and first coarse match followed by fine
exclusive deltas; D's exact X alignment is Ark X120 and eligible Y720..768.
Selector5 queues raw destination plus `(0,-16)`, then loader/arrival scheduling
produce the actual position; do not teleport directly to the record's spawn.
Native spawned/settled/walking endpoint evidence is below. The queued coordinate
is `(496,736)`, initialized Ark is `(504,752)`, and settled Ark is `(504,769)`.
These are three different stages, not alternative names for one spawn.

## Source reproduction

```sh
python3 -B tools/house-conversation-qualification/source.py \
  'local/Tenchi Souzou (Japan).sfc' > local/house-conversation-source.json
```

`source.json` pins bounded code/data investigation windows and typed source
operands, not raw scripts or dialogue. `source.py` is deliberately not a general
actor interpreter. The separate dialogue task owns requests, page boundaries,
choice text/layout and font decoding; the exterior task owns assets/collision.

## One retained, fresh input-only journey

`route.jsonl` runs after the existing 6800-frame new-game bootstrap. The process
uses `Session::new` (empty SRAM), real buttons only, no patch/warp/state load.
It ran **once**, streamed incrementally without reboots while discovering the
conversation. `save_state` synchronizes each retained checkpoint; WRAM/PPU/OAM
observations after that are passive. The final command flushes and uses
`process::exit(0)` rather than attempting another session/core teardown.

All 78 checkpoints (boot plus 77 commands), opaque states and captures remain
local; the checker selects 31 semantic checkpoints and hashes the complete
per-frame log. No original script/text/capture payload is committed. Command
labels record exploratory intentions, **not asserted game phases**: e.g.
`first-complete` was still a page wait, and `exit-spawn` was still loading.
Use `reference.json`/the tables below for actual observations.

### Acknowledgements, choices and negative controls

All positions in this table are Ark `(120,128)`, except B entry `(120,191)`.
Text cursors are **native wait positions**, not font-decoder page-key APIs.
`$0DC2=$FFFF` identifies a choice wait; low bank `$88` identifies active text;
zero identifies an inactive request (the old cursor remains in memory).

| Checkpoint / completed frame | Native phase / cursor | Global events |
|---|---|---|
| `north-door-settled` 7848 | B entry `$888FEF`, not progression conversation | `$20,$FB` |
| `entry-no-ack` 8028 | Entry gone after Up180; reached resident; inactive | `$20,$FB` |
| `entry-after-A` 8149 | First interaction, `$889027` acknowledgement wait | `$20,$FB` |
| `first-no-ack` 8389 | Right240: same wait and position, **no early grant** | `$20,$FB` |
| `first-A1` 8390 | A pulse admitted; still no immediate frame-level grant | `$20,$FB` |
| first changed frame **8409** | Remaining first-request presentation completes after A | **`$20,$26,$FB`** |
| `first-page2` 8570 | Choice0, `$889047`, selected option1 | `$20,$26,$FB` |
| `choice-no-confirm` 8750 | Neutral180: choice remains; flag is already set | same |
| `first-followup` 8931 | A accepts option1; follow-up `$8890F8` | same |
| `followup-no-ack` 9051 | Down120: no movement/page advance | same |
| `first-complete` 9172 | B pulse + wait: still `$8890F8`, **B is not page ack** | same |
| `repeat-wrong-wait` 9263 | A advances to `$889126`; still first follow-up | same |
| `repeat-start` 9444 | A advances to `$889155`; still first follow-up | same |
| `followup-closed` 9565 | X pulse + wait: still `$889155`, **X is not page ack** | same |
| `wrong-facing-settled` 9666 | A closes final first-option follow-up | same |
| `repeat-page` 9853 | New interaction: repeat choice1, `$88917E` | same |
| `repeat-choice-second` 9914 | Down moves selection to option2 | same |
| `repeat-cancel-page` 10095 | B cancels choice → `$8891EB` follow-up wait | same |
| `repeat-after-L` 10216 | L advances follow-up to `$889217` | same |
| `repeat-after-A` 10337 | A closes follow-up; no extra events | same |
| `away-negative` 10439 | Down-facing A: no request; position unchanged | same |
| `repeat2-choice` 10631 | Turn Up, A: repeat choice1 again | same |
| `repeat2-followup` 10812 | L accepts option1 → `$8891AA` | same |
| `repeat2-page2` 10993 | A advances to `$8891D5` | same |
| `repeat2-closed` 11114 | A closes repeat option1 | same |

Thus native first-option follow-up has three acknowledgement waits, repeat
option1 and cancel/option2 follow-ups have two each. First option2/cancel's
`$88905A` follow-up is **source-routed but not separately replayed**; the dialogue
owner supplies its exact page metadata. This journey does not pretend to cover
all combinations by loading old states. The checker rejects missing choices,
B-as-ack, early/deferred grants, wrong repeat branches and extra event changes.

Grant timing is semantic, not “wait 19 portable frames”: acknowledge `$889027`,
finish the remaining first request, execute the source write, then enter choice0.
The flag persists even while that choice/follow-up is unfinished. Preserve the
choice and its follow-up for real user interaction; do not silently dismiss it
because the exit is now eligible.

### Gate, departure, initialized spawn, settled endpoint and real walking

After leaving B normally, the same journey enters D at `open-D` frame11466:
Ark `(120,625)`, camera `(0,512)`, cell1415 `$0592`, and no linked actor with gate
continuation `$88A9BC`. The checker walks the actual native linked list, excluding
stale nonzero slots. The earlier closed-gate native witness is deliberately
reused from the existing census/background qualifications; this journey does
not revisit the pre-grant state. No already-loaded event mutation is performed.

| Completed frame / checkpoint | Map | Ark position | Meaning |
|---|---|---|---|
| 11673 `exit-trigger` | D | `(120,721)` | Fine exit match; last direct Down input |
| 11674 | D | `(120,722)` | Departure resume `$84B975` after input release |
| 11690 | A | `(120,738)` | Current-map store, **old departure coordinates**, not arrival |
| 11723 | A | `(0,0)` | Actor-loader clearing, not a playable frame |
| 11724 | A | **`(504,752)`** | Initialized player, source entry `$84A12E` |
| 11760 | A | **`(504,769)`** | Arrival complete, free-player `$84A258` |
| 11853 `landed-A` | A | `(504,769)` | Retained settled checkpoint, camera `(376,657)` |
| 11945 `exterior-walk-down-settled` | A | `(504,815)` | Down32 + neutral60; camera `(376,703)` |
| 12059 `exterior-walk-settled` | A | **`(538,815)`** | Right24 + neutral90; camera `(410,703)` |

The semantic walking segment is **Down from the house landing, then Right** on
A, without another transition or new event. Native held endpoints are
`(504,814)` / `(537,815)` before residual settling. A portable movement model
need not reproduce those scheduler/residual details to represent this segment;
it must start on A, admit real input, move through the qualified exterior
collision data and remain on A. Exterior art/collision belongs to its separate
qualification task, not to this tool's positional evidence.

## Reproduce / check retained evidence

```sh
# Fresh input-only replay: ONE boot/process; no supplied saves needed.
sh tools/house-conversation-qualification/replay.sh \
  'local/Tenchi Souzou (Japan).sfc'

# No new boot: verify the retained discovery journey in this task worktree.
python3 -B tools/house-conversation-qualification/check.py \
  'local/Tenchi Souzou (Japan).sfc' local/house-conversation-qualification/journey
python3 -O -B tools/house-conversation-qualification/test_check.py
```

The replay wrapper is provided for reproduction; qualification did not rerun
the entire journey per feature. The native probe was built and the single
streamed journey exited successfully. Source/native checkers passed in ordinary
and optimized Python. Sixteen ROM-free checker tests passed in both modes;
replacing the validator with a no-op produced 15 expected mutation failures
before restoring green. No portable core, host, assets or tracker files changed.

For the parent: the issue's source/native criteria are supported by this evidence
and the explicitly reused pre-grant gate witness. Tracker closure remains with
the parent (this task does not own tracker edits). Independent reviews precede
both the source-contract and native-evidence signed commits.
