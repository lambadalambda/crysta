#!/bin/sh
# Source-only export, paired input-only fresh art runs, paired D creation traces.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
for flags in '-B' '-O -B'; do
    python3 $flags tools/house-scene-qualification/test_art.py
    python3 $flags tools/house-scene-qualification/test_census.py
done
probe=local/house-scene-qualification/art-probe
mkdir -p "$probe/src"
for pair in 'probe native' 'setup_probe setup'; do
    set -- $pair
    sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' "tools/house-scene-qualification/$1.rs" >"$probe/src/$2.rs"
done
cp tools/new-game-qualification/bootstrap.rs "$probe/src/"
cp tools/house-scene-qualification/export.rs "$probe/src/"
cat >"$probe/Cargo.toml" <<'TOML'
[package]
name = "house-art-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
assets = { path = "../../../crates/assets" }
serde_json = "1"
[[bin]]
name = "house-art-native"
path = "src/native.rs"
[[bin]]
name = "house-art-export"
path = "src/export.rs"
[[bin]]
name = "house-art-setup"
path = "src/setup.rs"
TOML
cargo build --release --manifest-path "$probe/Cargo.toml"
binaries=${CARGO_TARGET_DIR:-$probe/target}/release
out=$(mktemp -d local/house-scene-qualification/art-XXXXXX)
"$binaries/house-art-export" "$rom" "$out/export"
for run in a b; do
    "$binaries/house-art-native" "$rom" "$out/$run" tools/house-scene-qualification/art-route.jsonl >"$out/$run.jsonl"
    "$binaries/house-art-setup" "$rom" "$out/setup-$run"
done
python3 -B tools/house-scene-qualification/art_check.py "$rom" "$out"
python3 -O -B tools/house-scene-qualification/art_check.py "$rom" "$out"
printf 'Matching fresh house source art: %s\n' "$out"
