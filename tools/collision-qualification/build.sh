#!/bin/sh
# Build the collision-discovery probe into ignored local/ space.
set -eu
probe=local/collision-qualification/probe
mkdir -p "$probe/src"
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' \
  tools/collision-qualification/probe.rs >"$probe/src/main.rs"
cp tools/new-game-qualification/bootstrap.rs "$probe/src/"
mkdir -p "$probe/src/bin"
sed 's|../new-game-qualification/bootstrap.rs|../bootstrap.rs|' \
  tools/collision-qualification/trace.rs >"$probe/src/bin/trace.rs"
cp tools/collision-qualification/pair_survey.rs "$probe/src/bin/pair_survey.rs"
cat >"$probe/Cargo.toml" <<'TOML'
[package]
name = "collision-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = { path = "../../../crates/assets" }
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
cargo build --release --manifest-path "$probe/Cargo.toml"
