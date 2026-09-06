#!/bin/sh
# Two fresh empty-SRAM shared bootstraps, no warps/patches/restores/save_state.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
mkdir -p local/house-npc-qualification/probe/src
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' tools/house-npc-qualification/probe.rs >local/house-npc-qualification/probe/src/main.rs
cp tools/new-game-qualification/bootstrap.rs local/house-npc-qualification/probe/src/
cp tools/house-npc-qualification/export.rs local/house-npc-qualification/probe/src/export.rs
cat >local/house-npc-qualification/probe/Cargo.toml <<'TOML'
[package]
name = "house-npc-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
assets = { path = "../../../crates/assets" }
serde_json = "1"
[[bin]]
name = "house-npc-export"
path = "src/export.rs"
TOML
cargo test -p assets --lib
python3 -B tools/house-npc-qualification/test_checks.py
cargo build --release --manifest-path local/house-npc-qualification/probe/Cargo.toml
out=$(mktemp -d local/house-npc-qualification/replay-XXXXXX)
local/house-npc-qualification/probe/target/release/house-npc-export "$rom" "$out/export"
for run in a b; do
    local/house-npc-qualification/probe/target/release/house-npc-probe "$rom" "$out/$run"
done
python3 -B tools/house-npc-qualification/check.py "$rom" "$out"
printf 'Matching fresh house NPC evidence: %s\n' "$out"
