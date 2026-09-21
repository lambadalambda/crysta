# Map-inspector slot3 observer renewal

Qualified epoch: **`headless-sync-video-v1`**. This renews only
`crates/map-inspector/tests/local_capture.rs`, not any other legacy wrapper.
The older map-inspector section in
[`../pandora-qualification/OBSERVER-MIGRATION.md`](../pandora-qualification/OBSERVER-MIGRATION.md)
records the pre-renewal failure; this is its separate recipe-specific evidence.
No Pandora empty-SRAM evidence is used.

## Frozen contract

- Japanese ROM SHA-256 `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
- Authentic 8,192-byte three-slot SRAM SHA-256
  `709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
- One native Session, `qualified-slot-3-right-movement`: labels 0 through 1840,
  Start `[400,408)`, A `[1100,1112)`, Right `[1800,1840)`; run the frame before
  observing labels1600/1840 (**frames1601/1841**). No saves or restores.
- Original conversion: first 240 rows, **even columns** of the 512×480 ABI;
  BGR→RGB, 256×240. Exported BMPs encode those RGB bytes in top-down BGR rows.
- Original process-exit behavior and singleton Session are unchanged.

The existing production `capture` mode already retains complete manifests,
WRAM/VRAM/CGRAM and BMPs. It follows exactly the same capture path as `verify`;
export only happens after both observations. No production changes were needed.

## Evidence and review

Source worktree: `/Users/lainsoykaf/repos/ilar-task-capture-renewal`.
Private retained roots below `local/map-inspector-renewal/`:

| Root | Producer |
| --- | --- |
| `old` | Fresh pre-fix build at `8034889f6dbc78181d2458e6f2e05881f48f5e5c` = `0db0aad^`, source worktree `../ilar-task-capture-old` |
| `fixed-a` | Fresh isolated fixed build from `7c5c90be43c4952995b4c4ab24b62aa71ceb6e56` |
| `fixed-b` | Second fresh process using the same fixed binary; **not** a second clean build |

Each root retains `capture/` (all ten files), executable, complete build log,
stdout/stderr and `producer.json`. `capture.py` uses a private Cargo target per
source worktree, and refuses to overwrite a capture root. Inputs are symlinked
at the test's exact owned paths, `local/Tenchi Souzou (Japan).sfc` and
`local/saves/Terranigma.srm`, from `/Users/lainsoykaf/repos/terranigma/local/`.
No raw cartridge, SRAM, capture or binary is committed.

`epochs/threaded-video-v0/local_capture.rs` is byte-preserved from Git; its
`reference.json` archives hash-only metadata, including the complete capture
inventory and canonical nonpixel-manifest digest. `observer.json` explicitly
versions the policy and 13 source files, including build.rs, shim, unity source,
patched ares.hpp and serializer. Old/fixed `main.rs` is byte-identical.
`migration.json` is the reviewed audit report, not a runtime pixel allowlist.
Its `changed` table contains BMP changes only; inventories also retain the
manifest/derived HTML changes caused solely by the second RGB metadata value.

Exhaustive comparisons and independent execution-backed review established:

- **394,240 bytes per run** of WRAM/VRAM/CGRAM across both checkpoints are exactly
  equal old→fixed and fixed→twin. No nonpixel pin changed.
- Complete manifests, including all cells and all other fields, agree after
  removing only checkpoint `rgb_sha256`. Canonical SHA-256:
  `7998be259cce4218983f03bb81e3bf189577958dc9f190ba38880036d680cb22`.
- All **ten fixed capture files** are byte-identical. All six BMPs were decoded
  independently; the first checkpoint RGB remains `78c20d6a…eaa7b0ea`.
- Frame1841 RGB alone changes from
  `93a224b980b538bf5bbed26c7d7052a6c14777608e6d02162eb55cfb2ef74c8a`
  to `3833dbdf403939dc5836dca4a49b49e36424360597e5acfc6eddb714372da432`.
  Both BMP extents remain 184,374 bytes.

