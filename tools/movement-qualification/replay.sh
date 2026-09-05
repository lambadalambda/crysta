#!/bin/sh
# Run from repository root. Every invocation below is a fresh process/boot.
set -eu
export PYTHONDONTWRITEBYTECODE=1
python3 tools/movement-qualification/test_flat.py
python3 - <<'PY'
from pathlib import Path
from hashlib import sha256
for p,h in [('local/Tenchi Souzou (Japan).sfc','f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'),('local/saves/Terranigma.srm','709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055')]:
    assert sha256(Path(p).read_bytes()).hexdigest()==h,p
PY
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
for d in Left Right Up Down; do
  "$p" local/movement/wall-$d 1790 "$d:1601:1780"
  "$p" local/movement/map10-$d 2000 Left:1601:1657 Down:1657:1682 "$d:1801:1990"
done
"$p" local/movement/doorway-approach 1681 Left:1601:1657 Down:1657:1682
"$p" local/movement/up-central 1780 Left:1601:1657 Up:1657:1770
"$p" local/movement/up-type12 1780 Left:1601:1712 Up:1712:1770
"$p" local/movement/corner-positive 1861 Left:1601:1657 Down:1657:1682 Down:1801:1809 Right:1813:1830
"$p" local/movement/corner-negative 1861 Left:1601:1657 Down:1657:1682 Down:1801:1814 Left:1820:1850
"$p" local/movement/map10-up-wall 2050 Left:1601:1657 Down:1657:1682 Left:1801:1825 Up:1840:2040
"$p" local/movement/cadence 1670 Right:1601:1611 Left:1611:1621 Up:1621:1631 Down:1631:1641 Right:1651:1652
"$p" local/movement/pulses 1665 Left:1601:1602 Left:1605:1607 Left:1610:1613 Left:1616:1620 Left:1621:1625 Right:1625:1629 Right:1631:1635 Up:1635:1638 Down:1638:1642
PCS=80dda0,80de57,80deb6,80df00,80d150 "$p" local/movement/trace-right 1603 Right:1601:1610 > local/movement/trace-right.log
PCS=80da31,80db40,80db8a,80d155 "$p" local/movement/trace-left 1723 Left:1601:1780 > local/movement/trace-left.log
PCS=80d682,80d798,80d7e2,80d186 "$p" local/movement/trace-down 1625 Down:1601:1700 > local/movement/trace-down.log
PCS=80d295,80d3a4,80d401,80d18b "$p" local/movement/trace-up-central 1713 Left:1601:1657 Up:1657:1770 > local/movement/trace-up-central.log
PCS=80d295,80d3a4,80d401,80d18b "$p" local/movement/trace-type12 1725 Left:1601:1712 Up:1712:1770 > local/movement/trace-type12.log
PCS=80d295,80d3a4,80d401,80d18b "$p" local/movement/trace-map10-up 1844 Left:1601:1657 Down:1657:1682 Left:1801:1825 Up:1840:2040 > local/movement/trace-map10-up.log
PCS=84a385,809c03,80f251,80d0cf "$p" local/movement/cadence-gap 1656 Left:1601:1780 > local/movement/cadence-gap.log
if NAIVE_CADENCE=1 python3 tools/movement-qualification/verify.py > local/movement/red-naive-cadence.log 2>&1; then
  echo 'naive cadence unexpectedly passed' >&2; exit 1
fi
python3 tools/movement-qualification/verify.py > local/movement/verification.log
cat local/movement/verification.log
