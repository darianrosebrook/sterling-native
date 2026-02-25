#!/usr/bin/env python3
"""
CAWS Document Authority Linter

Validates that markdown files in docs/ follow the doc authority policy:
- YAML front-matter with `authority:` field present
- authority value matches path convention (canonical/, policy/, adr/, architecture/)
- Authority-specific banned terms (e.g. canonical forbids ephemeral language)
- Link hygiene: resolves relative links and checks target existence
- No ephemeral docs staged for commit
- README/index files and docs/reference/v1/ are exempt

Usage:
    # Lint staged files only (for pre-commit hook)
    python tools/lint_docs.py --staged [files...]

    # Lint specific files
    python tools/lint_docs.py docs/canonical/glossary.md docs/policy/foo.md

    # Lint all docs
    python tools/lint_docs.py --all

Exit codes:
    0 = all checks pass
    1 = lint errors found
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

# --- Configuration ---

# Path prefix -> required authority value
PATH_AUTHORITY_MAP = {
    "docs/canonical/": "canonical",
    "docs/policy/": "policy",
    "docs/adr/": "adr",
    "docs/architecture/": "architecture",
}

# Paths that are exempt from authority checks
EXEMPT_PREFIXES = (
    "docs/reference/v1/",
    "docs/templates/",
    "docs/MOC/",        # generated inventory outputs
    "docs/_index/",     # generated structural index
)

# Filenames that are exempt (navigational, not authoritative)
EXEMPT_FILENAMES = {"README.md", "INDEX.md", "index.md"}

# Paths that should never be committed
BLOCKED_PREFIXES = (
    "docs/ephemeral/",
    "ephemeral/",
)

# Valid authority values
VALID_AUTHORITIES = {"canonical", "policy", "adr", "architecture", "reference", "ephemeral"}

# YAML front-matter regex: matches --- delimited block at start of file
FRONTMATTER_RE = re.compile(r"\A---\s*\n(.*?)\n---", re.DOTALL)

# authority field in YAML front-matter (simple extraction, no full YAML parser needed)
AUTHORITY_RE = re.compile(r"^authority:\s*(.+)$", re.MULTILINE)

# Markdown link regex for link hygiene
LINK_RE = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")

# --- Authority-aware banned terms ---
# Each authority level has terms/patterns that should not appear in its body text.
# These are checked against the content after stripping front-matter.
AUTHORITY_BANNED_TERMS: dict[str, list[tuple[re.Pattern, str]]] = {
    "canonical": [
        # Match "ephemeral" only when it looks like a doc-authority reference, not a technical term
        # e.g. "ephemeral docs", "authority: ephemeral", but not "Ephemeral, per-connection"
        (re.compile(r"\bephemeral\s+(doc|plan|roadmap|note|sprint|session)", re.IGNORECASE),
         "canonical docs must not reference ephemeral doc concepts"),
        (re.compile(r"\bwork[-\s]?in[-\s]?progress\b", re.IGNORECASE), "canonical docs must not be WIP"),
        # Match "draft" only when it's a status marker, not a technical reference
        # e.g. "Status: Draft", "this is a draft", but not "an earlier draft overloaded"
        (re.compile(r"(?:^|\n)\s*#+\s*.*\bdraft\b|^\s*>\s*.*\bdraft\s+document", re.IGNORECASE | re.MULTILINE),
         "canonical docs must not be marked as draft"),
        (re.compile(r"\bStage [A-Z]\b"), "v1 stage taxonomy (Stage K, Stage M, etc.) must not appear in canonical v2 docs"),
        (re.compile(r"\bStructural/Meaning/Pragmatic\b"), "v1 operator labels must not appear in canonical v2 docs"),
    ],
    "policy": [
        (re.compile(r"\bimplementation anchor\b", re.IGNORECASE), "policy docs must not contain implementation anchors"),
        (re.compile(r"\bsrc/\b|\.py:\d+"), "policy docs must not reference source file paths"),
    ],
    "adr": [
        (re.compile(r"\bephemeral\s+(doc|plan|roadmap|note|sprint|session)", re.IGNORECASE),
         "ADRs must not reference ephemeral doc concepts"),
    ],
}

def resolve_link(source_filepath: str, target: str) -> Path:
    """Resolve a relative link target against its source file's directory."""
    # Strip anchor fragments (e.g., file.md#section)
    target_path = target.split("#")[0]
    if not target_path:
        return Path(source_filepath)  # anchor-only link to self
    source_dir = Path(source_filepath).parent
    return (source_dir / target_path).resolve()


def check_link_exists(source_filepath: str, target: str, staged: bool = False) -> bool:
    """Check if a relative link target exists.

    In staged mode, checks both the git index and the working tree
    (since the target may be staged but not yet on disk, or vice versa).
    """
    resolved = resolve_link(source_filepath, target)
    # Check filesystem first
    if resolved.exists():
        return True
    if staged:
        # Check if the resolved path is in the git index
        # Normalize to repo-relative path
        try:
            repo_root = Path(
                subprocess.run(
                    ["git", "rev-parse", "--show-toplevel"],
                    capture_output=True, text=True
                ).stdout.strip()
            )
            rel = str(resolved.relative_to(repo_root))
            result = subprocess.run(
                ["git", "ls-files", "--cached", rel],
                capture_output=True, text=True
            )
            if result.returncode == 0 and result.stdout.strip():
                return True
        except (ValueError, Exception):
            pass
    return False