The execution-capable reviewer independently hashed owned inputs, every capture,
all producer sources/binaries/logs, checked source blobs/ancestry, and ran all
three retained binaries in `verify` mode: their complete manifests reproduced.
Normal and `-O` audit output both exactly reproduced the reviewed report SHA-256
`db249179718cb6bcf9c1755093d441d0094dd39fc836d079220defd3b289ab3c`.
Review scripts/logs: `../ilar-task-capture-review/local/review-output/`.
A separate static correctness/architecture review approved the bounded tooling.
Two initial review gaps (Rust source inventory completeness and archived
nonpixel-digest authentication) were fixed with red→green controls **before**
evidence approval and RGB renewal.

Source and retained-file hashes authenticate this recorded rebuild, not a
historical original binary or a reproducible compiler environment. Tool versions,
build/invocation commands and selected build environment are recorded and were
reviewed; the checker does not independently prove the truth of historical process
execution. Reusing/copying directories is not evidence of fresh runs.

## Reproduce the retained audit

From the source worktree (no emulation for the Python commands):

```sh
ROM='local/Tenchi Souzou (Japan).sfc'
SRAM=local/saves/Terranigma.srm
E=local/map-inspector-renewal
OLD_REPO=../ilar-task-capture-old
FIXED_REPO=../ilar-task-capture-renewal # explicit historical fixed sources, not current HEAD
T=tools/map-inspector-qualification
python3 -B "$T/check.py" "$ROM" "$SRAM" "$E/old" "$E/fixed-a" "$E/fixed-b" "$OLD_REPO" "$FIXED_REPO" > "$E/rechecked.json"
cmp "$E/rechecked.json" "$T/migration.json"
python3 -O -B "$T/check.py" "$ROM" "$SRAM" "$E/old" "$E/fixed-a" "$E/fixed-b" "$OLD_REPO" "$FIXED_REPO" > "$E/rechecked-O.json"
cmp "$E/rechecked-O.json" "$T/migration.json"
python3 -B "$T/test_check.py"
python3 -O -B "$T/test_check.py"
python3 -B "$T/test_mutations.py"
cargo test --locked -p map-inspector
```

For **new independent processes**, keep the old source worktree at `0db0aad^`
(create with `git worktree add --detach ../ilar-task-capture-old 0db0aad^` if
absent), then use new directories:

```sh
FRESH=$(mktemp -d "$PWD/local/map-inspector-replay-XXXXXX")
python3 -B "$T/capture.py" "$OLD_REPO" "$FRESH/old" "$ROM" "$SRAM"
python3 -B "$T/capture.py" "$FIXED_REPO" "$FRESH/fixed-a" "$ROM" "$SRAM"
python3 -B "$T/capture.py" "$FIXED_REPO" "$FRESH/fixed-b" "$ROM" "$SRAM"
python3 -B "$T/check.py" "$ROM" "$SRAM" "$FRESH/old" "$FRESH/fixed-a" "$FRESH/fixed-b" "$OLD_REPO" "$FIXED_REPO" > "$FRESH/audit.json"
```

The read-only audit enforces old archive authentication, source/input policy,
complete nonpixel equality and fixed-twin equality. It never rewrites a pin.
New audit producer paths/build logs/commit identities may differ; compare the
`old`, `fixed`, `twin`, `checkpoints`, `nonpixel_manifest_sha256` and `changed`
fields with the reviewed report, not the entire producer envelope. Any old RGB
non-reproduction or different fixed output requires investigation, not archive
replacement. Cargo's integration gate enforces the current exact RGB pins and
archived nonpixel digest, with a ROM-free exact source-inventory/hash test.

## Validation

- Initial owned-input Cargo run reproduced the old-pin failure; strengthened
  nonpixel/source assertions also passed while RGB remained red.
- Python gates: **13 tests pass in each mode**. Ten disabled-guard/conversion
  mutations are caught in both modes (**20/20**), using temporary source copies.
- Post-review renewal: full `cargo test --locked -p map-inspector` passes:
  **49 unit + 10 integration tests**, owned local inputs present, none skipped.
- Focused `rustfmt --check crates/map-inspector/tests/local_capture.rs` passes.
  Package-wide format check exposes an inherited `main.rs` module-order change;
  production code was deliberately left untouched.
- Capture subprocess/build glue was exercised with real old/fixed runs rather
  than mocked subprocess TDD. Validator/source/inventory controls used red→green.

Other legacy wrappers remain audited but **unrenewed**. Shared issue-index
closure and parent integration remain parent-owned.

## Parent acceptance

