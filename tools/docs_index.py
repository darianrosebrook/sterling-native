#!/usr/bin/env python3
"""
CAWS Structural Docs Indexer

Builds a deterministic JSON index of all docs/ files with their metadata:
- authority level (from YAML front-matter)
- path, title, status
- cross-reference links

Usage:
    # Build structural index (deterministic, no LLM)
    python tools/docs_index.py --mode structural

    # Check if index is stale (for pre-commit, reads working tree)
    python tools/docs_index.py --check

    # Check if index is stale against staged bytes (for pre-commit hook)
    python tools/docs_index.py --check --staged

    # Build with LLM-augmented summaries (optional, requires API key)
    python tools/docs_index.py --mode augmented

Output:
    docs/_index/docs_index.v1.json  (tracked in git)

Exit codes:
    0 = index is up to date (or was rebuilt successfully)
    1 = index is stale (--check mode)
"""

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

INDEX_DIR = Path("docs/_index")
INDEX_FILE = INDEX_DIR / "docs_index.v1.json"
INDEX_VERSION = "1.0.0"

FRONTMATTER_RE = re.compile(r"\A---\s*\n(.*?)\n---", re.DOTALL)
FIELD_RE = re.compile(r"^(\w[\w-]*):\s*(.+)$", re.MULTILINE)
TITLE_RE = re.compile(r"^#\s+(.+)$", re.MULTILINE)
LINK_RE = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")

SKIP_PREFIXES = (
    "docs/ephemeral/",
    "docs/_index/",
    "docs/MOC/",
)


def extract_frontmatter(content: str) -> dict[str, str]:
    """Extract YAML front-matter fields as a flat dict."""
    match = FRONTMATTER_RE.search(content)
    if not match:
        return {}
    fields = {}
    for m in FIELD_RE.finditer(match.group(1)):
        fields[m.group(1).lower()] = m.group(2).strip().strip('"').strip("'")
    return fields


def extract_title(content: str) -> str:
    """Extract first H1 heading from markdown."""
    stripped = FRONTMATTER_RE.sub("", content).strip()
    match = TITLE_RE.search(stripped)
    return match.group(1).strip() if match else ""


def extract_links(content: str, source_path: str) -> list[dict]:
    """Extract markdown links, resolving relative paths."""
    links = []
    source_dir = str(Path(source_path).parent)
    for match in LINK_RE.finditer(content):
        target = match.group(2)
        if target.startswith(("http://", "https://", "#", "mailto:")):
            continue
        resolved = str(Path(source_dir) / target)
        try:
            resolved = str(Path(resolved))
        except (ValueError, OSError):
            pass
        links.append({"text": match.group(1), "target": resolved})
    return links


