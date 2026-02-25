#!/usr/bin/env python3
"""
Sterling Native Project Inventory (MOC) Generator

Scans docs, scripts, tools, schemas, and config files to produce a
project-wide Map of Content (MOC) inventory.

Two modes:
  --mode structural   Deterministic, commit-worthy. Extracts metadata from
                      YAML front-matter, docstrings, and header comments.
  --mode augmented    LLM-powered descriptions, cache-backed by git blob sha.
                      Only regenerates entries whose content has changed.

Output:
  docs/MOC/PROJECT_INVENTORY.json  — machine-readable
  docs/MOC/PROJECT_INVENTORY.md    — human-readable, grouped by category

Cache (augmented mode only):
  .cache/inventory/descriptions.json  — keyed by blob_sha + prompt_version

Author: @darianrosebrook
"""

import argparse
import ast
import hashlib
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# --- Constants ---

INVENTORY_VERSION = "1.0.0"
PROMPT_VERSION = "v1"  # bump when prompt changes to invalidate cache

OUTPUT_DIR = Path("docs/MOC")
OUTPUT_JSON = OUTPUT_DIR / "PROJECT_INVENTORY.json"
OUTPUT_MD = OUTPUT_DIR / "PROJECT_INVENTORY.md"
CACHE_DIR = Path(".cache/inventory")
CACHE_FILE = CACHE_DIR / "descriptions.json"

# Regexes
FRONTMATTER_RE = re.compile(r"\A---\s*\n(.*?)\n---", re.DOTALL)
FIELD_RE = re.compile(r"^(\w[\w-]*):\s*(.+)$", re.MULTILINE)
TITLE_RE = re.compile(r"^#\s+(.+)$", re.MULTILINE)
SHELL_HEADER_RE = re.compile(r"^#\s+(.+)$", re.MULTILINE)
YAML_TITLE_RE = re.compile(r"^title:\s*[\"']?(.+?)[\"']?\s*$", re.MULTILINE)

# Sterling Native project context for LLM prompts
PROJECT_CONTEXT = """
Sterling Native is a clean-slate neurosymbolic reasoning substrate (v2).
Key architectural principles:

1. COMPILATION BOUNDARY: compile(payload, schema_descriptor, registry_snapshot,
   policy_snapshot) -> ByteState is the only entry point.
2. ByteTrace is the canonical persisted trace (ADR 0002); StateGraph is derived.
3. Neural is advisory only (ADR 0003) -- no mutation, no operator creation.
4. Operator taxonomy: S/M/P/K/C (Seek/Memorize/Perceive/Knowledge/Control).
5. v1 is a test oracle, not a dependency (ADR 0005).
6. DEV/CERTIFIED are the only governance modes.
7. Document authority regime: canonical, policy, adr, architecture, reference, ephemeral.
"""

# --- Scan configuration ---

# (glob_root, extensions, content_type, category_label)
SCAN_TARGETS: List[Tuple[str, List[str], str, str]] = [
    ("docs/canonical", [".md"], "documentation", "Canonical Contracts"),
    ("docs/policy", [".md"], "documentation", "Policy"),
    ("docs/adr", [".md"], "documentation", "Architecture Decision Records"),
    ("docs/architecture", [".md"], "documentation", "Architecture"),
    ("docs/specs", [".md"], "documentation", "Specs"),
    ("docs/templates", [".md"], "documentation", "Templates"),
    ("docs/reference/v1", [".md"], "documentation", "Reference (v1)"),
    ("tools", [".py"], "code", "Tools"),
    ("scripts", [".sh"], "script", "Scripts"),
    (".githooks", [".sh", ""], "script", "Git Hooks"),
    ("schemas", [".json"], "schema", "Schemas"),
    (".caws/specs", [".yaml"], "config", "CAWS Feature Specs"),
    ("benchmarks", [".md", ".json"], "documentation", "Benchmarks"),
]

SKIP_DIRS = {".git", "node_modules", "tmp", ".claude", "__pycache__", "docs/_index", "docs/MOC"}
SKIP_FILENAMES = {".DS_Store"}


# --- Metadata extraction ---

