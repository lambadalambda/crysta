#!/bin/sh
# Reuse authenticated, input-only census/returned-F captures; take new writer traces.
set -eu
if [ "$#" -ne 3 ]; then
    echo 'usage: replay.sh ROM CENSUS_REPLAY RETURNED_F_REPLAY' >&2
    exit 2
fi
rom=$1
census=$2
returned=$3
python3 -B tools/house-background-qualification/test_checks.py
python3 -O -B tools/house-background-qualification/test_checks.py
build=local/house-background-qualification/build
mkdir -p "$build/src/bin"
cp tools/house-background-qualification/export.rs "$build/src/bin/"
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' tools/house-background-qualification/probe.rs >"$build/src/bin/probe.rs"
cp tools/new-game-qualification/bootstrap.rs "$build/src/bin/"
cat >"$build/Cargo.toml" <<'TOML'
[package]
name = "house-background-qualification"
version = "0.0.0"
edition = "2021"
autobins = false
[workspace]
[[bin]]
name = "export"
path = "src/bin/export.rs"
[[bin]]
name = "probe"
path = "src/bin/probe.rs"
[dependencies]
assets = { path = "../../../crates/assets" }
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
cargo build --release --manifest-path "$build/Cargo.toml"
bin=${CARGO_TARGET_DIR:-$build/target}/release
out=$(mktemp -d local/house-background-qualification/run-XXXXXX)
"$bin/export" "$rom" "$out/export"
for segment in boot room10 settledC north-door-push settledD settled11; do
    "$bin/probe" "$rom" "$out/$segment" tools/house-scene-qualification/route.jsonl "$segment"
done
python3 -B tools/house-background-qualification/check.py "$rom" "$census" "$out" "$returned"
python3 -O -B tools/house-background-qualification/check.py "$rom" "$census" "$out" "$returned"
printf 'Background qualification: %s\n' "$out"
