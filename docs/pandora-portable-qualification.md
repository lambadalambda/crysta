# Offline Pandora portable qualification

Status: **aggregate compiler complete; full input-only route not yet qualified**. Nothing
here enables the live host. The completed source/navigation qualification remains
in [pandora-navigation.md](pandora-navigation.md); this document tracks the new
`PandoraData` adapter and actual input-only continuation separately.

## Reproduction

Requires the locally owned headerless Japanese ROM, SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
No ROM, extracted grids, captures or checkpoints are shipped.

```sh
sh tools/pandora-runtime-qualification/run.sh 'local/Tenchi Souzou (Japan).sfc'
```

The standalone crate is generated below ignored `local/`; it imports the source
compilers directly, without modifying host module registration. Optional second
argument selects a test. Current result: **9 tests pass**:

- Authentic 33-resource / 34-invocation text, source page IDs and choice contexts.
- Fourteen raw collision profiles, source pot catalog and temporary C occupancy.
  All four C variants retain source wooden-door words `(8,19)=1CF2`,
  `(8,20)=1CF3`. E/20 are unconsumed source bases, not captured/used pot states.
- Source-scoped raw material policies, plus exact resident/box contacts and
  structural compatibility with the delivered `PandoraData` constructor.
- Six COP14 reconstruction samples, source standing facing and mutation controls.
- All33 graph cue recipes: exact key/count, preserve-player operation, genuine
  reload samples and constructor compatibility. No executed Pandora route yet.
- Complete source-ordered exit lists, including the oversized Town record, and
  the exact two-boundary stair loads/arrivals.
- Full39-motion aggregate construction, repeatable identity, canonical snapshot
  roundtrip and identity sensitivity for every section plus the base-house data.
- The existing house input prefix reaches mapA `(538,815)` at tick1701 with26.
  This is only the regression prefix, **not** Pandora completion.

The runner starts solely from `GameState::new_game`. It applies each public input
both to the continuing state and a restored twin, compares result and snapshot,
then restores the continuing state again after success. Rejected actions must
leave canonical state unchanged. The prefix is copied as inputs from the existing
house qualification; its original fixture and compiler are untouched.

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

1. Execute the full input-only NewGame→final41 route and restore after every action.
2. Qualify direct pot hits, cue/contact/readiness/reload order, shared-sheet
   continuity and stable final two-axis control using this exact aggregate.
3. Parent-owned host wiring/presentation and fresh producer pin revalidation.

Semantic preview may use finite logical cue completion, not native frame-count
claims. Ordinary doors retain 17 departure / load / 17 arrival. Source COPC1
operands may count logical actor-delay units. Preserve prerequisites, actual
reloads and completion ownership; no fallback timers. C's temporary occupancy
clears after local6, while source input unmasking precedes final departure;
conservative ownership through departure must be labeled semantic policy.

The eventual Town return must use the bird-safe Y400 corridor. Keep frozen bird
`$838A19`; do not erase its `(28,25)` conflict to force the historical Y401 line.
Final acceptance requires completed flags243/244 and stable two-axis final41
player control, not simply reaching map41. The tracking issue remains open.
