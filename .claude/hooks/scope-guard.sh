#!/bin/bash
# CAWS Scope Guard Hook for Claude Code
# Validates file edits against the working spec's scope boundaries
# @author @darianrosebrook

set -euo pipefail

# Read JSON input from Claude Code
INPUT=$(cat)

# Extract file path from PreToolUse input
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // ""')
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // ""')

# Only check Write/Edit operations
if [[ "$TOOL_NAME" != "Write" ]] && [[ "$TOOL_NAME" != "Edit" ]]; then
  exit 0
fi

if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"
SPEC_FILE="$PROJECT_DIR/.caws/working-spec.yaml"
SCOPE_FILE="$PROJECT_DIR/.caws/scope.json"

# Check if spec file or scope.json exists
if [[ ! -f "$SPEC_FILE" ]] && [[ ! -f "$SCOPE_FILE" ]]; then
  exit 0
fi

# Get relative path from project root (portable — macOS realpath lacks --relative-to)
if [[ "$FILE_PATH" == "$PROJECT_DIR"/* ]]; then
  REL_PATH="${FILE_PATH#$PROJECT_DIR/}"
else
  REL_PATH="$FILE_PATH"
fi

# Lite mode: check scope.json if no working-spec.yaml
if [[ ! -f "$SPEC_FILE" ]] && [[ -f "$SCOPE_FILE" ]]; then
  if command -v node >/dev/null 2>&1; then
    LITE_CHECK=$(node -e "
      const fs = require('fs');
      const path = require('path');
      try {
        const scope = JSON.parse(fs.readFileSync('$SCOPE_FILE', 'utf8'));
        const filePath = '$REL_PATH';
        const dirs = scope.allowedDirectories || [];
        const banned = scope.bannedPatterns || {};

        // Check banned file patterns
        const basename = path.basename(filePath);
        const bannedFiles = banned.files || [];
        for (const pattern of bannedFiles) {
          const regex = new RegExp(pattern.replace(/\\*/g, '.*').replace(/\\?/g, '.'));
          if (regex.test(basename)) {
            console.log('banned:' + pattern);
            process.exit(0);
          }
        }

        // Check banned doc patterns
        const bannedDocs = banned.docs || [];
        for (const pattern of bannedDocs) {
          const regex = new RegExp(pattern.replace(/\\*/g, '.*').replace(/\\?/g, '.'));
          if (regex.test(basename)) {
            console.log('banned:' + pattern);
            process.exit(0);
          }
        }

        // Check allowed directories
        if (dirs.length > 0) {
          const normalized = filePath.replace(/\\\\\\\\/g, '/');
          let found = false;
          for (const dir of dirs) {
            const d = dir.replace(/\\/$/, '');
            if (normalized.startsWith(d + '/') || normalized === d) { found = true; break; }
          }
          // Allow root-level files and .caws/ directory
          if (!normalized.includes('/') || normalized.startsWith('.caws/')) found = true;
          if (!found) {
            console.log('not_allowed');
            process.exit(0);
          }
        }
        console.log('allowed');
      } catch (error) {
        console.log('error:' + error.message);
      }
    " 2>&1)

    if [[ "$LITE_CHECK" == banned:* ]]; then
      PATTERN="${LITE_CHECK#banned:}"
      echo '{
        "hookSpecificOutput": {
          "hookEventName": "PreToolUse",
          "permissionDecision": "ask",
          "permissionDecisionReason": "This file ('"$REL_PATH"') matches a banned pattern ('"$PATTERN"') in .caws/scope.json. Creating files with this pattern is blocked to prevent file sprawl."
        }
      }'
      exit 0
    fi

    if [[ "$LITE_CHECK" == "not_allowed" ]]; then
      echo '{
        "hookSpecificOutput": {
          "hookEventName": "PreToolUse",
          "permissionDecision": "ask",
          "permissionDecisionReason": "This file ('"$REL_PATH"') is outside the allowed directories in .caws/scope.json. Please confirm this edit is intentional."
        }
      }'
      exit 0
    fi

    # File is allowed - exit normally
    exit 0
  fi
fi

# Use Node.js to parse YAML and check scope across working spec + active feature specs
if command -v node >/dev/null 2>&1; then
  SPECS_DIR="$PROJECT_DIR/.caws/specs"
  SCOPE_CHECK=$(node -e "
    const yaml = require('js-yaml');
    const fs = require('fs');
    const path = require('path');
    const glob = require('path');

    try {
      const filePath = '$REL_PATH';

      // Root-level files (no directory component) and .caws/ are always allowed
      if (!filePath.includes('/') || filePath.startsWith('.caws/')) {
        console.log('in_scope');
        process.exit(0);
      }

      // Collect all specs: working-spec + active feature specs
      const specs = [];
      const specFile = '$SPEC_FILE';
      if (fs.existsSync(specFile)) {
        specs.push({ source: 'working-spec', spec: yaml.load(fs.readFileSync(specFile, 'utf8')) });
      }
      const specsDir = '$SPECS_DIR';
      if (fs.existsSync(specsDir)) {
        for (const f of fs.readdirSync(specsDir).filter(f => f.endsWith('.yaml') || f.endsWith('.yml'))) {
          try {
            const s = yaml.load(fs.readFileSync(path.join(specsDir, f), 'utf8'));
            // Only include active specs (not closed)
            if (s.status !== 'closed') {
              specs.push({ source: f, spec: s });
            }
          } catch (_) {}
        }
      }

      if (specs.length === 0) {
        console.log('in_scope');
        process.exit(0);
      }

      // Check if file is explicitly out of scope in ANY spec
      for (const { source, spec } of specs) {
        for (const pattern of (spec.scope?.out || [])) {
          const regex = new RegExp(pattern.replace(/\*/g, '.*').replace(/\?/g, '.'));
          if (regex.test(filePath)) {
            console.log('out_of_scope:' + source + ':' + pattern);
            process.exit(0);
          }
        }
      }

      // Union all in-scope patterns across specs
      const allInScope = specs.flatMap(({ spec }) => spec.scope?.in || []);
      if (allInScope.length > 0) {
        let found = false;
        for (const pattern of allInScope) {
          const regex = new RegExp(pattern.replace(/\*/g, '.*').replace(/\?/g, '.'));
          if (regex.test(filePath)) {
            found = true;
            break;
          }
        }
        if (!found) {
          console.log('not_in_scope');
          process.exit(0);
        }
      }

      console.log('in_scope');
    } catch (error) {
      console.log('error:' + error.message);
    }
  " 2>&1)

  if [[ "$SCOPE_CHECK" == out_of_scope:* ]]; then
    DETAIL="${SCOPE_CHECK#out_of_scope:}"
    SOURCE="${DETAIL%%:*}"
    PATTERN="${DETAIL#*:}"
    echo '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "ask",
        "permissionDecisionReason": "This file ('"$REL_PATH"') is marked as out-of-scope in '"$SOURCE"' (pattern: '"$PATTERN"'). Editing it may cause scope creep. Please confirm this edit is intentional."
      }
    }'
    exit 0
  fi

  if [[ "$SCOPE_CHECK" == "not_in_scope" ]]; then
    echo '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "ask",
        "permissionDecisionReason": "This file ('"$REL_PATH"') is not in the defined scope of any active spec. Editing it may cause scope creep. Please confirm this edit is intentional."
      }
    }'
    exit 0
  fi
fi

# File is in scope or scope couldn't be checked - allow
exit 0
