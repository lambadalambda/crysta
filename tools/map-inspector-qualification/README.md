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
T=tools/map-inspector-qualification
python3 -B "$T/check.py" "$ROM" "$SRAM" "$E/old" "$E/fixed-a" "$E/fixed-b" "$OLD_REPO" > "$E/rechecked.json"
cmp "$E/rechecked.json" "$T/migration.json"
python3 -O -B "$T/check.py" "$ROM" "$SRAM" "$E/old" "$E/fixed-a" "$E/fixed-b" "$OLD_REPO" > "$E/rechecked-O.json"
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
python3 -B "$T/capture.py" . "$FRESH/fixed-a" "$ROM" "$SRAM"
python3 -B "$T/capture.py" . "$FRESH/fixed-b" "$ROM" "$SRAM"
python3 -B "$T/check.py" "$ROM" "$SRAM" "$FRESH/old" "$FRESH/fixed-a" "$FRESH/fixed-b" "$OLD_REPO" > "$FRESH/audit.json"
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
