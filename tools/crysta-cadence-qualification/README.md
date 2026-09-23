# Crysta ordinary walker cadence qualification

Bounded source-to-native witnesses for map `$000D`, resident slot `$1040`, and
a class-2 town walker, map `$000A` slot `$1300`.
No production runtime changes and no embedded ROM, decoded assets or captures.
`REPORT.md` records the source chain and interpretation. It covers classes 0
and 2 on the common movement base only; it does not qualify other classes,
private movement resources or arbitrary VM waits.

## Build

From the repository root, with the repository Rust toolchain, a C++ compiler
(for the existing `oracle` core), and Python 3:

```sh
sh tools/crysta-cadence-qualification/build.sh
python3 -B -m unittest discover -s tools/crysta-cadence-qualification -p 'test_*.py'
```

The small standalone Cargo package/lockfile and binaries are generated under
ignored `local/crysta-cadence-qualification/`. `CARGO_TARGET_DIR` may override
the target directory. There are no production workspace/manifest changes.
`decode.rs` links **`assets::compression`**, authenticates via `rom::Rom::load`,
requires Japan, checks source bindings and packet bounds, and sends decoded
bytes to the verifier over stdout. It does not save extracted resources.
`probe.rs` references **`tools/new-game-qualification/bootstrap.rs` directly**;
neither bootstrap nor codec is copied. `serde_json` is the same MIT/Apache-2.0
dependency already used by the repository.

## Verify existing private evidence

Set these paths to your owned ROM and retained probe outputs (ROM normalization,
including supported copier headers, is handled by `rom::Rom::load`):

```sh
ROM='/path/to/Tenchi Souzou (Japan).sfc'
WRAM='local/crysta-cadence/native/settledD.wram'
SAMPLES='local/crysta-cadence/native.jsonl'
BIN="${CARGO_TARGET_DIR:-$PWD/local/crysta-cadence-qualification/target}/release"
python3 -B tools/crysta-cadence-qualification/verify.py \
  --rom "$ROM" --decoder "$BIN/crysta-cadence-decode" \
  --wram "$WRAM" --samples "$SAMPLES"
```

The verifier reads evidence without rewriting it and prints `PASS` only after:

- authenticating the Japanese ROM and checking its bounded source bindings;
- comparing **all 6668 decoded common-resource bytes** with `$7F:6000..7A0C`
  in a full 128-KiB native WRAM image, including its cached source pointer;
- checking all 401 ordered, map-D/class-0 samples at frames **8175..8575**;
- matching every X/Y delta in three **32-frame** down/up/right observations
  against the decoded common streams, including display countdown, list index,
  repeat count and cleared movement accumulators;
- checking **16 frames** of the single-duration-0 idle/refusal list.

These exact frame numbers intentionally bind the accepted itinerary, not an
arbitrary user recording. Mismatches in the asserted source, fields or frame
windows fail nonzero, as do truncated inputs. Do not use `python -O`: the
verifier explicitly refuses it so
qualification assertions cannot be skipped. Use the decoder built from these
sources, not an arbitrary executable returning prepared JSON.

## Repeat a fresh native probe

Run from the repository root. Outputs must be new directories under `local/`;
the probe refuses to overwrite an existing output directory. Raw evidence and
any redirected decoder output must remain under ignored `local/`.

```sh
out=$(mktemp -d local/crysta-cadence-qualification/replay-XXXXXX)
for run in a b; do
  "$BIN/crysta-cadence-probe" "$ROM" "$out/$run" \
    tools/crysta-cadence-qualification/route.jsonl >"$out/$run.jsonl"
  python3 -B tools/crysta-cadence-qualification/verify.py \
    --rom "$ROM" --decoder "$BIN/crysta-cadence-decode" \
    --wram "$out/$run/settledD.wram" --samples "$out/$run.jsonl"
done
cmp "$out/a.jsonl" "$out/b.jsonl"
cmp "$out/a/settledD.wram" "$out/b/settledD.wram"
```

Each probe starts one fresh empty-SRAM Session, holds only recorded real
buttons through the shared 6800-frame bootstrap and committed itinerary,
then explicitly releases all buttons and samples 400 further neutral native
frames (401 snapshots including the initial settledD state). Accessors are
passive: no savestate, memory write, warp or debugger intervention.

## Class-2 town walker

`town-route.jsonl` is the arrival route's input-only itinerary up to
`settleTown-open`, where the elder has opened the house door and the player
stands in the town. The probe's optional `SLOT-HEX SAMPLES` arguments select
another slot; without them it samples map D's `$1040` for 400 frames as before.

```sh
out=$(mktemp -d local/crysta-cadence-qualification/town-XXXXXX)
for run in a b; do
  "$BIN/crysta-cadence-probe" "$ROM" "$out/$run" \
    tools/crysta-cadence-qualification/town-route.jsonl 1300 200 >"$out/$run.jsonl"
  python3 -B tools/crysta-cadence-qualification/verify.py \
    --rom "$ROM" --decoder "$BIN/crysta-cadence-decode" --town \
    --wram "$out/$run/settleTown-open.wram" --samples "$out/$run.jsonl"
done
cmp "$out/a.jsonl" "$out/b.jsonl"
```

`--town` samples slot `$1300` (record `$83:8A2D`) over frames
**11394..11594** and checks every one: map `$0A` and class 2 throughout; all
6668 common-resource bytes against the town WRAM; four complete steps
(right, down, up, right) and a final partial step against the decoded
class-0 row streams; two 16-frame idles and an idle tail against packet
`$D4:7890`'s two-record lists; and the seven one-frame loop gaps with list
index and repetition count zero. See [REPORT.md](REPORT.md).

## Retention checks

- TDD: synthetic tests failed before `verify.py` existed, then passed after
  retention/refactoring. They cover valid bounded evidence, signed/looping/null
  streams, and rejection of altered movement/timing, incomplete samples/WRAM,
  and a WRAM difference outside the selected velocity streams.
- Production-API verification passed against the existing private native
  capture. A fresh probe also passed; its complete stdout JSONL and settledD
  WRAM were byte-identical to that capture.
- Only original source, input itinerary and reverse-engineering annotation are
  retained here. Local disassemblies, packet scanner, copied codec, raw captures,
  ROM and derived CSV/resource binaries are deliberately excluded.
