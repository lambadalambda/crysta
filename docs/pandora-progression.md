# Pandora progression: fresh source/reference boundary

[Issue](../meta/issues/qualify-pandora-route.md). Japanese image SHA-256:
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.

**Endpoint: `pandora-tour-control`, inside map `$41`, after the first-time box
interior tour.** This is not a claimed return to the field or a frozen Crysta.
Strict qualification now uses **`headless-sync-video-v1`**. Both independently
produced parent fixed roots and both video-task roots pass the migrated exact
checker normally and under `-O`, with unchanged source/gameplay semantics. See
[observer migration and remaining wrapper gates](../tools/pandora-qualification/OBSERVER-MIGRATION.md).
Old threaded pixels are archived, not backward-equal. Parent post-cherry-pick
strict verification and tracker closure remain parent-owned. Source-only
projection received an independent review before signed commit `380d0b1`.

## What the retained journeys establish

`tools/pandora-qualification/route.jsonl` preserves all 77 commands of the accepted
[house conversation](house-conversation.md) recipe, omits its finish, and extends
the **same empty-SRAM Session**. Its prefix frame-log hash and all non-pixel
surfaces match the accepted house reference exactly. All 31 selected capture sets
match Pandora's versioned `prefix-reference.json`; four pixel pins were renewed
under the fixed observer, without modifying the standalone conversation fixture.
No SRAM was supplied, no save was loaded, and no debug warp, memory patch or
fixture initialization was used. The native game's own scripted map changes
are real progression, not tool-injected warps.

The admitted **story branch** uses map13 result 1 and C result 1 (`$2E`). The
itinerary is intentionally not a minimum-frame speedrun: it preserves the
accepted prefix's repeat/choice controls, movement corrections, waits and one
off-target pot throw. Labels describe exploratory intentions, not guaranteed
outcomes: notably `door-hit1` is a **miss**. The checker uses its actual zero hit
counter. Required story steps are separated from these incidental inputs below.

An earlier, separate fresh discovery process retained missing-prerequisite,
cancellation, push/dash, off-target and box-warning controls. It explored C's
longer `$2F` refusal branch. Its later actor geometry is **not** used as evidence
for the direct `$2E` route. Both processes flushed and exited successfully; neither
restored a state. `epochs/threaded-video-v0/discovery-reference.json` selects its
historical control evidence, not an alternative production initialization. The
discovery observer epoch has not been renewed. Optional X/Select exploration
after its control witness is outside the endpoint claim.

Historical opening/Pandora scenarios prove no endpoint here. The historical ares
“same input causes script desync” claim was explicitly corrected in
`meta/issues/switch-reference-core.md` and `initial-reference-scenarios.md`.
A stale milestone summary is not evidence of an emulator defect. The current
fresh route reaches Pandora without a gameplay oracle fix. The independently
observed pixel publication defect below is distinct from that historical claim.

### Observation policy is part of the recipe

The unmodified conversation probe runs 6800 bootstrap `run_frame` calls, then
consumes real-button commands. It calls **`oracle::save_state` at boot and after
every command**, writes that state, then captures WRAM/VRAM/CGRAM/pixels/OAM.
The shim invokes **`serialize(true)`**. This synchronizes/advances emulation and
can change game state; it is **not passive observation**. Buttons remain held
through that synchronization. Frame numbers count the probe's frame calls and
must not be interpreted as cycle-accurate durations excluding synchronization.

There are no extra capture/synchronization calls in the retained replay. Per-frame
WRAM observations are passive; checkpoint observations happen after the explicit
sync. The checker requires the exact command/capture schedule, every frame row
and checkpoint order, the unchanged prefix, and the complete log hash. Omitting
a capture is a different, unqualified timing policy. The wrapper uses one Session
per process and `finish` flushes then calls `process::exit(0)` to avoid teardown.
Observer source hashes now include the build definition, both translation units,
patched video configuration header, Screen/PPU publication/color code, Rust API
and serializer. Probe/bootstrap/build-script hashes are unchanged. Completed
pixels are from the **last explicit `run_frame` before the save-state sync**, not
the later synchronized machine instant. This is output/source-pinned, not a claim
of a fully dependency-locked build environment or historical binary identity.