The parent independently recomputed both audit modes and matched `migration.json`,
reran 13 tests per mode and detected all20 mutations. Its rebuilt complete host
suite passed 51 unit +10 integration tests with owned inputs; the capture test and
strict workspace Clippy also passed after a hash-helper allocation lint correction.
Reports are retained in `terranigma/local/map-inspector-renewal/`; the full suite
log is `terranigma/local/map-research/pandora-parent-full-host.txt`. The dedicated
renewal issue is accepted; earlier SRAM-red handoff statements are historical.
Other wrappers and portable Pandora gameplay are not accepted by this result.

## Current preview producer: same epoch, no output renewal

The final source relay is parent `d293c62`, merged in the producer worktree at
`8060d04`. The retained captures were built at `204f3c3` after source authentication.
No production files changed here. The production `serve-room` preview uses its
qualified profile while the legacy constructor remains false; neither is called
by this SRAM capture recipe. Independent source review traced the unchanged
Session/input/observation/conversion/process-exit path described above.

`observer.json`, `migration.json`, `epochs/threaded-video-v0/`, historical producer
envelopes and **every output pin remain unchanged**. `current-producer.json` is an
explicit same-epoch descriptor, not a producer registry or fallback allowlist.
It retains the original 13-key `source_hashes` inventory with only main changed.
Twelve separate `additional_source_hashes` bind the reviewed preview modules/hooks,
including the final camera, preview/server wiring and embedded preview HTML. These
are provenance, not a retroactive historical inventory or a complete transitive
build-input inventory. Separately included test-only module files are outside
this production hook set; whole-file hashes still include inline test code in
pinned files.

### Exact source and evidence identities

| Artifact | SHA-256 |
| --- | --- |
| Original observer | `7fabf5688943eca89c43ad5aee02d348187fc3491296b1c401535553fbe3a718` |
| Original migration report | `db249179718cb6bcf9c1755093d441d0094dd39fc836d079220defd3b289ab3c` |
| Authenticated old main | `7736b543c442e6e4c2789fb13f6f177d1335e78f11810d5023a49c313b27a4d3` |
| Current main | `2f77608e3d004f4cc480d1b650073b5e16d680e230437570aa0ed1c1b4504e2d` |
| `current-producer.json` | `85de8d72d6f0a433345645f5dd86f5c80f8e1ffd18549fb357a59b2d97590714` |
| `producer-bridge.json` | `46a2b7fda7525c8c7da664b83ec182160b39f4a67d8d0e958c573cea606795c0` |

The only main delta is the exact **57-byte insertion** below, once, immediately
after `mod opening_qualification;\n`:

```text
pub mod pandora_navigation;
pub mod pandora_progression;
```

The old Git blob at `8034889` and both old/fixed worktree main files authenticate
identically. Python compares every current byte to old plus that insertion. Rust
undoes only that exact anchored insertion and requires the original whole-file
SHA-256. Neither strips arbitrary lines nor normalizes whitespace. The earlier
navigation-only main, non-registration edits (even with a resealed new main pin),
identity substitution, missing/extra sources and changed hook bytes are rejected.
The Rust gate explicitly selects `current-producer.json`; it does not reinterpret
the historical descriptor. The runtime capture-test body remains byte-identical
to `204f3c3`, including all observation/output assertions.

### Retained current runs and independent review

Producer worktree: `/Users/lainsoykaf/repos/ilar-task-preview-producer`.
Private roots: `local/map-inspector-preview/{current-a,current-b}`. Each root was
new, got its own clean `target/`, complete build log and retained executable, then
ran exactly one new singleton capture process. Thus **both** A and B are fresh
isolated builds, stronger than the historical fixed A/B pair.

| Run | Capture PID | Retained executable SHA-256 |
| --- | --- | --- |
| Current A | 31891 | `4c74152dcf301fe24641c6545fe9d03c6d67158e35b219064b3d20450586b835` |
| Current B | 32710 | `20ec1f7c21c016c32ac24aa7eededd0903472dc92e9039695893516b48d106ac` |

Schema-2 producer envelopes retain descriptor/source identities, exact authenticated
input paths/hashes, commands/tool versions, build environment/log hashes, binary
hashes, stdout/stderr hashes, PID, start/end times, exit code and distinct run IDs.
Sources and descriptor were checked before build and after capture. These fresh
isolated builds produced different executable digests; this is **not** a
reproducible-binary claim. No raw inputs, captures, binaries or logs are committed.

