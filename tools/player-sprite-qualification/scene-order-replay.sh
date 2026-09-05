#!/bin/sh
# Reuse an existing two-boot sprite capture pair; mode traces use new fresh boots.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
captures=${2:?usage: scene-order-replay.sh ROM local/SPRITE-REPLAY}
python3 -B tools/player-sprite-qualification/test_scene_order.py
mkdir -p local/player-sprite-qualification/scene-probe/src
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' tools/player-sprite-qualification/scene_order_probe.rs >local/player-sprite-qualification/scene-probe/src/main.rs
cp tools/new-game-qualification/bootstrap.rs local/player-sprite-qualification/scene-probe/src/
cat >local/player-sprite-qualification/scene-probe/Cargo.toml <<'TOML'
[package]
name = "scene-order-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = { path = "../../../crates/assets" }
rom = { path = "../../../crates/rom" }
oracle = { path = "../../../crates/oracle" }
serde_json = "1"
TOML
cargo build --release --manifest-path local/player-sprite-qualification/scene-probe/Cargo.toml
out=$(mktemp -d local/player-sprite-qualification/scene-order-XXXXXX)
for spec in f:2340 10:6967; do
    name=${spec%:*}; start=${spec#*:}
    local/player-sprite-qualification/scene-probe/target/release/scene-order-probe "$rom" "$out/$name" "$start"
done
python3 -B tools/player-sprite-qualification/scene_order.py "$rom" "$out" "$captures"
printf 'Read-only house mode writer evidence: %s\n' "$out"
