#!/bin/bash
# Session Logger for Claude Code → ChatGPT Context Transfer
#
# On Stop/PreCompact: reads the full transcript from ~/.claude/ and generates:
#   session.md         — lightweight index (header + turn list + exploration + audit)
#   turn-001.md        — per-turn markdown (user message + reasoning + key tool output)
#   turn-001.json      — per-turn structured data (reasoning + tools + edits + results)
#
# Output: ./tmp/<session-id>/
#
# Wired into: SessionStart (metadata), Stop (generate), PreCompact (safety net)

set -euo pipefail

INPUT=$(cat)

# --- Parse common fields ---
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
HOOK_EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // "unknown"')
CWD=$(echo "$INPUT" | jq -r '.cwd // "."')
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // ""')
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# --- Log directory ---
LOG_DIR="${CWD}/tmp/${SESSION_ID}"
mkdir -p "$LOG_DIR"

SESSION_MD="$LOG_DIR/session.md"
META_FILE="$LOG_DIR/.meta.json"

# ============================================================
# Helper: resolve transcript path
# ============================================================
resolve_transcript() {
  if [ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ]; then
    echo "$TRANSCRIPT_PATH"
    return
  fi
  local slug
  slug=$(echo "$CWD" | sed 's|/|-|g; s|^-||')
  local candidate="$HOME/.claude/projects/${slug}/${SESSION_ID}.jsonl"
  if [ -f "$candidate" ]; then
    echo "$candidate"
    return
  fi
  candidate="$HOME/.claude/projects/-${slug}/${SESSION_ID}.jsonl"
  if [ -f "$candidate" ]; then
    echo "$candidate"
    return
  fi
  echo ""
}

# ============================================================
# Helper: make path relative to project
# ============================================================
rel_path() {
  echo "$1" | sed "s|${CWD}/||"
}