The bridge authenticates the original accepted producer envelopes unchanged,
validates each complete manifest and derived HTML, and byte-compares **all ten
files (855,207 bytes per root)** across accepted fixed A/B and current A/B. All
files match the frozen fixed inventory; no output pins were renewed.

- Complete `capture.json` SHA-256:
  `a7f23508b13727e8ab26df65e153f7788fc37182111446a68e37c61dc83c664a`.
- Complete canonical nonpixel manifest SHA-256 remains
  `7998be259cce4218983f03bb81e3bf189577958dc9f190ba38880036d680cb22`.
- All 394,240 WRAM/VRAM/CGRAM bytes per root also match the old producer.

An execution-capable independent reviewer rehashed the old Git blob, source/hook
pins, retained binaries/logs/envelopes and original capture directories, checked
fresh target provenance, independently decoded all ten BMPs across the five
roots, and reproduced both audit modes exactly. It then ran both current binaries
in **two additional singleton `verify` processes**, PIDs 35717/35762: exit 0,
empty stderr, complete retained manifests reproduced. Review records are in
`../ilar-task-preview-producer-review/local/review-output/`; the hash-only
`independent-summary.json` digest is
`cf8ff09bc853e0dc8a3653f7842772df81e5abb54bff2451af556aa97d571fa2`.
Sourcegate and source-descriptor changes also received independent static
correctness/architecture reviews. These checks authenticate retained evidence;
process records corroborate freshness but cannot cryptographically prove an honest
recorder. Parent acceptance/reproduction remains required before issue archive.

### Reproduce retained audits (from parent or producer checkout)

The bridge's explicit current repository argument is the **recorded producer
worktree**, not whichever checkout happens to pass. The caller's own sources are
separately checked by the Rust sourcegate. Historical audit also requires explicit
old **and fixed** source repositories and uses that fixed viewer for derived HTML.

```sh
T=tools/map-inspector-qualification
P=../ilar-task-preview-producer
ROM="$P/local/Tenchi Souzou (Japan).sfc"
SRAM="$P/local/saves/Terranigma.srm"
OLD_REPO=../ilar-task-capture-old
FIXED_REPO=../ilar-task-capture-renewal
ACCEPTED="$FIXED_REPO/local/map-inspector-renewal"
CURRENT="$P/local/map-inspector-preview"
D="$T/current-producer.json"
AUDIT=$(mktemp -d "$PWD/local/map-inspector-recheck-XXXXXX")
for OPT in '' -O; do
  python3 $OPT -B "$T/check.py" "$ROM" "$SRAM" "$ACCEPTED/old" \
    "$ACCEPTED/fixed-a" "$ACCEPTED/fixed-b" "$OLD_REPO" "$FIXED_REPO" \
    > "$AUDIT/historical$OPT.json"
  cmp "$AUDIT/historical$OPT.json" "$T/migration.json"
  python3 $OPT -B "$T/bridge.py" "$ROM" "$SRAM" "$ACCEPTED/old" \
    "$ACCEPTED/fixed-a" "$ACCEPTED/fixed-b" "$CURRENT/current-a" "$CURRENT/current-b" \
    "$OLD_REPO" "$FIXED_REPO" "$P" "$D" > "$AUDIT/bridge$OPT.json"
  cmp "$AUDIT/bridge$OPT.json" "$T/producer-bridge.json"
  python3 $OPT -B "$T/test_check.py"
  python3 $OPT -B "$T/test_bridge.py"
  python3 $OPT -B "$T/test_capture.py"
done
python3 -B "$T/test_mutations.py"
cargo test --locked -p map-inspector --test local_capture
```

For new independent builds/processes, do **not** reuse the retained roots:

```sh
FRESH=$(mktemp -d "$PWD/local/map-inspector-replay-XXXXXX")
for RUN in current-a current-b; do
  python3 -B "$T/capture.py" . "$FRESH/$RUN" "$ROM" "$SRAM" \
    --current-descriptor "$D" --fixed-source-repo "$FIXED_REPO"
done
python3 -B "$T/bridge.py" "$ROM" "$SRAM" "$ACCEPTED/old" \
  "$ACCEPTED/fixed-a" "$ACCEPTED/fixed-b" "$FRESH/current-a" "$FRESH/current-b" \
  "$OLD_REPO" "$FIXED_REPO" . "$D" > "$FRESH/bridge.json"
```

