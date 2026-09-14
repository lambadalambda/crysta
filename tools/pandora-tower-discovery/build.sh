#!/bin/sh
# Build the discovery trace probe into ignored local/ space.
set -eu
probe=local/pandora-tower-discovery/trace-probe
mkdir -p "$probe/src"
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' \
  tools/pandora-tower-discovery/trace-probe.rs >"$probe/src/main.rs"
cp tools/new-game-qualification/bootstrap.rs "$probe/src/"
cat >"$probe/Cargo.toml" <<'TOML'
[package]
name = "tower-trace-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
cargo build --release --manifest-path "$probe/Cargo.toml"
