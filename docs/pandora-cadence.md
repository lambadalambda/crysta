# Canonical Pandora cadence qualification

Issue: [continuous browser journey](../meta/issues/verify-pandora-browser-journey.md).
Consumer contract: [Pandora browser cadence API](pandora-browser.md).
This is **host test-only proof production**, not browser acceptance or native-frame
qualification. No server, browser session, gameplay rule or controller is changed.

## Producer and boundaries

`crates/map-inspector/src/room_preview/cadence_tests.rs` runs the exact committed
`tools/pandora-runtime-qualification/route.json` (11590 expanded public commands,
SHA256 `b969d6877f595ff811a0de8a912de307aaf5a3eb2b42e1720f2f5da6db53b830`).
It constructs `Preview::new_profile(rom, true)` and calls `new_game()` **twice**,
compiling two independent hosts. Each host advances only through `Preview::step`.
Each also has its own continuous, fresh `GameState` public-command mirror: every
raw apply must return `Ok`, with exact host/raw snapshot and output agreement.
In particular, the host's harmless-wrong-action handling cannot hide a raw
`Interaction` failure. A private-ROM negative test exercises that distinction.

The offline and projected arrays each contain their own fresh tick0 GET-shaped
`state()` and every post-action state. Nothing reads the parent's existing export
as input. No state clone, capture, restored checkpoint, flag initializer, position
initializer or normalized snapshot is used to initialize or advance either run.

A boundary's `continuation` is SHA256 of a **read-only hashing copy**. The input
must be exactly320 bytes with header `b'RSLC' + [5,13,0,1]`. Only bytes`[72,80)`
(global tick) are zeroed in the copy; no copy is ever restored. All other bytes,
including source/compiler identity, flags, animation, input history, motion age,
coordinates, pot state, reservations and sheet mutations, remain in the hash.
Exhaustive synthetic controls flip every bit outside the erased interval; header
mutations and wrong lengths reject instead of silently changing normalization.

Blocking dialogue means `state.dialogue != null && state.dialogue_ready !== false`.
The producer requires the **actual GET readiness boolean**; missing readiness is
an error, not permission to invent a replacement field. Only commands0..4 with
blocking dialogue, identical continuation, and identical observations after
removing **only** `tick` and `snapshot_sha256` may disappear. Unknown/new GET fields
are compared too. Every manual action remains. Ordinary non-dialogue neutral
no-ops remain; visible-but-unready dialogue permits neutral Resume, not ack or
movement. The projected run independently replays every retained command and
checks every retained boundary, not merely semantic landmarks or the final state.

The fixed-route qualification computes omissions and retained visible/unready
updates, then asserts181 omissions and18 mandatory updates:17 `CEntry` arrivals
and1 `BoxEntry` completion. These are assertions against the full execution, not
an omission schedule or initializer. Divergence, raw rejection, unemittable input,
partial arrays, missing tick0, wrong provenance or changed unprojected final
canonical snapshot causes failure before any accepted proof is published.

## Run from committed source

The parent owns the test-module registration in `room_preview.rs`:

```rust
#[cfg(test)]
mod cadence_tests;
```

Once that hook and the reviewed producer are committed, use a **clean checkout**:

```sh
cargo test -p map-inspector --bin map-inspector room_preview::cadence_tests
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/pandora-cadence-qualification -p 'test_*.py' -v
python3 tools/pandora-cadence-qualification/run.py \
  'local/Tenchi Souzou (Japan).sfc' debug reviewed-run
python3 tools/pandora-cadence-qualification/run.py \
  'local/Tenchi Souzou (Japan).sfc' release reviewed-run
```

The ROM-backed tests are ignored in ordinary `cargo test`, but the wrapper runs
them explicitly and **never skips missing evidence**. It verifies test registration
before invoking exact test names, so a zero-test success cannot generate a proof.
It refuses dirty tracked or untracked source and existing output directories.

All generated evidence lives privately under
`local/pandora-cadence-qualification/<run-name>/<debug|release>/`:

- `source.tar`: exact `git archive HEAD`, private despite containing source only;
- `commands.log`, `build.jsonl`, compiler version output and individual test logs;
- wrapper `provenance.json`, the raw test executable SHA256, source archive SHA256,
  Git revision/tree, Rust compiler version SHA256 and `Cargo.lock` SHA256;
- on full success only, `proof.json` and `proof.sha256`.

The proof is schema1 with `offline`, `projected` and required provenance fields
`route_sha256`, `rom_sha256`, `start: NewGame`, `policy: SemanticPreview`,
`tick_erasure: global-tick-only`. Additional provenance is `generator_revision`,
`source_tree`, `source_archive_sha256`, `compiler_content_sha256` (canonical source
compiler identity bytes40..72), `rustc_vv_sha256`, `cargo_lock_sha256`,
`executable_sha256` and `build_profile`. `qualification` reports computed original/
projected action counts, zero-based omitted indices and retained arrival counts.

The wrapper captures build/test commands and identities **outside Rust**, checks
clean source and executable identity again after successful generation, and only
then publishes. The **final atomic rename to `proof.json` is the commit point**:
checksum writing, log close, final checks and console reporting all precede it.
A process killed after that successful rename does not revoke completed evidence.
Failed attempts may retain diagnostics, a prepared checksum or `proof.pending.json`;
without `proof.json` these are **not accepted proofs**. Private evidence and all ROM-derived data must
never be committed. A JSON digest authenticates bytes, not the truth of execution.

## Review gate / current status

Normalizer and validator followed red→green, including every-bit identity checks,
partial/shifted/mismatched replay rejection and the raw `Interaction` negative
control. The clean-source wrapper also has publication failure tests.

**No final proof or browser acceptance is claimed here yet.** The parent must
independently review this producer, rerun both debug and release from committed
source, compare complete arrays and qualified counts, and pin the private proof
file before the browser owner consumes it. Build/source metadata can differ
between independent runs; all replay boundary observations and continuations must
agree. Parent-owned UI/GET readiness is observational only; no core ticks, graph
rules or snapshot schema changes are authorized by this qualification.
