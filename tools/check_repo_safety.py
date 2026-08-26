#!/usr/bin/env python3
"""Repository safety checks that must pass in a ROM-free environment.

Checks:
  1. No tracked file uses known ROM extensions or generated-asset locations.
  2. No tracked file's SHA-256 matches a known full-ROM digest.
  3. No tracked binary blob larger than MAX_BLOB_BYTES outside allowed paths.
  4. No ignored local/ or private/ content is tracked.
  5. No non-const file-scope static in vendor/lakesnes/*.c (Send-soundness
     invariant).

Exit code 1 prints each violation.
"""

from __future__ import annotations

import hashlib
import re
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

# Vendored core files whose file-scope statics must stay const, because the
# oracle crate's `Send` soundness relies on the core holding no mutable
# global state.
STATICS_GUARDED_PREFIX = "vendor/lakesnes/"

# Non-const file-scope static declarations: `static` at column 0 after
# spaces/tabs, not `const`/`inline`, with no `(` up to a `=` or `;` (so
# function declarations/definitions are excluded)...
NON_CONST_STATIC_RE = re.compile(r"^[ \t]*static\s+(?!const\b|inline\b)[^(\n]*[;=]", re.MULTILINE)
# ...except mutable function pointers, which are still mutable global state.
FUNC_PTR_STATIC_RE = re.compile(r"^[ \t]*static\s+(?!const\b|inline\b)[^;\n]*\(\s*\*\s*\w+[^;\n]*;", re.MULTILINE)
# `/* ... */` block comments (non-greedy, no nesting in vendored C style),
# replaced with newlines so reported line numbers stay accurate.
BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.DOTALL)


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
        if "local" in parts or "private" in parts:
            if parts[:1] in (("local",), ("private",)):
                violations.append(f"ignored local content tracked: {rel}")
            elif "generated" not in parts:
                violations.append(f"ignored nested local/private path tracked: {rel}")
        if "generated" in parts:
            violations.append(f"generated-asset location tracked: {rel}")

        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest in KNOWN_ROM_SHA256:
            violations.append(f"tracked file matches known ROM digest: {rel}")
        if (
            len(data) > MAX_BLOB_BYTES
            and not rel.startswith(ALLOWED_BINARY_PREFIXES)
            and rel != "Cargo.lock"
        ):
            violations.append(
                f"binary blob {len(data)} bytes exceeds {MAX_BLOB_BYTES}: {rel}"
            )

        if rel.startswith(STATICS_GUARDED_PREFIX) and rel.endswith(".c"):
            text = data.decode("utf-8", errors="replace")
            text = BLOCK_COMMENT_RE.sub(
                lambda m: "\n" * m.group().count("\n"), text
            )
            for m in NON_CONST_STATIC_RE.finditer(text):
                line = text.count("\n", 0, m.start()) + 1
                violations.append(
                    f"non-const file-scope static in {rel}:{line}: {m.group().strip()}"
                )
            for m in FUNC_PTR_STATIC_RE.finditer(text):
                line = text.count("\n", 0, m.start()) + 1
                violations.append(
                    f"non-const file-scope function pointer in {rel}:{line}: {m.group().strip()}"
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
