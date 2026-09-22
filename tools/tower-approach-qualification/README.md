# Tower approach qualification: spear and frozen return

[Owned issue](../../meta/issues/qualify-tower-approach-route.md). The first accepted
segment extends the existing Pandora tour to **`frozen-return-stable`, frame53586,
map `$21`, `(120,448)`**. This first segment alone does **not** qualify the frozen town, overworld,
tower approach/interior, equipment selection or combat. No portable/runtime
behavior is changed by this tooling.

The independently verified [second segment](TOWER.md) now reaches **first-tower
interior map `$101`**, with two-axis control and neutral stability through
frame65608, `(112,607)`. The native route issue is complete; portable integration
remains open. The first-segment recipe and pins below are unchanged.

## Corrected progression blocker

The earlier discovery concluded that map41 was terminal because floor sweeps
could not reach its arches. That conclusion was false. The current directional
collision model suggested a **turning corridor**, then real native input proved
it: from `(120,192)`, Left8, Up32, Up32, Left8, Up32 (neutral12 between legs)
settles successively at `(110,192)`, `(104,155)`, `(104,109)`, `(94,109)`, `(72,80)`.
The last leg has neutral120. A6 + neutral180 enters map42 at `(136,464)`.

This does not claim the navigation model is qualified everywhere it searched.
It supplied a hint only; the retained journey uses fixed real inputs, never
model-initialized native positions. The hall controller is indeed quiescent after
the tour, but room-door interactions remain available. The old implication
“no polling controller means no departure” must not be reused.

## Source and native contract

- `$89DCB5 COP14`: map42, mode0, selector2, raw `(128,448)`; native settled
  player `(136,464)`. Navigation around pedestals reaches the spear at `(72,384)`,
  facing Up.
- Callback `$89DA56`: first interaction sets `$240`; choice table `$89DA7E`
  routes cancel/result2 to refusal, result1 to consent. Consent sets `$241`
  **after** its response completes. Neither refusal nor consent grants inventory.
- A later correctly faced interaction sets `$242` and starts collection. The
  recipe retains an actual wrong-facing A miss and neutral waits: labels such as
  `spear-collect-request`/`spear-grant-*` are intentions, **not successful grants**.
  `spear-retry-choice` is an acknowledgement page; the retry choice is
  `spear-repeat-2`. All these observations remain checked, not deleted.
- `$89DA20 COP60 [81 A4 01 34]`: item `$81`, presentation word `$01A4`, worker
  parameter `$34`. This is not an item/count pair. `$8D9653` inserts an existing
  ID or uses the first empty two-byte record in `$7F8048..$7F8060`, incrementing
  quantity by1 (cap9). COP60 ignores failure carry, so **event `$242` alone does
  not prove acquisition**. Actual fresh inventory changes `00 00 → 81 01` at
  `$7F8048/49`; the rest of that region remains zero. Acquisition is not equipping.
- `$89DA2A` uses bank-first COP1C to request `$88D165`; the automatic presentation
  and subsequent request lead through `$89DA49` to map21 (mode4, selector1,
  raw `(128,352)`). Native return is `(136,368)`.
- Guide `$88AEB8` and resident `$88B2FA` cooperate via locals3,2,A. Their requests,
  movement/handoff sites and waits are separately source-pinned. COP20 retries
  until dialogue completion; it is not interchangeable with COP1F.
- Final `$88B2F9` acknowledgement still has no `$FE/$23`. Only after completion
  do `$88AF3F/$88AF43` set them and release input. Neutral/two-axis controls prove
  `(136,464) → (120,464) → (120,448)`, then stability. This is the source-backed
  frozen-return story boundary, not an assertion about every town resident.

| Retained checkpoint | Completed frame | Evidence |
|---|---:|---|
| `spear-refusal-1` | 44305 | Idle after cancellation; no241/242, empty weapon region |
| `spear-accept-1` | 45396 | 241 set, 242 clear, weapon region still empty |
| `spear-take-A` | 47412 | 242 set, weapon region still empty |
| `spear-take-request` | 47592 | Actual `81 01`; input mask FF50, not ordinary control |
| `frozen-return-stable` | 53586 | Map21, `(120,448)`, FE/23 and inventory retained |

These are checkpoint bounds, not exact CPU write-cycle claims. Per-frame event
logging covers only indices0..511; full0..1023 events are decoded from checkpoint
WRAM. The log first observes the return to21 at48709 and FE/23 at52952.

## Discipline and verification

One empty-SRAM `oracle::Session`, unchanged 6800-frame bootstrap and all382
accepted Pandora commands/observations, followed by118 extension commands.
No warp, memory patch, save loading, state restoration, per-frame reset or dropped
observation. States are synchronized **outputs**; save-state synchronization can
advance emulation and is part of the exact recipe. Same `headless-sync-video-v1`
epoch as the accepted prefix. Source metadata never initializes production WRAM.

Two separately executed fresh sessions match **all3,508 files** and the complete
61,351,421-byte log:501 checkpoints plus46,786 frame rows. Independent review of
the producer confirms real-input-only execution. This is not a reproducible-build
attestation. SHA-256:

- ROM: `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`
- Recipe: `3d4ac4d759eb97e68b5c57e8be23739bf49cee79a010beeda0b55a176dbdfd75`
- Log: `94b6056106357d486840e56927474e9f04fbf7b3907fe6449dfabc4712115cd6`
- Producer executable: `6dafa682a8d8a465559e086e342f4283e41e6875cf4d1200cc67f19ba7dbf3fa`
- Sorted artifact manifest: `429fe0353a1a8b57cd7334c3f8420adc45e20751a96613efc29c9ad3c54c0ecb`
  (each line is `<file SHA256>  <relative path>\n`).

The checker strictly revalidates the accepted prefix, exact source/observer/tool
provenance, complete input/frame/checkpoint schedule, artifact set, all extension
surfaces and independent semantic boundaries. Source and semantic mutation tests
were red before implementation; disabling the semantic validator produced95
failures, then green was restored. Tests run normally and under `-O`.

```sh
sh tools/tower-approach-qualification/replay.sh "$ROM"
# Existing journey, with sibling journey.jsonl:
python3 -B tools/tower-approach-qualification/check_departure.py "$ROM" "$CAPTURE"
python3 -O -B tools/tower-approach-qualification/check_departure.py "$ROM" "$CAPTURE"
```

Native discovery preceded TDD; its ignored route-search helper is not shipped.
The shell replay wrapper composes existing producer/checker commands and is
verified through those end-to-end gates rather than a separate shell unit test.
Raw text, graphics, ROM, layers, captures and disassembly remain ignored.

## Portable follow-up scope

Before integrating this segment: qualify/admit the turning map41 corridor and
map42 navigation; add door controllers and spear callback/choice phases; model
item81 acquisition separately from equipped state; implement bounded award
presentation, map21 return handoffs and cooperating request/flag phases; render
the acquired object and return/freeze presentation from source assets. Do not
instantiate these from captured WRAM. Equipment UI, frozen town/field traversal
and any tower/combat behavior remain separate portable scopes. The completed
[native second segment](TOWER.md#bounded-portable-follow-up) records their bounded
follow-up requirements; no combat is required for its entrance-only endpoint.
