#!/bin/sh
# Two independent fresh boots. ROM-backed output stays in local/.
set -eu
python3 -B tools/player-sprite-qualification/test_checks.py
mkdir -p local/player-sprite-qualification/probe/src
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' tools/player-sprite-qualification/probe.rs >local/player-sprite-qualification/probe/src/main.rs
cp tools/new-game-qualification/bootstrap.rs local/player-sprite-qualification/probe/src/
cp tools/player-sprite-qualification/export.rs local/player-sprite-qualification/probe/src/export.rs
cat >local/player-sprite-qualification/probe/Cargo.toml <<'TOML'
[package]
name = "player-sprite-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
assets = { path = "../../../crates/assets" }
serde_json = "1"
[[bin]]
name = "player-sprite-export"
path = "src/export.rs"
TOML
cargo build --release --manifest-path local/player-sprite-qualification/probe/Cargo.toml
out=$(mktemp -d local/player-sprite-qualification/replay-XXXXXX)
rom=${1:-local/Tenchi Souzou (Japan).sfc}
local/player-sprite-qualification/probe/target/release/player-sprite-export "$rom" "$out/export"
for run in a b; do
    local/player-sprite-qualification/probe/target/release/player-sprite-probe "$rom" "$out/$run"
    python3 -B tools/player-sprite-qualification/evidence.py "$rom" "$out/$run" >"$out/$run.json"
done
cmp "$out/a.json" "$out/b.json"
python3 -B tools/player-sprite-qualification/check.py "$rom" "$out"
printf 'Matching fresh sprite evidence: %s\n' "$out"
