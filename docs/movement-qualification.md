# Bounded player movement qualification (experimental)

This is evidence for [the portable room slice](../meta/issues/portable-room-slice.md),
**not a production movement implementation**. Scope: ordinary cardinal walking
on selected interior floor/full-wall paths in maps `$000F` and `$0010`. No claim
that an arbitrary room, input sequence, material, or player mode is supported.
PCs/addresses are hexadecimal; collision type numbers and coordinates are decimal.

## Recommended smallest profile: floor, full wall, doorway handoff

**Corner handling is not required to reach the doorway.** The `flat_only`
experiment narrows the rules below: accept only unflagged O={0,2,22} and
S={12,14}; sample the old/new edges as specified; **stop on a mixed O/S pair**.
O/O passes, S/S performs the directional snap/rollback, with no perpendicular
nudge code. An aligned edge has just one sample. Keep the bit-3 correction;
replacing it with a general AABB implementation is still unnecessary and unproven.
Unknown cells stop the operation rather than returning a blocked result.

This strict profile matches **1,204 steps** across six retained ordinary-walking
segments plus the new 80-step doorway approach. These are `wall-Left`,
`wall-Right`, `wall-Down`, `up-central`, `up-type12`, `map10-Down`, and
`doorway-approach`. The first five establish all four cardinal flat-wall paths
in F. The unaligned `map10-Left`, `map10-Right`, and `map10-up-wall` routes meet
mixed pairs: their earlier position matches do **not** admit them to this smaller
profile. They can simply stop until corner responses are separately wanted.

### Immutable data and snapshot state for the parent core

No emulator addresses or stream pointers are needed in portable state:

- **Room data:** integer grid width/height and row-major collision cells retaining
  stored type plus the high-bit override flag (raw u16 is sufficient). Both rooms
  are 32×64; reject boundary samples rather than emulate coordinate masking.
  Bounds `offset=(-8,-16), extent=(16,16)` can be constants for this profile.
- **Exit data:** the parent's independently qualified ordered exit records and
  runtime admission rules. Type 2 remains open floor; its presence alone is **not**
  an exit trigger. Evaluate the qualified exit selector after resolved movement.
- **Mutable walking state:** `(x,y)`, one-frame delayed input, active direction,
  cadence phase, and input-admission history. Horizontal phase `0..53` suffices
  (0=setup/gap). Vertical phase can be `0=setup,1=odd,2=even`, advancing
  `0→1→2→1…`; a direction change resets phase to 0. Retain facing if the parent
  transition interface needs it. There is no persistent pending delta after a step.
- **Mode/status:** Walking, transition handoff, or Unqualified, rather than using
  a zero velocity to conceal an unsupported action or tile. Rendering/camera and
  the transition controller are separate consumers, not movement dependencies.

A conservative input-admission guard records a four-bit **used-directions mask**.
The existing delayed-input field also identifies the previous submitted input;
no second copy is needed. Allow a direction's first contiguous activation interval;
reject any later
reactivation of it, even after release or an intervening direction. This is a
small **scope restriction**, not a claim about the game's actual dash cooldown.
It rejects `Left,neutral,Left` and `Left,Right,Left`, while admitting the doorway's
single Left→Down change. Reject diagonals/actions. It does not by itself qualify
arbitrary idle durations, mode histories, event/actor encounters, or all possible
unique-direction permutations. Start from the authenticated ordinary checkpoints
and keep these remaining boundaries explicit.

### Exact doorway ownership boundary

From completed 1601 `(472,176)`, apply Left `[1601,1657)` then Down
`[1657,1682)`. Every step through completed **1681 `(392,209)`** passes the strict
profile, with no rejected material or corner response:

| Completed frame | Position | Walking result |
|---:|---|---|
| 1657 | (393,176) | Horizontal phase 0 gap |
| 1658 | (392,176) | Residual Left step after submitted Down |
| 1659 | (392,176) | Down setup, vertical phase 0 |
| 1660 | (392,177) | First Down step, phase 1 |
| 1680 | (392,207) | Down phase 1 |
| **1681** | **(392,209)** | **Down phase 2; exit has matched** |

