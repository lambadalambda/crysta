#!/bin/sh
# No native Session and no navigation. Export exclusively from the owned ROM.
set -eu
rom=$1
out=$2
probe=local/pandora-scene-qualification/exporter
mkdir -p "$probe/src"
cp tools/pandora-scene-qualification/export.rs "$probe/src/main.rs"
cat > "$probe/Cargo.toml" <<'TOML'
[package]
name = "pandora-scene-export"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = {path="../../../crates/assets"}
rom = {path="../../../crates/rom"}
serde_json = "1"
TOML
cargo run --quiet --manifest-path "$probe/Cargo.toml" -- "$rom" "$out"
