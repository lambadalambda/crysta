# Qualify the route from the Pandora tour to the first tower

## Summary

Extend the accepted input-only reference journey from its current endpoint to
the first tower transition, and record the source contract for everything the
segment requires. This is the evidence gate that unblocks the remaining
portable work in
[Complete the Crysta and Pandora vertical slice](opening-vertical-slice.md);
it is not portable implementation.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Implement bounded Pandora story state and continuation](port-pandora-story-state.md)

## Requirements

- Extend the same empty-SRAM Session used by `tools/pandora-qualification/route.jsonl`,
  preserving its commands and observation schedule, and continue from the named
  `pandora-tour-control` witness. No warps, state restoration, memory patches or
  save loading in the qualifying journey.
- Record the required maps, exits, interactions, request/choice boundaries, event
  flag changes and immediate player/world effects from that witness to the first
  tower transition. Separate required story steps from incidental movement.
- Establish explicitly whether the segment requires equipment acquisition,
  combat, or a frozen-town field return, since all three are currently outside
  every accepted claim. Record what is mandatory versus avoidable rather than
  assuming either.
- Name a stable post-transition control witness the way `pandora-tour-control`
  names the current endpoint, with its completed frame and position.
- Pin source bytes/operands and retained semantic checkpoints under the current
  `headless-sync-video-v1` observer epoch. Source metadata must not initialize
  production state from captured WRAM.
- Extend the existing checker with mutation controls that fail when the new
  semantic evidence is altered, and keep it passing normally and under `-O`.
- Keep raw ROM, text, graphics, traces and captures ignored under `local/`.
  Commit only tooling, selected source metadata and hashes.

## Acceptance Criteria

- A reproducible input-only fresh route reaches a named first-tower-transition
  control checkpoint, with an independent replay confirming the declared scope.
- The prerequisite and effect contract is source-backed, with controls
  distinguishing missing prerequisites from reordered or omitted interactions
  where practical.
- The checker rejects mutated semantic evidence; disabling a validator produces
  the expected failures before green is restored.
- The asset, runtime and rendering work the segment newly requires is written
  down as bounded follow-up scope before any portable integration begins.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Complete the Crysta and Pandora vertical slice](opening-vertical-slice.md)
- Original endpoint per
  [the source/reference contract](../../docs/pandora-progression.md):
  `pandora-tour-control`, map `$41`, completed frame 41344, `(136,208)`; the
  final two-axis controls finish at completed 41788, `(120,192)`. Later field
  return, `$21/$23/$FE`, equipment acquisition and combat are explicitly not
  claimed there.
- Needs the owned Japanese ROM. RE discovery may precede executable tests, but
  retained evidence and checker work must still show red → green mutation
  controls.
- If the segment turns out to need combat, the actor/combat port becomes a hard
  prerequisite for the slice rather than a parallel track. Determining that is
  part of this issue's value, so resolve it early and report it.

## Completed qualification

The [full source/reference contract](../../tools/tower-approach-qualification/TOWER.md)
now reaches **`first-tower-transition-control`, completed65332, map `$101`,
`(128,623)`**. Horizontal and vertical inputs then prove control; neutral stability
ends at **65608, `(112,607)`**. This is the interior entrance, not just map100.

- One empty-SRAM input-only boot preserves the complete Pandora and frozen-return
  prefixes:615 commands,616 checkpoints,58,808 explicit route frames. An independent
  fresh replay matches **all4,313 files** and every byte of the76,532,961-byte log.
  No warp, memory patch, restoration, save loading or filtered observations.
- The source-backed route separates spear consent, collection and inventory;
  frozen return FE/23; Elder initiation21 versus completed response296; town3C;
  distinct world-map movement; tower introduction100 and guardian completion115;
  and the actual source exit100→101. Alternate Elder/guardian answers are
  source-only where not exercised natively.
- Acquisition and frozen return are part of this route. Equipped weapon/armor
  words remain zero throughout the tower extension while inventory retains81/1.
  **Neither equipment selection nor combat is required for the entrance endpoint.**
  This does not qualify equipment UI, HP/XP, enemy behavior or later tower progress.
