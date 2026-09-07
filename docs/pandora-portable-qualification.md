# Offline Pandora portable qualification

Status: **aggregate compiler and restored input-only route pass offline**. Nothing
here enables the live host. The completed source/navigation qualification remains
in [pandora-navigation.md](pandora-navigation.md); this document tracks the new
`PandoraData` adapter and actual input-only continuation separately.

## Host-free Wasm library boundary

`map-inspector` now has an `rlib` target whose normal `wasm32-unknown-unknown`
dependency closure is only `assets`, `rom`, `room-core`, and `serde_json`; the
native `oracle` dependency is target-gated. `PandoraPreview::from_rom_bytes`
authenticates an in-memory Japanese ROM and compiles the accepted Pandora
profile. The stateful Rust API is deliberately small:

- `step(u8)`, `new_game()`, `reset()`, and `state()` retain the existing input
  protocol and schema-versioned `serde_json::Value` projection;
- `art()`, `bitmap()`, `exterior_bitmap()`, and `extra_bitmap(key)` borrow
  immutable source-derived buffers owned by the preview;
- interior/exterior BMP construction is pure and in memory. The native
  `render-map` command calls the same renderer before writing its unchanged
  files; preview construction no longer exports and reads them back.

The compile gate is
`cargo build --locked --target wasm32-unknown-unknown -p map-inspector --lib`,
plus a target-resolved graph check that rejects `oracle`. This is an `rlib`
portability boundary, not a browser-loadable ABI. A follow-up state-owning
`wasm-bindgen` facade must translate errors/JSON, copy borrowed output buffers,
and supply local-file bootstrap and browser transport. Caching, input/render
scheduling, deployment, and end-to-end browser qualification remain outside
this stage; no gameplay/compiler logic belongs in JavaScript.

The native capture producer still compiles the shared source modules in its
historically pinned binary context. Moving that host to consume the library
would alter the exact authenticated `main.rs` registration proof, so it is
intentionally deferred to explicit same-output producer revalidation. The
frozen observer, migration, epochs, and output pins are unchanged.

## Reproduction

Requires the locally owned headerless Japanese ROM, SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
No ROM, extracted grids, native captures or runtime snapshot initializers are shipped.

```sh
sh tools/pandora-runtime-qualification/run.sh 'local/Tenchi Souzou (Japan).sfc'
```

The standalone crate is generated below ignored `local/`; it imports the source
compilers directly, without modifying host module registration. Optional second
argument selects a test. Current result: **11 tests pass**:

- Authentic 33-resource / 34-invocation text, source page IDs and choice contexts.
- Fourteen raw collision profiles, source pot catalog and temporary C occupancy.
  All four C variants retain source wooden-door words `(8,19)=1CF2`,
  `(8,20)=1CF3`. E/20 are unconsumed source bases, not captured/used pot states.
- Source-scoped raw material policies, plus exact resident/box contacts and
  structural compatibility with the delivered `PandoraData` constructor.
- Six COP14 reconstruction samples, source standing facing and mutation controls.
- All33 graph cue recipes: exact key/count, preserve-player operation, genuine
  reload samples and constructor compatibility.
- Complete source-ordered exit lists, including the oversized Town record, and
  the exact two-boundary stair loads/arrivals.
- Full39-motion aggregate construction, repeatable identity, canonical snapshot
  roundtrip and identity sensitivity for every section plus the base-house data.
- The existing house input prefix reaches mapA `(538,815)` at tick1701 with26.
  This remains a separate unchanged-base regression.
- Full fixed input-only route:11,590 actions/restores, all34 direct invocations,
  true door/hit/readiness/ordered-load semantics, stable two-axis final41 control.
- Removing the real North-door interaction makes the fixed replay fail; additional
  live-state negative probes check wrong facing, repeated opening, manual interaction
  during handoff and acknowledgement after final control.

