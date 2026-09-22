#!/bin/sh
# One fresh boot through first-tower interior control; never restores checkpoint states.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
python3 -B tools/tower-approach-qualification/tower_source.py "$rom"
for mode in '' '-O'; do
  for test in test_source_contract test_departure test_tower_source test_tower; do
    python3 $mode -B "tools/tower-approach-qualification/$test.py"
  done
done
sh tools/house-conversation-qualification/build.sh
probe=local/house-conversation-qualification/probe
binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-conversation-probe
mkdir -p local/tower-approach-qualification
parent=$(mktemp -d local/tower-approach-qualification/tower-XXXXXX)
out=$parent/journey
printf 'Fresh first-tower capture root: %s\n' "$out"
"$binary" "$rom" "$out" <tools/tower-approach-qualification/tower-route.jsonl >"$out.jsonl"
python3 -B tools/tower-approach-qualification/check_tower.py "$rom" "$out"
python3 -O -B tools/tower-approach-qualification/check_tower.py "$rom" "$out"
printf 'Verified through first-tower interior entrance/control, not combat: %s\n' "$out"
