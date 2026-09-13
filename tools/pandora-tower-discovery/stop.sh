#!/bin/sh
# Retire a discovery session. The probe never exits on its own: the holder keeps
# its stdin open, and probe.rs only stops on a finish command or EOF.
set -eu
dir=${DISCOVERY_DIR:-local/pandora-tower-discovery/session}
for name in probe holder; do
  file=$dir/$name.pid
  [ -f "$file" ] || continue
  pid=$(cat "$file")
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    printf 'stopped %s %s\n' "$name" "$pid"
  fi
  rm -f "$file"
done