The runner starts solely from `GameState::new_game`. It applies each public input
both to the continuing state and a restored twin, compares result and snapshot,
then restores the continuing state again after success. Frame output, semantic
story output and the fully patched `effective_room` must also match after restore.
Rejected actions must leave canonical state unchanged. The prefix is copied as inputs from the existing
house qualification; its original fixture and compiler are untouched.

## Fixed portable itinerary

`tools/pandora-runtime-qualification/route.json` contains **276 run-length encoded
public input commands**, expanding to11,590 actions. It starts at NewGame, not at
mapA or a checkpoint. Eighty-neutral-input waits are deliberate player inputs;
8,667 total neutral inputs are included, not hidden engine completion timers.
The route deliberately misses once with source pot5, then lands two actual door
hits with pot3/4. All34 direct `Invocation` keys are checked in source order,
including the four distinct repeated-resource leave invocations.

The qualification test does **no planning, BFS, coordinate injection, flag write,
ledger initialization or captured-checkpoint initialization**. Source geometry and forward input
simulations were discovery aids only; they are absent from the fixed replay.
Expected positions and semantic/snapshot hashes are assertions after inputs, never
initializers. Canonical per-action restoration is the only explicit restore.

Selected portable milestones (logical ticks, **not native video frames**):

| Tick | Observation |
| ---: | --- |
| 965 | grant26 from the existing house conversation |
| 1701 | A `(538,815)`, continuous house-prefix endpoint |
| 2618 | actual North Interact at `(472,304)`, Town mask1 |
| 3187 | ResidentGrant returns at13 `(360,144)`, grant28 |
| 4333 / 4517 | Town return crosses `(472,400)` → `(504,400)`, frozen bird retained |
| 5520 | actual Home Interact at `(504,768)`, Town mask2 |
| 5706 / 5778 | C load grants27 / direct choice return grants2E |
| 6431 | off-target throw recovered empty; consumed mask4 and counter0 |
| 7657 | first actual door hit; consumed mask5, damaged sheet |
| 9128 / 9189 | second actual hit, consumed mask7 / completion grants292 |
| 9353 / 9558 | E/20 loads retain open cellar+mask7, reset locals/counter |
| 9763 |21 replaces the sheet, consumed mask0 |
| 10087 | first box callback/recoil completes at `(136,359)`, local1 only |
| 10121 | warning return sets local2; still outside opening gate |
| 10129 | gate `(136,368)` owns BoxAcquireControl;22 still absent |
| 10130 / 10131 | successful handoff grants22 / real same-map reload resets locals |
| 11218 / 11222 | final41 startup grants243 / TourFinal return grants244 and control |
| 11314 / 11406 / 11498 / 11590 | player-controlled `(120,208)` → `(120,192)` → `(136,192)` → `(136,208)` |

A fresh Down approach fromY360 reaches the exactY370 first-contact witness. A
continuous hold from the stair arrival skipped that witness in discovery; it was
not repaired by broadening contact or collision admission. Opening remains a
separate local1/local2 polling gate, not a second callback.

Each run writes ignored `local/pandora-runtime-qualification/report.json`, binding
ROM/profile, input-fixture digest, final snapshot digest, action/restore count and
152 semantic-change observations. The fixture pins both final snapshot and full
semantic trace hashes; checkpoint assertions independently cover the prefix,
Town interactions/Y400 corridor, contact/gate and final two-axis control. The
report is output only and is never read to initialize or drive the replay.

Debug and optimized release replay agree. After the normal harness builds:

```sh
PANDORA_ROM="$PWD/local/Tenchi Souzou (Japan).sfc" cargo test --quiet --release \
  --manifest-path local/pandora-runtime-qualification/build/Cargo.toml \
  input_only -- --include-ignored --nocapture
cargo clippy --manifest-path local/pandora-runtime-qualification/build/Cargo.toml \
  --all-targets -- -D warnings
```

The generated harness now also applies the workspace all/pedantic/missing-docs
lint policy. Actual workspace public-registration Clippy remains a separate check.
No new native producer/observer equivalence is claimed: the earlier navigation
qualification and its36 frozen-bird differences remain distinct evidence. Parent
owns fresh producer pin revalidation and host/presentation acceptance.