New process/build envelopes differ by definition; compare identity, inventory,
complete-manifest and nonpixel fields with `producer-bridge.json`, not the entire
fresh report. Any output difference requires investigation, never a pin rewrite.

### Final local validation

- TDD: final old-source gate reproduced the expected red; camera inventory,
  identity and non-registration-main negative controls went red→green.
- Python: **14 historical + 13 bridge + 4 recorder tests**, normal and `-O`;
  all 20 original mutation checks detected. Recorder mocks verify wiring only;
  the retained real executions above provide execution evidence.
- Rust: **six local_capture tests pass**, owned inputs present, none skipped;
  focused rustfmt and Clippy `-D warnings` pass.
- Full map-inspector package: **74 unit tests pass, 14 pre-existing explicit
  ignores; all 13 integration tests pass**. Other wrappers' pins remain untouched.
- Private validation logs: producer `local/map-inspector-preview/`, including
  `source-gate-red.txt`, `main-negative-red.txt`, `identity-negative-red.txt`,
  `local-capture-green.txt`, `local-capture-clippy.txt`, `map-inspector-tests.txt`,
  both `historical*.json` and `bridge*.json` audits.

No server/browser ownership transfers or shared-index edits are part of this work.

## Library/Wasm producer: explicit successor, same native output

The separated producer is source commit `8ae412c36e92b870c34bd4676ef6d8450d5a289a`.
It is qualified by the sibling `library-producer.json` descriptor and
`library-producer-bridge.json` report. The predecessor descriptor and report
remain byte-frozen at `85de8d72…0714` and `46a2b7fd…95c0`; the new bridge calls
the old bridge with explicit historical, fixed and predecessor source trees
before authenticating this successor. It is not a registry or fallback.

The successor records exactly four real replacements (`Cargo.lock`,
map-inspector `Cargo.toml`, `room_preview.rs`, and `room-slice.html`) with both
old and current identities. `main.rs` was replaced later, by the separate repin
stage below; this stage's four are unchanged. Separate exact inventories bind the three library
boundary files, public-preview qualification test, root workspace manifest,
every file in `crates/pandora-web/`, and all nine files in
`tools/pandora-preview/`. Unchanged predecessor sources remain derived from and
checked against the frozen predecessor descriptor. The exact 57-byte `main.rs`
insertion/reconstruction proof is unchanged at this stage; the repin stage below
adds a second exact delta on top of it.

Native-host consumption was considered and deliberately left as the current
shared-source implementation. The native exporter already reaches the pure
renderer through the library boundary; constructing `PandoraPreview` directly
would add architecture churn without fixing a correctness problem.

Descriptor/report SHA-256 (both still current; the repin stage freezes rather
than rewrites them):

- `library-producer.json`: `00298d9350a143abeb83bb95ae093feba81d6c9850ab4722bf015834d88f6143`
- `library-producer-bridge.json`: `18cfd3ec329e70159d3ad7613dd73f826d03b55c573274661337f9277060b75d`

Private evidence is retained in this source worktree under
`local/map-inspector-library-producer/{library-a,library-b}`. Both roots own a
clean Cargo target, build log, copied binary, ten-file capture, stdout/stderr,
and schema-3 process/toolchain envelope. They are two isolated builds and two
singleton capture processes (PIDs 87534 and 88336); binary SHA-256 values are
`b1ef58f1…8ced` and `8acee140…7aca`. All ten files and complete manifests match
the accepted fixed roots and each other. The complete manifest remains
`a7f23508…664a`; canonical nonpixels remain `7998be25…b22`. Post-review
singleton `verify` processes (PIDs 94512 and 94571) parsed equal to each retained
complete manifest, exited zero and emitted empty stderr; metadata is retained in
`local/map-inspector-library-producer/retained-binary-review.json`. An independent
execution-capable GPT-5.6-Sol review then reproduced all three bridge stages in
normal and optimized Python, byte-matched every report, independently compared
all ten files across the five successor roots, reran the focused gates, and ran
both retained binaries in fresh singleton `verify` processes (PIDs 98813/98872).
Both parsed manifests matched exactly with empty stderr. Its private command and
hash records are under `/tmp/ilar-final-review-98347`.

