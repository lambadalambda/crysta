#!/bin/sh
# Local evidence only: two empty-SRAM sessions, shared real-input bootstrap.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
python3 -B tools/house-navigation-qualification/test_check.py
python3 -O -B tools/house-navigation-qualification/test_check.py
probe=local/house-navigation-qualification/probe
mkdir -p "$probe/src"
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' tools/house-navigation-qualification/probe.rs >"$probe/src/main.rs"
cp tools/new-game-qualification/bootstrap.rs "$probe/src/"
sed 's|../../crates/map-inspector/src/new_game.rs|new_game.rs|' tools/house-navigation-qualification/core_probe.rs >"$probe/src/core.rs"
sed 's|../../../tools/new-game-qualification/sources.json|sources.json|' crates/map-inspector/src/new_game.rs >"$probe/src/new_game.rs"
cp tools/new-game-qualification/sources.json "$probe/src/"
cat >"$probe/Cargo.toml" <<'TOML'
[package]
name = "house-navigation-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
assets = { path = "../../../crates/assets" }
room-core = { path = "../../../crates/room-core" }
[[bin]]
name = "house-navigation-core"
path = "src/core.rs"
TOML
cargo build --release --manifest-path "$probe/Cargo.toml"
binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-navigation-probe
out=$(mktemp -d local/house-navigation-qualification/replay-XXXXXX)
printf "Capture root: %s\n" "$out"
for run in a b; do
    "$binary" "$rom" "$out/$run" tools/house-scene-qualification/route.jsonl >"$out/$run.jsonl"
    "$binary" "$rom" "$out/extra-$run" tools/house-navigation-qualification/route.jsonl >"$out/extra-$run.jsonl"
done
python3 -B tools/house-navigation-qualification/check.py "$rom" "$out"
python3 -O -B tools/house-navigation-qualification/check.py "$rom" "$out"
HOUSE_NAVIGATION_FIXTURES="$out" cargo test -p room-core --test local_house_navigation
core_binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-navigation-core
"$core_binary" "$rom" tools/house-background-qualification/profiles.json tools/house-navigation-qualification/core-route.jsonl >"$out/core.jsonl"
printf 'Fresh navigation evidence: %s\n' "$out"
