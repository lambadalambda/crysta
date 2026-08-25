#!/usr/bin/env python3
"""Repository safety checks that must pass in a ROM-free environment.

Checks:
  1. No tracked file uses known ROM extensions or generated-asset locations.
  2. No tracked file's SHA-256 matches a known full-ROM digest.
  3. No tracked binary blob larger than MAX_BLOB_BYTES outside allowed paths.
  4. No ignored local/ or private/ content is tracked.

Exit code 1 prints each violation.
"""

from __future__ import annotations

import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

ROM_EXTENSIONS = {".sfc", ".smc", ".fig", ".swc"}

# Known full-ROM SHA-256 digests (normalized images).
KNOWN_ROM_SHA256 = {
    "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548",  # Japan
    "93ba50d853e98e1ca227a2ca72389c0e3ac18d6b50c946b3f618c16c2d3edd38",  # Europe
}

# Suspicious large binaries must be under an allowed prefix (reviewed fixtures).
ALLOWED_BINARY_PREFIXES = ("tests/fixtures/",)

MAX_BLOB_BYTES = 512 * 1024


def tracked_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [line for line in out.splitlines() if line]


def main() -> int:
    violations: list[str] = []

    for rel in tracked_files():
        path = ROOT / rel
        if path.is_dir():
            continue
        lower = rel.lower()
        suffix = Path(lower).suffix
        base = Path(lower).name
        parts = Path(lower).parts

        if suffix in ROM_EXTENSIONS:
            violations.append(f"ROM extension tracked: {rel}")
        if base.startswith("baserom"):
            violations.append(f"baserom-like file tracked: {rel}")
        if parts[:1] in (("local",), ("private",)):
            violations.append(f"ignored local content tracked: {rel}")
        if "generated" in parts:
            violations.append(f"generated-asset location tracked: {rel}")

        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest in KNOWN_ROM_SHA256:
            violations.append(f"tracked file matches known ROM digest: {rel}")
        if (
            len(data) > MAX_BLOB_BYTES
            and not rel.startswith(ALLOWED_BINARY_PREFIXES)
            and suffix not in {".lock", ".md"}
        ):
            violations.append(
                f"binary blob {len(data)} bytes exceeds {MAX_BLOB_BYTES}: {rel}"
            )

    if violations:
        print("repository safety check FAILED:", file=sys.stderr)
        for v in violations:
            print(f"  {v}", file=sys.stderr)
        return 1
    print(f"repository safety check passed ({len(tracked_files())} tracked files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
