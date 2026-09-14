#!/bin/sh
# Serpentine sweep of the reachable floor, watching for any progression.
# usage: sweep.sh PREFIX [ROWS] [ROW_FRAMES]
#
# A held direction walks the player across every x on its row, so only the
# vertical resolution matters: ROW_FRAMES of `Up` between passes sets it.
#
# ROW_FRAMES must exceed the input-admission delay of roughly 7 frames, or the
# `Up` steps move nothing and the sweep silently repeats one row; see
# docs/input-admission.md.
#
# Each pass exits the sweep if `control` did not reach 160 while a direction was
# held. Without that check a swallowed-input state (see README: Start) produces a
# full set of "blocked everywhere" readings indistinguishable from geometry.
set -eu
dir=${DISCOVERY_DIR:-local/pandora-tower-discovery/session}
prefix=${1:?usage: sweep.sh PREFIX [ROWS] [ROW_FRAMES]}
rows=${2:-13}
row_frames=${3:-10}
pass=1
while [ "$pass" -le "$rows" ]; do
  for direction in Left Right; do
    label=$prefix-$pass-$direction
    sh tools/pandora-tower-discovery/step.sh "$label" 150 "$direction" >/dev/null
    if ! python3 -B tools/pandora-tower-discovery/observe.py --sweep "$dir/journey/$label.wram"; then
      echo "sweep stopped at $label" >&2
      exit 1
    fi
  done
  # Observe the row step too: it is where the admission bug hides.
  sh tools/pandora-tower-discovery/step.sh "$prefix-$pass-up" "$row_frames" Up >/dev/null
  if ! python3 -B tools/pandora-tower-discovery/observe.py --sweep \
      "$dir/journey/$prefix-$pass-up.wram"; then
    echo "sweep stopped at $prefix-$pass-up" >&2
    exit 1
  fi
  pass=$((pass + 1))
done
