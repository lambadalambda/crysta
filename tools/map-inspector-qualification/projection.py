"""Build-input files pinned by projection rather than whole-file hash.

`Cargo.lock` and the workspace `Cargo.toml` are producer sources because they
decide what the producer compiles against. Both also record things the producer
cannot reach: adding an unrelated workspace member rewrites ``members`` and
appends a ``[[package]]`` block without changing a single input to the capture.

Each is therefore pinned as a projection against a frozen copy of the exact file
the descriptor named, under ``pinned/``. The Rust sourcegate in
``crates/map-inspector/tests/local_capture.rs`` implements the same two
projections; both compare the live file to the same fixture, so a divergence
between the two implementations makes one of them red.

This is not a relaxation of the descriptor. `effective_sha` returns the
fixture's whole-file hash **only** when the live file projects onto it, and the
live file's own hash otherwise, so every existing equality check in the bridge
keeps its meaning.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
PINNED = HERE / 'pinned'
LOCK = 'Cargo.lock'
ROOT_MANIFEST = 'Cargo.toml'
PROJECTED = (LOCK, ROOT_MANIFEST)
ROOT_PACKAGE = 'map-inspector'

UNIT = '\x1f'
RECORD = '\x1e'
GROUP = '\x1d'


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def lockfile_packages(lock: str) -> list[dict]:
    """Parses the ``[[package]]`` blocks of a Cargo.lock."""
    packages: list[dict] = []
    current: dict | None = None
    in_dependencies = False
    for line in lock.splitlines():
        trimmed = line.strip()
        if trimmed == '[[package]]':
            if current is not None:
                packages.append(current)
            current = {'name': '', 'version': '', 'source': '',
                       'checksum': '', 'dependencies': []}
            in_dependencies = False
            continue
        if current is None:
            continue
        if in_dependencies:
            if trimmed == ']':
                in_dependencies = False
            else:
                current['dependencies'].append(trimmed.rstrip(',').strip('"'))
            continue
        if trimmed == 'dependencies = [':
            in_dependencies = True
        elif trimmed.startswith(('name =', 'version =', 'source =', 'checksum =')):
            key, _, value = trimmed.partition('=')
            current[key.strip()] = value.strip().strip('"')
        elif trimmed.startswith('['):
            # A new top-level table ends the package blocks.
            packages.append(current)
            current = None
            break
    if current is not None:
        packages.append(current)
    return packages


def dependency_reference(reference: str) -> tuple[str, str | None, str | None]:
    """Splits ``name``, ``name version`` or ``name version (source)``."""
    head, sep, rest = reference.partition(' (')
    source = rest.rstrip(')') if sep else None
    fields = head.split()
    name = fields[0] if fields else ''
    version = fields[1] if len(fields) > 1 else None
    return name, version, source


def identity(package: dict) -> tuple[str, str, str]:
    # Keyed on the source too: cargo emits a reference's source exactly when
    # name and version are ambiguous, which a patched git fork that kept its
    # version number produces. Merging those would drop the second one's edges.
    return package['name'], package['version'], package['source']


def lockfile_subtree(lock: str, root: str = ROOT_PACKAGE) -> str:
    """Canonical text of ``root``'s transitive dependency closure."""
    packages = lockfile_packages(lock)
    wanted: set[tuple[str, str, str]] = set()
    queue = [root]
    while queue:
        name, version, source = dependency_reference(queue.pop())
        for package in packages:
            # An omitted field is ambiguous only when several packages share
            # what the reference does give; taking all of them can only widen.
            if package['name'] != name:
                continue
            if version is not None and package['version'] != version:
                continue
            if source is not None and package['source'] != source:
                continue
            key = identity(package)
            if key not in wanted:
                wanted.add(key)
                queue.extend(package['dependencies'])
    canonical = []
    by_identity = {identity(package): package for package in packages}
    for key in sorted(wanted):
        package = by_identity[key]
        canonical.append(UNIT.join([
            package['name'],
            package['version'],
            package['source'],
            package['checksum'],
            GROUP.join(sorted(package['dependencies'])),
        ]) + RECORD)
    return ''.join(canonical)


def bracket_depth(line: str, depth: int) -> int:
    """Bracket nesting after ``line``, ignoring quoted text."""
    quoted = False
    for character in line:
        if character == '"':
            quoted = not quoted
        elif character == '[' and not quoted:
            depth += 1
        elif character == ']' and not quoted:
            depth = max(0, depth - 1)
    return depth


def manifest_without_members(manifest: str) -> str:
    """The manifest with only ``[workspace] members`` elided.

    The producer is built by a pinned command, ``cargo build --locked -p
    map-inspector``, which selects one package, so the *set* of other workspace
    members cannot reach it. Everything else stays byte-for-byte.

    The elision is bounded by the ``[workspace]`` table and by real bracket
    depth counted outside quoted strings; a trailing-bracket heuristic would let
    ``members = [...] #`` run the elision on to the next bracketed line.
    """
    out = []
    table = ''
    depth = 0
    for line in manifest.splitlines():
        trimmed = line.strip()
        if depth > 0:
            depth = bracket_depth(line, depth)
            continue
        if trimmed.startswith('[') and not trimmed.startswith('[['):
            table = trimmed.strip('[]')
        key = trimmed.partition('=')[0].strip() if '=' in trimmed else None
        if table == 'workspace' and key == 'members':
            out.append('members = <elided>')
            depth = bracket_depth(line, 0)
            continue
        out.append(line)
    return ''.join(line + '\n' for line in out)


PROJECTIONS = {
    LOCK: lambda text: lockfile_subtree(text),
    ROOT_MANIFEST: manifest_without_members,
}


def effective_sha(repo: Path, name: str) -> str:
    """Hash the bridge should compare against the descriptor for ``name``.

    For a projected file this is the frozen fixture's whole-file hash when the
    live file projects onto it, and the live file's own hash otherwise — so a
    real change still fails every existing equality check.
    """
    data = (repo / name).read_bytes()
    project = PROJECTIONS.get(name)
    if project is None:
        return sha(data)
    fixture = (PINNED / name).read_bytes()
    if project(data.decode()) == project(fixture.decode()):
        return sha(fixture)
    return sha(data)