Reproduce the old → predecessor → library chain with explicit recorded sources:

```sh
T=tools/map-inspector-qualification
ROOT=local/map-inspector-library-producer
ROM='/Users/lainsoykaf/repos/terranigma/local/Tenchi Souzou (Japan).sfc'
SRAM='/Users/lainsoykaf/repos/terranigma/local/saves/Terranigma.srm'
OLD_REPO=/Users/lainsoykaf/repos/ilar-task-capture-old
FIXED_REPO=/Users/lainsoykaf/repos/ilar-task-capture-renewal
PRIOR_REPO=/Users/lainsoykaf/repos/ilar-task-preview-producer
ACCEPTED="$FIXED_REPO/local/map-inspector-renewal"
PRIOR="$PRIOR_REPO/local/map-inspector-preview"
D="$T/library-producer.json"
ARGS=("$ROM" "$SRAM" "$ACCEPTED/old" "$ACCEPTED/fixed-a" "$ACCEPTED/fixed-b" \
  "$PRIOR/current-a" "$PRIOR/current-b" "$ROOT/library-a" "$ROOT/library-b" \
  "$OLD_REPO" "$FIXED_REPO" "$PRIOR_REPO" . "$D")
python3 -B "$T/library_bridge.py" "${ARGS[@]}" > "$ROOT/rechecked.json"
cmp "$ROOT/rechecked.json" "$T/library-producer-bridge.json"
python3 -O -B "$T/library_bridge.py" "${ARGS[@]}" > "$ROOT/rechecked-O.json"
cmp "$ROOT/rechecked-O.json" "$T/library-producer-bridge.json"
python3 -B "$T/test_library_bridge.py"
python3 -O -B "$T/test_library_bridge.py"
python3 -B "$T/test_capture.py"
python3 -O -B "$T/test_capture.py"
python3 -B "$T/test_library_mutations.py"
cargo test --locked -p map-inspector --test local_capture
```

For two new isolated builds/processes, use new refused-if-existing roots:

```sh
FRESH=$(mktemp -d "$PWD/local/map-inspector-library-replay-XXXXXX")
for RUN in library-a library-b; do
  python3 -B "$T/capture.py" . "$FRESH/$RUN" "$ROM" "$SRAM" \
    --library-descriptor "$D" --fixed-source-repo "$FIXED_REPO" \
    --predecessor-source-repo "$PRIOR_REPO"
done
python3 -B "$T/library_bridge.py" "$ROM" "$SRAM" "$ACCEPTED/old" \
  "$ACCEPTED/fixed-a" "$ACCEPTED/fixed-b" "$PRIOR/current-a" "$PRIOR/current-b" \
  "$FRESH/library-a" "$FRESH/library-b" "$OLD_REPO" "$FIXED_REPO" \
  "$PRIOR_REPO" . "$D" > "$FRESH/bridge.json"
```

Fresh envelopes differ by construction. Compare identities, bounded inventories,
complete manifests, nonpixels and output bytes; never rewrite an output pin.

## Repin producer: explicit successor, rustfmt-only source delta

`crates/map-inspector/src/main.rs` was the last file failing
`cargo fmt --all -- --check`. Its only rustfmt diff is a declaration reorder,
`mod house_progression;` / `mod house_profiles;` → `mod house_profiles;` /
`mod house_progression;`. Module declaration order is inert, but a pinned
producer source does not get changed on that argument alone.

This is a **third stage**, built the same way as the library stage: the
library descriptor and report stay byte-frozen at `00298d93…6143` and
`18cfd3ec…0b75d`, and the successor records exactly one replacement over them.
`observer.json`, `migration.json`, `current-producer.json`, `producer-bridge.json`
and `epochs/` are untouched. No output pin was renewed.

