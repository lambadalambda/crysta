#!/bin/sh
# One empty-SRAM Session process. Every retained capture synchronizes; never restore.
set -eu
rom=${1:-local/Tenchi Souzou (Japan).sfc}
python3 -B tools/pandora-qualification/source.py "$rom"
python3 -B tools/pandora-qualification/test_source.py
python3 -O -B tools/pandora-qualification/test_source.py
python3 -B tools/pandora-qualification/test_check.py
python3 -O -B tools/pandora-qualification/test_check.py
sh tools/house-conversation-qualification/build.sh
probe=local/house-conversation-qualification/probe
binary=${CARGO_TARGET_DIR:-$probe/target}/release/house-conversation-probe
mkdir -p local/pandora-qualification
parent=$(mktemp -d local/pandora-qualification/replay-XXXXXX)
out=$parent/journey
printf 'Fresh Pandora capture root: %s\n' "$out"
"$binary" "$rom" "$out" <tools/pandora-qualification/route.jsonl >"$out.jsonl"
python3 -B tools/pandora-qualification/check.py "$rom" "$out"
python3 -O -B tools/pandora-qualification/check.py "$rom" "$out"
printf 'Fresh New Game → Pandora tour/control verified: %s\n' "$out"