At the handoff: delayed input=Down, active=Down, full vertical age=22 (compact
phase 2), used directions={Left,Down}, pending displacement zero. Bounding
origin `(384,193)` matches the parent's first exit record `(24,12,1,2)`.
**Transfer ownership after this resolved movement step**, not after another
ordinary-walking tick. The next completed frame, 1682, has player flags `$1411`
and resume `$84B975`; it is already transition-controlled. Its one-pixel delta
happens to equal the next walking magnitude and is therefore a dangerous false
positive if only coordinates are checked. The strict verifier rejects that frame
on the player collision-mode flags before attempting movement.

The new `doorway-approach/frames.csv`, snapshots, and `flat-qualification.json`
are reproducible ignored fixtures, authenticated like the existing captures.
The strict verifier fails immediately on unqualified input, cells, mixed pairs,
or changed collision-mode flags; the older diagnostic profile can still enumerate
rejected frames for research. Five ROM-free synthetic tests cover four-wall
blocking, floor steps, mixed/unknown/flagged rejection, and input admission:

```sh
python3 tools/movement-qualification/test_flat.py
```

This remains experimental qualification only. Native/Wasm production boundary
implementation and target validation belong to the parent core work; no target
support claim is inferred from this Python hypothesis checker or native oracle.

## Reproduction and provenance

Run from the repository root:

```sh
sh tools/movement-qualification/replay.sh
```

The script authenticates both private inputs, builds a standalone ignored probe,
and generates per-frame CSV, WRAM snapshots, instruction traces, CPU-register
logs, and per-frame qualification lists below `local/movement/`. Nothing extracted
is committed. Tools require the existing Rust/C++ oracle build prerequisites.
There is exactly **one boot per process**, with `std::process::exit` on completion
(the reference singleton/destructor is not suitable for repeated session boots).
No SRAM write, state injection, CPU-register mutation, or parent-checkout edit is used.
A second complete execution of `replay.sh` reproduced **45 regenerated CSV and
collision-trace WRAM files byte-for-byte** against the initial captures (local
`original-hashes.json` / `reproduction-check.txt`). Independent read-only review
of the static paths and final probe/verifier/report found no blocking issues.

- ROM SHA-256: `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
- SRAM SHA-256: `709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
- Oracle/source starting revision: `b25145e7a133b27576223a61e61f3e39a53a9eb6`.
- Fresh **SRAM-backed** boot: Start `[400,408)`, Up `[900,908)`,
  Up `[950,958)`, A `[1100,1112)`, then neutral through completed frame 1601.
- Required checkpoint: map F, entity `$1000`, position `(472,176)`.
  An empty-SRAM boot does **not** produce this checkpoint (it remains in map 4).
- Input numbered F is set **before** `run_frame`; its CSV row is completed F+1.
  CPU stop logs report the oracle's current frame counter, not a newly completed
  frame. A trace started after completed 1603 can inspect integration that will
  be visible in completed 1604 while its counter still reads 1603.

CSV fields are direct observations. `ptrx/ptry` are `$7F1010/12`, counters are
`$7E1028/2A`, `outx/outy` are `$7F100C/0E`, pending `dx/dy` are `$7F1018/1A`,
coordinates are `$7E1000/02`. `anim` is `$7F3014`; the historical column names
`joy/edge` mean the raw words `$7E0454/0456`, not a promise of generic held/edge
semantics. Signed outputs/counters are decimal; pointers/flags are hexadecimal.

## Exact measured ordinary-walking cadence

For a new cardinal input starting at F from qualified idle:

| Completed frame | Result |
|---|---|
| F+1 | Previous input still governs movement; no new displacement |
| F+2 | Direction/setup changes; stream displacement zero |
| F+3 | First displacement, magnitude 1 |
| F+4 | Magnitude 2 |

A direction change at input F similarly leaves **one old-direction movement**
at completed F+1, gives zero at F+2, then starts the new direction at magnitude
1 at F+3. It is not a smooth rotation of an existing velocity. The measured
Right→Left→Up→Down sequence resets cadence at each change, including reversal.

Release at input R retains one old-direction displacement at completed R+1,
then zero at R+2. At R+3 the ordinary idle resume is visible. A single-frame tap
from ordinary idle can select a direction but produce no displacement: by the
time the first stream step would run, release has canceled it.