# ============================================================
# Generate per-turn files + session.md index from transcript
# ============================================================
generate_session_output() {
  local transcript="$1"
  local branch head_sha dirty_count
  branch=$(cd "$CWD" 2>/dev/null && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
  head_sha=$(cd "$CWD" 2>/dev/null && git rev-parse --short HEAD 2>/dev/null || echo "unknown")
  dirty_count=$(cd "$CWD" 2>/dev/null && git status --porcelain 2>/dev/null | wc -l | tr -d ' ' || echo "0")

  # --- Read metadata if available ---
  local started_at model start_sha
  if [ -f "$META_FILE" ]; then
    started_at=$(jq -r '.local_time // "unknown"' "$META_FILE")
    model=$(jq -r '.model // "unknown"' "$META_FILE")
    start_sha=$(jq -r '.head_sha // ""' "$META_FILE")
  else
    started_at="(resumed session)"
    model="unknown"
    start_sha=""
  fi

  if [ -z "$transcript" ] || [ ! -f "$transcript" ]; then
    cat > "$SESSION_MD" << MDEOF
# Session Log: $(basename "$CWD")

| Field | Value |
|-------|-------|
| Session ID | \`${SESSION_ID}\` |
| Started | ${started_at} |
| Model | ${model} |
| Branch | \`${branch}\` @ \`${head_sha}\` |

---

_No transcript found. Narrative extraction unavailable._
MDEOF
    return
  fi

  # --- Generate per-turn files via python ---
  # Streams transcript through jq (extract events) then python (split into turns)
  jq -c '
    if .type == "user" then
      if (.message.content | type) == "string" then
        {ev: "user_text", text: .message.content}
      elif (.message.content | type) == "array" then
        # Extract tool_result content (especially errors and test output)
        {ev: "tool_results", results: [.message.content[]? | select(.type == "tool_result") | {id: .tool_use_id, content: ((.content // "") | tostring | .[:2000]), is_error: (.is_error // false)}]}
      else
        empty
      end
    elif .type == "assistant" then
      {ev: "assistant",
       texts: [.message.content[]? | select(.type == "text") | .text],
       tools: [.message.content[]? | select(.type == "tool_use") | {
         name, id,
         file: (.input.file_path // null),
         command: (.input.command // null),
         description: (.input.description // null),
         pattern: (.input.pattern // null)
       }]}
    else
      empty
    end
  ' "$transcript" 2>/dev/null | python3 - "$LOG_DIR" "$CWD" "$SESSION_ID" "$started_at" "$model" "$branch" "$head_sha" "$dirty_count" "$start_sha" << 'PYEOF'
import json, sys, os, hashlib

log_dir = sys.argv[1]
cwd = sys.argv[2]
session_id = sys.argv[3]
started_at = sys.argv[4]
model = sys.argv[5]
branch = sys.argv[6]
head_sha = sys.argv[7]
dirty_count = sys.argv[8]
start_sha = sys.argv[9]

def rel(path):
    if path and path.startswith(cwd + "/"):
        return path[len(cwd) + 1:]
    return path or ""

# ---- Accumulate turns ----
turns = []
current = {
    "user": None,
    "reasoning": [],
    "tools": [],
    "tool_results": {},  # tool_use_id -> result content
    "edits": [],
    "reads": [],
    "searches": [],
    "commands": [],
}

def new_turn(user_text):
    return {
        "user": user_text[:1000] if user_text else None,
        "reasoning": [],
        "tools": [],
        "tool_results": {},
        "edits": [],
        "reads": [],
        "searches": [],
        "commands": [],
    }

NOISE_PREFIXES = ("<local-command", "<command-name", "<local-command-stdout",
                  "<local-command-caveat", "This session is being continued")

for line in sys.stdin:
    try:
        entry = json.loads(line)
    except json.JSONDecodeError:
        continue

    ev = entry.get("ev")

    if ev == "user_text":
        text = entry["text"]
        if any(text.startswith(p) for p in NOISE_PREFIXES):
            continue
        if not text.strip():
            continue
        # Save current turn, start new one
        if current["user"] or current["reasoning"]:
            turns.append(current)
        current = new_turn(text)

    elif ev == "tool_results":
        for r in entry.get("results", []):
            rid = r.get("id", "")
            content = r.get("content", "")
            is_error = r.get("is_error", False)
            if rid:
                current["tool_results"][rid] = {"content": content, "is_error": is_error}

    elif ev == "assistant":
        for t in entry.get("texts", []):
            if len(t) > 80:
                current["reasoning"].append(t)
        for tool in entry.get("tools", []):
            name = tool.get("name", "")
            tid = tool.get("id", "")
            current["tools"].append({"name": name, "id": tid})

            if name in ("Write", "Edit"):
                f = rel(tool.get("file"))
                if f and f not in current["edits"]:
                    current["edits"].append(f)
            elif name == "Read":
                f = rel(tool.get("file"))
                if f and f not in current["reads"]:
                    current["reads"].append(f)
            elif name in ("Grep", "Glob"):
                pat = tool.get("pattern", "")
                if pat:
                    current["searches"].append(pat)
            elif name == "Bash":
                cmd = tool.get("command", "")
                desc = tool.get("description", "")
                if cmd:
                    current["commands"].append({"cmd": cmd[:200], "desc": desc or ""})

# Flush last turn
if current["user"] or current["reasoning"]:
    turns.append(current)

# ---- Write per-turn files ----
turn_index = []  # for session.md index

for i, turn in enumerate(turns):
    num = i + 1
    padded = f"{num:03d}"

    # --- Build per-turn markdown ---
    md_lines = []
    md_lines.append(f"# Turn {num}")
    md_lines.append("")

    if turn["user"]:
        md_lines.append(f"> **User:** {turn['user']}")
        md_lines.append("")

    # Reasoning with interleaved key tool results
    tool_idx = 0
    for text in turn["reasoning"]:
        # Truncate very long messages
        if len(text) > 3000:
            md_lines.append(text[:3000])
            md_lines.append("")
            md_lines.append("_(message truncated at 3000 chars)_")
        else:
            md_lines.append(text)
        md_lines.append("")
        md_lines.append("---")
        md_lines.append("")

    # Key tool results: errors, test output, refusals
    notable_results = []
    for tool in turn["tools"]:
        tid = tool.get("id", "")
        result = turn["tool_results"].get(tid, {})
        content = result.get("content", "")
        is_error = result.get("is_error", False)
        name = tool.get("name", "")

        if not content:
            continue

        # Include: errors, test results, short meaningful output
        if is_error:
            notable_results.append((name, content, "error"))
        elif name == "Bash" and any(kw in content.lower() for kw in
                ["error", "fail", "refusal", "mismatch", "passed", "assert", "traceback", "exception"]):
            notable_results.append((name, content, "output"))
        elif name == "Bash" and any(kw in content for kw in
                ["pytest", "PASSED", "FAILED", "warnings summary"]):
            notable_results.append((name, content, "test"))

    if notable_results:
        md_lines.append("### Key Tool Output")
        md_lines.append("")
        for name, content, kind in notable_results:
            # Truncate long output
            if len(content) > 1500:
                content = content[:1500] + "\n...(truncated)"
            md_lines.append(f"**{name}** ({kind}):")
            md_lines.append("```")
            md_lines.append(content)
            md_lines.append("```")
            md_lines.append("")

    # Summary footer
    if turn["edits"] or turn["reads"] or turn["commands"]:
        md_lines.append("### Activity")
        md_lines.append("")
        for f in turn["edits"]:
            md_lines.append(f"- EDIT `{f}`")
        for f in turn["reads"]:
            md_lines.append(f"- READ `{f}`")
        for cmd in turn["commands"][:10]:
            short = cmd["cmd"][:120]
            if cmd["desc"]:
                md_lines.append(f"- BASH `{short}` — {cmd['desc']}")
            else:
                md_lines.append(f"- BASH `{short}`")
        md_lines.append("")

    # Write turn markdown
    turn_md_path = os.path.join(log_dir, f"turn-{padded}.md")
    with open(turn_md_path, "w") as f:
        f.write("\n".join(md_lines))

    # --- Build per-turn JSON ---
    tool_summary = {}
    for tool in turn["tools"]:
        name = tool.get("name", "")
        tool_summary[name] = tool_summary.get(name, 0) + 1

    turn_json = {
        "turn": num,
        "user": turn["user"],
        "reasoning": turn["reasoning"],
        "tool_summary": tool_summary,
        "files_edited": turn["edits"],
        "files_read": turn["reads"],
        "searches": turn["searches"],
        "commands": [c["cmd"] for c in turn["commands"]],
        "notable_results": [
            {"tool": name, "kind": kind, "content": content[:2000]}
            for name, content, kind in notable_results
        ] if notable_results else [],
    }

    turn_json_path = os.path.join(log_dir, f"turn-{padded}.json")
    with open(turn_json_path, "w") as f:
        json.dump(turn_json, f, indent=2)

    # Index entry
    user_preview = (turn["user"] or "(no user message)")[:120]
    turn_index.append({
        "num": num,
        "padded": padded,
        "user_preview": user_preview,
        "reasoning_count": len(turn["reasoning"]),
        "tool_count": sum(tool_summary.values()),
        "edits": turn["edits"],
    })

# ---- Write session.md index ----
with open(os.path.join(log_dir, "session.md"), "w") as f:
    f.write(f"# Session Log: {os.path.basename(cwd)}\n\n")
    f.write("| Field | Value |\n")
    f.write("|-------|-------|\n")
    f.write(f"| Session ID | `{session_id}` |\n")
    f.write(f"| Started | {started_at} |\n")
    f.write(f"| Model | {model} |\n")
    f.write(f"| Branch | `{branch}` @ `{head_sha}` |\n")
    f.write(f"| Turns | {len(turn_index)} |\n")
    f.write("\n---\n\n")

    f.write("## Turns\n\n")
    for t in turn_index:
        edits_str = ", ".join(f"`{e}`" for e in t["edits"][:3])
        if len(t["edits"]) > 3:
            edits_str += f" +{len(t['edits'])-3} more"
        summary = f"{t['reasoning_count']} msgs, {t['tool_count']} tools"
        if edits_str:
            summary += f" | {edits_str}"
        f.write(f"- **[Turn {t['num']}](turn-{t['padded']}.md)** — {t['user_preview']}\n")
        f.write(f"  _{summary}_\n")

    f.write("\n---\n\n")

    # Exploration summary (deduplicated across all turns)
    all_reads = []
    all_searches = []
    all_edits = []
    all_commands = []
    for turn in turns:
        all_reads.extend(turn["reads"])
        all_searches.extend(turn["searches"])
        all_edits.extend(turn["edits"])
        all_commands.extend(turn["commands"])

    f.write("## Exploration\n")
    f.write("_Files read and searches performed (deduplicated)._\n\n")
    for r in sorted(set(all_reads)):
        f.write(f"- READ `{r}`\n")
    for s in sorted(set(all_searches)):
        f.write(f"- SEARCH `{s}`\n")
    f.write("\n")

    f.write("## Audit\n")
    f.write("_Edits, commands, git activity._\n\n")
    for e in sorted(set(all_edits)):
        f.write(f"- EDIT `{e}`\n")
    for cmd in all_commands:
        short = cmd["cmd"][:120]
        # Only log meaningful commands
        meaningful = any(kw in short for kw in [
            "pytest", "cargo test", "ruff", "mypy", "npm test",
            "git log", "git diff", "git status", "git add", "git commit",
            "git merge", "caws ", "pip install", "make", "cargo build"
        ])
        if meaningful:
            if cmd["desc"]:
                f.write(f"- BASH `{short}` — {cmd['desc']}\n")
            else:
                f.write(f"- BASH `{short}`\n")
    f.write("\n")

    f.write("## Session Snapshot\n\n")
    f.write("| Field | Value |\n")
    f.write("|-------|-------|\n")
    f.write(f"| Branch | `{branch}` @ `{head_sha}` |\n")
    f.write(f"| Dirty files | {dirty_count} |\n")
    f.write(f"| Total turns | {len(turn_index)} |\n")

PYEOF
}

# ============================================================
# EVENT: SessionStart — save metadata
# ============================================================
handle_session_start() {
  local model source branch head_sha dirty_count full_time
  model=$(echo "$INPUT" | jq -r '.model // "unknown"')
  source=$(echo "$INPUT" | jq -r '.source // "unknown"')
  branch=$(cd "$CWD" 2>/dev/null && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
  head_sha=$(cd "$CWD" 2>/dev/null && git rev-parse --short HEAD 2>/dev/null || echo "unknown")
  dirty_count=$(cd "$CWD" 2>/dev/null && git status --porcelain 2>/dev/null | wc -l | tr -d ' ' || echo "0")
  full_time=$(date +"%Y-%m-%d %H:%M:%S %Z")

  jq -cn \
    --arg sid "$SESSION_ID" \
    --arg ts "$TIMESTAMP" \
    --arg lt "$full_time" \
    --arg model "$model" \
    --arg source "$source" \
    --arg branch "$branch" \
    --arg head "$head_sha" \
    --arg dirty "$dirty_count" \
    --arg project "$(basename "$CWD")" \
    --arg transcript "$TRANSCRIPT_PATH" \
    '{session_id: $sid, started_at: $ts, local_time: $lt, model: $model, source: $source, branch: $branch, head_sha: $head, dirty_files: $dirty, project: $project, transcript_path: $transcript}' \
    > "$META_FILE"

  # Generate initial output (may be empty if transcript not ready)
  generate_session_output "$(resolve_transcript)"
}

# ============================================================
# EVENT: Stop — regenerate from transcript
# ============================================================
handle_stop() {
  generate_session_output "$(resolve_transcript)"
}

# ============================================================
# EVENT: PreCompact — safety net before context eviction
# ============================================================
handle_pre_compact() {
  generate_session_output "$(resolve_transcript)"
}

# ============================================================
# DISPATCH
# ============================================================
case "$HOOK_EVENT" in
  SessionStart)   handle_session_start ;;
  Stop)           handle_stop ;;
  PreCompact)     handle_pre_compact ;;
  *)              ;; # Other events: no-op
esac

exit 0
