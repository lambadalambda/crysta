#!/usr/bin/env python3
"""Validate meta/ issue tracker structure: indexes, links, membership, cycles.

Exit code 1 prints each violation.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
META = ROOT / "meta"


def read_entries(path: Path, checked: bool) -> list[tuple[str, str]]:
    marker = "x" if checked else " "
    pattern = rf"^- \[{re.escape(marker)}\] \[([^]]+)\]\(([^)]+)\)$"
    return re.findall(pattern, path.read_text(), re.M)


def main() -> int:
    violations: list[str] = []

    open_entries = read_entries(META / "issues.md", checked=False)
    archived = read_entries(META / "issues_archive.md", checked=True)
    all_entries = open_entries + archived

    paths = [rel for _, rel in all_entries]
    if len(paths) != len(set(paths)):
        violations.append("issue duplicated across indexes")

    for title, rel in all_entries:
        p = META / rel
        if not p.exists():
            violations.append(f"missing issue detail: {p}")
        elif p.read_text().splitlines()[0] != f"# {title}":
            violations.append(f"title mismatch: {p}")

    details = {f"issues/{p.name}" for p in (META / "issues").glob("*.md")}
    if details != set(paths):
        violations.append(f"detail/index mismatch: {details ^ set(paths)}")

    roadmap = (META / "milestones.md").read_text()
    roadmap_paths = re.findall(r"^- \[[^]]+\]\((issues/[^)]+\.md)\)$", roadmap, re.M)
    for rel in set(roadmap_paths):
        if roadmap_paths.count(rel) != 1:
            violations.append(f"roadmap duplicate: {rel}")
    non_archived = details - {rel for _, rel in archived}
    if set(roadmap_paths) != non_archived:
        violations.append(f"roadmap/detail mismatch: {set(roadmap_paths) ^ non_archived}")

    graph: dict[str, list[str]] = {}
    for p in (META / "issues").glob("*.md"):
        text = p.read_text()
        if text.count("## Dependencies") != 1:
            violations.append(f"{p.name}: dependency section count")
        section = text.split("## Dependencies", 1)[1].split("## Requirements", 1)[0]
        graph[p.name] = re.findall(r"\[[^]]+\]\(([^)#]+\.md)\)", section)

    state: dict[str, int] = {}

    def visit(node: str) -> None:
        mark = state.get(node, 0)
        if mark == 1:
            violations.append(f"dependency cycle at {node}")
            return
        if mark == 2:
            return
        state[node] = 1
        for nxt in graph.get(node, []):
            visit(nxt)
        state[node] = 2

    for node in graph:
        visit(node)

    # Markdown link resolution across the repo.
    for p in ROOT.rglob("*.md"):
        if ".git" in p.parts:
            continue
        for match in re.finditer(r"\[[^]]+\]\(([^)]+)\)", p.read_text()):
            raw = match.group(1)
            if raw.startswith(("http://", "https://", "mailto:", "#")):
                continue
            target, _, fragment = raw.partition("#")
            if not target:
                continue
            if not (p.parent / target).resolve().exists():
                violations.append(f"{p}: broken link {raw}")
            elif fragment:
                t = (p.parent / target).resolve().read_text()
                if f'id="{fragment}"' not in t:
                    violations.append(f"{p}: missing anchor {raw}")

    if violations:
        print("tracker check FAILED:", file=sys.stderr)
        for v in violations:
            print(f"  {v}", file=sys.stderr)
        return 1
    print(
        f"tracker check passed "
        f"({len(open_entries)} open, {len(archived)} archived, "
        f"{len(roadmap_paths)} milestone issues)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
