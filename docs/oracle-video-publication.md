# Oracle completed video publication

## Scope and boundary (`headless-sync-video-v1`)

The headless oracle now selects ares's **existing synchronous Screen path**.
`crates/oracle/build.rs` defines `ARES_ORACLE_SYNCHRONOUS_VIDEO=1` uniformly;
a narrowly marked configuration patch in `vendor/ares/ares/ares/ares.hpp`
sets `Video::Threaded=false` only for that build. Without the macro, the upstream
threaded default is unchanged. Both project translation units assert the policy.
The patch retains the ares ISC license at `vendor/ares/LICENSE.txt`; no upstream
Screen algorithm, PPU renderer, game code, portable core, host or assets changed.

For the supported single-thread-confined, one-boot `Session`, the boundary is:

```
accurate PPU screen->frame()
  swap input buffers → refresh → entire OraclePlatform::video callback → clear input
  → scheduler.exit(Event::Frame) → root->run() returns
  → snes_setPixels copies the completed image → Session::run_frame() returns
```

This removes the OS video worker rather than locking its output. Previously,
`Screen::frame()` queued the current publication and returned; the worker could
zero/fill/reallocate `lastFrame` while `snes_setPixels` read it. A mutex alone
would still choose old/new epochs nondeterministically. The prior diagnosis's
partial-zero captures are consistent with that established race, not proof of
each historical interleaving.

**Pixels mean the last frame flushed by `run_frame`, not the current serialized
machine instant.** `save_state` invokes `serialize(true)`, which advances emulation
to synchronization points. It does not flush Rust pixels. Neither tracing nor
state loading refreshes that buffer. In Pandora the unchanged probe saves first,
then captures state/memory and the last flushed pixels. This fix does not change
that schedule or falsely make save-state a passive observer. No extra execution,
sleeps, save calls, restores, warps or RAM patches were added to the qualifying
routes. Calls and inputs are unchanged, including the 6800-call bootstrap.

## Source/lifecycle audit

- `node/video/screen.cpp`: the constructor does not create a worker in synchronous
  mode; `_frame` starts false and is never set true in this branch. `frame`,
  `refresh` and palette refresh use the existing recursive mutex, so nested
  acquisition is safe. The optional refresh hook completes inline too.
- `Screen::quit()` joins unconditionally, but `nall/nall/thread.hpp` (POSIX) and
  `thread.cpp` (Windows) test for a nonzero handle and clear it after joining.
  Joining an uncreated/already joined thread is a no-op. The destructor already
  guards its join with `Threaded`. There is no pending publication to drain.
- The accurate PPU registers `PPU::color`, a presentation-only palette calculation
  reading the deep-black setting. It does not execute a bus access or update the
  DAC's CGRAM latch. No SFC `setRefresh` callback is registered. Screen refresh
  changes presentation buffers, not serialized machine state. libco emulation
  scheduling is distinct from the removed OS worker. Run-ahead remains disabled.
- `OracleCore`/Rust retain the permanent one-boot claim, including failed loads;
  free/reset remain no-ops. Reload/unload is not newly supported. Safe Rust's
  Session is neither Send nor Sync. No raw C ABI concurrent-caller guarantee is
  added. The video callback copies locally and retains no Rust output pointer.
- ARGB8888 → B,G,R,0/XRGB8888, pitch handling and cropping remain unchanged.
  The accurate PPU canvas is 564×484 NTSC / 564×576 PAL, progressive callback
  height 242/288, interlaced doubled; the public buffer remains 512×480×4.
  Destination rows outside the copied extent retain their previous contents.
  In particular, interlaced→progressive padding and immediate post-load pixels
  remain separate limitations, not fixes bundled into this epoch change.

Independent read-only source review approved this configuration and found no new
callback, join, recursive-lock or emulated-scheduler problem. It also identified
pre-existing threaded unload/palette lifetime fragility and rejected-state frame
counter mutation; these are outside this bounded fix. Do not broaden lifecycle
or restore guarantees based on this qualification. PAL runtime was not tested
because the owned input supplied for this work is Japanese NTSC.