## Independent replay diagnosis

**Historical `threaded-video-v0` investigation (before the separately reviewed
synchronous fix and migration).** Statements below describe the old checker and
source epoch; current acceptance is documented in the migration audit.

Parent capture `replay-JmCgU8/journey` ran the frozen recipe to completion. The
checker runs source/ROM, recipe, timeline and accepted-prefix checks, then
`validate(result)`, **before** the final exact-reference comparison. Calling
`report()` on the parent capture in both checkouts succeeds. Every semantic
field, full frame log, source/provenance field and non-pixel capture matches the
original. Thus the parent actually reproduced the declared `$28/$2E/$292/$22`
progression, mandatory tour, final events and named two-axis/stability witnesses.
The failed exact comparison is not evidence of a semantic/native-state desync.

An exhaustive byte comparison of all **2682 files** (383 checkpoints × seven
surfaces, plus recipe) finds exactly four parent differences, all `.pixels`:

| Checkpoint | Completed frame | Differing bytes | Strict checker coverage |
| --- | ---: | ---: | --- |
| `roomC` | 7260 | 129095 | Unselected prefix checkpoint |
| `repeat-wrong-facing` | 9173 | 88192 | Unselected one-frame prefix checkpoint |
| `13-followup-3` | 14648 | 88153 | Selected reference pixel hash |
| `C28-entry-request3` | 17444 | 29289 | Selected reference pixel hash |

All four retain the same 983040-byte extent; every changed byte is nonzero in
the original and zero in the parent. These are **pixel data differences**, not
metadata, unused alpha-byte padding or evidence of a changed Cargo lock. The
last two are the *only* differences in the complete checker report. Selected
source/observer/probe provenance values are identical in both checkouts.

A further fresh process, `diagnosis-gAPhmb/journey`, used the current parent
executable and the same redirected recipe, without restore/patch/seed. Its full
log and every non-pixel file again match byte-for-byte, but its pixel mismatches
move to `repeat2-closed` (11114; 96897 bytes), `middle-route-left` (21409; 69310)
and `landed-21` (26299; 5119). The previous four now match. Again all replacements
are zero. This run fails earlier at the selected `repeat2-closed` prefix hash;
its strict checker never reaches `validate`. This demonstrates within-binary
pixel nondeterminism, not a deterministic build-provenance mismatch. All three
capture sets' final control-witness surfaces match exactly.

The bounded source diagnosis identifies an **unsynchronized video publication
race**, consistent with the observed zero bands (not proof of each precise
thread interleaving):

- `vendor/ares/ares/ares/ares.hpp:61–63` enables threaded video;
  `node/video/screen.cpp:202–214` queues the current refresh without waiting for
  its completion. `sfc/ppu/main.cpp:51–52` immediately returns the frame event.
- The worker's `Screen::refresh` calls the platform video callback. In
  `vendor/ares/shims.cpp:168–180`, that callback zero-fills `lastFrame` with
  `assign(...)`, then fills it in row order. `snes_setPixels` at lines338–350
  reads the dimensions/vector without a lock or completion wait.
- `crates/oracle/src/lib.rs:485–490` copies those pixels immediately after the
  frame call. `pixels()` returns this cached Rust buffer. The subsequent
  synchronized save in the probe does not recopy pixels or synchronize the
  separate OS video worker. Serialized PPU state excludes these image buffers.

The current standalone probe manifests, **standalone** Cargo locks and generated
probe/bootstrap sources match across trees. The root workspace's Wasm crate/lock
change is not a demonstrated cause. Report provenance hashes current checkout
source, **not** the historical producer executable/compiler/full dependency tree;
those historical build identities were not retained. The observed race and
same-current-binary repeatability failure cannot be dismissed as metadata noise.
Original adaptive command delivery versus batch delivery can change host thread
scheduling despite identical emulated input and save-state schedules.

