# Bounded Pandora navigation qualification

Owned issue: [qualify navigation/contact](../meta/issues/qualify-pandora-navigation.md).
This additive contract does not enable movement in the old house profiles. ROM
assets are inputs; neither a captured grid nor the observer is an initializer.
Scope is the direct A → 13 → A → D → C/pots → E → 20 → 21 route,
forced first-time tour, and final41 control. It is not a town simulation or VM.

## Source discovery (before implementation tests)

The headerless JP ROM is authenticated by SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
The existing background decoder supplies full attributed sheets, not admission.
Source words remain `tile | ((attribute[tile] & 127) << 9)` including bit15.

### Actual samples and materials

Ark's collision origin is `(x-8,y-16)`. Left/Up sample that origin;
Right samples `(x+7,y-16)`, Down `(x-8,y-1)`. The second sample is the
next perpendicular cell boundary only when the first is not aligned. Validate
both old and tentative edges; do not introduce bounding-circle probes. Old-edge
stored slopes6/7 are tested before the new-edge occupancy override. Under the
bounded passive policy `$0980 & $0050 == 0`, a bit15 sample is solid. Carrying
`$0020` is separately admitted by the pot component, not ordinary action walking.

Four first/O/P/S tables per direction start at `$80D542/$80D8E8/$80DC60/$80DFDC`,
with subtable strides64. All sixteen give type25 **the identical handler to12**,
and type5 the identical handler to16. The route needs town25 as solid and C's
closed lower door type5 as partial. The latter is confined to C `(11,21)` Up.
Type29 equals open0 in fifteen tables, **not** Right S-first `$80E09C`. Only Up
at C `(11,21)`, E `(6,53)` and20 `(22,53)` is stair admission. No global29 alias.
Other unknown types and samples outside the bounded profile fail atomically.

Source-frozen town occupancy is a portable policy, not native wandering timing.
The six scene instances `$8389BF/$8389C9/$838A05/$838A19/$838A23/$838A2D` use
geometry bytes `F8 10 F0 10` from their decoded source frames, interpreted by
`$86BAEF..BB49` as `(-8,16,-16,16)`. COP3B `$80BE8E` uses the small-footprint
stamp `(actor.x-8,actor.y-16)`, **not inclusive rectangle intersection**.
Birds freeze at source spawns; their native wandering occupancy is not production
initialization. In particular the recorded return at Y401 can sample a frozen
bird's row25. Conservative samples find **36 occurrences at `(28,25)`** where
source-frozen `$838A19` supplies `$801E` while the selected native witness is
`$001E`. No other bit difference is exempted. Geometric connectivity tests find
a bounded route with these stamps retained; the exact native Y401 return is
not a portable replay recipe. Y400 avoids the frozen bird row horizontally.
A native-cadence portable itinerary remains parent reproduction work.

Direct C residents retain source-relocated `(152,368),(56,384),(184,416),(216,368)`.
The pot carry route must go around them. The departed phase is distinct from the
direct phase. D's source-origin wanderer remains frozen; `$26` removes only the
separate hidden exterior gate. E/20 have no admitted resident stamps.

### Map13 exact interaction

Spawn `$838EBE` gives `(360,128)`; `$88B63B COP21` installs callback `$88B653`.
Descriptor `$83ECC6` points to compressed packet `$D65EA2`; list0 geometry offset
`$72` and list2 offset `$120` both contain `F8 10 F0 10` (anchor+4).
Thus the actor interaction bounds are inclusive X352..368/Y112..128.
`$87C783` walks root `$0DFA`, links `+$2E`, eligible `+$04 & $0100`. It tries
**far16 then near8 within each actor**, measured from `$0966/$0968`.
`$8791A2..91C5` projects these to **raw Ark `(x,y-8)`**. Thus Up samples raw
`(x,y-24)` then `(x,y-16)`. `$8793B9` performs callback policy afterward;
failure does not resume the search.

