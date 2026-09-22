# Native first-tower approach and interior entrance

[Owned issue](../../meta/issues/qualify-tower-approach-route.md). This second
segment extends the [spear/frozen-return qualification](README.md), preserving
its complete input and observation prefix. The named entrance witness is
**`first-tower-transition-control`, completed65332, map `$101`, `(128,623)`**.
Two-axis movement and neutral stability finish at **65608, `(112,607)`**.
This is the first tower's **interior entrance**, not merely its outside approach
map `$100`, and not completion of the tower or its combat.

The discovery session used only real input from an empty-SRAM boot. No native
warp, memory patch, state restoration, save loading or position resets were used.
All exploratory misses and waits remain in `tower-route.jsonl`; the original
Pandora and frozen-return recipes and reference files are unchanged.

## Required story steps versus incidental movement

### Return stairs and the Elder

The observed map path after frozen return is **21 → 20 → E → C → D → A**.
Return-selector ownership is not ordinary walking merely because input masks
look clear: `return-stairs-21-rest` and `return-20-stair-rest` still show
`$84BD3E`, not free control. Later neutral/input legs finish those arrivals.
The accepted route retains them exactly rather than relabeling them as settled.

The doorway Elder is mapD scene record `$838CBE`, header `$888BEB`, callback
`$888C17`. Its membership is `$23 XOR $21`. The actual A interaction at
`(120,704)`, facing Down, works; the earlier pulse at `(120,701)` misses.
This is a measured anchor, not a general geometric cutoff claim.

- `$888C17` sets **`$21` immediately**, before the prompt/choice finishes.
- `$888C21` uses catalog1 and table `$888C27`: option1 enters `$888C2D`;
  cancel/option2 enters `$888C41`.
- The accepted route chooses option1. Request `$888D1F` must finish before
  `$888C33` sets **`$296`**. Therefore `$21` is not an acceptance flag.
- Source refusal and ordinary-room retry branches are distinct; this journey
  does not claim a native refusal/retry replay for the Elder.

The retained choice and final acknowledgement checkpoints prove that `$296` is
not granted early. They do **not** prove that every possible geographic route
requires that choice; the qualified route takes it.

### Frozen town and overworld

Town controller `$8884EF` tests `$296 XOR $3C`. On this first accepted visit it
runs requests `$88855F/$88857B` and a player handoff, then sets **`$3C` at
`$888538`**. Native player movement `(504,769)→(504,868)` occurs during this
presentation. The second request uses COP20 completion; merely publishing the
request does not complete the town sequence.

Town exit `$818DB3`, rectangle `(0,62,80,2)` cells, targets map03 with mode0,
selector `$55`, raw `(528,528)`. Native arrival is **`(536,544)`**. Map03 uses a
**different world-player/controller path** (`$84DFxx`), not the ordinary-room
walking contract. The retained cardinal legs navigate around the terrain to
`(216,880)`, then Up reaches the southwest tower record `$818CCD`:

| Source record | Rectangle in cells | Destination | Mode/selector | Raw position |
|---|---|---|---|---|
| `$818DB3` | `(0,62,80,2)` | `$03` | `0/$55` | `(528,528)` |
| `$818CCD` | `(13,49,1,2)` | `$100` | `0/$66` | `(248,992)` |
| `$81C2EE` | `(15,54,2,2)` | `$101` | `0/$62` | `(120,608)` |

These are source exit operands, **not initialized or settled native positions**.
No portable implementation of world-map motion, camera or rendering is inferred
from having replayed a native route through it.

### Tower approach, guardian and actual interior crossing

Map100 selects the **bank82 scene override `$8288C1`**, not the zero bank83
pointer. Its entrance presentation grants `$100` and holds requests ending at
`$908F69/$908F7D`. The retained `tower-door-up` input happens during that request:
it does not move the player and is not a successful guardian approach.

The later Up approach activates the guardian automatically and stops the player
at **`(256,959)`**, script `$90FB23`. No A interaction is needed to start this
encounter. Source guardian record `$8288CA` points to `$908BF0`; its first-visit
path is separate from the `$196` branch (**not** Elder event `$296`).

- Requests `$908D44/$908D75/$908DA4` precede choice `$908C40`, catalog1, table
  `$908C45`.
- This journey takes option1 and finishes `$908DD2/$908E11` and their shared text
  tail. Source cancel/option2 takes different responses but converges on the
  same completion. Thus **`$115` is not an affirmative-only answer flag**;
  no alternate guardian branch is claimed natively here.
- Local1/2/3 coordinate presentation. `$908C75` sets `$115` after completion;
  script-control release is later than the first observation of that flag.
- `guardian-complete-stable` restores ordinary script `$84A258`. Up56 followed
  by neutral240 crosses actual exit `$81C2EE` into **map101 `(128,623)`**.
  The guardian does not directly issue that map transfer.

## Native checkpoints

| Label | Completed frame | Map / player | Meaning |
|---|---:|---|---|
| `elder-page-4` | 56866 | D `(120,704)` | Choice pending;21 set,296 clear |
| `elder-accept-3` | 57830 | D `(120,704)` | Final response acknowledgement;296 still clear |
| `elder-accept-4` | 58071 | D `(120,704)` | Response completed;296 set |
| `frozen-town-page-4` | 59304 | A `(504,868)` | Town request completed;3C set |
| `underworld-arrival` | 59760 | 03 `(536,544)` | Distinct world-player mode |
| `tower-approach-arrival` | 60666 | 100 `(256,1007)` | Outside approach; entry presentation not finished |
| `guardian-page-4` | 63229 | 100 `(256,959)` | Guardian choice pending;115 clear |
| `guardian-response-5` | 64675 | 100 `(256,959)` | Last response acknowledgement;115 clear |
| `guardian-response-6` | 64916 | 100 `(256,959)` | 115 set; player handoff not yet ordinary |
| `guardian-complete-stable` | 65036 | 100 `(256,959)` | Ordinary control restored |
| `first-tower-transition-control` | 65332 | 101 `(128,623)` | Interior entrance/control witness |
| `first-tower-left-rest` | 65356 | 101 `(112,623)` | Horizontal control |
| `first-tower-up-rest` | 65488 | 101 `(112,607)` | Vertical control |
| `first-tower-neutral-stable` | 65608 | 101 `(112,607)` | Stable endpoint |