[`replay-diagnosis.json`](../tools/pandora-qualification/replay-diagnosis.json)
retains every differing file's hashes, offset/count metadata, comparison counts
and current producer/standalone-lock hashes. It is **not an alternate golden or
allowlist**. No recipe, reference, source operand, observation schedule or capture
surface has been repinned, masked or omitted. Strict comparisons now report each
differing field, separating semantic-validator success from exact-output failure.
Tests reject mutations of all seven surfaces, each retained checkpoint field,
source/build provenance and observation policy, plus missing/extra fields; a
no-op comparator produces 42 expected failures. Existing semantic mutation
controls remain independent and unchanged.

**Historical gate disposition:** the diagnosis deliberately left pixels strict
and required an observer fix plus renewed evidence. That fix is now separately
reviewed; the migration uses independently reproduced completed publications,
not a mutex-only previous-frame policy, sleeps, state loads, dropped pixels or a
single favorable race outcome. Discovery and unrelated wrapper epochs remain
unrenewed; see the migration audit rather than generalizing Pandora's pass.

## Required story contract

### 1. Leave the house and get `$0028` in map13

The accepted B conversation grants `$26` at `$888F08`, allowing the subsequent
D load to omit its hidden occupant. It does **not** grant `$28` or open the cellar.
The accepted exterior halo is insufficient for this town journey; wider movement
and actor admission must be compiled from the newly required route, not silently
inherited from the house-front profile.

Map A's northern building exit **`$818D8F`** targets map `$13`. Native A interaction
at `(472,304)`, facing Up, opens its ordinary door; subsequent Up traversal lands
at **13 `(392,207)`**. Resident **`$838EBE`**, header `$88B61E`, descriptor `$83ECC6`,
stands at `(360,128)`. Its callback `$88B653` is registered at `$88B63B`.
A facing Up at **`(360,144)`** admits interaction; the earlier `(360,145)` probe
in discovery did not. This uses the existing forward-sample/actor-rectangle
interaction rule, not a radial proximity trigger.

| Phase | Source | Effect |
|---|---|---|
| First request | `$88B673 → $88B6C7` | One acknowledged page then retained choice context |
| First choice | `$88B679`, catalog 1, table `$88B68B` | Cancel/result2 → `$88B691`; result1 → `$88B69C` |
| Result1 request | `$88B69C → $88B758` | Four acknowledged pages; still no `$28` until return |
| Grant | **`$88B6A2 COP07 $8028`** | Sets `$28`; callback becomes `$88B6AB` |
| Cancel/result2 | `$88B691 → $88B7E3` | Two acknowledged pages, then local `$0001`, not `$28` |
| Retry after refusal | `$88B67F → $88B722`; choice `$88B685` | Retained prompt; same result table |

Native direct-result1 cursors: **`B77E, B79F, B7C7, B7E2`**. All four require
acknowledgement; the final request return is the grant boundary. Fresh discovery
canceled first and confirmed the absence of `$28`, then retried successfully.
Callback story exclusions include `$74`, `$101/$109`, `$27`, and `$28`; do not
flatten later-story branches into this fresh transcript. Choice catalog semantics
reuse the independently qualified house choice consumer, not guessed labels.

### 2. Return to C and choose the direct branch

A→D→C reloads the house with `$28` set. C resident `$838C1E → $889A6A` now runs
its changed source branch, relocates, and sets **`$27` at `$889ADB` before the
entry request returns**. Required requests are `$889E9C`, `$889EE3`, `$889EFC`;
the last supplies catalog 1 through either `$889B17/$889B40`, table `$889D80`.
The admitted result1 targets **`$889DA9 → $889FA9`**, whose two pages return before
**`$889DAF` sets `$2E`** and restores the ordinary C interaction phase.

The alternative first result2/cancel introduces another request and choice.
Choosing its result1 sets `$2F`, then adds `$3F/$42` and a different resident
sequence. It is proven discovery, **not part of the admitted direct story branch**.
The main checker rejects those flags throughout its extension.

With only `$26`, discovery completed C's warning, set local `$0001`, and remained
blocked at `(184,368)` after Up, A and another Up. The source controller
`$88ACA9` writes that local bit at `$88ACC8`; it is not a cellar-opening flag.

