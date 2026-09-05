#!/bin/sh
# Run from repository root. Source ROM is opened read-only; no SRAM file is used.
set -eu
export PYTHONDONTWRITEBYTECODE=1
python3 tools/new-game-qualification/test_verify.py
mkdir -p local/new-game-qualification/probe/src
cp tools/new-game-qualification/probe.rs local/new-game-qualification/probe/src/main.rs
cat >local/new-game-qualification/probe/Cargo.toml <<'TOML'
[package]
name = "new-game-qualification"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
cargo build --release --manifest-path local/new-game-qualification/probe/Cargo.toml
p=local/new-game-qualification/probe/target/release/new-game-qualification
# Preserve previous runs; the probe refuses existing output directories.
out=$(mktemp -d local/new-game-qualification/replay-XXXXXX)
rom=${1:-local/Tenchi Souzou (Japan).sfc}
for run in a b; do
    "$p" "$rom" "$out/$run"
    python3 tools/new-game-qualification/verify.py "$out/$run"
done
cmp "$out/a/checkpoints.json" "$out/b/checkpoints.json"
cmp "$out/a/frames.csv" "$out/b/frames.csv"
# Controlled input omissions must fail the same checker. They are still fresh
# boots with normal emulated execution, not patches or snapshot experiments.
for mode in no-confirm no-movement; do
    "$p" "$rom" "$out/$mode" "$mode"
    if python3 tools/new-game-qualification/verify.py "$out/$mode" >"$out/$mode/rejection.log" 2>&1; then
        echo "ERROR: $mode unexpectedly qualified" >&2
        exit 1
    fi
done
printf 'Two matching exit-0 fresh boots; both negative controls rejected. Captures: %s\n' "$out"
