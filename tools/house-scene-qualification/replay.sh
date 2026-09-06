#!/bin/sh
# Two independent empty-SRAM boots for each finite input-only itinerary.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
python3 -B tools/house-scene-qualification/test_census.py
python3 -O -B tools/house-scene-qualification/test_census.py
probe=local/house-scene-qualification/probe
mkdir -p "$probe/src"
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' tools/house-scene-qualification/probe.rs >"$probe/src/main.rs"
cp tools/new-game-qualification/bootstrap.rs "$probe/src/"
cat >"$probe/Cargo.toml" <<'TOML'
[package]
name = "house-scene-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
cargo build --release --manifest-path "$probe/Cargo.toml"
binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-scene-probe
out=$(mktemp -d local/house-scene-qualification/replay-XXXXXX)
for run in a b; do
    "$binary" "$rom" "$out/$run" tools/house-scene-qualification/route.jsonl >"$out/$run.jsonl"
    "$binary" "$rom" "$out/exception-$run" tools/house-scene-qualification/exception-route.jsonl >"$out/exception-$run.jsonl"
done
python3 -B tools/house-scene-qualification/check.py "$rom" "$out"
python3 -O -B tools/house-scene-qualification/check.py "$rom" "$out"
printf 'Matching fresh whole-house census: %s\n' "$out"
