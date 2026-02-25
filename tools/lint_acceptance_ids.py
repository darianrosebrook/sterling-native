#!/usr/bin/env python3
"""
Spec Traceability Linter (multi-spec)

Two lint modes, both enforced in CI, applied to all .caws/specs/*.yaml files:

1. Acceptance ID anchoring: every acceptance ID in any spec file
   must appear as a literal string in at least one tracked source file
   outside the spec itself.

2. Claim pointer resolution: every "file.rs::fn_name" entry under
   pointers.tests in any spec must resolve to an actual Rust
   test function (matched by `fn fn_name(` in the resolved file).

Usage:
    python tools/lint_acceptance_ids.py

Exit codes:
    0 = all checks pass
    1 = one or more checks failed
"""

import re
import subprocess
import sys
from pathlib import Path

SPEC_DIR = Path(".caws/specs")

# Directories to search for anchors (relative to repo root).
SEARCH_PATHS = ["kernel/", "search/", "harness/", "tests/", ".github/"]

# Directories to search when resolving bare filenames in pointers.tests.
RESOLVE_ROOTS = [
    Path("kernel/src"),
    Path("search/src"),
    Path("harness/src"),
    Path("tests/lock/tests"),
    Path("tests/lock/src"),
]

# Acceptance ID patterns — each spec namespace.
# S1-M* for SPINE-001, SC1-M* for SEARCH-CORE-001, and a general fallback.
ACCEPTANCE_ID_RE = re.compile(r"\b(?:S1|SC1)-M\d+(?:-[A-Z0-9]+)+\b")

# Pointer pattern: "filename.rs::fn_name" in YAML.
POINTER_RE = re.compile(r'"([^"]+\.rs)::([^"]+)"')


def find_spec_files() -> list[Path]:
    """Find all YAML spec files in .caws/specs/."""
    if not SPEC_DIR.exists():
        return []
    specs = sorted(SPEC_DIR.glob("*.yaml"))
    return specs


def extract_acceptance_ids(spec_path: Path) -> set[str]:
    """Extract all acceptance IDs from the spec file."""
    text = spec_path.read_text()
    return set(ACCEPTANCE_ID_RE.findall(text))


def extract_test_pointers(spec_path: Path) -> list[tuple[str, str, int]]:
    """Extract all file.rs::fn_name pointers from the spec.

    Returns list of (filename, fn_name, line_number).
    """
    pointers = []
    for i, line in enumerate(spec_path.read_text().splitlines(), start=1):
        for match in POINTER_RE.finditer(line):
            pointers.append((match.group(1), match.group(2), i))
    return pointers


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


class AmbiguousFile(Exception):
    """Raised when a bare filename resolves to multiple files."""

    def __init__(self, filename: str, candidates: list[Path]):
        self.filename = filename
        self.candidates = candidates


def resolve_file(filename: str) -> Path | None:
    """Resolve a bare or prefixed filename to a workspace path.

    Handles both bare names like "compile.rs" (searched under RESOLVE_ROOTS)
    and prefixed paths like "harness/src/bundle.rs".

    Raises AmbiguousFile if a bare filename matches more than one file.
    Use a prefixed path in the spec to disambiguate.
    """
    # Try as a direct relative path first.
    direct = Path(filename)
    if direct.exists():
        return direct

    # Search under known roots — collect all matches.
    basename = Path(filename).name
    hits: list[Path] = []
    for root in RESOLVE_ROOTS:
        for candidate in root.rglob(basename):
            if candidate.is_file():
                hits.append(candidate)

    if len(hits) == 1:
        return hits[0]
    if len(hits) > 1:
        raise AmbiguousFile(filename, hits)
    return None


def file_contains_fn(path: Path, fn_name: str) -> bool:
    """Check whether a file contains `fn fn_name(`."""
    try:
        text = path.read_text()
        # Match `fn fn_name(` with optional whitespace variations.
        pattern = re.compile(rf"\bfn\s+{re.escape(fn_name)}\s*\(")
        return bool(pattern.search(text))
    except OSError:
        return False


# ---------------------------------------------------------------------------
# Lint 1: Acceptance ID anchoring (across all specs)
# ---------------------------------------------------------------------------


def lint_acceptance_ids_for_spec(spec_path: Path) -> tuple[set[str], list[str]]:
    """Returns (all_ids, unanchored_ids) for a single spec."""
    ids = extract_acceptance_ids(spec_path)
    unanchored = []
    for aid in sorted(ids):
        hits = grep_for_id(aid, SEARCH_PATHS)
        if not hits:
            unanchored.append(aid)
    return ids, unanchored


# ---------------------------------------------------------------------------
# Lint 2: Claim pointer resolution (across all specs)
# ---------------------------------------------------------------------------


def lint_test_pointers_for_spec(
    spec_path: Path,
) -> tuple[list[tuple[str, str, int]], list[str]]:
    """Returns (all_pointers, broken_descriptions) for a single spec."""
    pointers = extract_test_pointers(spec_path)
    broken = []
    for filename, fn_name, line_no in pointers:
        try:
            resolved = resolve_file(filename)
        except AmbiguousFile as e:
            candidates = ", ".join(str(c) for c in e.candidates)
            broken.append(
                f"  line {line_no}: {filename}::{fn_name} — ambiguous: "
                f"resolves to {len(e.candidates)} files ({candidates}). "
                f"Use a prefixed path in the spec to disambiguate."
            )
            continue
        if resolved is None:
            broken.append(
                f"  line {line_no}: {filename}::{fn_name} — file not found"
            )
        elif not file_contains_fn(resolved, fn_name):
            broken.append(
                f"  line {line_no}: {filename}::{fn_name} — fn not found in {resolved}"
            )
    return pointers, broken


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> int:
    spec_files = find_spec_files()
    if not spec_files:
        print(f"ERROR: no spec files found in {SPEC_DIR}", file=sys.stderr)
        return 1

    print(f"Found {len(spec_files)} spec(s): {', '.join(p.name for p in spec_files)}\n")

    failed = False
    total_ids = 0
    total_unanchored = 0
    total_pointers = 0
    total_broken = 0

    for spec_path in spec_files:
        spec_name = spec_path.stem
        print(f"=== {spec_name} ===")

        # --- Lint 1: Acceptance ID anchoring ---
        ids, unanchored = lint_acceptance_ids_for_spec(spec_path)
        total_ids += len(ids)
        total_unanchored += len(unanchored)

        print(f"  [1/2] Acceptance IDs: {len(ids)} in spec")
        if unanchored:
            print(f"    FAIL: {len(unanchored)} unanchored:")
            for aid in unanchored:
                print(f"      - {aid}")
            print(
                "\n    Fix: add '// ACCEPTANCE: {ID}' above the relevant test cluster."
            )
            failed = True
        else:
            print(f"    OK: all {len(ids)} anchored")

        # --- Lint 2: Claim pointer resolution ---
        pointers, broken = lint_test_pointers_for_spec(spec_path)
        total_pointers += len(pointers)
        total_broken += len(broken)

        print(f"  [2/2] Claim pointers: {len(pointers)} in spec")
        if broken:
            print(f"    FAIL: {len(broken)} broken:")
            for msg in broken:
                print(f"  {msg}")
            print(
                "\n    Fix: update the pointer to match the actual file/fn name, "
                "or create the missing test."
            )
            failed = True
        else:
            print(f"    OK: all {len(pointers)} resolve")

        print()

    # Summary
    print(f"--- Summary: {total_ids} acceptance IDs, {total_unanchored} unanchored; "
          f"{total_pointers} pointers, {total_broken} broken ---")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
