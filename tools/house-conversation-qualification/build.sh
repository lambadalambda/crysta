#!/bin/sh
set -eu
probe=local/house-conversation-qualification/probe
mkdir -p "$probe/src"
sed 's|../new-game-qualification/bootstrap.rs|bootstrap.rs|' tools/house-conversation-qualification/probe.rs >"$probe/src/main.rs"
cp tools/new-game-qualification/bootstrap.rs "$probe/src/"
cat >"$probe/Cargo.toml" <<'TOML'
[package]
name = "house-conversation-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1"
TOML
cargo build --release --manifest-path "$probe/Cargo.toml"
