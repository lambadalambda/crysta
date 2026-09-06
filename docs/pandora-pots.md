# Bounded Pandora pot component

[Issue](../meta/issues/qualify-pandora-pot-actions.md). Scope: source FA/FB pots in
**direct-branch C**, including the miss and two real cellar-door contacts. This is
not the Pandora story graph, a combat system, or a general projectile simulator.

## Evidence status and reproduction

The retained original empty-SRAM journey supplies **2,184 uninterrupted frame
calls in three segments**. Each segment starts before its sampled lift A and
continues through lift, carrying, throw, flight, and neutral player recovery:

| Segment | Inclusive frame calls | Source lift | Throw | Actual contact |
|---|---:|---|---|---|
| `miss` | 18833–19372 | FA, (104,352) Left, cell (5,21) | (136,368) Up | **none**, counter remains 0 |
| `fa-hit` | 20031–20938 | FA, (40,352) Right, cell (3,21) | (184,368) Up | **20778**, counter 1 |
| `fb-hit` | 21470–22205 | FB, (88,352) Left, cell (4,21) | (184,368) Up | **21985**, counter 2 |

The optional Rust test compares **every player position, facing, control word,
and admitted pot flight coordinate**, with component snapshot/continuation at
every tick. There are no position corrections or native restores between frames.
These are three independently initialized **test segments**, not a claim that the
component implements the ordinary/story route between them. The entire original
log, command/capture schedule and all segment checkpoint WRAM are hash-pinned.

Per-frame logs contain selected WRAM fields, including actor positions/scripts,
not full per-frame WRAM. Full WRAM exists at checkpoints. In particular **0988,
098A/098F records, 09C7, 0640 and collision grids are checkpoint fields**, not direct
per-frame observations. Source execution boundaries plus actor-script transitions
support the internal release/reservation distinction below; do not call those
raw held-slot values directly observed on every frame.

The original observer synchronizes after every command; these synchronizations
are not passive frame observations and remain part of the exact recipe. The
projector uses the source owner's unchanged timeline checker and pins the complete
original log. It does not accept a different observer/capture schedule by dropping
rows or weakening comparisons.

**Parent update:** source diagnosis (`5b7b88b`, parent `42fe162`) found original,
parent, and another same-binary fresh run identical in frame logs, native-state and
non-pixel captures/provenance. The few intermittent zero-filled `.pixels` regions
come from the shim's unsynchronized asynchronous video-publication race. Pot
qualification does **not read pixels**. This is not a claim of uniform trustworthy
native RGB or acceptance of the newly fixed observer replay; parent owns that
renewed strict gate. Existing source references are not modified here.

Run from this worktree (raw output stays ignored):

```sh
mkdir -p local/pandora-pots
ROM='/Users/lainsoykaf/repos/terranigma/local/Tenchi Souzou (Japan).sfc'
ORIGINAL='/Users/lainsoykaf/repos/ilar-task-pandora-source/local/pandora-qualification'
python3 -B tools/pandora-pot-qualification/source_contract.py "$ROM" > local/pandora-pots/source.json
python3 -B tools/pandora-pot-qualification/qualify.py \
  "$ORIGINAL/journey.jsonl" "$ORIGINAL/journey" local/pandora-pots/fixtures \
  > local/pandora-pots/reference.json
python3 -B -m unittest discover -s tools/pandora-pot-qualification -v
python3 -B -O -m unittest discover -s tools/pandora-pot-qualification -v
PANDORA_POT_FIXTURES="$PWD/local/pandora-pots/fixtures" \
  cargo test -p room-core --test pots --test local_pots
cargo test -p room-core
```

Both Python CLIs require exact equality to their pot-owned committed references.
No pin-generation/accept-different-run switch exists. The native test also verifies
its own CSV/grid hashes using host `shasum`; no runtime **or dev dependencies** are
added to the `no_std + alloc` core. Missing optional local fixtures print a skip;
setting the environment variable makes absent, corrupted or wrong inputs fail.
No new emulator session or diagnostic restore was needed for this component.
A failed projector can leave partial local output; discard it. Successful exact
verification and the Rust fixture hashes, not file existence, establish acceptance.

## Source facts and limits

`source_contract.py` authenticates Japanese ROM SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
Its typed operand checks and window hashes extend, rather than alter,
`tools/pandora-qualification/source.json`.

### Source cell and held record

- `$87C7F1..C8F6` recognizes FA/FB source tiles; `$879683` selects **098A for FA,
  098F for FB**, and stores the reservation in **0988**. Other grabbables fail
  closed in this component.
- `$8DB8A5..B915` loads map-specific records. Map C's entry at `$96DDC5` points to
  **$96E1A6**. Both replacement words are zero, selecting **F8 fallback** in the
  lift routine. This is source-backed, not a WRAM initializer or universal rule
  for every map's FA/FB objects.
