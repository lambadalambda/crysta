#!/bin/sh
# Fresh input-only saved-game F -> 10 -> F witness; not new-game qualification.
set -eu
python3 -B tools/movement-qualification/house-return-check.py --sources
mkdir -p local/movement/probe/src
cp tools/movement-qualification/probe.rs local/movement/probe/src/main.rs
cat > local/movement/probe/Cargo.toml <<'EOF'
[package]
name = "movement-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
EOF
cargo build --release --manifest-path local/movement/probe/Cargo.toml
p=local/movement/probe/target/release/movement-probe
for n in a b; do
  CHECKPOINTS=1801,1815,1816,1831,1832,1842,1871,1872 "$p" "local/movement/return-bedroom-$n" 1950 Left:1601:1657 Down:1657:1682 Up:1801:1820
  python3 -B tools/movement-qualification/house-return-check.py "local/movement/return-bedroom-$n"
done
diff -r local/movement/return-bedroom-a local/movement/return-bedroom-b
# Independent diagnostic boots. Instruction stops do not replace normal frame pins.
PCS=8d8797,84b979,84b9a7 "$p" local/movement/return-departure-trace 1814 Left:1601:1657 Down:1657:1682 Up:1801:1820 > local/movement/return-departure-trace.log
PCS=84bb74,84bba2 "$p" local/movement/return-arrival-trace2 1849 Left:1601:1657 Down:1657:1682 Up:1801:1820 > local/movement/return-arrival-trace2.log
python3 - <<'PY'
from pathlib import Path
for name, targets, anchor in [('return-departure-trace', ['8d8797','84b979','84b9a7'], '392,336'), ('return-arrival-trace2', ['84bb74','84bba2'], '392,208')]:
    lines=Path(f'local/movement/{name}.log').read_text().splitlines()
    assert len(lines)==len(targets), name
    for line, target in zip(lines, targets):
        assert f'target={target} stop=TargetReached' in line and f'pos={anchor} ' in line, line
print('Two byte-identical return replays and five native dispatch stops qualified')
PY