## Compiled source inputs so far

`crates/map-inspector/src/pandora_progression.rs` exposes
`pub fn compile(rom: &rom::Rom) -> Result<GameData>`. It extends the existing
`house_progression` compiler; no fresh fixture replaces that base. Raw words are never
rewritten to disguise material types. Text metadata includes source page keys,
rasters/dimensions/boundaries/acknowledgements and invocation sites. The pot catalog
is derived from the admitted source C halo and FA/FB fallback operands.

Shared-sheet ownership is core's: constructor C grids retain original pot and
wooden-door cells; the live ledger and authoritative wooden-door flag apply
patches. Core commit `a2a6afe` is present locally as `d11c3df`, after the equivalent
`ce617cf` polling prerequisite (`da42ea2`). The adapter does not duplicate the
cache lifecycle or initialize consumed pots.

### Forced arrival samples

COP14 `$808A23` queues coordinates; `$80F7F3` installs raw+(8,16). Ordinary
exit adjustments do not apply. `forced_arrivals` supplies reconstruction endpoints;
`compile_cues` surrounds these with source-linked semantic completion/delay samples.

| Cue | COP14 | Mode / selector | Loaded and settled Ark |
| --- | --- | --- | --- |
| BoxReload | `$88AD53` | 7 / 1 | 21 `(136,368)` Down |
| OpeningFourth return | `$88AEAB` | 4 / 2 | 41 `(136,208)` Up |
| TourLeave41 return | `$89D476` | 0 / 2 | 44 `(392,464)` Up |
| TourLeave44 return | `$89D4B4` | 0 / 2 | 42 `(136,464)` Up |
| TourLeave42 return | `$89D4D9` | 0 / 2 | 43 `(392,208)` Up |
| TourLeave43 return | `$89D4FE` | 0 / 2 | 41 `(136,208)` Up |

Arrival table `$848800/$848802` selects `$8488AC/$8489D1`; their player targets
are `$84A2F3/$84A308`. Both tails clear actor bit1000, run COPB6, select stationary
table0 with null movement using COP84, COP8E, then jump `$84A254`. Sequence0 is
Down, sequence1 Up. Source bytes for the complete Up tail `$84A308..$84A31D`:

```text
BD 04 00 29 FF EF 9D 04 00 02 B6 02 84 01 00 00 02 8E 4C 54 A2
```

The existing player-animation qualification authenticates the standing-script
range `$84A2E9..$84A389`; no second animation decoder is introduced.

### Complete cue catalog and semantic pacing

`compile_cues` authenticates the owned ROM and expands33 fixed source recipes into
immutable `MotionSpec`s:27 non-load cues and six forced-load cues. It does not
interpret events or put recipe instructions/source addresses into core. Metadata
retains each recipe, delay/completion site, dependency-range digest and policy.
All non-reload samples preserve Ark's position/facing, except successful
`BoxAcquireControl` selects stationary Down. Only the six COP14 samples set an
absolute position. The cue catalog is combined with six ordinary/stair travel
motions by the aggregate compiler below.

Notation below: **B** is one semantic finite cooperative-completion sample,
**D(site,n)** reads and checks the exact COPC1 operand as n logical actor-delay
units, and **L** is one source reload sample. B is not a native frame or a timer
that makes a prerequisite disappear. Worker/resident/guide internal movement and
loop timing are collapsed at the named completion, while Ark remains stationary.
The final sample immediately completes the graph; no extra trailing wait.