# --- Core logic ---

def get_staged_docs():
    """Get list of staged .md files from git index."""
    result = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACM"],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"error: git diff failed: {result.stderr.strip()}", file=sys.stderr)
        sys.exit(1)
    return [f for f in result.stdout.strip().split("\n") if f.endswith(".md") and f]


def get_all_docs():
    """Get all .md files under docs/."""
    docs_dir = Path("docs")
    if not docs_dir.exists():
        return []
    return [str(p) for p in docs_dir.rglob("*.md")]


def is_exempt(filepath: str) -> bool:
    """Check if a file is exempt from authority checks."""
    if Path(filepath).name in EXEMPT_FILENAMES:
        return True
    for prefix in EXEMPT_PREFIXES:
        if filepath.startswith(prefix):
            return True
    return False


def is_blocked(filepath: str) -> bool:
    """Check if a file should never be committed."""
    for prefix in BLOCKED_PREFIXES:
        if filepath.startswith(prefix):
            return True
    return False


def read_file_content(filepath: str, staged: bool = False) -> str:
    """Read file content, from git index if staged."""
    if staged:
        result = subprocess.run(
            ["git", "show", f":{filepath}"],
            capture_output=True, text=True
        )
        if result.returncode != 0:
            return ""
        return result.stdout
    else:
        try:
            return Path(filepath).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            return ""


def extract_authority(content: str) -> tuple[str | None, str | None]:
    """Extract authority value from YAML front-matter.

    Returns (authority_value, error_message).
    """
    match = FRONTMATTER_RE.search(content)
    if not match:
        return None, "missing YAML front-matter (expected --- delimited block at start of file)"

    frontmatter = match.group(1)
    auth_match = AUTHORITY_RE.search(frontmatter)
    if not auth_match:
        return None, "YAML front-matter present but missing `authority:` field"

    value = auth_match.group(1).strip().strip('"').strip("'").lower()
    return value, None


def expected_authority(filepath: str) -> str | None:
    """Return the expected authority value for a filepath, or None if no constraint."""
    for prefix, authority in PATH_AUTHORITY_MAP.items():
        if filepath.startswith(prefix):
            return authority
    return None


def lint_file(filepath: str, staged: bool = False) -> list[str]:
    """Lint a single file. Returns list of error messages."""
    errors = []

    # Check blocked paths first
    if is_blocked(filepath):
        errors.append(f"BLOCKED: {filepath} — ephemeral docs must not be committed (gitignored by policy)")
        return errors

    # Skip non-docs files
    if not filepath.startswith("docs/"):
        return errors

    # Skip exempt files
    if is_exempt(filepath):
        return errors

    content = read_file_content(filepath, staged=staged)
    if not content:
        return errors

    authority, error = extract_authority(content)

    if error:
        errors.append(f"{filepath}: {error}")
        return errors

    # Validate authority is a known value
    if authority not in VALID_AUTHORITIES:
        errors.append(
            f"{filepath}: unknown authority '{authority}' "
            f"(valid: {', '.join(sorted(VALID_AUTHORITIES))})"
        )
        return errors

    # Validate authority matches path convention
    expected = expected_authority(filepath)
    if expected and authority != expected:
        errors.append(
            f"{filepath}: authority is '{authority}' but path requires '{expected}' "
            f"(see docs/policy/doc_authority_policy.md)"
        )

    # Strip front-matter for body checks
    body = FRONTMATTER_RE.sub("", content).strip()

    # Authority-specific banned terms
    banned = AUTHORITY_BANNED_TERMS.get(authority, [])
    for pattern, reason in banned:
        if pattern.search(body):
            errors.append(f"{filepath}: {reason}")

    # Link hygiene: resolve relative links and check target existence
    for link_match in LINK_RE.finditer(body):
        target = link_match.group(2)
        if target.startswith(("http://", "https://", "#", "mailto:")):
            continue
        if not check_link_exists(filepath, target, staged=staged):
            errors.append(f"{filepath}: broken link '{target}' — target does not exist")

    return errors


def main():
    parser = argparse.ArgumentParser(description="CAWS Document Authority Linter")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--staged", action="store_true", help="Lint staged files only (for pre-commit)")
    group.add_argument("--all", action="store_true", help="Lint all docs")
    parser.add_argument("files", nargs="*", help="Specific files to lint")
    args = parser.parse_args()

    if args.staged:
        # If explicit files given with --staged, use those; otherwise discover from index
        files = args.files if args.files else get_staged_docs()
    elif args.all:
        files = get_all_docs()
    elif args.files:
        files = args.files
    else:
        parser.print_help()
        sys.exit(0)

    if not files:
        sys.exit(0)

    all_errors = []
    for filepath in files:
        all_errors.extend(lint_file(filepath, staged=args.staged))

    if all_errors:
        print("Doc authority lint errors:", file=sys.stderr)
        for error in all_errors:
            print(f"  {error}", file=sys.stderr)
        print(f"\n{len(all_errors)} error(s) found. See docs/policy/doc_authority_policy.md.", file=sys.stderr)
        sys.exit(1)

    sys.exit(0)


if __name__ == "__main__":
    main()
