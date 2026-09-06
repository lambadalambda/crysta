# Pandora observer epoch migration and wrapper audit

## Qualified scope

Current epoch: **`headless-sync-video-v1`**, following the separately reviewed
video fix (`0db0aad` in the parent; cherry-pick `51972b0` here). This is an
explicit publication-policy change, not backward equality with threaded output.
No runtime, assets, host code, input recipe or save schedule changes are part of
this migration.

The completed image is the last explicit `run_frame` publication **before** the
probe's `save_state` synchronization. It is not an image recaptured at the later
serialized machine instant. `epoch.py` pins that policy and nine source files:
oracle `build.rs`/Rust API, `ares-unity.cpp`, shim, patched `ares.hpp`, Screen,
PPU frame/color code and system serializer. Main and prefix references enforce
those hashes, including the build macro and both compile-time synchronous-policy
assertions. Probe, bootstrap and standalone build-script hashes are unchanged.
Current source hashes do not authenticate a historical binary/compiler. The
video qualification's `evidence.json` separately records its producer/build
identity; its source-after hashes authenticate the admitted migration policy.
The root workspace lock is not the standalone probe lock; this is not a promise
of a fully dependency-locked environment.

## Complete old/new audit

Private capture roots:

- Original: source worktree `local/pandora-qualification/journey`.
- Parent fixed twins: parent worktree
  `local/oracle-video-qualification/replay-WKFf0d/{a,b}/journey`.
- Video sibling fixed twins: video worktree
  `local/oracle-video-qualification/replay-Tk2j5r/{a,b}/journey`.

Recomputed exhaustive comparisons establish:

| Comparison | Artifacts | Full log | Differences |
| --- | ---: | --- | --- |
| Parent fixed a vs b | 2682 each | Exact | None |
| Sibling fixed a vs parent fixed a | 2682 each | Exact | None |
| Original vs parent fixed a | 2682 each | Exact | **171 `.pixels` only**, same extents |
| Old parent `replay-JmCgU8` vs parent fixed a | 2682 each | Exact | **174 `.pixels` only**, same extents |

The parent's `old-comparison.json` uses **old parent `replay-JmCgU8`**, not the
original source capture. Its entire report was independently reproduced exactly.
All 383 captures of **state, WRAM, VRAM, CGRAM, OAM and OBJ** are byte-identical
across the original/fixed comparison. The complete log SHA remains
`d19191a33eb0e41603d2d80d24e054bb4e6510ec0218d94f57bb43303bb09339`;
the frozen 383-line recipe SHA remains
`d43c3fd2b6ff49aa8780e67d8f943aa036935686ae8dbd8912d6abc3ae97e69c`.
Every frame, button command, synchronizing checkpoint and final finish is retained.
This is broader than just comparing the selected semantic checkpoint table.

`migration.json` retains all old→fixed changed artifact names, extents and
old/new hashes, per-surface inventory hashes, exact fixed-twin/sibling comparison
reports, and old/current fixture hashes. It is an audit record, **not** a
runtime pixel allowlist. `migrate.py` rechecks complete inventories/schedules,
authenticates original selected/prefix captures against archived references,
authenticates ROM/source/probe policy, and rejects any non-pixel/log/recipe change
or unreproduced fixed output before offering the explicit `--write` operation.
It never updates the archive. Default invocation compares, without writing.

Migration changes only:

- **106 of 213** main selected pixel hashes; every non-pixel/semantic field stays
  equal. Main observer source hashes expand from three to nine, the shim hash
  changes, and the explicit epoch/completed-publication policy is added.
- **4 of 31** embedded prefix pixel hashes: `exit-trigger`, `landed-A`,
  `exterior-walk-down-settled`, `exterior-walk-settled`. Prefix policy/provenance
  are added in Pandora's own `prefix-reference.json`. The original standalone
  house-conversation reference is unchanged.
- Original main/discovery/prefix metadata are byte-preserved in
  `epochs/threaded-video-v0/`. Discovery is not silently assigned the new epoch.

