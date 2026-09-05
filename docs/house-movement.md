# House movement: corner qualification

This is incremental progress toward [starting a new game and exploring Ark's
house](../meta/issues/start-and-explore-arks-house.md), **not completion of that
goal**. Fresh new-game initialization and repeatable input admission are separate
open work. The existing preview still starts at a saved-checkpoint equivalent.

## Collision profile v2

`room-core` now implements the four-direction open/solid corner branches decoded
in [movement qualification](movement-qualification.md). It preserves the original
player edge samples, positive edge-minus-one convention, bit-3 snap/rollback and
perpendicular nudge. It is not generalized AABB clamping.

For unflagged O={0,2,22}, S={12,14}, first sample upper (horizontal motion) or
left (vertical motion), and perpendicular pixel remainder q:

| First/second | Result |
| --- | --- |
| O/O | Keep tentative movement |
| S/S | Correct main axis; no nudge |
| O/S | Nudge perpendicular −1 only if q<8; correct main axis |
| S/O | Nudge perpendicular +1 only if q>=8; correct main axis |

The nudge survives both snapping and rollback of the main coordinate. All
arithmetic/bounds remain checked; failure is atomic. Type16, flagged cells,
slopes, dash and event interaction effects remain unqualified. Old-edge special
materials are still rejected rather than routed through the ordinary table.

The source witnesses are the special-player tables/correction paths around
`$80:DC60/DB8A`, `$80:DFDC/DF00`, `$80:D542/D401`, `$80:D8E8/D7E2`.
The geometric branches do not establish whole interaction-hook fidelity.

Walking snapshot version **2** and slice collision profile **2** reject previous
flat-only semantics. The walking encoding remains 16 bytes. The adapter includes
the profile version in its compiled-data identity; grid hashes are unchanged.

## Authenticated trajectory coverage

The prior strict subset matched 1,204 steps. Three previously excluded map10
routes (Left, Right, Left→Up) add 647 steps with mixed edges. Two dedicated nudge
routes add 120, for **1,971 position/stream steps across twelve trajectories**.
Every step also repeats through a restored snapshot. The harness's blocked-output
comparison is a property of these fixtures (actual delta differs from stream),
not an independent native carry/dispatch oracle or a universal blocking formula.

The two new routes start with the same authenticated JP ROM / three-slot SRAM
bootstrap and genuine F→10 doorway as the previous research. Verification starts
at completed frame 1801 in map10 `(392,353)`, not during transition control.
Inputs below are half-open labels before `run_frame`; capture ends at completed
1861, with all unspecified inputs released:

| Route | Additional held inputs | Observed corner effect |
| --- | --- | --- |
| `corner-positive` | Down `[1801,1809)`, Right `[1813,1830)` | Frames 1816–1820: X stays392 while Y increments364→368, despite Y stream0 |
| `corner-negative` | Down `[1801,1814)`, Left `[1820,1850)` | Frames1834–1836: X stays376 while Y decrements370→368, despite Y stream0 |

These prove surviving ±1 perpendicular corrections, including magnitude-2 main
stream attempts. Each route has 60 consecutive admitted steps, no excluded
frames. Two independent fresh boots reproduced both CSV files and before/after
WRAM snapshots **byte-for-byte** (six files).

| Route | CSV SHA-256 | Final WRAM SHA-256 |
| --- | --- | --- |
| positive | `109df44c44689ac15c266b19852f103a3fb40f17d7f9d9df4e7671ea7bee15f8` | `112a0602bb001230c6748cd9bdc5ced1becbb0ce0cf71f6d998b3cf53ac5ff0d` |
| negative | `718e668a4154f594eadd83273966423688362632a0f6d205d081db10973b1665` | `613c1b81fbdbbfe81f46f53c99c4f21cfec253d932caf0c3e4821a54c72d1d55` |

## Reproduce and verify

```sh
# Requires the independently authenticated private ROM and SRAM.
sh tools/movement-qualification/replay.sh
ROOM_CORE_FIXTURES="$PWD/local/movement" \
  cargo test -p room-core --test local_trajectories -- --nocapture

# ROM-free corner/rollback/version regressions and portable build:
cargo test -p room-core --test walking
cargo build -p room-core --target wasm32-unknown-unknown
```

Tests cover all four directions, 15 nonzero remainders, open/solid type
combinations and both orders; targeted cases cover mixed rollback, magnitude2,
q=1/15 realignment, actual/attempted deltas and restored continuation. Retained
flat-wall/latency/gap tests remain green. Unknown and flagged materials still
fail, rather than allowing an otherwise convenient test path through them.

Exploratory longer rightward routes encountered flagged cell `$8078`; those full
routes are **not admitted**. The checked routes intentionally stop before that
unqualified interaction. No raw graphics, ROM, CSV or WRAM is committed.