At `(360,144)` Up the far point is `(360,120)`, near `(360,128)`. At Y145,
far121 still hits! **Y145 is not a source rectangle cutoff.** The historical
failed A attempt was Up, mask0, no text and held A at its checkpoint; those
observations do not prove the consumer executed that pulse. Its precise cause
remains unresolved. `resident13_witness` deliberately admits only `(360,144)`
Up as the successfully qualified portable witness, not all geometric hits.
An earlier discovery revision incorrectly treated the projected Y as raw Y;
this correction must accompany the compiler. Remote actors/later story remain
unadmitted.

### Exits and transfer ownership

Retain complete ordered 12-byte source exit lists, including unsupported targets.
Selection is first coarse match, then that record's fine test; failed fine must
not fall through. Use the existing source exit decoder, not destination lookup.
The required outgoing records are A `$818D6B/$818D8F`,13 `$818EAC`,
D `$818DFE/$818E0A`, C `$818DF1`, E `$818E2F`,20 `$818FC1`.
These are source-list addresses, not destination lookup keys.

Ordinary doors retain the existing **17 departure / load / 17 arrival** portable
policy. Selector14 (decimal) is not a duration. `$8D89BD` supplies signed initial
adjustment `(-14,-23)`, and `$8D8982` selects departure `$84BAFB`. Its adjusted
loaded anchors precede the settled destination by `(14,23)`:

| Source exit | Raw destination | Settled destination |
| --- | --- | --- |
| C `$818DF1`, `(11,21,1,1)` | E `(144,864)` | `(152,880)` |
| E `$818E2F`, `(6,53,1,1)` |20 `(400,864)` | `(408,880)` |
|20 `$818FC1`, `(22,53,1,1)` |21 `(128,112)` | `(136,128)` |

Keep stairs a separate forced-ownership transfer; intermediate map-ID writes
are not settled arrivals. Finite portable interpolation/durations are explicitly
pacing policy, not inferred native constants from capture labels. Discard input
while forced and begin a fresh walking-admission epoch on actual release.
Box reload/tour retain forced ownership even when the player script looks idle.
The final release is the last request return and `$244`, not reaching map41.

### Box opening is a polling predicate, not an input edge

`$88AD35 COP0D FF FF FF 01 01` uses `$8087C2` and signed-byte-times16
`$80BC2F`. At box origin `(136,384)` its **projected-coordinate** spatial gate
is X120..152/Y360..392. Since `$0968=rawY-8`, the **raw Ark anchor gate is
X120..152/Y368..400 inclusive**, any facing. `$88AD2F` requires local1;
`$88AD3E COP09` requires local1 AND local2. There is no Down, A or second-callback
condition. Neutral after warning remains outside at raw Y359; the next approach
first enters at raw Y368. Both are independently checked in the native log.

The warning controller `$88ADA5` tests local1 XOR local2. It masks input, waits
COPC1 `$20`, requests `$88ADF2`, waits for return, waits COPC1 `$20` again,
clears mask `$FF50` at `$88ADC1`, then sets local2 at `$88ADC5`.
COPC1 `$80AB17` stores **32 actor-delay units**, not a proved rendered-frame count.
After proximity/flags pass, `$88AD46` masks input, `$88AD4A COPDF $888EA6` must
succeed before `$88AD4F` grants22 and `$88AD53` reloads21. COPDF `$80B827`
yields/retries while `$097C & $0810 != 0`; do not grant/reload ahead of it.

First-contact callback registration is `$88AD03 STA $7F1010,X`, distinct from
COP21. The actual pair pass `$85D30C → $85D648` scans the zero-terminated word
array `$0C24` in increasing nested order, **not the A-interaction linked list**:

- Outer box requires category0200, rejects mask01D0; `$85F63E` reads frame+8.
  All eight selector3 records supply `F8 10 F0 10`: world X128..144/Y368..384.
- Inner Ark requires category0400, rejects0262, same `+$16`, zero extended1020,
  grounded/no-guard policy. `$85F78E` reads frame+12; standing Down and six
  walking Down records supply `FB 0A F0 0E`: X(x−5)..(x+5)/Y(y−16)..(y−2).