### 3. Lift, carry and hit the cellar door twice

Hidden door controller **`$838C32 → $88AAE9/$88AAEE`**, position `(184,352)`,
requires `$28` set and rejects itself if **`$0292`** is set. Granting `$28` admits
this controller on load; it does not directly replace the door cells.

Pots are source **tile IDs `$FA/$FB`**, not ordinary residents or an arbitrary
“use item” action. `$87C7F1..C8F6` recognizes those tile targets; `$879683` selects
held record **`$098A/$098F`** respectively. A successful A interaction removes the
source pot cell, installs the carried object and selects carrying control/poses
(e.g. idle `$84B4FC`, Up `$84B4DF`). Ordinary action-free walking is not a sufficient
model for carrying: native held movement sets `$0980=$0020`.

Qualified grabs include FA from `(104,352)` facing Left, FA from `(40,352)` facing
Right, and FB from `(88,352)` facing Left. Direct C retains residents at source
positions `(152,368)`, `(216,368)`, `(184,416)` and `(56,384)`. Their occupancy
matters. The direct route carries around the table and residents, aligns at
**`(184,368)` facing Up**, and throws with A. A throw from `(136,368)` misses the
door and does not increment its counter. Do not replace hit testing with “two
pots consumed,” drop NPC collision, or reuse the refusal branch's cleared room.

The lift, carry and thrown-object source windows are pinned in `source.json`.
This is a bounded FA/FB hit contract, **not** a qualification of all portable dash,
jump, attack, grabbable, projectile, damage or collision modes.

| Hit stage | Source effect | Native cells `(11,20)/(11,21)` |
|---|---|---|
| Closed | Counter `$0640=0` | `$1D80 / $0B81` |
| First actual hit | `$88AB81 COP4B` increments BCD counter; case1 → `$88AB96`; local1 | **`$1DA7 / $0B81`**, still no `$292` |
| First response | `$88ABBC → $88A15F`, return clears local1 at `$88ABC6` | Remains closed |
| Second actual hit | Case2 → `$88ABCD`; local2; **`$88ABEE` sets `$292`** | Temporary **`$9CF6 / $BACB`** occupancy |
| Response complete | Temporary occupancy removed | **`$1CF6 / $3ACB`**, traversable cellar stair |

COP44 patch sites `$88ABA6/ABAC/ABD8/ABDE` select low-nine tiles
`$181/$1A7/$0CB/$0F6`. Operand offsets are `(0,0)/(0,-16)` relative to the cell
origin **actor `(-8,-16)`**, not directly relative to its visual center. The packed
words carry delay1 and first-layer selection. The lower opened cell has stored
**type29**; it is not silently ordinary floor.

The direct branch's second-hit reaction includes requests `$88A17B`, `$889DC3`,
`$889DEB`, `$88A295`, `$88A420`, `$889E0A`. Three source residents cooperate via
local flags 2,4,5,6,7,8,9, including `$88A252/$88A3F5` writes. Keep those request/
completion boundaries and the temporary input/palette/occupancy phases; a finite
sequence is sufficient, not a new general event VM. Native selected boundaries
and their actual source D3/D5 bytes are in the evidence table.

At room initialization, **`$8D8735 → $8D8AED`** loads X=0 at `$8D8AF9` and writes
zero to `$06C0/$06C2/$0640` at `$8D8B0B/0E/11`. Thus low local events 0–31 and the
counter reset on loading; `$292` persists. The old 64-byte event projection misses
`$243/$244/$292`. This checker reads a **bounded 0..1023-bit projection** from fresh
WRAM, while the source helper proves the individual event operands. It does not
claim a complete canonical event-memory layout.

### 4. Stairs, contact, warning, second contact

| Direct exit | Source rectangle | Destination/raw position | Selector | Native settled player |
|---|---|---|---:|---|
| C `$818DF1` | `(11,21,1,1)` | E `(144,864)` | 14 | **E `(152,880)`** |
| E `$818E2F` | `(6,53,1,1)` | 20 `(400,864)` | 14 | **20 `(408,880)`** |
| 20 `$818FC1` | `(22,53,1,1)` | 21 `(128,112)` | 14 | **21 `(136,128)`** |

