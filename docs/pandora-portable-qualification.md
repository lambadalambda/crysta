# Offline Pandora portable qualification

Status: **compiler groundwork, not a qualified Pandora runtime route**. Nothing
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
argument selects a test. Current result: **6 tests pass**:

- Authentic 33-resource / 34-invocation text, source page IDs and choice contexts.
- Fourteen raw collision profiles, source pot catalog and temporary C occupancy.
  All four C variants retain source wooden-door words `(8,19)=1CF2`,
  `(8,20)=1CF3`. E/20 are unconsumed source bases, not captured/used pot states.
- Source-scoped raw material policies, plus exact resident/box contacts and
  structural compatibility with the delivered `PandoraData` constructor.
- Six COP14 reconstruction samples, source standing facing and mutation controls.
- The existing house input prefix reaches mapA `(538,815)` at tick1701 with26.
  This is only the regression prefix, **not** Pandora completion.

The runner starts solely from `GameState::new_game`. It applies each public input
both to the continuing state and a restored twin, compares result and snapshot,
then restores the continuing state again after success. Rejected actions must
leave canonical state unchanged. The prefix is copied as inputs from the existing
house qualification; its original fixture and compiler are untouched.

## Compiled source inputs so far

`crates/map-inspector/src/pandora_progression.rs` currently supplies private
building blocks, not an aggregate `compile` entry point. Raw words are never
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
exit adjustments do not apply. These samples are reconstruction endpoints only;
preceding source delays/cue completion are not yet compiled.

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
samples; `Preserve { facing }` is available for the upcoming non-reload cues.
The enabled core envelope is schema4/profile12, still320 bytes.

`compile_rooms` now installs core `MaterialRule`s over unchanged source words:
TownSolid25 only over Town's authenticated halo; ClosedDoorPartial5 and
StairOpen29 Up-only at C `(11,21)`; StairOpen29 Up-only at E `(6,53)` and20
`(22,53)`. Other rooms have no aliases. The source navigation compiler authenticates
all sixteen dispatch tables before adaptation. Core retains delayed collision
direction, old-edge slope rejection before bit15, and aliases after occupancy.
The policy is carried by `Room` and must be included in aggregate identity.

## Remaining integration gates

1. Source-backed Town wooden-door interaction/patch for A→13 and A→D. Do not start
   Town with open doors or travel from a closed approach.
2. Compile the cue catalog using preserve-player samples. Absolute anchors cannot
   express the rectangular, any-facing box gate without teleport. C residents and
   tour guide move; their coordinates are not Ark's coordinates.
3. Ordered source exit selection: first coarse match, then that record's fine
   test, never fallthrough on failed fine; unsupported records fail closed.
   Delivered runtime currently selects exact motion triggers instead.
4. Aggregate identity binding all base-house and Pandora data/policies; complete
   39-motion catalog; actual New Game→final41 input itinerary and restoration.

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