The strict main checker passed normally and under `-O` on **both parent and both
sibling fixed roots**, including source checks, all prefix surfaces, semantic
validation and exact selected-reference equality. No pixel surface is ignored,
masked or conditionally accepted. Old main roots fail the current epoch, as they
should. `--discovery` explicitly reports its unrenewed epoch. Historical discovery
semantic controls still run as ROM-free mutation tests. The source warning at
`88ADF2` still has only `88AE29 D5` and `88AE5E D3`; cursor `88AE50` is glyph56,
already correctly recorded as `text`/no boundary. No native/source metadata
needed a page-count correction.

## Older wrapper audit: not implicitly renewed

This is a read-only audit of existing gates. No wrapper below or its fixtures
were changed. Rebuilding against the fixed oracle changes the observer; a green
Pandora route does not renew a different route or observation schedule.

| Existing wrapper/gate | Contract and remaining renewal requirement |
| --- | --- |
| `house-conversation-qualification/replay.sh` | Seven surfaces at 31 points, complete route/log/bootstrap and semantic checks (`check.py:101–137`). Four old pixel pins are now incompatible. Pandora's embedded prefix is qualified, **not this standalone checker**: it rejects a full Pandora recipe. A separately reviewed authenticated prefix-reading mode or fresh standalone route is required before renewing its fixture. |
| `house-exterior-qualification/replay.sh` | Three complete pixel hashes plus WRAM/VRAM/CGRAM/OAM/OBJ and source/background/grid checks (`check.py:98–161`). All three pixel labels are among the changed prefix points. Same authenticated-prefix reuse is possible, but current full-route validation rejects Pandora; source export is unaffected. Descriptive provenance is not a complete enforced observer pin. |
| `house-dialogue-qualification/export.sh`, `native_check.py` | Source-only exporter; native evidence uses WRAM/VRAM-derived font indices, **not framebuffer pixels** (`native_check.py:12–77`). Parent reports independent text/native checks green. Fixed-prefix non-pixels match all original captures, including unselected `repeat-A`/`followup-X`. No video-driven text/font repin is justified; native checker alone does not authenticate the route/producer. |
| `house-background-qualification/replay.sh` | WRAM/VRAM/CGRAM and reconstructed indices/priorities; no framebuffer gate (`check.py:95–212`). Own census, six writer-stop runs and returned-F evidence are required for epoch renewal. Do not change non-pixel pins merely because the oracle changed. |
| `house-navigation-qualification/replay.sh` | Paired main/extra full logs, endpoints, door changes and walking WRAM (`check.py:118–166`); pixels emitted, not pinned. Own routes and downstream core-route checks remain separate; Pandora save schedule is not a substitute. |
| `house-npc-qualification/replay.sh` | Four framebuffer hashes **and opaque-pixel assertions**, six other surfaces and source/order checks (`check.py:42–128`). Own paired no-save route required; pixel repin alone is insufficient. |
| `house-scene-qualification/replay.sh` | Main/exception census deliberately omits framebuffer hashes but requires pixel extent (`check.py:21–73`, `census.py:111–118`). Own paired routes remain unrenewed; do not silently expand or change this contract. |
| `house-scene-qualification/art-replay.sh` | Art hardware digest omits framebuffer hash, **but isolated/opaque pixel assertions remain** (`art_check.py:75–179`). Own paired art captures plus D-creation traces required. Passive art capture boundaries differ from Pandora's synchronizing saves. |
| `new-game-qualification/replay.sh` | 13 framebuffer checkpoints, WRAM/VRAM/events/entity data, exact 7100-row CSV, actual omission controls and five native trace modes (`verify.py:49–63`, wrapper:32–44). Own complete bootstrap/control evidence required. `semantic_check.py` first calls this strict pixel-bearing verifier, so it is not independently renewed. |
| `new-game-qualification/route-replay.sh` | Paired F→10→F logs/reports/grids/WRAM/events; emitted pixels are not pinned (`route_check.py:27–60`). Own return route needed for renewed status, not a video-driven non-pixel repin. Source-only `startup.py` remains independent. |
| Initial oracle fixture corpus | No `tools/initial*` wrapper. JP/EU smoke fixtures in `crates/oracle/tests/fixtures/` have no pixel-reference schema; native JP/EU assertions, restore/restart and EU coverage remain separate gates, not renewed by Pandora. |