These are actual stair/diagonal transitions, not the wooden-door selector.
Early map-ID writes and intermediate stairs scripts are not settled landings.
The selector table/arrival-source windows and retained intermediate checkpoints
are pinned. E/20 traversal is Left to the next stair, then Up.

Map21 box source **`$83928F → $88ACF5/$88ACFA`**, origin `(136,384)`, has a contact
callback at **`$88AD69`**, not the ordinary resident COP21 interaction. Entry
controller `$88AD89` requests `$88ADCB` (boundary `$88ADF1`). Walking Down into the
box sets local1, stamps its occupancy and bumps Ark to **`(136,359)`**. Controller
request **`$88ADF2`** has exactly two page boundaries: **`$88AE29 D5`** and
**`$88AE5E D3`**. The observed cursor `$88AE50` points to glyph byte `$56`, not
an acknowledgement; see [qualified dialogue](pandora-dialogue.md). Its return and delay
set local2 at `$88ADC5`. Holding Down while that request is pending does not open
it. Neither does neutral waiting after the warning returns.

A **second Down/contact** reaches `(136,368)` and admits the source proximity/
local-flag gate. `$88AD4A` hands player control to a script, **`$88AD4F` sets `$22`**,
and `$88AD53 COP14` reloads map21 (mode7, selector1, raw128,352). Local flags reset;
player is `(136,368)`. `$045E=$FF50` is still set: an ordinary-looking idle script
alone does **not** prove regained control.

### 5. Mandatory first-time tour and named stable control witness

With `$22` set and `$244` clear, `$88AE64` requests `$88AFA6`, `$88AFE1`, `$88B064`,
`$88B0FB`, with local `$0A` animation cues between returns. It then changes to
map41 at **`$88AEAB`** (mode4, selector2, raw128,192).

The controller **`$89D3B1`** and resident **`$89D2AD`** conduct the first-time tour.
A source counter at `$04BC` coordinates the guide's movements and requests. This
is forced presentation, not a player-controlled trip through the inventory rooms.
The required map path is **41 → 44 → 42 → 43 → 41**. Transition sites are
`$89D476/$89D4B4/$89D4D9/$89D4FE`; `$89D508` sets `$243` on the final return.
The last request **`$89D48B → $89D735`** has native boundaries
**`D753,D77C,D7AA,D7C0`**. Only after its final acknowledgement/return does
**`$89D495` set `$244`** and complete the first-time tour.

The preceding `tutorial-052` already retains the completed-tour control state at
**41224**. `pandora-tour-control` is the deliberately named **neutral-stability
witness** after 120 more calls, not a claim that control first returned at 41344.
Neither capture pin identifies the precise native cycle of the `$244` write.

The direct fresh witness:

| Checkpoint | Completed frame calls | Map/player | Meaning |
|---|---:|---|---|
| `tutorial-052` | **41224** | 41 `(136,208)` | First retained completed-tour control state |
| `pandora-tour-control` | **41344** | 41 `(136,208)` | Neutral stable, ordinary script `$84A258`, no text/mask/carry |
| `pandora-left-rest` | **41476** | 41 `(120,208)` | Left12 + neutral120 |
| `pandora-up-rest` | **41608** | 41 `(120,192)` | Up12 + neutral120 |
| `pandora-neutral-stable` | **41788** | 41 `(120,192)` | Additional neutral180, unchanged |

Final observed event set is exactly **`$20,$22,$26,$27,$28,$2E,$FB,$243,$244,$292`**.
No `$21`, `$23`, `$FE`, refusal flags, equipment acquisition, field return, frozen
village, tower arrival or combat is claimed. Source `$88AF3F/AF43` writes `$FE/$23`
in a later continuation; it has not executed at this endpoint. Do not apply its
world effects early or claim captured inventory/stat bytes are qualified new
production initialization data.

## Required source/asset and implementation boundary

