#!/usr/bin/env python3
"""
Acceptance ID Anchoring Linter

Validates that every acceptance ID declared in .caws/specs/SPINE-001.yaml
is referenced at least once in tracked source files outside the spec itself.

"Referenced" means a literal string match (comment, doc, test name, etc.).
This ensures acceptance IDs are mechanically traceable from spec to proof.

Usage:
    python tools/lint_acceptance_ids.py

Exit codes:
    0 = all acceptance IDs are anchored
    1 = one or more acceptance IDs are unanchored
"""

import re
import subprocess
import sys
from pathlib import Path

SPEC_PATH = Path(".caws/specs/SPINE-001.yaml")

# Directories to search for anchors (relative to repo root).
SEARCH_PATHS = ["kernel/", "harness/", "tests/", ".github/"]

# Acceptance ID pattern: S1-M followed by digit(s), then optional dash-separated
# uppercase segments (e.g., S1-M1-DETERMINISM-CROSSPROC).
ACCEPTANCE_ID_RE = re.compile(r"\bS1-M\d+(?:-[A-Z0-9]+)+\b")


def extract_acceptance_ids(spec_path: Path) -> set[str]:
    """Extract all acceptance IDs from the spec file."""
    text = spec_path.read_text()
    return set(ACCEPTANCE_ID_RE.findall(text))


def grep_for_id(acceptance_id: str, search_paths: list[str]) -> list[str]:
    """Search for a literal acceptance ID string in the workspace."""
    try:
        result = subprocess.run(
            [
                "grep",
                "-rn",
                "--include=*.rs",
                "--include=*.yml",
                "--include=*.yaml",
                "--include=*.toml",
                "--include=*.md",
                "-l",
                acceptance_id,
                *search_paths,
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return [f for f in result.stdout.strip().split("\n") if f]
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return []


def main() -> int:
    if not SPEC_PATH.exists():
        print(f"ERROR: spec file not found: {SPEC_PATH}", file=sys.stderr)
        return 1

    ids = extract_acceptance_ids(SPEC_PATH)
    if not ids:
        print("WARNING: no acceptance IDs found in spec", file=sys.stderr)
        return 0

    unanchored: list[str] = []
    anchored: list[str] = []

    for aid in sorted(ids):
        hits = grep_for_id(aid, SEARCH_PATHS)
        if hits:
            anchored.append(aid)
        else:
            unanchored.append(aid)

    print(f"Acceptance IDs in spec: {len(ids)}")
    print(f"  Anchored: {len(anchored)}")
    print(f"  Unanchored: {len(unanchored)}")

    if unanchored:
        print("\nUNANCHORED acceptance IDs (no reference outside spec):")
        for aid in unanchored:
            print(f"  - {aid}")
        print(
            "\nFix: add a comment like '// ACCEPTANCE: {ID}' above the "
            "relevant test cluster."
        )
        return 1

    print("\nAll acceptance IDs are anchored.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
