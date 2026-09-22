#!/bin/sh
# Build original tool sources in ignored local/; do not copy the codec/bootstrap.
set -eu
cd "$(dirname "$0")/../.."
build=local/crysta-cadence-qualification/build
mkdir -p "$build"
cat >"$build/Cargo.toml" <<'TOML'
[package]
name = "crysta-cadence-qualification"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = { path = "../../../crates/assets" }
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
serde_json = "1" # MIT OR Apache-2.0; already used by oracle
[[bin]]
name = "crysta-cadence-decode"
path = "../../../tools/crysta-cadence-qualification/decode.rs"
[[bin]]
name = "crysta-cadence-probe"
path = "../../../tools/crysta-cadence-qualification/probe.rs"
TOML
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/local/crysta-cadence-qualification/target}"
cargo build --release --manifest-path "$build/Cargo.toml"
printf 'Binaries: %s/release/crysta-cadence-{decode,probe}\n' "$CARGO_TARGET_DIR"