The source projection authenticates the entire ROM and pins code/data-window
hashes, typed operands, map/scene table entries and selected exits. It does not
export art or implement collision, a text engine, inventory UI or rendering.

| New required map/profile | Source boundary |
|---|---|
| Wider A corridor | Accepted A base sheet, but new town/door positions and residents; outside the old house-front halo |
| 13 | Scene `$838EAB`; first layer **`$AFB5A9`**, secondary `$B1B88D`; resident `$838EBE/$83ECC6` |
| C changed story | Source resident roster/occupancy; FA/FB lifting and carrying, real throws/hits, door patch, finite reaction sequence |
| E and 20 | Shared house first layer **`$AFCBB3`**; source scenes `$838CFA/$83923A`; selector14 stair motion |
| 21 | Scene `$839268`; first layer **`$B281D1`**; contact box/guide actors, opening reload/forced sequence |
| 41–44 | Shared first layer **`$B0AAC5`**; scenes `$839527/$839569/$8395B7/$8395F4`; guide/room object and tutorial presentation |

Map41's map script `$98831C` supplies layer/definitions but not ordinary graphics/
palette loads. Compact controller **`$89D24E`** instead supplies graphics source
**`$E15DD5`** to `$8684E1` at VRAM base0. Its seven COP5A transfers copy colors into
`$7F0600 + 2*destination`. **COP5A is bank-first**, not an ordinary little-endian
24-bit pointer:

| Source | Color destination/count |
|---|---|
| `$B1DEFA` | 16 / 112 |
| `$AFE45B` | 24 / 8 |
| `$AFE47B` | 40 / 8 |
| `$AFE49B` | 56 / 8 |
| `$AFE4BB` | 72 / 8 |
| `$AFE49B` again | 88 / 8 |
| `$AFE4FB` | 104 / 8 |

Apply transfers in order. Maps42–44 only reload the shared layer; do not pretend
these are standalone house-style map-load recipes or assume absent graphics/
palette commands mean absent resources. Full resource extents, composition,
layer/camera profiles, expanded movement/material admission, native text rasters,
animations and portable integration remain the respective implementation/asset
owners' work. The captures make those tasks possible **without further navigation**.
No whole-frame/RGB equivalence, general town simulation, general item system or
complete animation scheduler is accepted by this source gate. In particular,
the historical cellar-band RGB discrepancy remains separate from this route.

## Reproduce and verify

```sh
sh tools/pandora-qualification/replay.sh "$ROM"
# Existing fixed-epoch captures, no new Session ($CAPTURE is a journey directory):
python3 -B tools/pandora-qualification/check.py "$ROM" "$CAPTURE"
python3 -O -B tools/pandora-qualification/check.py "$ROM" "$CAPTURE"
python3 -B tools/pandora-qualification/test_source.py
python3 -O -B tools/pandora-qualification/test_source.py
python3 -B tools/pandora-qualification/test_check.py
python3 -O -B tools/pandora-qualification/test_check.py
python3 -B tools/pandora-qualification/test_epoch.py
python3 -O -B tools/pandora-qualification/test_epoch.py
```

`reference.json` contains only selected semantic observations and capture/source/
recipe hashes for `headless-sync-video-v1`. The archived discovery reference
retains missing `$28`, cancel/retry, failed pushing, off-target throw, two actual
hits and warning/open/control distinctions; `--discovery` reports its unrenewed
epoch instead of accepting it under current provenance. No checker loads a state.
Raw ROM, scripts/text, graphics, per-frame traces and every capture stay ignored
under `local/`. `check.py --record` was removed: current pin changes require the
explicit four-root, full-invariant `migrate.py` audit.

RE discovery preceded executable checker work. Source helper tests went red for
the missing bank-first palette reader, then green; the semantic checker was
replaced with a no-op and its mutation controls failed before restoring green.
Normal and optimized Python both run the tests and native/source comparisons.
Exact metadata equality additionally rejects changed capture/provenance hashes;
semantic mutation tests do not rely on that equality. A checker alone cannot
prove capture provenance: probe/source review and an independent fresh replay
are required, not a claim that a plausible JSON file proves input-only play.