- `$879724` snaps the object origin to `(candidate_x & FFF0)+8`,
  `(candidate_y & FFF0)+16`. COP45 at `$879705` patches its source cell using the
  decoded replacement tile metadata. Do not preserve the old pot collision type:
  the observed full words **18FA/18FB become 00F8**, not 18F8.
- `SourceObject` contains original cell index/raw word and the decoded replacement
  word. The canonical catalog is sorted and unique, maximum 64 entries. Membership
  is retained by **source cell**, not a number of pots used. A consumed source
  cannot be relifted, even after release, recovery and snapshot restore.

### Carrying is not ordinary walking

The carry controller writes **0980=0020** (e.g. `$84ACE1 COPCB $20,$84B500`).
Carry loops use **COP83**, table1 sequences 09/0A/0B at
`$84B509/$84B51A/$84B52F`. They do **not** reload COP84's null horizontal stream.
Native Right66 is continuous past the ordinary walker's 54-phase restart.

A privately reviewed helper shares the existing collision, input delay, and
conservative 11-tick onset refusal, changing only the horizontal cadence selector:
held motion uses setup then **1,2,1,2…** in every cardinal. Ordinary public walking
still has its existing 54-phase horizontal cadence and snapshot version. Carry
entry requires settled neutral/onset-expired state; no live ordinary phase is
reinterpreted as held phase. Neutral-only lift/throw presentation retains that
settled history through recovery. Moving actions, diagonals, dash/attack/jump,
quick same-direction acceleration and input during presentation remain refused.

The retained native carry segments cover Down, Right, Up, turns, release,
blocking, a corner nudge and the >54-phase Right loop. Left carrying uses the
source's corresponding signed stream/controller and shared collision policy;
it is **source/synthetic coverage, not a retained native Left-carry segment**.
Do not infer whole-room unrestricted action admission from these comparisons.

Direct C retains source residents at **(152,368), (216,368), (184,416), (56,384)**.
The native first carry blocks at (136,368), rather than passing through the first
resident. Parent/compiler collision must include their occupancy and the table;
never substitute the refusal branch's cleared room. `Admission.room` includes a
source-qualified sample halo. Passive flagged collision is separately admitted
for these held steps because **0020 & 0050 == 0**; this does not generalize ordinary
walking admission to arbitrary action hooks.

### Exact cellar boundary: type5 and type29 are not ordinary floor

The four first/O/P/S movement tables per direction are source-pinned. Type5's
entries match P16, not solid. The unflagged **Up** action hook excludes types1/9/A
then falls through `$80E22C → $80E2E6`; the latter's `0980 & 0050` gate returns
for admitted 0020/00A0 movement.

For the exact source cell **(11,21)** only, the private collision view admits:

- **0B81** as Partial for Up contact;
- **3ACB** (stored type29) as traversable for Up;
- **BACB** is untouched: temporary flagged occupancy still blocks.

Both aliases require `cellar_up_lanes` and the **old delayed Up input governing
this collision tick**, not newly submitted input. Other words/cells/directions
retain the normal fail-closed classifier. Authoritative source raw words stay
unchanged. Private classifier surrogates are never patch events or exit metadata.

Important source counterexample: Right S-first table **$80E09C** maps type29 to
**$DEFC**, while type0 maps to **$DEE6**. Thus even 15 matching entries do **not**
justify a global type29→floor alias. All four Up entries match open; the bounded
Up staircase boundary is sufficient. Parent owns C exit `$818DF1`, selector14,
transition timing and stopping/handoff—traversable classification is not a stair
transition implementation.

### Release, flight and contact admission

Only **Up from (136,368) or (184,368)** is admitted, after settled carrying, with
source door words **1D80 or 1DA7 / 0B81** at (11,20)/(11,21). The compiler's
`cellar_up_lanes` assertion additionally certifies intact native intervening lane
geometry and direct occupancy. Freeze that immutable pot collision/lane view for
the throw; parent presentation/door patches may progress separately. Unsupported
origins/directions or changed source door geometry fail atomically, not as an
invented ballistic miss.

A is sampled for one tick before the action begins. With action-start tick = 0:

| Boundary | Tick | Meaning |
|---|---:|---|
| Lift starts | 0 | Consume source cell, acquire held reservation |
| Carry ready | lift 23 | Native player returns to carry idle |
| Throw starts | 0 | Still in hand, no hit |
| Release / first flight | throw **18** | Up offset −8 at `$84C4A5`, selector7C; first world sample `(launch_x,357)` |
| Actual door contact | throw **19** | Exact admitted pot point **(184,354)**, Up, callback enabled; semantic hit once |
| Flight ends at door | throw 22 | Four admitted samples 357,354,351,348; break, **0988 clears at $84BFE8** |
| Flight ends in miss lane | throw 27 | Nine admitted samples 357 through333, stride−3; no door contact |
| Player recovery | throw **32** | Ordinary idle script reached; parent story input may still be disabled |

