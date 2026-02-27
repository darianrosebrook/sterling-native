#!/bin/bash
# Session Logger for Claude Code → ChatGPT Context Transfer
#
# Lightweight during the session (just raw.jsonl + files-touched).
# On Stop: reads the full transcript from ~/.claude/ and generates a clean
# three-section session.md (Narrative / Exploration / Audit).
#
# Output: ./tmp/<session-id>/session.md
#
# Wired into: SessionStart (metadata), Stop (generate session.md), PreCompact (safety net)

set -euo pipefail

INPUT=$(cat)

# --- Parse common fields ---
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
HOOK_EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // "unknown"')
CWD=$(echo "$INPUT" | jq -r '.cwd // "."')
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // ""')
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
LOCAL_TIME=$(date +"%H:%M")

# --- Log directory ---
LOG_DIR="${CWD}/tmp/${SESSION_ID}"
mkdir -p "$LOG_DIR"

SESSION_MD="$LOG_DIR/session.md"
META_FILE="$LOG_DIR/.meta.json"
# Track how many narrative entries we've written (for incremental Stop calls)
NARR_COUNT_FILE="$LOG_DIR/.narr_count"

# ============================================================
# Helper: resolve transcript path
# If hook input has it, use it. Otherwise construct from session_id.
# ============================================================
resolve_transcript() {
  if [ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ]; then
    echo "$TRANSCRIPT_PATH"
    return
  fi
  # Construct from CWD + session_id
  local slug
  slug=$(echo "$CWD" | sed 's|/|-|g; s|^-||')
  local candidate="$HOME/.claude/projects/${slug}/${SESSION_ID}.jsonl"
  if [ -f "$candidate" ]; then
    echo "$candidate"
    return
  fi
  # Try with leading dash (common pattern)
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
# Generate session.md from transcript
# Extracts three streams: Narrative, Exploration, Audit
# ============================================================
generate_session_md() {
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

  # --- Header ---
  cat > "$SESSION_MD" << MDEOF
# Session Log: $(basename "$CWD")

| Field | Value |
|-------|-------|
| Session ID | \`${SESSION_ID}\` |
| Started | ${started_at} |
| Model | ${model} |
| Branch | \`${branch}\` @ \`${head_sha}\` |

---

MDEOF

  if [ -z "$transcript" ] || [ ! -f "$transcript" ]; then
    echo "_No transcript found. Narrative extraction unavailable._" >> "$SESSION_MD"
    return
  fi

  # --- NARRATIVE: Extract assistant text messages ---
  # These are Claude's reasoning, decisions, realizations — the "why"
  {
    echo "## Narrative"
    echo "_Claude's reasoning and decisions, in chronological order._"
    echo ""

    # Use NUL delimiter to preserve multi-line messages as single units
    jq -j '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "text") |
      .text + "\u0000"
    ' "$transcript" 2>/dev/null | while IFS= read -r -d $'\0' msg; do
      # Skip short confirmations
      if [ ${#msg} -gt 80 ]; then
        # Truncate very long messages (>3000 chars) to keep session.md portable
        if [ ${#msg} -gt 3000 ]; then
          echo "${msg:0:3000}"
          echo ""
          echo "_(message truncated at 3000 chars)_"
        else
          echo "$msg"
        fi
        echo ""
        echo "---"
        echo ""
      fi
    done

    echo ""
  } >> "$SESSION_MD"

  # --- EXPLORATION: Extract file reads, searches, agent dispatches ---
  {
    echo "## Exploration"
    echo "_Files read and searches performed._"
    echo ""

    # Read tool calls
    jq -r '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "tool_use") |
      select(.name == "Read") |
      .input.file_path // empty
    ' "$transcript" 2>/dev/null | sort -u | while IFS= read -r fpath; do
      [ -n "$fpath" ] && echo "- READ \`$(rel_path "$fpath")\`"
    done

    # Grep/search calls
    jq -r '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "tool_use") |
      select(.name == "Grep") |
      (.input.pattern // "") + " in " + (.input.path // ".")
    ' "$transcript" 2>/dev/null | sort -u | while IFS= read -r search; do
      [ -n "$search" ] && echo "- SEARCH \`$search\`"
    done

    # Glob calls
    jq -r '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "tool_use") |
      select(.name == "Glob") |
      .input.pattern // empty
    ' "$transcript" 2>/dev/null | sort -u | while IFS= read -r pat; do
      [ -n "$pat" ] && echo "- GLOB \`$pat\`"
    done

    # Task/Agent dispatches (first 120 chars of prompt)
    jq -r '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "tool_use") |
      select(.name == "Task") |
      .input.prompt // empty
    ' "$transcript" 2>/dev/null | while IFS= read -r prompt; do
      if [ -n "$prompt" ]; then
        local short
        short=$(echo "$prompt" | head -1 | cut -c1-120)
        echo "- AGENT: ${short}"
      fi
    done

    echo ""
  } >> "$SESSION_MD"

  # --- AUDIT: Extract edits, test runs, git commands, plan writes ---
  {
    echo "## Audit"
    echo "_Edits, test results, git activity._"
    echo ""

    # Write calls
    jq -r '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "tool_use") |
      select(.name == "Write") |
      .input.file_path // empty
    ' "$transcript" 2>/dev/null | while IFS= read -r fpath; do
      [ -n "$fpath" ] && echo "- WRITE \`$(rel_path "$fpath")\`"
    done

    # Edit calls (with old/new lengths)
    jq -r '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "tool_use") |
      select(.name == "Edit") |
      (.input.file_path // "") + "\t" + ((.input.old_string // "") | length | tostring) + "\t" + ((.input.new_string // "") | length | tostring)
    ' "$transcript" 2>/dev/null | while IFS=$'\t' read -r fpath old_len new_len; do
      [ -n "$fpath" ] && echo "- EDIT \`$(rel_path "$fpath")\` (${old_len}→${new_len} chars)"
    done

    # Bash commands — only meaningful ones (tests, git, build)
    jq -r '
      select(.type == "assistant") |
      .message.content[]? |
      select(.type == "tool_use") |
      select(.name == "Bash") |
      (.input.command // "") + "\t" + (.input.description // "")
    ' "$transcript" 2>/dev/null | while IFS=$'\t' read -r cmd desc; do
      case "$cmd" in
        *pytest*|*cargo\ test*|*ruff*|*mypy*|*npm\ test*|git\ log*|git\ diff*|git\ status*|git\ add*|git\ commit*|git\ merge*|*caws\ *|*pip\ install*|*make*|*cargo\ build*)
          local short_cmd
          short_cmd=$(echo "$cmd" | head -1 | cut -c1-120)
          if [ -n "$desc" ]; then
            echo "- BASH \`${short_cmd}\` — ${desc}"
          else
            echo "- BASH \`${short_cmd}\`"
          fi
          ;;
      esac
    done

    # Git commits during session
    if [ -n "$start_sha" ] && [ "$start_sha" != "$head_sha" ]; then
      echo ""
      echo "### Commits this session (\`${start_sha}..${head_sha}\`)"
      echo ""
      cd "$CWD" 2>/dev/null && git log --oneline "${start_sha}..${head_sha}" 2>/dev/null | while IFS= read -r line; do
        echo "- \`${line}\`"
      done
      echo ""
      echo "**Diff stat:**"
      echo '```'
      cd "$CWD" 2>/dev/null && git diff --stat "${start_sha}..${head_sha}" 2>/dev/null || echo "(no diff)"
      echo '```'
    fi

    echo ""
  } >> "$SESSION_MD"

  # --- Footer ---
  {
    echo "## Session Snapshot"
    echo ""
    echo "| Field | Value |"
    echo "|-------|-------|"
    echo "| Time | $(date +"%Y-%m-%d %H:%M:%S %Z") |"
    echo "| Branch | \`${branch}\` @ \`${head_sha}\` |"
    echo "| Dirty files | ${dirty_count} |"
    echo ""
  } >> "$SESSION_MD"
}

# ============================================================
# EVENT: SessionStart — just save metadata
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

  # Write a minimal placeholder session.md
  generate_session_md "$(resolve_transcript)"
}

# ============================================================
# EVENT: Stop — regenerate session.md from transcript
# ============================================================
handle_stop() {
  local transcript
  transcript=$(resolve_transcript)
  generate_session_md "$transcript"
}

# ============================================================
# EVENT: PreCompact — same as Stop (safety net before context eviction)
# ============================================================
handle_pre_compact() {
  local transcript
  transcript=$(resolve_transcript)
  generate_session_md "$transcript"
}

# ============================================================
# DISPATCH
# ============================================================
case "$HOOK_EVENT" in
  SessionStart)   handle_session_start ;;
  Stop)           handle_stop ;;
  PreCompact)     handle_pre_compact ;;
  *)              ;; # Other events: no-op (transcript captures everything)
esac

exit 0