- `$85F835` accepts touching edges. The resulting **raw-anchor first-contact
  geometry is X123..149/Y370..400 inclusive**. This is not the opening gate.
- On overlap, queue box pending0200 before processing Ark. The scene sets Ark's
  `+$06 & $20`, bypassing damage **but not recoil**. `$85D735` writes `$FF38`
  into Ark extended1020, computes recoil direction (`$85F8D1`, horizontal on
  axis ties), then COPCB selects `$848000`.
- Scheduler `$80CAD5` later installs the queued callback. `$88AD69` sets local1,
  clears box category0200, stamps `(8,23)` with COP3B, and returns to its loop.
  Clearing category0200 is why opening is not another first-contact callback.

Native frame26805 is `(136,370)` without local1; frame26806 is recoil `(136,369)`
with local1. Rest is `(136,359)`. These are adjacent completed-frame witnesses,
not direct observation of a within-frame queue. Only this northern centerline
recoil endpoint is admitted; other approaches, poses/air/guard/combat and exact
recoil pacing are source-only or unsupported. Full rectangle boundaries have
source/synthetic tests, not a new native side-contact capture.

### Shared-sheet lifecycle

Layer handler `$868AAD..AB8` calls `$869145` using cache `$043F`. The helper
compares the incoming pointer's low word and bank at `$869148..9156`: equal
returns carry-clear, clearing the first-layer load bit; unequal stores the new
pointer and returns carry-set. `$868AD0..AD7` returns when no layer bits remain.
Consequently B/C/D/E/20 sharing `$AFCBB3` retain live **tile/attribute patches**,
including consumed FA/FB cells and opened doors. A/13/21 replace that source.
A→D therefore rebuilds the house base, not the old opened C→B door. A→13→A
likewise reconstructs A's closed wooden doors; no persistent door-open event is
invented. Room-local reset `$8D8735 → $8D8AED` clears `$06C0/$06C2/$0640` via X=0;
this does not clear global `$292`. Occupancy is rebuilt for the new scene rather
than carrying the old scene's actors across the shared resource cache.

Native `replay-WKFf0d/a/journey/door-passable.wram`, first layer `$A000`,
retains the departing C actor's final `(7,31)` stamp (`$1CE8 → $9CE8`);
settled E/20 do not. The five persistent E/20 grid deltas are precisely the two
opened C door cells and three removed pots. Box21 has no initial stamp, then
contact stamps `(8,23)`, and opening reload removes it. Captures validate these
facts; they do not supply the compiler's cells or actor positions.

## Additive host compiler and admission

`crates/map-inspector/src/pandora_navigation.rs` is intentionally independent of
the parent-owned main/module registration and core runtime. `compile(&image)`
authenticates the full JP ROM; `Navigation` returns immutable `Profile` slices,
`ContactSpec`, and metadata for the parent adapter. `Profile::room()` retains
**raw source words**; do not pass it to old house walking and expect new types to
work. `Profile::sample(cell, delayed_direction, old_edge, control)` is the bounded
classification contract for integration. Direction is the direction governing
the delayed collision tick, **not newly submitted input**. Existing rooms and
core behavior are unchanged. Unknown names, materials, modes, exits and samples
must not select a fallback profile.

| Immutable profile family | Half-open sample halo `[L,T,R,B]` |
| --- | --- |
| `a`, `a-north-open`, `a-home-open` | `[21,16,36,53]` |
| `13` | `[22,7,25,16]` |
| `d-return` (`$26` gate omitted on load) | `[7,32,9,46]` |
| C six phases below | `[1,20,14,32]` |
| `e` | `[6,52,10,55]` |
| `20` | `[22,52,26,55]` |
| `21`, `21-contact` | `[8,7,9,25]` |
| `41-control` (only after244) | `[6,10,9,14]` |