A separate execution-capable reviewer approved the final file set, independently
recomputed all source/binary/report hashes and all four exhaustive byte comparisons,
and reran the new regressions in normal/optimized Python before commit. Its review
confirmed the strict-gate failures and disclosed test skips; Cargo results were
log-inspected during review, not independently rerun. Known optional follow-ups
are explicit empty/missing-root rejection in the diagnostic comparator and CI
wiring for these currently manual regressions; neither changes this evidence.

## Red/green and regressions

`tools/oracle-video-qualification/test_publication.py` compiles the **actual
vendored Screen implementation**, not a mock renderer. Before the fix the
policy check failed and the executable exited with “oracle requires synchronous
completed-frame publication.” Afterward it verifies 512 complete 512×480
asymmetric changing RGB frames, callback caller-thread identity and completion
before return, including deliberate yields inside the callback, repeated quit,
and direct destruction. Separate checks retain the upstream default and enforce
both headless translation units/build policy. Tests use explicit checks, also
under optimized Python; no timing-sensitive race must happen for a red result.

The exact-byte comparator was first red for its missing implementation, then
verified against changed pixel/non-pixel bytes, missing/extra artifacts and
changed full logs. It exports *all* differences and exits 1 for **any** mismatch;
it is not an alternate acceptance checker or a pixel allowlist.

Results in this worktree:

| Command / suite | Result |
| --- | --- |
| `python3 [-O] -B tools/oracle-video-qualification/test_publication.py` | 3/3 each; 512 publications each |
| `python3 [-O] -B tools/oracle-video-qualification/test_compare.py` | 1/1 each |
| `cargo test -p oracle [--release] -- --nocapture` | Debug and release: 21 unit tests; JP reset trace/scenario and two-boot sprite-read test pass. SRAM trace and EU scenario skip: inputs absent |
| `cargo clippy -p oracle --all-targets -- -D warnings` | Pass |
| New Game `test_route.py` / `test_startup.py` / `test_verify.py`, normal and `-O` | 3 / 3 / 2 pass each |
| `cargo test -p assets -- --nocapture` | All 17 test binaries pass (106 reported tests); EU ROM and ignored room-runtime capture checks skip |
| Pandora `test_source.py` / `test_check.py`, normal and `-O` | 6 / 12 pass each |
| House conversation `test_check.py`, normal and `-O` | 16 pass each |
| Player sprite `test_checks.py` / `test_scene_order.py`, normal and `-O` | 2 / 3 reported pass each; export mutation test skips absent local export |
| Player animation `test_compare.py`, normal and `-O` | Baseline + 9 negative controls pass, using copied existing non-pixel house-a inputs and freshly built comparator |
| Frozen Pandora strict checker, fresh A/B, normal and `-O` | **All four fail**, first at prefix `exit-trigger.pixels`; expected migration blocker, not suppressed |

The existing oracle scenario tests include their existing snapshot round trips;
those are separate from the fresh no-restore Pandora runs. No old golden was
edited to make a regression pass.

## Fresh complete Pandora evidence

Local worktree: `/Users/lainsoykaf/repos/ilar-task-oracle-video`.
Two separate fresh processes of the same rebuilt producer ran the frozen route:

- `local/oracle-video-qualification/replay-Tk2j5r/a/journey`
- `local/oracle-video-qualification/replay-Tk2j5r/b/journey`

Each has its sibling `journey.jsonl`. Both reach final call **41788**. Their
**2,682 files are all byte-identical**: 383 checkpoints × seven complete capture
surfaces plus the recipe. All 383 full `.pixels` files (983040 bytes each) match,
not just crops or final images. The full 35,371-line log also matches.

Exact comparisons against the three supplied old routes:

| Old route | Identical non-pixel files (including recipe) | Identical full log | Changed full pixels |
| --- | ---: | --- | ---: |
| Source original `pandora-source/.../journey` | 2299/2299 | Yes | 171/383 |
| Parent `terranigma/.../replay-JmCgU8/journey` | 2299/2299 | Yes | 174/383 |
| Same old binary `pandora-source/.../diagnosis-gAPhmb/journey` | 2299/2299 | Yes | 173/383 |

Non-pixel equality includes every entire synchronized `.state` (CPU, PPU, APU
and other serialized machine state), WRAM, VRAM, CGRAM, OAM and OBJ byte, not only
selected semantic fields, positions or the shim's dummy zero cycle counter.
`source.json`/recipe/checkers and the standalone manifest, lock and generated
probe/bootstrap sources are unchanged. Producer source/configuration hashes and
the executable necessarily change; they are explicitly retained in
[`evidence.json`](../tools/oracle-video-qualification/evidence.json).

Many moving/transient images change because publication now completes before
flush instead of observing a potentially previous frame. Do not interpret only
the seven historical torn captures as needing migration, or assign an exact old
pixel epoch that was never guaranteed. `tutorial-052`, `pandora-tour-control`,
`pandora-left-rest`, `pandora-up-rest`, and `pandora-neutral-stable` pixels match
all three old sets. This is stable **observer reproducibility**, not a new claim
of portable RGB fidelity or correctness of unrelated rendering/crop policies.

Full private exports are `fresh-comparison.json`, `old-pandora-qualification.json`,
`old-replay-JmCgU8.json`, and `old-diagnosis-gAPhmb.json` under the replay root.
They retain every differing filename, size and old/new hash. Public evidence
retains their hashes, per-surface inventory hashes/counts, full-log hashes and
producer/source identities. No raw captures, executable, ROM or game data is
committed. An initial invocation failed at `create_dir` before ROM loading
because its output parent was absent; the retained A/B runs are new successful
processes, not resumed sessions.

## Reproduce without repinning

```sh
python3 -B tools/oracle-video-qualification/test_publication.py
python3 -O -B tools/oracle-video-qualification/test_publication.py
python3 -B tools/oracle-video-qualification/test_compare.py
python3 -O -B tools/oracle-video-qualification/test_compare.py
sh tools/house-conversation-qualification/build.sh
mkdir -p local/oracle-video-qualification
out=$(mktemp -d local/oracle-video-qualification/replay-XXXXXX)
binary=${CARGO_TARGET_DIR:-local/house-conversation-qualification/probe/target}/release/house-conversation-probe
shasum -a 256 "$binary"
for run in a b; do
  mkdir "$out/$run"
  "$binary" "$ROM" "$out/$run/journey" \
    <tools/pandora-qualification/route.jsonl >"$out/$run/journey.jsonl"
done
python3 -B tools/oracle-video-qualification/compare.py \
  "$out/a/journey" "$out/b/journey" >"$out/fresh-comparison.json"
# Expected nonzero status if ANY old pixel epoch differs; retain the full report.
python3 -B tools/oracle-video-qualification/compare.py \
  "$OLD_CAPTURE_DIR" "$out/a/journey" >"$out/old-comparison.json"
# Also run the real gate unchanged. Do not use --record.
python3 -B tools/pandora-qualification/check.py "$ROM" "$out/a/journey"
python3 -O -B tools/pandora-qualification/check.py "$ROM" "$out/a/journey"
```

**Migration remains parent/source-owner work.** Independently repeat A/B, review
this completed-publication epoch, then version observer policy/provenance and
requalify all affected prefix/selected pixel fixtures. Provenance must include
the build definition, unity source and patched ares configuration header, not
just the shim and serializer. The old strict checker currently stops at its
prefix pixel hash *before* final `validate`; its semantic gate is not reported
as passing on these new runs. Exhaustive unchanged machine evidence is reported
separately and honestly. No checker masks, reference repins, source operand
changes or Pandora reference edits belong in this fix. The issue stays open
until independent parent acceptance and reviewed migration.