- All58 qualification tests pass normally and under `-O`; the tower semantic
  no-op mutant yields168 failures. Artifact-gate, inventory-slice and equipment-width
  pipeline controls fail as intended. Both final checker modes pass on the original
  and independent captures. Independent source, correctness/DRY and executable
  reviews found no blockers, including direct equipment read/write verification.
- Release workspace tests, three local collision tests and16 local world tests
  pass. Candidate geometry stays24/24 outbound,23/23 returns; production stays19/24.
  Missing first8/Partial8/Solid8/slope8 witnesses remain separate open issues.
- [Bounded asset/runtime/render follow-up](../../tools/tower-approach-qualification/TOWER.md#bounded-portable-follow-up)
  is recorded before integration. This closes the native evidence issue, **not**
  the portable vertical slice. Raw artifacts remain ignored under `local/`.

### First segment

### Corrected blocker: spear and frozen return reproduced

The earlier **terminal-hall/unreachable-arches conclusion is refuted**.
A turning corridor, missed by the old sweeps, reaches `(72,80)`; A enters map42.
The hall controller being idle does not disable the door interactions. Native
input follows the current directional-model navigation hint exactly, without any
native position initialization from that model.

The [new bounded qualification](../../tools/tower-approach-qualification/README.md)
preserves the accepted prefix and extends the same boot session through spear
refusal/retry, consent `$241`, separate collection `$242`, actual inventory
`$7F8048/49 = 81 01`, map21 return and completed `$FE/$23` effects. Two-axis
control and neutral stability end at **53586, map21 `(120,448)`**. Independent
fresh replay matches all3,508 artifacts and every log byte. This qualifies
acquisition, **not equipping**, town/overworld traversal or the first tower.

Source/checker work uses red→green mutation controls and retains every failed
approach/misnamed discovery checkpoint. No portable behavior or production
collision admission is widened. Onward stair return, Elder encounter and tower
entrance are now qualified by the second segment above; the first segment's
recipe and pins remain unchanged.

### Earlier investigation (terminal-hall inference superseded)

The following is historical investigation, not the accepted departure contract.
Negative sweeps were not exhaustive connectivity proofs; the turning corridor
above refutes the terminal-room interpretation.

- The accepted baseline reproduces on this machine: `replay.sh` produced a fresh
  root of 2,682 artifacts and the strict checker passed normally and under `-O`.
  Source, checker and epoch suites pass 6/13/11 in both modes. No pins touched.
- [Discovery harness](../../tools/pandora-tower-discovery/README.md) added.
  It replays the accepted prefix once and holds the probe's stdin open, so
  exploration past the endpoint costs seconds per command instead of a full
  boot-to-endpoint replay. Same discipline as the accepted route: one empty-SRAM
  Session, input-only, no warp, patch, save or state restore.
- First session: 149 commands, frames 41,788 → 53,634, itinerary retained as
  `discovery-route.jsonl`. **The continuation did not fire.** Final flags are
  exactly the documented endpoint set, with no `$23`, no `$FE` and no map change.
  The reachable area, its two pockets, the blocking floor object and the
  arches thought unreachable at the time are tabulated in the harness README.
- Behavioral finding worth carrying into any future sweep: `Start` opens an
  invisible state that swallows all input until `Start` is pressed again, while
  leaving map, position, flags and script unchanged. It silently invalidated the
  first sweep, which read as uniform geometry blocking. `sweep.sh` now stops when
  `control` does not reach `160` under a held direction.
- Breakpoint session (`trace-probe.rs`, `Session::trace_until_pc`) found the
  gate. Across ~2.4 s of idle at the endpoint — the probe's 2M-instruction cap,
  about 144 frames, which is the binding budget — `$88AF3F` is not reached, and
  neither is the `$8087C2` position predicate, so nothing is polling for a spot.
  `$88AE64`, the controller that ran the box sequence and the tour, has ended.
  The only live entity is the guide actor in slot `$1040`, parked on `COP $91`
  at `$89D2E5`, which dispatches through
  `$0084D4` to `$80:A395`: `TYX` / `JSL $80ED75` / `BCS $80A396`, looping on the
  `JSL` while carry is set.
  Each link is confirmed by a live trace (`A=X=0x0122`, `Y=0x1040`, return
  address `$89D2E7`).
- The trace probe's endpoint was verified against the accepted capture before any
  of that was trusted: identical frame, map, position, facing, control, script
  and flag set.
- A rerun sweep at 13-19px rows with `control` verified at 160 throughout found
  no progression either, so the positional hypothesis is tested and negative
  against 32px-scale trigger boxes. It remains sampling, not proof. Movement also
  needs ~7 frames of held input before admission; shorter row steps silently move
  nothing, which is what broke the first attempt.
- Spun out: [Correct the COP service table bound](correct-cop-table-bound.md).
  All eight selector bits reach the dispatch index, so the documented
  125-selector bound in `docs/rom-map.md` is a scan artefact, and `$80:A395` is
  unclassified.
- **Correction:** `$80ED75` is not a gate, and the earlier note here calling it
  one was wrong. It is an animation-frame stepper. Its carry-set return means it
  hit the list terminator, zeroed the step at `$80ED51` and wants calling again;
  `$80A395`'s `BCS` exists to skip that terminator inside one call. Predicted by
  evaluating the routine against captured WRAM (the guide's list is in `$7E`
  at base `$7000`, selector 3, step 16, and `$7E:7070` is `$FFFF`), then
  confirmed by profiling: across two frames, three of four calls fall through
  and one takes the reset path.
- `COP $91` ends in `PLA / PLA / RTL`, yielding to the actor dispatcher without
  advancing the actor's script pointer, so field `$0A` stays at `$D2E5` forever.
  The guide loops by design; it is idle-animating, not waiting on the player.
- Corrected net: the tour controller is **quiescent, not gated**. This does not
  disable room-door interactions or rule out departure; that earlier inference
  was false.
- The `$2E`-vs-`$2F` branch hypothesis is **refuted**. The archived discovery
  reference's own retained points end its `$2F` refusal journey in map `$41` at
  `(120,192)` — the same room and position as the accepted `$2E` endpoint — with
  `$243`/`$244` set, no `$23` and no `$FE`, and the same forced
  `41 → 44 → 42 → 43 → 41` tour path. It carries `$2F`, `$3F` and `$42` instead
  of `$2E`, and still lands in the same hall. Note that the discovery
  observer epoch is unrenewed, so this is the best available semantic evidence,
  not a renewed claim.
- Former open question: how does the player leave map `$41`? Resolved by the
  turning corridor and arch interaction, not an interrupted tour controller.
- Confirmed directly: replaying the discovery route under the trace probe
  reproduces its semantic endpoint (frame 48,259, map `$41`, `(120,192)`, flags
  `$20,$22,$26,$27,$28,$2F,$3F,$42,$FB,$243,$244,$292`) and breakpoints
  identically to the accepted branch. `$88AF3F` is not reached; `$89D2E5` runs
  every frame from `$80C745` with the same registers. No pixel claim is renewed.
- Three tour-controller hypotheses tested; none survives. **Nothing polls:** over
  two frames at the endpoint, bank `$88` executes zero instructions, so the
  controller bank holding `$88AF3F` is dormant rather than un-triggered; bank
  `$89` runs nine addresses once per frame each, all actor ticks. **Nothing is
  missing:** the controller actor moves `$89D48B` → `$89D49B` at `tutorial-052`,
  from the last request script to an `RTL` stub, right after `$89D495` sets
  `$244` — an inert ending by design. **Nothing was interrupted:** replaying the
  prefix truncated at `tutorial-052`, with the trailing directional presses
  absent, then idling 10,000 frames with zero input, leaves state unchanged.
- Recorded against [decoding map and collision
  formats](decode-map-collision-formats.md): the community overlay code
  `(raw >> 8) & $FE` does **not** predict passability in map `$41`. Calibrated
  against measured movement it implies an open column at `x=96` from `y=176` up
  to `y=80` and an open row `y=80` reaching the arch centres; a column-wise probe
  refutes that, with rightward limits `x=72` at `y=144`/`y=156`, `x=120` at
  `y=176`, and full traversal only at `y=192`. A concrete counter-example for
  that hypothesis.
- Methodological note: the serpentine sweep only tested vertical movement at each
  row's extreme, so it could not have found an interior column. Column-wise
  probing is the missing complement; it confirmed the same envelope here, but the
  gap was real.
- Superseded next-step inference: the interior was thought terminal. No earlier
  route repair was needed; normal input through the missed turning corridor
  reaches the spear and the later continuation.