def content_hash(filepath: str, content: str) -> str:
    """Compute a stable content hash using git blob sha with sha256 fallback."""
    try:
        result = subprocess.run(
            ["git", "hash-object", filepath],
            capture_output=True, text=True
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()[:16]
    except Exception:
        pass
    return hashlib.sha256(content.encode("utf-8")).hexdigest()[:16]


def content_hash_from_bytes(content: str) -> str:
    """Compute content hash from bytes (for staged content without a file path)."""
    try:
        result = subprocess.run(
            ["git", "hash-object", "--stdin"],
            input=content, capture_output=True, text=True
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()[:16]
    except Exception:
        pass
    return hashlib.sha256(content.encode("utf-8")).hexdigest()[:16]


def read_staged_content(filepath: str) -> str | None:
    """Read file content from the git index (staged bytes)."""
    result = subprocess.run(
        ["git", "show", f":{filepath}"],
        capture_output=True, text=True
    )
    if result.returncode == 0:
        return result.stdout
    return None


def index_file(filepath: str, content: str | None = None) -> dict:
    """Build index entry for a single doc file.

    If content is provided, uses that (for staged mode).
    Otherwise reads from the working tree.
    """
    if content is None:
        content = Path(filepath).read_text(encoding="utf-8")

    fm = extract_frontmatter(content)
    title = extract_title(content)
    links = extract_links(content, filepath)

    # Use content_hash_from_bytes when we have raw content (staged mode)
    chash = content_hash_from_bytes(content)

    return {
        "path": filepath,
        "title": title,
        "authority": fm.get("authority", None),
        "status": fm.get("status", None),
        "scope": fm.get("scope", None),
        "content_hash": chash,
        "links": links,
    }


def get_tracked_docs() -> list[str]:
    """Get all tracked .md files under docs/ from git."""
    result = subprocess.run(
        ["git", "ls-files", "docs/"],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        return []
    return [f for f in result.stdout.strip().split("\n") if f.endswith(".md") and f]


def build_index(staged: bool = False) -> dict:
    """Build the full structural index.

    If staged=True, reads content from git index (staged bytes).
    Otherwise reads from the working tree.
    """
    if staged:
        # Use git ls-files to get the list of tracked docs, then read staged content
        files = get_tracked_docs()
    else:
        docs_dir = Path("docs")
        if not docs_dir.exists():
            return {"version": INDEX_VERSION, "entries": []}
        files = [str(p) for p in sorted(docs_dir.rglob("*.md"))]

    entries = []
    for filepath in sorted(files):
        if any(filepath.startswith(p) for p in SKIP_PREFIXES):
            continue
        try:
            if staged:
                content = read_staged_content(filepath)
                if content is None:
                    continue
                entry = index_file(filepath, content=content)
            else:
                entry = index_file(filepath)
            entries.append(entry)
        except (OSError, UnicodeDecodeError) as e:
            print(f"warning: skipping {filepath}: {e}", file=sys.stderr)

    aggregate = hashlib.sha256(
        json.dumps(entries, sort_keys=True).encode()
    ).hexdigest()[:16]

    return {
        "version": INDEX_VERSION,
        "aggregate_hash": aggregate,
        "entry_count": len(entries),
        "entries": entries,
    }


def write_index(index: dict) -> None:
    """Write index to disk."""
    INDEX_DIR.mkdir(parents=True, exist_ok=True)
    INDEX_FILE.write_text(
        json.dumps(index, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def read_existing_index() -> dict | None:
    """Read existing index from disk, if any."""
    if not INDEX_FILE.exists():
        return None
    try:
        return json.loads(INDEX_FILE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def read_staged_index() -> dict | None:
    """Read existing index from the git index (staged version)."""
    result = subprocess.run(
        ["git", "show", f":{INDEX_FILE}"],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        return None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return None


def check_staleness(staged: bool = False) -> bool:
    """Check if the index is stale. Returns True if stale.

    If staged=True, compares the staged index file against staged doc content.
    Otherwise compares the on-disk index against working tree doc content.
    """
    if staged:
        existing = read_staged_index()
    else:
        existing = read_existing_index()

    if existing is None:
        print("docs index does not exist — run: python tools/docs_index.py --mode structural", file=sys.stderr)
        return True

    fresh = build_index(staged=staged)
    if existing.get("aggregate_hash") != fresh.get("aggregate_hash"):
        print(
            "docs index is stale — run: python tools/docs_index.py --mode structural",
            file=sys.stderr,
        )
        return True

    return False


def main():
    parser = argparse.ArgumentParser(description="CAWS Structural Docs Indexer")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--mode", choices=["structural", "augmented"],
        help="Build mode: structural (deterministic) or augmented (LLM summaries)"
    )
    group.add_argument(
        "--check", action="store_true",
        help="Check if index is stale (exit 1 if stale)"
    )
    parser.add_argument(
        "--staged", action="store_true",
        help="With --check: compare against staged bytes (git index) instead of working tree"
    )
    args = parser.parse_args()

    if args.check:
        sys.exit(1 if check_staleness(staged=args.staged) else 0)

    if args.staged and not args.check:
        print("--staged is only valid with --check", file=sys.stderr)
        sys.exit(1)

    if args.mode == "augmented":
        print("LLM-augmented mode is not yet implemented. Use --mode structural.", file=sys.stderr)
        sys.exit(1)

    # Structural mode: build and write (always from working tree)
    index = build_index(staged=False)
    write_index(index)
    print(f"Indexed {index['entry_count']} docs -> {INDEX_FILE}")


if __name__ == "__main__":
    main()