Frame numbers count explicit producer frame calls, not CPU cycles. The exact
save-state synchronization/observation schedule is part of the recipe. Full
checkpoint events include indices0..1023; the per-frame log only includes0..511.

## Prerequisites and scope limits

- **Spear acquisition:** the retained path needs the separate consent/collection
  sequence to leave Pandora, with actual item81 inventory proof. Acceptance alone
  or event242 alone is insufficient; the preceding segment supplies native
  cancellation, consent-without-item and wrong-facing controls.
- **Frozen return:** yes, this route completes FE/23, returns through the house
  and traverses the corresponding town state. No universal claim about every
  resident's freeze behavior or every later story state is made.
- **Equipment selection:** not required for this entrance. Source-backed 16-bit
  equipped IDs at `$7E064A` (weapon) and `$7E064C` (armor) are both **zero/none**
  in the retained entrance/control snapshots; item81 remains owned in inventory.
  Menu equip/clear writes and zero-checked readers establish these fields.
  This is not a qualification of the equipment UI or general stat calculation.
- **Combat:** no fight or attack on an enemy is required to reach this entrance.
  The route ends before tower combat/progression. It does not qualify an enemy
  scheduler, attack/hit resolution, experience gains or the rest of Chapter1.
- Earlier candidate collision figures remain **24/24 outbound,23/23 returns**,
  with conservative production **19/24**. This new native journey does not widen
  those host policies or qualify the missing first8/Partial8/Solid8/slope8 cases.

## Reproduction and verification

```sh
sh tools/tower-approach-qualification/replay-tower.sh "$ROM"
# Existing full journey, with sibling journey.jsonl:
python3 -B tools/tower-approach-qualification/check_tower.py "$ROM" "$CAPTURE"
python3 -O -B tools/tower-approach-qualification/check_tower.py "$ROM" "$CAPTURE"
```

SHA-256 of the complete input-only recipe:
`27c5f38d3d8d28dd52230255460ed4c9e5283b082c37b86d6db6a87f5d8c7fd8`.
Complete native log:
`cf06b42829ff31587acbec7d14f21e741701321261bbc15c71825593a4630f22`.
The owned JP ROM and unmodified producer are the same as the preceding segment.
Raw native artifacts, ROM/text/graphics/layers and disassembly remain ignored.

An independent fresh boot reproduces **all4,313 files** byte-for-byte, with no
missing/extra files or log differences:615 input commands,616 checkpoints,
58,808 explicit route frames and59,424 log records (76,532,961 bytes). All3,507
frozen-prefix capture files remain identical and its earlier log is an exact byte
prefix. The final120 neutral frames plus checkpoint remain at `(112,607)`.
Sorted artifact manifest SHA-256:
`d524bc22c3bd455d47025d3909e6a56dd85b974c9e50164f32f98547ca778a13`
(format as in the preceding segment).

All58 source/checker tests pass normally and under `-O`; both source projections
and the final checker pass against the owned ROM and independent capture in both
modes. Source and semantic mutation controls were red before green; disabling
the tower semantic validator yields168 failures. Separate pipeline controls reject
an omitted artifact gate, the wrong inventory slice and byte-width equipment
reads. Independent source and checker reviews found no blockers; executable
review also verified the equipment interpretation directly against ROM bytes.

The all-file equality above is an independent replay comparison, not a claim
that the checker content-pins every historical prefix snapshot. The inherited
house/Pandora gates keep their selected snapshot coverage; the complete file-set
gate and all new extension checkpoint hashes do not broaden those older pins.

Release workspace tests, the three local collision tests and16 local world tests
pass. Candidate geometry remains24/24 outbound and23/23 returns; production
admission remains19/24. No runtime/library code changed.

<a id="bounded-portable-follow-up"></a>
## Bounded portable follow-up

Before implementing this route in the portable slice:

1. Add source-bound post-return resident/event variants and doorway Elder
   interaction, distinct21/296 milestones, and the town request/handoff/3C phase.
2. Admit/replay the actual return selectors and this frozen-state navigation;
   retain transition ownership rather than sampling arrival poses as free motion.
3. Decode required map03/100/101 layers, graphics, palettes, camera profiles and
   scene/controller records. Respect the tower's bank82 override. Map03's world
   projection and movement need a separate contract from ordinary room walking.
4. Implement the finite tower-introduction and guardian request/choice/presentation
   sequence, including100/115, local handshakes and script handoffs; then the
   source exit100→101 and its owned arrival. Render source actor/background art,
   not captured whole frames or WRAM-derived production state.
5. Carry the acquired item81 through transitions without equating ownership with
   equipment selection. Equipment UI and combat remain the existing separate
   [menu/inventory](../../meta/issues/menus-inventory-save.md) and
   [actor/combat](../../meta/issues/port-actors-combat.md) scopes; combat is not a
   prerequisite for this entrance-only portable slice.

No native CPU execution, captured-state initialization or silent broadening of
production map/material admission is authorized by these evidence files.