| Cue (`R` means invocation return) | Recipe |
| --- | --- |
| R(CEntry), R(CApproach), R(FirstHit) | B at next-request/local1-clear boundary |
| SecondHitPatched | B + D(`889B9B`,60) |
| R(SecondHit) | D(`889BA5`,16) + B (fourth resident departed) |
| ReactionColorMath | B (worker local4 at `889D1F`) |
| R(ReactionSpeaker) | D(`889BC8`,60) |
| ReactionColorReturn | B (worker local6) + D(`889BD9`,60) + B (speaker next request) |
| R(ReactionRequest/Right/Left/Final) | B each; complete the respective cooperative movement/departure |
| R(BoxWarning) | D(`88ADBD`,32) |
| BoxAcquireControl | B (successful-COPDF certificate below; stationary Down) |
| R(OpeningFirst/Second/Third) | D(`88AE81`,60) / D(`88AE8F`,30) / D(`88AE9D`,30) |
| R(TourIntro/One/Two/Three/Four/Five) | B each, guide/controller agreement at04BC=2/4/6/8/10/12 |
| R(TourSix) | B (04BC=14) + D(`89D46C`,32) |
| R(Tour44/42/43) | D(`89D4AA`,32) / D(`89D4CF`,32) / D(`89D4F4`,32) |
| BoxReload | L + D(`88AE6C`,120) + D(`88AE73`,240); source COP30 between delays |
| R(OpeningFourth) | D(`88AEA7`,120) + L + D(`89D3DC`,60) |
| R(TourLeave41/44/42/43) | L + destination delay60 at `89D4A0/89D4C5/89D4EA/89D487` |

The six forced recipes include destination startup delays **before** their first
request. Ordinary17/load/17 pacing does not apply to these COP14 transfers.
FirstHit's inspected `88ABC2..88ABCD` return is unmask→local1 clear→jump, with no
COPC1. Right/left return windows likewise contain movement/cooperation, no COPC1.
Do not await future local9 inside R(ReactionRequest); later requests own it.
No returned cue exists for ResidentFirst/Retry/Refusal/Grant, CChoice/CDirect,
BoxEntry or TourFinal. In particular, TourFinal returns directly to control/244.

**Graph-atomic visibility, not source-scheduler fidelity:** local0A writes precede
the three opening delays natively, but core writes them at final completion; cue
scenes expose the intended presentation independently. Local6 likewise becomes
visible only after the combined return-worker/speaker recipe. Locals2/7/8/9 may
precede subordinate movement natively but commit after its portable boundary.
Source292 precedes the second-hit request delay; source243 follows queued load
before destination startup. Core exposes both only at cue completion. Native
occupancy clears after local6; portable control stays conservatively owned through
final C departure. These are delayed atomic commits, not skipped prerequisites.

**Outside this cue API:** BoxEntry pre-request delay80 at `88AD91`, post-return
32 at `88AD9B`, and warning pre-request32 at `88ADB3` are not represented by core's
direct request entry/return paths. This catalog does not reconstruct those waits
or relocate them into R(BoxWarning). Such direct paths are atomic semantic
presentation boundaries, not a native elapsed-time claim. Full aggregate/runtime
acceptance must retain this explicit fidelity limit.

### Bounded successful-COPDF certificate

Source advisor independently traced the admitted northern centerline contact:
`$85D745` / `$85F8D1` chooses recoil direction1, `$85D748..D754` schedules
`$848000`, and the direction1 continuation `$84802B..$84803B` returns through
`$8488AC`. Completed restoration executes `$8488CD STZ $097C` and schedules
standing Down. Thus the semantic result `(136,359)` Down must represent
**completed recoil/rest**, not merely reaching a coordinate.

After completed rest, admit only the qualified ordinary grounded walking/neutral
subset and warning continuation. The ordinary standing/walking interval
`$84A258..$84A389` has no readiness-word writes; warning `$88ADA5..$88ADCB`
changes masking/locals, not `$097C`. Exclude other action/recoil/airborne/guard
continuations from this certificate. The later opening gate is not a second
contact callback. Within that closed subset, a logical successful-COPDF boundary
is justified without a wait constant. It is not a claim about every native
scheduler state with a similar pose.

COPDF `$80B827` retries while `$097C & $0810 !=0`. Source mask `$88AD46`,
COPDF `$88AD4A`, grant22 `$88AD4F` and reload `$88AD53` must retain that order.
Installed `$888EA6` selects stationary Down without changing coordinates; it is
**post-success facing evidence, not evidence for its own readiness precondition**.
If the runtime admission cannot carry completed-rest semantics, omit handoff
completion and fail closed. Proximity, warning completion or a timer alone is
insufficient.