Let age=0 on the completed setup frame F+2; increment each following completed
frame while that same cardinal direction remains active:

- **Vertical:** age 0 is zero; thereafter odd age gives 1, even age gives 2.
  No periodic setup gap was observed during the qualified vertical holds.
- **Horizontal:** use `phase=age mod 54`. Phase 0 gives zero; odd phases give 1,
  even nonzero phases give 2. Thus each cycle is a setup gap plus 53 movement
  frames, totaling **79 pixels per 54 frames** before collision.
- Apply sign for Left/Up. A wall does **not** freeze the movement-stream phase.
- The per-frame validation state is just delayed input, active cardinal, and
  age/phase, plus integer position, **within these qualified input/mode limits**.

For Left `[1601,1780)`: completed frames 1601–1603 remain `(472,176)`, 1604 is
`(471,176)`, 1605 `(469,176)`. Horizontal gaps occur at **1657, 1711, 1765**.
At 1657 position is `(393,176)`; 1658 is `(392,176)` if Left continues.
A perpetual 1,2 alternation predicts movement at 1657 and fails the red probe.

### Why that cadence occurs

- COP61 direction handling: `809C03`, with direction selection through `809C67`.
  It uses `$0454`, compares `$7F2014+entity`, and conditionally reloads streams
  through `80BBDB`; same selected stream is retained when already live.
- `80BBDB–80BC08` loads X/Y pointers and resets `$28/$2A` counters.
- `80F251`: each non-null stream counter decrements; a negative counter advances
  the pointer to a duration/value pair; negative duration is a loop/end marker.
  It writes `$7F000C/0E+entity`, adds to pending `$18/$1A`, and returns. X sign
  inversion tests entity `$08 & $4000`; Y inversion tests `$8000`.
- Observed horizontal value pointers alternate `$62DC/$62E0` in bank 7F;
  Down `$62C4/$62C8`, Up `$62D0/$62D4`. Loaded duration is zero: one value per
  frame, including blocked frames.
- The horizontal gap is **animation/script restart**, not a stream counter
  fractional-speed effect. `cadence-gap` traces `84A385` (COP8E), animation-end
  return via `80ED51/80ED72`, `84A387→84A377`, COP84 at `84A380`, then
  `80A295→80A2AB→80BBDB`, which reloads null initial streams. The next
  `80F251` therefore produces zero. Resume/timer again become `84A385/8`.
- Vertical resumes are `84A351` (Down) and `84A367` (Up); their corresponding
  loops do not introduce that measured 54-frame horizontal restart gap.

### Do not generalize this to arbitrary cardinal taps

`pulses/frames.csv` deliberately disproves that generalization: Left for one
frame at 1601, then Left `[1605,1607)` enters resume **`84A471`** by completed
1607. At 1608 output is **−3**, then −2, −2; motion continues after release.
This is an accelerated/dash action, not ordinary walking. The verifier explicitly
rejects it at 1608. Dash activation/cooldown, quick same-direction retaps,
braking, and recovery timing remain **unqualified**. An honest minimal core
must reject that input/mode history, not apply the walk formula to it.

## Four actual-player collision paths

Normal collision integration at **`80D0CF`** requires entity `$04 & 4 != 0`,
`$04 & 2 == 0`. The actual player has `$04 & $0400 != 0`, selecting the special
player branches, **not** the generic collision dispatch. Geometry measured in
`$7F1028/2A/2C/2E` is offset `(-8,-16)`, extent `(16,16)`.

The integrator stores the tentative position **before** calling collision,
processes **X first**, then Y using the resulting X. Carry set replaces the
axis with returned A; carry clear retains it. Pending displacement is cleared.
Original arithmetic is wrapping 16-bit; the experiment rejects map-boundary
samples instead of pretending to qualify wrapping/overflow gameplay.

Let x′=x+dx and y′=y+dy. For Y rows, x is already post-X-resolution:

| Direction / routine | Correction edge | New first sample | Old first sample | Perpendicular q |
|---|---|---|---|---|
| −X `80DA31` | L=x′−8 | (x′−8,y−16) | (x−8,y−16) | (y−16)&15 |
| +X `80DDA0` | R=x′+8 | **(x′+7,y−16)** | (x+7,y−16) | (y−16)&15 |
| −Y `80D295` | T=y′−16 | (x−8,y′−16) | (x−8,y−16) | (x−8)&15 |
| +Y `80D682` | B=y′ | **(x−8,y′−1)** | (x−8,y−1) | (x−8)&15 |

Old-edge type-6/7 special handling precedes ordinary new-edge classification.
With q=0 only one new tile participates. Otherwise the second sample is the
**next tile row** for X or **next tile column** for Y; it is not a generic scan
of the entity bounds. Helpers are `80E777→8D8D3D` and `80E750→8D8CE1`.
Positive sample subtraction is explicit in `80E7A4` / `80E796`.

### Material lookup and dispatch

For map F, `$0827=2`, `$085A=$01F8`, `$085E=$03F8`, `$0862=$07FF` specialize
`8D8C7E` to column=`(u & $01F0)>>4`, row=`(v & $03F0)>>4`,
index=`(row*32+column)&$07FF`. This is coordinate **masking**, not room clamping.
The verifier derives interior row width from `$0827*16` for each room snapshot.

`raw=LE16[$7EA000+2*index]`; stored type=`(raw>>9)&31`. Ordinary dispatch
reads at `$7EA001+2*index`: **raw bit `$8000` overrides the stored type with
class 3**. Therefore checking just the 5-bit type is incorrect.

For unflagged cells define O={0,2,22}, S={12,14}. With q=0 O passes and S blocks.
With q!=0, the first sample is upper (X) or left (Y):

| First / second | Action |
|---|---|
| O/O | Pass, retain tentative axis |
| S/S | Block, no perpendicular nudge |
| O/S | If q<8, perpendicular coordinate −=1; otherwise no nudge; block |
| S/O | If q>=8, perpendicular coordinate +=1; otherwise no nudge; block |

Ordinary blocking returns these coordinates (carry set), **after any nudge**:

| Axis | Snap condition | Snapped coordinate | Otherwise |
|---|---|---|---|
| −X | L&8 != 0 | (L&$FFF0)+24 | restore old x |
| +X | R&8 == 0 | (R&$FFF0)−8 | restore old x |
| −Y | T&8 != 0 | (T&$FFF0)+32 | restore old y |
| +Y | B&8 == 0 | B&$FFF0 | restore old y |

This **edge-bit-3 snap/rollback plus surviving perpendicular nudges** is why a
conventional AABB clamp is not a faithful replacement. Mixed-cell nudge rules
are statically decoded; the retained plain-wall fixtures do not exhaust every
q value or every pair in every direction.

| Direction | First table | Second tables (first O / type16 / S) | Correction | Interaction hook |
|---|---|---|---|---|
| −X | `80DC60` | `80DCA0 / 80DCE0 / 80DD20` | `80DB8A` | `80E47A` |
| +X | `80DFDC` | `80E01C / 80E05C / 80E09C` | `80DF00` | `80E5CC` |
| −Y | `80D542` | `80D582 / 80D5C2 / 80D602` | `80D401` | `80E1DF` |
| +Y | `80D8E8` | `80D928 / 80D968 / 80D9A8` | `80D7E2` | `80E32E` |

Each table has 32 bank-80 little-endian word targets, indexed by twice the type.
Hooks are **not assumed to be no-ops**: they inspect `$097C`, facing, material,
and input/event conditions. The bounded model validates positions only for the
recorded neutral/action-free wall interactions, not their full event state.

### Actual register/WRAM witnesses

These fresh-input traces stop at entry, ordinary dispatch, correction, and return.
X=`$1000` at entries and returns; return carry is set. Trace counter is shown;
the resulting position becomes visible in the following completed frame.