| Identity | Value |
| --- | --- |
| Library main (frozen predecessor) | `2f77608e3d004f4cc480d1b650073b5e16d680e230437570aa0ed1c1b4504e2d` |
| Reformatted main | `5eeb11bd799b75b4c41561e876a03d4a50056a72a1fe71292eb318cf017173ed` |
| `repin-producer.json` | `e58a0f232a8ce9cc86186e515a9156ca32c4fd992c4a7f38cd859c117a247c33` |
| `repin-producer-bridge.json` | `5fed82d65b682ca001dfa55b3ce6f9c60f9911d7b9cd0313e461e688b167173f` |

### Two exact deltas, not a relaxed gate

The old main is reconstructed by undoing **two** exact anchored deltas: the
reorder, then the unchanged 57-byte registration insertion. It still
reconstructs to `7736b543…a4d3`. Neither side strips lines nor normalizes
whitespace; each delta must occur exactly once in the current file, and each
anchor must occur exactly once in the authenticated old blob. That second
requirement is what makes the reconstruction injective — the accepted set is a
**singleton**, so no other `main.rs` passes.

`bridge.verify_main_delta` keeps the registration-only model, which is still
correct for the predecessor and library stages;
`repin_bridge.verify_repin_main_delta` wraps it for this one.

### Output neutrality

Two fresh isolated builds of the reformatted producer,
`local/map-inspector-repin/{repin-a,repin-b}`, each with its own clean Cargo
target, build log, copied binary and one singleton capture process. Both
reproduce the frozen library producer's output exactly:

- all ten capture files, **855,207 bytes** per root, byte-identical to each
  other and to the frozen inventory (which carries per-file sizes and SHA-256,
  so matching it is byte equality with the accepted output);
- complete manifest `a7f23508…664a`;
- canonical nonpixel manifest `7998be25…0cb22`.

### Why this stage has its own comparison

`compare_to_frozen` replaces `bridge.compare_unchanged` rather than calling it.
The five accepted capture roots the library audit consumes were retained under
ignored `local/` in worktrees that no longer exist, and `migration.json` pins
their `binary_sha256` and build-log identities from builds the README itself
records as not bit-reproducible. That chain therefore cannot be re-derived by
anyone, now or later — it is not a property of this change. The frozen report's
own inventory is the strongest still-available expectation, and this stage is
checked against it.

The source worktrees were restored from Git (`7c5c90b` fixed, `ad0c049`
predecessor) and the descriptor chain reproduces against them unmocked.

### Reproduce

```sh
T=tools/map-inspector-qualification
R=local/map-inspector-repin
ROM='local/Tenchi Souzou (Japan).sfc'
SRAM=local/saves/Terranigma.srm
FIXED_REPO=../ilar-task-capture-renewal   # git worktree add --detach "$FIXED_REPO" 7c5c90b
D="$T/repin-producer.json"
for OPT in '' -O; do
  python3 $OPT -B "$T/repin_bridge.py" "$ROM" "$SRAM" "$R/repin-a" "$R/repin-b" \
    "$FIXED_REPO" . "$D" > "$R/recheck$OPT.json"
  cmp "$R/recheck$OPT.json" "$T/repin-producer-bridge.json"
  python3 $OPT -B "$T/test_repin_bridge.py"
done
python3 -B "$T/test_repin_mutations.py"
cargo test --locked -p map-inspector --test local_capture
```

For two new isolated builds, use new refused-if-existing roots:

```sh
FRESH=$(mktemp -d "$PWD/local/map-inspector-repin-replay-XXXXXX")
for RUN in repin-a repin-b; do
  python3 -B "$T/capture.py" . "$FRESH/$RUN" "$ROM" "$SRAM" \
    --repin-descriptor "$D" --fixed-source-repo "$FIXED_REPO"
done
```

### Gates

The Rust sourcegate resolves expected sources latest-stage-first: repin over
library over frozen predecessor. `check_repin_report` additionally requires
every retained producer envelope in the report to bind to the repin descriptor
— same `descriptor_sha256`, `kind` and `replaced_source_hashes`, and a
`source_hashes` main identity equal to the declared replacement, with two
distinct run IDs and targets. A report whose header and body disagree is
rejected; an earlier draft of this work had exactly that defect and no gate
caught it.

`test_repin_mutations.py` disables each of the nine repin gates in turn and
requires a red test, in normal and `-O` Python: descriptor fields, predecessor
identity, replacement inventory, real-delta, unchanged-source, reorder anchor,
old-main injectivity, producer fields and bounded producer sources. 18/18
detected; records are retained under ignored `local/map-inspector-repin/`.

