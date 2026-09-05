#!/bin/sh
# Fresh input-only startup and F -> 10 -> F route, one Session/process.
set -eu
export PYTHONDONTWRITEBYTECODE=1
python3 tools/new-game-qualification/test_route.py
mkdir -p local/new-game-qualification/route-probe/src
cp tools/new-game-qualification/route_probe.rs local/new-game-qualification/route-probe/src/main.rs
cp tools/new-game-qualification/bootstrap.rs local/new-game-qualification/route-probe/src/
cat >local/new-game-qualification/route-probe/Cargo.toml <<'TOML'
[package]
name = "new-game-route"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
assets = { path = "../../../crates/assets" }
serde_json = "1"
TOML
cargo build --release --manifest-path local/new-game-qualification/route-probe/Cargo.toml
rom=${1:-local/Tenchi Souzou (Japan).sfc}
out=$(mktemp -d local/new-game-qualification/route-XXXXXX)
python3 tools/new-game-qualification/startup.py "$rom" >"$out/semantic-new-game.json"
for run in a b; do
    local/new-game-qualification/route-probe/target/release/new-game-route "$rom" "$out/$run"
    python3 tools/new-game-qualification/route_check.py "$out/$run"
done
cmp "$out/a/frames.csv" "$out/b/frames.csv"
cmp "$out/a/route.json" "$out/b/route.json"
printf 'Two matching exit-0 fresh house round trips. Captures: %s\n' "$out"