| Trace | Counter | Tentative player | New raw cell/type | Return A |
|---|---:|---|---|---:|
| `trace-right` | 1603 | (473,176), dx=1 | `$1C0B` / 14, offset `$02BC` | 472 at `80D150` |
| `trace-left` | 1723 | (295,176), dx=−1 | `$1C0A` / 14, offset `$02A2` | 296 at `80D155` |
| `trace-down` | 1625 | (472,209), dy=1 | `$1C0E` / 14, offset `$037A` | 208 at `80D186` |
| `trace-up-central` | 1713 | (392,95), dy=−1 | `$1C20` / 14, offset `$0130` | 96 at `80D18B` |
| `trace-type12` | 1725 | (311,158), dy=−2 | `$1872` / 12, offset `$0224` | 160 at `80D18B` |
| `trace-map10-up` | 1844 | (360,351), dy=−1 | `$1C21` / 14, offset `$052C` | 352 at `80D18B` |

Type-12 and q!=0 full/full blocking are thus also reached by the real player,
not merely named in a static table.

## Qualified trajectory fixtures and falsification

The tracked verifier is an **experimental hypothesis checker**, not an API or a
safe general simulation. `collide` is a pure integer/data function. It compares
predicted stream outputs and per-step positions to the actual player, rejecting
unqualified old/new cell types and flagged cells. It uses the end snapshot's
collision grid within one-room test segments, so this qualifies the observed
static paths, **not arbitrary dynamic collision-grid updates**.

Each fixture has every completed frame in `frames.csv`; `qualification.json`
contains CSV/grid SHA-256 plus explicit matched/rejected completed-frame lists.
The transition preamble is retained for reproducibility but excluded from map10
movement verification, which begins at completed **1801, (392,353)**.

| Fixture | Position comparisons | Endpoint / purpose |
|---|---:|---|
| `wall-Left` | 189 | F (296,176), west full wall, multiple horizontal cycles |
| `wall-Right` | 189 | F (472,176), east full wall from the first step |
| `wall-Down` | 189 | F (472,208), south full wall |
| `up-central` | 179 | F (392,96), north full wall after walking left |
| `up-type12` | 179 | F (311,160), unflagged type-12 north wall |
| `map10-Left` | 199 | 10 (360,353), west wall, unaligned perpendicular edge |
| `map10-Right` | 199 | 10 (392,353), east wall, unaligned perpendicular edge |
| `map10-Down` | 199 | 10 (392,464), south wall |
| `map10-up-wall` | 249 | 10 (360,352), north wall away from doorway |

Those **nine complete segments match all 1,771 position steps** plus their
stream outputs. Two diagnostic fixtures add 71 matched steps and **187 explicit
rejections**: initial `wall-Up` meets flagged raw `$9845` (stored type12, dispatch
3; 178 rejected steps), while `cadence` meets type16 (9 rejected steps).
Do not label those whole diagnostic trajectories supported. `map10-Up` returns
through the doorway into F and is likewise **not** a movement-only fixture.

Red→green evidence: `NAIVE_CADENCE=1` intentionally models uninterrupted horizontal
alternation and fails at completed **1657**; the measured 54-frame restart rule
passes `verification.log`. The accelerated `pulses` fixture must fail ordinary
walking at 1608. This is experimental differential testing, not production TDD;
no production tests/core were added.

## Type16 and honest production boundary

The four static player tables agree on a **distinct partial class P=16**:
P/P blocks; O/P behaves like O/S, P/O like S/O; crucially **S/P chooses the
positive-nudge path and P/S the negative-nudge path**, unlike S/S. With q=0,
P blocks. This does not establish a universal half-tile geometry. The diagnostic
cadence trajectory actually meets it, but complete type16 trajectory qualification
was deliberately not folded into simple-floor support.

A minimal pure core can honestly start with the authenticated idle checkpoint,
measured ordinary cardinal stream cadence, integer coordinates, and the interior
O/S sampling/correction rules above for the nine retained segments. It needs
explicit input-history/mode admission and interaction/transition handoff; a pair
of coordinates and a constant speed is insufficient.

**Reject or report unqualified**, rather than silently approximate: type16,
raw bit `$8000`, all other materials (especially 6/7 slopes), different entity
geometry/flags, diagonals or opposing buttons, rapid same-direction retaps/dash,
actions/combat/jump, dynamic actors/pushing, interaction-hook effects, map edges,
coordinate overflow, scripted movement, transition frames, and unmeasured idle
or mode histories. The flat rule formulas are static evidence; only the listed
trajectories are reference-qualified. This issue remains open for production
integration and broader qualification owned by the parent work.
