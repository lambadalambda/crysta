# Trace and qualify the movement collision predicate

## Summary

Find the routine that decides whether the player may enter a cell, and decode
the collision semantics it actually uses, rather than inferring a mask from
data. The published `(raw >> 8) & $FE` community overlay code is an explicit
hypothesis in [the map format record](../../docs/maps.md), and it is already
refuted as a passability predicate for map `$41` by measured movement in
[the tower-approach route](qualify-tower-approach-route.md). Settle it with
source evidence.

## Dependencies

- [Decode map, metadata, and collision formats](decode-map-collision-formats.md)
- [Qualify static map loading and decode the cavern layer](static-map-cavern.md)

## Requirements

- Locate the admission predicate by differential CPU trace: one frame where a
  held direction moves the player, one where the same input is refused at the
  same facing, with everything else equal. The executed-address difference
  bounds the candidate code.
- Disassemble the isolated routine from the byte-exact Japanese reconstruction
  and state what it reads: which buffer, at which stride, with which mask, and
  how the result maps to admitted/refused.
- Distinguish the cell attribute from the dynamic/object bits the runtime layer
  also carries, and say which participate in the decision.
- Validate the decoded predicate against already-qualified movement, at minimum
  the house trajectories in `crates/room-core/tests/local_trajectories.rs` and
  the measured map `$41` envelope recorded in the tower-approach route. A
  predicate that cannot reproduce both is not qualified.
- Correct `docs/maps.md` so the community mask is described by what the trace
  shows, not by its published label, including where it coincides and where it
  does not.
- Keep raw ROM, traces and captures ignored under `local/`; commit tooling,
  decoded tables and hashes only.

## Acceptance Criteria

- The admission predicate is named by address and explained from disassembly,
  with a live trace confirming it is reached on both the admitted and refused
  paths.
- A pure decoder reproduces admitted/refused for every qualified sample, with
  mutation controls that fail when the mask, stride or branch sense is altered.
- The map `$41` envelope measured by the tower-approach sweep is predicted by
  the decoder, or the disagreement is explained by a named additional input.
- `docs/maps.md` no longer presents an unverified mask as the collision code.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Parent: [Decode map, metadata, and collision formats](decode-map-collision-formats.md)
- This is the binding constraint on
  [porting map loading and collision](port-map-loading-collision.md), which the
  vertical slice needs; the portable core currently relies on per-room
  qualified collision responses rather than a decoded format.
- It may also explain the tower-approach impasse. Map `$41` reads as a terminal
  hall with no reachable exit, and the same issue records that the community
  code predicts an open column and row that column-wise probing refutes. If the
  passability model is wrong, "no reachable exit" is a conclusion drawn from a
  wrong map.
- `Session::trace_until_pc` with an unreachable target and a large instruction
  limit yields a full bounded instruction trace for a frame, which is the
  mechanism for the differential step. Prior probes show ~2M instructions covers
  roughly 144 frames, so a single-frame diff is well inside budget.

## Progress: attribute field decoded, four base values measured

The attribute field is now separated from the bit that is not attribute. The
qualified loader transform in [static maps](../../docs/static-maps.md) writes
`index | ((attributes[index] & $7F) << 9)`, but that same document records word
bit 15 being set later during play. Bit 15 is attribute bit 6, so on a runtime
layer only bits 9..14 are unambiguous. `MapCell::base_attribute()` returns
those; `attribute()` inverts initialization and is documented as such.

This matters concretely: map `$000F` has exactly two cells with the dynamic bit
set, `$9ce8` and `$9845`. A seven-bit reading turns them into apparently
uncovered "attributes" 76 and 78 when they are really 14 and 12 — one of them
the most common wall word in the room, which would have resolved as unknown.

**The community mask reads the correct field.** It is exactly `2 * attribute`
for every 16-bit word. What it never carried is a mapping to movement.

Measured from two input-only sweeps of map `$000F`: 3,224 frames, 1,075
distinct positions covering **66 cells**, 17 distinct sustained contacts.
`derive.py` solves for the collision reference offset instead of assuming one
and reports per-offset vote counts:

- walkable `{0, 22}` under all 192 consistent offsets;
- solid `{12, 14}` under 156, `{2, 12, 14}` under 36;
- **attribute `2` is therefore unresolved**, from a single down-stall where
  `dx >= 5` rounds into a different column.

`12` and `14` each block from opposing directions, which rules out reading them
as one-way ledges. The `STALL_FRAMES = 20` threshold is measured-robust: the
longest stretch after which movement resumed is 2 frames, and the partition
holds for any threshold in `8..=40`.

Recorded in [collision](../../docs/collision.md); `docs/maps.md` no longer
presents the mask as the open question.

## Remaining

- Attribute `2` needs one more contact to resolve.
- The reference point is bounded to `dx` in `-8..=7`, `dy` in `-12..=-1`, not
  pinned. The `dx` span is exactly one cell period because every horizontal
  stall halts at `x % 16 == 8`, so those contacts carry no sub-cell
  information; pinning it needs contacts at differing sub-cell phases.
- Coverage is four base attributes over 66 cells, ~12% of one room's floor, in
  one map. `5`, `16` and `29` in the same map are uncovered.
- **The admission routine is still untraced**, so this remains inference from
  movement. The differential trace in the requirements above is the next step
  and is what would state the offset and any second input from source.
- The layer is snapshotted once per map, so in-run drift is not excluded —
  which matters precisely because bit 15 is written during play.
- The map `$41` check that motivated this issue needs the tower prefix replay.