def git_blob_sha(filepath: str) -> str:
    """Get the git blob sha for a file (working tree version)."""
    try:
        result = subprocess.run(
            ["git", "hash-object", filepath],
            capture_output=True, text=True
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except Exception:
        pass
    # Fallback: content hash
    try:
        content = Path(filepath).read_bytes()
        return hashlib.sha256(content).hexdigest()[:16]
    except Exception:
        return "unknown"


def extract_frontmatter(content: str) -> Dict[str, str]:
    """Extract YAML front-matter fields as a flat dict."""
    match = FRONTMATTER_RE.search(content)
    if not match:
        return {}
    fields = {}
    for m in FIELD_RE.finditer(match.group(1)):
        fields[m.group(1).lower()] = m.group(2).strip().strip('"').strip("'")
    return fields


def extract_md_title(content: str) -> str:
    """Extract first H1 heading from markdown, skipping front-matter."""
    stripped = FRONTMATTER_RE.sub("", content).strip()
    match = TITLE_RE.search(stripped)
    return match.group(1).strip() if match else ""


def extract_python_meta(content: str) -> Tuple[str, str]:
    """Extract module docstring and title from Python file.

    Returns (title, summary).
    """
    try:
        tree = ast.parse(content)
        docstring = ast.get_docstring(tree) or ""
    except SyntaxError:
        docstring = ""

    if docstring:
        lines = docstring.strip().split("\n")
        title = lines[0].strip().rstrip(".")
        summary = docstring.strip()
    else:
        title = ""
        summary = ""
    return title, summary


def extract_shell_meta(content: str) -> Tuple[str, str]:
    """Extract title and summary from shell script header comments.

    Returns (title, summary).
    """
    lines = content.split("\n")
    comment_lines = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("#!"):
            continue
        if stripped.startswith("#"):
            comment_lines.append(stripped.lstrip("# ").strip())
        elif stripped == "":
            continue
        else:
            break

    if comment_lines:
        title = comment_lines[0]
        summary = "\n".join(comment_lines)
    else:
        title = ""
        summary = ""
    return title, summary


def extract_yaml_meta(content: str) -> Tuple[str, str]:
    """Extract title from YAML file."""
    match = YAML_TITLE_RE.search(content)
    title = match.group(1) if match else ""
    return title, ""


def file_metadata(filepath: str) -> Dict:
    """Get basic file metadata."""
    stat = Path(filepath).stat()
    content = Path(filepath).read_text(encoding="utf-8", errors="replace")
    return {
        "lines": len(content.splitlines()),
        "size_bytes": stat.st_size,
        "modified": datetime.fromtimestamp(stat.st_mtime).isoformat(),
    }


# --- Inventory building ---

def scan_files() -> List[Tuple[str, str, str]]:
    """Scan project for inventoriable files.

    Returns list of (filepath, content_type, category).
    """
    results = []
    for root_dir, extensions, content_type, category in SCAN_TARGETS:
        root_path = Path(root_dir)
        if not root_path.exists():
            continue
        for path in sorted(root_path.rglob("*")):
            if not path.is_file():
                continue
            if path.name in SKIP_FILENAMES:
                continue
            # Check skip dirs
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            # Check extension
            if extensions:
                if path.suffix not in extensions and "" not in extensions:
                    continue
                # For extensionless files (git hooks), check they have no suffix
                if "" in extensions and path.suffix and path.suffix not in extensions:
                    continue
            results.append((str(path), content_type, category))
    return results


def build_entry(
    filepath: str,
    content_type: str,
    category: str,
    include_v1: bool = False,
) -> Optional[Dict]:
    """Build a single inventory entry."""
    # Skip v1 reference unless requested
    if not include_v1 and filepath.startswith("docs/reference/v1/"):
        return None

    try:
        content = Path(filepath).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None

    meta = file_metadata(filepath)
    blob_sha = git_blob_sha(filepath)

    # Extract metadata based on content type
    authority = None
    title = ""
    summary = ""

    if content_type == "documentation":
        fm = extract_frontmatter(content)
        authority = fm.get("authority")
        title = extract_md_title(content)
        # Use front-matter scope or status as summary hint
        summary = fm.get("scope", fm.get("status", ""))
    elif content_type == "code" and filepath.endswith(".py"):
        title, summary = extract_python_meta(content)
    elif content_type == "script":
        title, summary = extract_shell_meta(content)
    elif content_type == "config" and filepath.endswith(".yaml"):
        title, summary = extract_yaml_meta(content)
    elif content_type == "schema":
        title = Path(filepath).stem
        summary = ""

    # Fallback title
    if not title:
        title = Path(filepath).name

    return {
        "path": filepath,
        "content_type": content_type,
        "category": category,
        "authority": authority,
        "title": title,
        "description": summary if summary else None,
        "blob_sha": blob_sha,
        "metadata": meta,
    }


def build_structural_inventory(include_v1: bool = False) -> Dict:
    """Build the full structural inventory (no LLM)."""
    files = scan_files()
    entries = []
    for filepath, content_type, category in files:
        entry = build_entry(filepath, content_type, category, include_v1=include_v1)
        if entry:
            entries.append(entry)

    return {
        "version": INVENTORY_VERSION,
        "generated": datetime.now(timezone.utc).isoformat() + "Z",
        "project": "sterling-native",
        "entry_count": len(entries),
        "entries": entries,
    }


# --- Augmented mode (LLM descriptions) ---

def load_cache() -> Dict:
    """Load the description cache."""
    if CACHE_FILE.exists():
        try:
            return json.loads(CACHE_FILE.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            pass
    return {}


def save_cache(cache: Dict) -> None:
    """Save the description cache."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    CACHE_FILE.write_text(
        json.dumps(cache, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def cache_key(blob_sha: str, model_id: str) -> str:
    """Build a cache key from blob sha, prompt version, and model."""
    return f"{blob_sha}:{PROMPT_VERSION}:{model_id}"


def augment_inventory(inventory: Dict, model: str = "olmo-3:latest") -> Dict:
    """Add LLM-generated descriptions to inventory entries.

    Only regenerates entries whose content has changed (by blob sha).
    """
    # Lazy import to avoid requiring LLM deps for structural mode
    sys.path.insert(0, str(Path(__file__).parent))
    from llm_client import build_moc_description_prompt, generate_description

    cache = load_cache()
    updated = 0
    cached = 0
    failed = 0

    for entry in inventory["entries"]:
        key = cache_key(entry["blob_sha"], model)

        # Check cache first
        if key in cache:
            entry["description"] = cache[key]
            cached += 1
            continue

        # Build prompt
        content_type = entry["content_type"]
        preview = ""
        try:
            raw = Path(entry["path"]).read_text(encoding="utf-8", errors="replace")
            preview = raw[:2500]
        except OSError:
            pass

        prompt, system_prompt = build_moc_description_prompt(
            content_type,
            path=entry["path"],
            title=entry.get("title"),
            category=entry.get("category"),
            summary=entry.get("description"),
            preview=preview,
            project_context=PROJECT_CONTEXT,
            max_chars=500,
        )

        result = generate_description(
            prompt, system_prompt,
            content_type=content_type,
            model=model,
            max_chars=500,
        )

        if result:
            entry["description"] = result
            cache[key] = result
            updated += 1
            print(f"  + {entry['path']}")
        else:
            failed += 1
            print(f"  - {entry['path']} (no LLM response, keeping fallback)")

    save_cache(cache)
    print(f"\nAugmented: {updated} new, {cached} cached, {failed} failed")
    return inventory


# --- Output generation ---

def write_json(inventory: Dict) -> None:
    """Write JSON inventory."""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    OUTPUT_JSON.write_text(
        json.dumps(inventory, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def write_markdown(inventory: Dict) -> None:
    """Write human-readable markdown inventory."""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    # Group entries by category
    by_category: Dict[str, List[Dict]] = {}
    for entry in inventory["entries"]:
        cat = entry.get("category", "Other")
        by_category.setdefault(cat, []).append(entry)

    lines = [
        "# Sterling Native Project Inventory",
        "",
        f"**Generated**: {now}",
        f"**Entries**: {inventory['entry_count']}",
        f"**Version**: {inventory['version']}",
        "",
    ]

    # Summary table
    lines.append("## Summary")
    lines.append("")
    lines.append("| Category | Count |")
    lines.append("|----------|-------|")
    for cat in sorted(by_category.keys()):
        lines.append(f"| {cat} | {len(by_category[cat])} |")
    lines.append("")

    # Category sections
    for cat in sorted(by_category.keys()):
        entries = by_category[cat]
        lines.append(f"## {cat}")
        lines.append("")
        lines.append("| Path | Title | Lines | Authority |")
        lines.append("|------|-------|-------|-----------|")
        for entry in sorted(entries, key=lambda e: e["path"]):
            path = entry["path"]
            title = entry.get("title", "")[:60]
            line_count = entry.get("metadata", {}).get("lines", "")
            authority = entry.get("authority", "")
            lines.append(f"| `{path}` | {title} | {line_count} | {authority} |")
        lines.append("")

        # Descriptions (if augmented)
        has_descriptions = any(e.get("description") for e in entries)
        if has_descriptions:
            lines.append("### Descriptions")
            lines.append("")
            for entry in sorted(entries, key=lambda e: e["path"]):
                desc = entry.get("description", "")
                if desc:
                    lines.append(f"- **`{entry['path']}`**: {desc}")
            lines.append("")

    OUTPUT_MD.write_text("\n".join(lines) + "\n", encoding="utf-8")


# --- CLI ---

def main():
    parser = argparse.ArgumentParser(
        description="Sterling Native Project Inventory (MOC) Generator"
    )
    parser.add_argument(
        "--mode", choices=["structural", "augmented"], required=True,
        help="structural = deterministic (commit-worthy); augmented = LLM descriptions (cache-backed)"
    )
    parser.add_argument(
        "--include-v1-reference", action="store_true",
        help="Include docs/reference/v1/ in the inventory"
    )
    parser.add_argument(
        "--model", default="olmo-3:latest",
        help="LLM model for augmented mode (default: olmo-3:latest)"
    )
    args = parser.parse_args()

    print(f"Scanning project files...")
    inventory = build_structural_inventory(include_v1=args.include_v1_reference)
    print(f"Found {inventory['entry_count']} entries")

    if args.mode == "augmented":
        print(f"\nRunning LLM augmentation (model: {args.model})...")
        inventory = augment_inventory(inventory, model=args.model)

    write_json(inventory)
    write_markdown(inventory)
    print(f"\nOutput:")
    print(f"  {OUTPUT_JSON}")
    print(f"  {OUTPUT_MD}")


if __name__ == "__main__":
    main()