`held_slot_in()` means **still in Ark's hands**; it becomes None at release.
`reserved_slot_in()` projects **0988**, which stays 098A/098F during flight until
break. These are deliberately different. Fragment coordinates/elevation are not
invented from actor world Y: flight output stops at the proven pre-fragment end.

The actual door callback is **COP65 → $88AB81**, which increments BCD 0640.
**$88AB46 is not a pot-hit test**: it is a player-Up accumulation/request route
using 0642. The general collision dispatcher that invokes COP65 is **not fully
qualified here**. Instead the component admits the exact contiguous flight/contact
boundary above, with owned/released pot, immutable source lane, direction,
coordinates and parent callback gate. It does not use player interaction distance,
a pot counter, destruction time, or consumption as evidence of a hit. The miss
at (136,368) never matches the admitted flight contact point.

This is the intentional **narrow native admission** replacing an unjustified
broad physics model. Arbitrary collision/hit rectangles, other directions/origins,
intervening moving objects and input during flight require new source/native work.
Do not represent this API as a general pot or projectile engine.

## Parent/compiler and sprite-owner contract

The new public module is `room_core::pots`:

1. Compile immutable raw collision from the ROM/static scene and direct-branch
   resident occupancy. **Do not use the test WRAM grid as production initialization.**
   Supply a stable sorted `SourceObject` catalog with full reconstructed 00F8
   replacement words and a source-qualified sample halo.
2. Construct `PotState` from the parent's ordinary `WalkingState` and facing on
   room admission. Do not reconstruct it at every pot: its consumed-cell ledger
   belongs to the whole room visit. Player positions thereafter change only through
   collision stepping; there is no teleport/captured-memory initializer API.
3. Feed neutral/cardinal/one-frame A to `step`. Errors leave the entire component
   byte-identical, including queued input, ownership and movement. Unsupported A
   samples are rejected before they can poison a delayed action.
4. Apply `consumed_cell` to the parent semantic/rendering grid. `held_changed`
   signals hand ownership; `flight` supplies admitted world samples. On `door_hit`,
   parent invokes its finite source door reaction/counter sequence. Parent owns
   $28/$292 callback admission, all requests, temporary occupancy/palette/input
   masks and the entire story graph. `control_restored` is not a story grant.
5. Snapshot the **40-byte POT1 component encoding** together with parent room,
   catalog/content identity, lane/callback admission and parent reaction/counter
   state. Seven reserved bytes must be zero. Held cadence cannot restore an
   ordinary horizontal phase 3–53. Restoration is structural continuation, not
   evidence provenance or authorization for a qualifying native restore. Do not
   replay an event into an unrewound parent counter.

For sprite ownership, `phase`, `phase_tick`, `facing`, read-only `walking`, the
hand/reserved slot getters and `Output.flight` supply semantic selection needs.
No assets crate changes or copied graphics are involved. Source selector needs:

| Phase | Ark selector | Pot selector / source |
|---|---|---|
| Lift | directional lift presentation; native horizontal resume84BE9D/BEA1 | COPD8 **A2C000**; COP82 2B/2C/2D |
| Held idle Down | table0 seq03 | COP82 19,0 at84C3BF |
| Held idle Up | table0 seq04 | COP82 1A,0 at84C3CD |
| Held idle horizontal | table0 seq05, source B6/B7 mirror | seq1A, facing mirror variants |
| Held walk Down/Up | table1 seq09/0A, COP83 | COP80 1D at84C41A |
| Held walk horizontal | table1 seq0B, COP83, mirror | COP80 1E at84C423 |
| Throw Down/Up/horizontal | table3 seq0F/10/11, COP84 movement0 | COP80 2E/2F/30 at84C47F dispatch; only Up trajectory admitted |
| Flight / break | preserve throw/recovery phase above | Up COPAF 7C →84C697/84C721; fragment source84C7B8, outside core world-flight output |

These are source descriptors/animation needs, **not decoded composition addresses
or a claim that all listed directional actions are admitted**. Sprite owner owns
composition, lift poses, held alignment, elevation and fragments. Background owner
owns source/static grids and cameras. Existing slice/conversation/transition/house
integration, profiles and live preview8765 are untouched.

### Component acceptance status

The parent has independently reproduced and accepted this component and the
strict direct Pandora source route under `headless-sync-video-v1`. This
supersedes earlier pending observer/parent handoff statements, **not** the
source-only coverage or fidelity/admission limits in this contract. Portable
Pandora integration remains open and the live host stays on the house profile.