C phases are `c-direct`, `c-after-miss` (remove pot column5), `c-held-fa`
(remove3+5), `c-first-hit` (same removals, cracked upper door), `c-held-fb`
(remove3+4+5, cracked), and `c-departed` (opened door, departed residue). These
are the **bounded main-route ledger**, not all pot combinations. E/20 keep all
three removals and the two opened cells, without departed residue. The compiler
is not a phase scheduler. During second-hit temporary `$9CF6/$BACB` occupancy
and moving-resident reactions, input remains forced under the source/pot/story
owner; no ordinary profile is granted for that transient. Do not switch early
to `c-departed` merely because event292 was set. Camera/art ownership is separate.

The map13 non-interacting object at X424 and final guide outside the halo are
not omitted residents **inside admitted samples**; those outside-halo cells
have no occupancy admission. Full sheets are storage extent, not whole-map
movement. Source phase/art IDs, relocation operands, and exact grid hashes are
retained. Metadata includes complete ordered exits, eight tagged transfers and
six source COP14 forced operations. Forced maps42–44 are presentation, not
walking profiles. A compiler success is not a core runtime itinerary replay.

## Evidence, tests and reproduction

The checker compares source grids with **16 fixed settled native phase witnesses**
at conservative edge samples collected from **2,613 eligible movement frames**:
**10,851 sample occurrences**, including the 36 explicitly enumerated frozen-bird
differences above. It does **not** observe native collision grids every frame or
reconstruct native velocity. It checks source-phase old/forward1/forward2 probe
admission, phase denominator/exclusions, exact transfer identities/endpoints,
contact/opening witnesses, full witness WRAM hashes and the complete log/recipe.
Per-phase excluded scripts/commands/transfers are reported, not silently omitted.
Missing a phase or accepting an unexpected map mismatch fails.

Read-only repaired-observer twins are
`terranigma/local/oracle-video-qualification/replay-WKFf0d/{a,b}/journey`, with
adjacent JSONL logs. Their navigation reports agree exactly. Original discovery
WRAM is explicitly **historical** evidence for the failed145 attempt, not a
renewed strict discovery replay or an alternate reference epoch. The separate
source-owner synchronous-observer migration does not change these source
operands/nonpixel/log facts. This checker reads no pixels or observer contract
metadata, and changes none of the source owner's files.

```sh
ROM="$PWD/local/Tenchi Souzou (Japan).sfc"
OUT=local/pandora-navigation-export # must not exist
sh tools/pandora-navigation-qualification/export.sh "$ROM" "$OUT"
A=/path/to/replay-WKFf0d/a/journey
B=/path/to/replay-WKFf0d/b/journey
for flags in '-B' '-O -B'; do
  python3 $flags tools/pandora-navigation-qualification/test_check.py
  python3 $flags tools/pandora-navigation-qualification/test_local.py "$OUT" "$A"
  python3 $flags tools/pandora-navigation-qualification/check.py "$OUT" "$A" "$B"
done
```

The standalone exporter harness runs ROM-free and explicitly owned-ROM tests
without editing parent `main.rs`. Tests cover source-reader/dispatch mutations,
contact bounds and projection, exact full-grid patch lifecycle deltas, old/new
sample rejection, and geometric connections between required anchors. The
connectivity test uses open sampled cardinal pixel steps with stamps retained;
it **does not** prove input cadence, nudge pacing, an action scheduler or native
actor motion. Native fixture mutations cover changed source/native samples,
coordinated grid/hash changes, missing frames, phase/exit nonvacuity, early local1,
wrong contact/opening bounds, low/extended event provenance and full-WRAM changes.
Checks use exceptions, not removable Python assertions; JSON round-trip and
no-op mutation controls protect strict comparison.

Source discovery preceded TDD. Independent compiler/source/API and native-evidence
reviews prompted provenance, full-delta, nonvacuity and projection corrections.
**Keep the issue open pending parent core integration/reproduction.** No native
cadence, recoil/stair/reaction/tour pacing, general town simulation or frame-exact
continuous portable route is claimed. If the parent cannot traverse the admitted
geometry under its actual input policy, report that blocker rather than silently
widening the material/actor contract. No raw captures, extracted grids or pixels
are committed.
