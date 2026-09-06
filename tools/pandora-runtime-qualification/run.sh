#!/bin/sh
# Compile/run the CPU-free adapter independently of parent main/host registration.
set -eu
[ "$#" -ge 1 ] || { echo 'usage: run.sh ROM [test-filter]' >&2; exit 2; }
rom=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
build=local/pandora-runtime-qualification/build
mkdir -p "$build/src"
printf '//! Offline source compiler and input-only runtime qualification.\n' > "$build/src/main.rs"
for module in house_profiles house_navigation house_progression new_game room_dialogue pandora_navigation pandora_progression; do
  printf '#[allow(dead_code)]\n#[path = "%s/crates/map-inspector/src/%s.rs"]\nmod %s;\n' "$PWD" "$module" "$module" >> "$build/src/main.rs"
done
cat tools/pandora-runtime-qualification/driver.rs >> "$build/src/main.rs"
cat > "$build/Cargo.toml" <<'EOF'
[package]
name = "pandora-runtime-qualification"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
assets = { path = "../../../crates/assets" }
rom = { path = "../../../crates/rom" }
room-core = { path = "../../../crates/room-core" }
serde_json = "1"
# Match the actual workspace policy; registration is checked separately there.
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = "warn"
EOF
PANDORA_ROM="$rom" cargo test --quiet --manifest-path "$build/Cargo.toml" "${2:-pandora_progression}" -- --include-ignored --nocapture
