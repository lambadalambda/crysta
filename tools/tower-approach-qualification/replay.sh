#!/bin/sh
# One empty-SRAM native Session; checkpoint states are outputs, never restored.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
python3 -B tools/tower-approach-qualification/source_contract.py "$rom"
for mode in '' '-O'; do
  python3 $mode -B tools/tower-approach-qualification/test_source_contract.py
  python3 $mode -B tools/tower-approach-qualification/test_departure.py
done
sh tools/house-conversation-qualification/build.sh
probe=local/house-conversation-qualification/probe
binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-conversation-probe
mkdir -p local/tower-approach-qualification
parent=$(mktemp -d local/tower-approach-qualification/replay-XXXXXX)
out=$parent/journey
printf 'Fresh spear/frozen-return capture root: %s\n' "$out"
"$binary" "$rom" "$out" <tools/tower-approach-qualification/route.jsonl >"$out.jsonl"
python3 -B tools/tower-approach-qualification/check_departure.py "$rom" "$out"
python3 -O -B tools/tower-approach-qualification/check_departure.py "$rom" "$out"
printf 'Verified through frozen map21 control, not first-tower arrival: %s\n' "$out"