Same label, map or frame is insufficient for reuse. In particular, New Game's
unsynchronized frame6800 WRAM hash begins `49ab74b6`, whereas the conversation
save-synchronized boot hash begins `c02d3c83`. Keeping the exact observation
schedule is mandatory, not optional provenance decoration.

### Map-inspector integration gate remains red

Parent reports `cargo test -p map-inspector`: 47 unit tests pass, but
`crates/map-inspector/tests/local_capture.rs:62` fails the second RGB pin:

- Old expected: `93a224b980b538bf5bbed26c7d7052a6c14777608e6d02162eb55cfb2ef74c8a`.
- Fixed observed: `3833dbdf403939dc5836dca4a49b49e36424360597e5acfc6eddb714372da432`.

This is **not the empty-SRAM Pandora fixture**. `src/main.rs:104–139` authenticates
SRAM `709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`
and runs `qualified-slot-3-right-movement`: Start `[400,408)`, A `[1100,1112)`,
Right `[1800,1840)`, observing after frames1601/1841 without checkpoint saves.
RGB extraction selects even columns of the ABI image into 256×240 BGR→RGB output.
Reaching this assertion implies preceding map/dimensions/cells/layer-hash/frame/
camera/player assertions passed, **not** complete non-pixel equality. The producer
reports WRAM/VRAM/CGRAM hashes, but this integration test does not pin them.

Renewal needs its own old/new manifests or captures, exact fixture/build policy,
full WRAM/VRAM/CGRAM equality at both checkpoints, and repeated fixed RGB evidence.
No pin was changed from the single failure message, no conversion/host workaround
was made, and this gate must stay reported red until its separate audit completes.

## Independent review and regressions

Separate static architecture/policy and execution-capable evidence/correctness
reviews approved the final migration. The latter independently verified all six
native roots, all three parent comparison reports, complete fixture differences
and archived bytes against Git `5b7b88b`. Migration verification passed normally
and under `-O`; all four fixed roots passed both strict checks. Source/checker/
epoch tests passed **6/13/11** each in both modes. Old-main, discovery and `--record`
rejections were also verified. Corrupted observer hashes and legacy authentication
were rejected; no-op exact equality caused **49 failures** per mode. Disabling
alias, full-log or non-pixel/extent guards caused their targeted negative tests
to fail. Publication tests separately passed **3 per mode**, including 512
completed-frame publications each. These are bounded source/reference results,
not a rerun of every wrapper, host integration or whole workspace suite.

## Reproduce checks (no emulation)

From the source worktree, with owned `ROM` and the four capture roots:

```sh
python3 -B tools/pandora-qualification/migrate.py "$ROM" "$OLD" "$FIXED" "$TWIN" "$SIBLING"
python3 -O -B tools/pandora-qualification/migrate.py "$ROM" "$OLD" "$FIXED" "$TWIN" "$SIBLING"
python3 -B tools/pandora-qualification/check.py "$ROM" "$FIXED"
python3 -O -B tools/pandora-qualification/check.py "$ROM" "$FIXED"
python3 -B tools/pandora-qualification/test_epoch.py
python3 -O -B tools/pandora-qualification/test_epoch.py
```

`replay.sh` retains the original one-process input/save recipe and now also runs
the epoch regression tests. Parent post-cherry-pick strict verification and issue
closure remain parent-owned. No shared issue indices were changed. Repository
safety passes. The source worktree's tracker check reports the inherited missing
index/roadmap entries for `fix-oracle-video-publication.md`: the parent cherry-pick
retained that detail without its shared tracker entries. Parent integration must
reconcile those entries; this migration does not edit shared indices.