## Delivered early APIs

`051b127`, `73f51ff`, `51d558c` are applied locally as `ea81720`, `85c6fe1`,
`5f0821b`. The adapter uses `MotionPose::Absolute` for genuine reconstruction
samples; the33 compiled cue motions use `Preserve { facing }` between reloads.
The enabled core envelope is schema4/profile12, still320 bytes.

`compile_rooms` now installs core `MaterialRule`s over unchanged source words:
TownSolid25 only over Town's authenticated halo; ClosedDoorPartial5 and
StairOpen29 Up-only at C `(11,21)`; StairOpen29 Up-only at E `(6,53)` and20
`(22,53)`. Other rooms have no aliases. The source navigation compiler authenticates
all sixteen dispatch tables before adaptation. Core retains delayed collision
direction, old-edge slope rejection before bit15, and aliases after occupancy.
The policy is carried by `Room` and must be included in aggregate identity.

## Ordered navigation and aggregate identity

Core `26210f2`, `b69d358`, `041dcf2` are applied as `ad1df0f`, `e26970c`,
`0a88364`. Oversized-exit correction `0fd1f08` is applied as `cb57353`. The enabled
envelope is schema5/profile13, still320 bytes. All source lists for A/13/C/D/E/20
are retained in order, including A ordinal8 `$818DB3`:
`00 3E 50 02 03 00 00 55 10 02 10 02`. Its width80 deliberately overhangs the
width64 grid. It is neither dropped nor clipped; core selects first coarse then
only that record's fine test and rejects unsupported selected exits atomically.

Six travel bindings identify source record ordinals, not destination searches.
A→13/13→A/A→D compile35 samples from the source selector adjustments and shared
17/load/17 policy. C→E/E→20/20→21 compile two semantic boundaries: source-adjusted
load then arrival completion `(+14,+23)`, not ordinary walking or native timing.
Town's two doors start closed and require their real Up interactions. Patches are
derived from authenticated closed/open source profiles, upper then lower:
North `(472,304)` → handoff `(472,288)`; Home `(504,768)` → `(504,752)`.
No trigger bypass replaces ordered selection.

Aggregate identity hashes a canonical JSON manifest with full ordered exits,
bindings/door patches, raw grid digests, dimensions/halos/material policies,
text/raster metadata, shared source objects, contacts/opening gate/lane assertion,
all39 expanded motion samples including pose/scene/reload/trigger, source evidence
and semantic policy. It also hashes the existing house compiler's freshly
ROM-derived NewGame serialization, which contains that base's data identity.
This serialization is **hash input only**, never restored or used to initialize
Pandora. Runtime construction remains `base.with_pandora(...)`; the itinerary must
start separately with `GameState::new_game`.

Independent source/API/identity review found no blocker; the known oversized-exit
constructor failure is now green. Actual workspace Clippy is run with temporary
public navigation/progression registration; registration is restored, not committed.
Standalone harness lint configuration alone is not equivalent to workspace pedantic.

## Remaining integration gates

1. Parent reproduction/acceptance of this exact compiler and fixed portable route.
2. Parent-owned host wiring, presentation/carry/world-patch acceptance and fresh
   producer pin revalidation. Offline success does not implicitly enable live play.

Semantic preview may use finite logical cue completion, not native frame-count
claims. Ordinary doors retain 17 departure / load / 17 arrival. Source COPC1
operands may count logical actor-delay units. Preserve prerequisites, actual
reloads and completion ownership; no fallback timers. C's temporary occupancy
clears after local6, while source input unmasking precedes final departure;
conservative ownership through departure must be labeled semantic policy.

The fixed Town return uses the bird-safe Y400 corridor. Frozen bird `$838A19`
and its `(28,25)` conflict remain intact, rather than forcing the historical Y401
line. Final control checks require completed flags243/244 and stable two-axis
movement, not simply reaching map41. The issue stays open for parent acceptance.