`test_capture.py` covers the recorder's repin branch: schema-4 envelope shape,
inherited (not restated) group inventories, the required fixed-source argument
and descriptor-mode exclusivity.

## Build-input pins: two files pinned by projection, not whole file

The workspace root `Cargo.toml` and `Cargo.lock` are producer sources because
they decide what the producer compiles against. But both also record things the
producer cannot reach: adding an unrelated workspace member rewrites `members`
and appends a `[[package]]` block without changing a single input to the
capture.

Rather than spend a repin on a provable no-op, those two are pinned by
**projection**, anchored on a frozen copy of the exact file the descriptor
named, under `pinned/`:

```text
descriptor entry == sha256(pinned fixture)   descriptor <-> fixture
projection(fixture) == projection(live)      fixture    <-> live
```

There is no free-floating constant to edit. An earlier draft compared two
constants to two constants and so dropped the descriptor's binding to the tree
entirely; this chain restores it.

| File | Pinned as | Elided |
| --- | --- | --- |
| `Cargo.lock` | transitive dependency closure of `map-inspector` | packages outside that closure |
| `Cargo.toml` | the file with `[workspace] members` elided | only that one key |

### Both gates, one definition of each projection

`projection.py` and the Rust sourcegate implement the same two projections, and
**both compare the live file to the same fixture**, so a divergence between the
two implementations makes one of them red. Routing the Python bridge through
`projection.effective_sha` keeps every existing equality check in
`library_bridge.py` intact: it returns the fixture's whole-file hash only when
the live file projects onto it, and the live file's own hash otherwise.

Narrowing the Rust gate alone would have been a regression, not a narrowing —
the Python bridge is what actually authenticates a capture, and it whole-file
hashes both files.

### Why each is no weaker

`Cargo.lock`: the closure carries every name, version, source, checksum and
edge inside it, keyed on `(name, version, source)`. The source belongs in the
key — cargo emits a dependency reference's source exactly when name and version
are ambiguous, which a patched git fork that kept its version number produces,
and merging those would leave the fork's subtree unwalked. A reference that
omits a field matches every package sharing what it does give, so ambiguity
widens the closure rather than narrowing it. 31 of the workspace's 42 packages
are in the closure, byte-identical to `cargo tree -p map-inspector`.

`Cargo.toml`: the producer's build command is pinned as exactly
`cargo build --locked -p map-inspector`. That selects one package, so feature
resolution covers only that package's graph and the *set* of other workspace
members cannot reach it. Everything else stays pinned byte-for-byte —
`resolver`, `default-members`, `exclude`, `[workspace.package]`,
`[workspace.lints]`, `[workspace.dependencies]`, `[patch]` and any profile
table.

The elision is bounded by the `[workspace]` table and by real bracket depth
counted outside quoted strings. A line-prefix match with a trailing-bracket
heuristic is not enough: `members = [...] #` does not end in a bracket, so the
elision would run on to the next bracketed line and swallow whatever sat
between — an injected `[workspace.dependencies]` and `[patch.crates-io]` were
demonstrated hiding there, projecting to the expected hash exactly.

**Stated assumption, not a guarantee:** the `Cargo.toml` argument holds for the
default resolver behaviour. `resolver.feature-unification = "workspace"` would
unify features across all members regardless of `-p`. No cargo configuration
sets it and none is pinned.

### Known losses

The projections do not see the `Cargo.lock` `version = N` header, `[metadata]`
or `[patch.unused]` tables, or lockfile whitespace and comments. All are
build-neutral; `[patch.unused]` appearing is a signal that a `[patch]` stopped
applying, and that signal is now invisible.

### Evidence

Both projections were established from the exact files the library stage
pinned, *before* any change to them — `Cargo.lock` at `23e335de...d070`,
`Cargo.toml` at `6d5ae00e...1937`, which are the fixtures' own hashes — and
were then shown unchanged after the workspace gained a member. The two
implementations agree: both produce lockfile projection `7d9b629d...9e5f`.

`test_projection.py` (10 tests) and the Rust sourcegate (9 projection tests) fix
the strictness in both directions, including the two holes an adversarial review
found in the first draft: a git fork sharing a name and version, and the
trailing-comment elision runaway.
