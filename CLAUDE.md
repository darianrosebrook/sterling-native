
# Session Protocol for All Projects

These rules apply to every Claude Code session regardless of which repository you're working in.

## Session Start

1. **Note the baseline**: Record `HEAD` sha and check `git status`. If the working tree is dirty from a prior session, commit the dirty state as `wip(<scope>): checkpoint from prior session` before starting new work. Do not silently inherit uncommitted changes.
2. **Check for CAWS**: If `.caws/` exists in the project root, this is a CAWS-managed project. Run `caws status` to see project health, then follow the CAWS workflow described in the project's CLAUDE.md.
3. **Run fast tests**: Run the project's unit test suite (or equivalent fast gate) and note whether it passes. If tests fail before you've changed anything, that's pre-existing breakage -- document it, don't ignore it.

## How to present results summaries

It's great if tests pass, but the user needs a clearer signal than quantity of green checks. Do tests pass or fail in any meaningful way? Provide a critical evaluation of the session's results and progress, along with next steps laid out for the next stretch of work. If more work is warranted, explain what you would want to investigate/implement/change, where, and why.

## Session Discipline

- **Commit after each logical unit of work.** A module + its tests, a bugfix, a refactor pass. Do not accumulate uncommitted changes across multiple concerns.
- **Never leave uncommitted changes at session end.** If your work is incomplete, commit what you have as `wip(<scope>): <description>` so the next session starts clean.
- **Use conventional commits**: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`, `perf:` prefixes.

## Repo State Ownership

You are responsible for the state of the repo during your session. The context window resets between sessions, but the repo doesn't. If you encounter broken tests, dirty state, or unexpected files:

- **Classify it**: Is it pre-existing (from a prior session) or something you introduced?
- **Fix or document it**: Either fix the issue or note it explicitly so the next session has context.
- **Never disclaim it**: Do not say "I didn't make that mess" or "that was broken before I started." The repo is the shared workspace. If you're asked to help with something, help with it. Note the baseline you inherited, then address the problem.

## Multi-Agent Coordination

Multiple Claude Code sessions may be working on the same repo:

### If CAWS is available

1. **Create a feature spec before starting work**: `caws specs create <feature-id> --type feature --title "description"`. This creates a feature-level working spec at `.caws/specs/<id>.yaml` with its own scope and change budget. **Never edit the project-level `.caws/working-spec.yaml`** -- that's the shared baseline. Your feature spec is where you define what you're working on.
2. **Use `--spec-id` on all CAWS commands**: `caws validate --spec-id <id>`, `caws iterate --spec-id <id>`, etc. This targets your feature spec, not the project-level one.
3. **Use worktrees for parallel work**: If instructed to work in parallel with another agent, use `caws worktree create <name> --scope "pattern"` to get a physically isolated workspace.
4. **Stay within your spec's scope boundaries**: Your feature spec defines `scope.in` and `scope.out`. Do not edit files outside your scope.
5. **Check for scope conflicts**: If you need to touch files that might overlap with another agent's work, run `caws specs conflicts` first.

### If CAWS is not available

1. **Work on a branch**: Create a descriptive branch (`feat/<description>`, `fix/<description>`) rather than committing directly to main.
2. **Commit early and often**: Small, atomic commits with clear messages.
3. **Don't touch files outside your task scope**: If your task is about auth, don't refactor the logging system.

## Quality Standards

- **No shadow files**: Never create `*-enhanced.*`, `*-new.*`, `*-v2.*`, `*-final.*`, `*-copy.*` duplicates. Edit in place.
- **No fake implementations**: No placeholder stubs, no `TODO` in committed code, no hardcoded mock responses pretending to be real.
- **No marketing language**: Avoid "revolutionary", "cutting-edge", "state-of-the-art", "enterprise-grade" in docs and comments.
- **Prove claims**: Don't assert "production-ready" or "complete" without evidence. Provide test results, not assertions.
- **Tests first when practical**: Write failing tests before implementation for new features and bug fixes.

This project uses CAWS (Coding Agent Working Standard) for quality-assured AI-assisted development.

## Build & Test

```bash
# Install dependencies
npm install

# Run tests
npm test

# Lint
npm run lint

# Type check (if TypeScript)
npm run typecheck

# Run all quality gates
caws validate
```

## CAWS Workflow

Before writing code, check the working spec:

```bash
# Validate the working spec
caws validate

# Get iteration guidance
caws agent iterate --current-state "describe what you're about to do"

# After implementation, evaluate quality
caws agent evaluate
```

### Working Spec

The project spec lives at `.caws/working-spec.yaml`. It defines:

- **Risk tier**: Quality requirements (T1: critical, T2: standard, T3: low risk)
- **Scope**: Which files you can edit (`scope.in`) and which are off-limits (`scope.out`)
- **Change budget**: Max files and lines of code per change
- **Acceptance criteria**: What "done" means

Always stay within scope boundaries and change budgets.

### Quality Gates

Quality requirements are tiered:

| Gate | T1 (Critical) | T2 (Standard) | T3 (Low Risk) |
|------|---------------|----------------|----------------|
| Test coverage | 90%+ | 80%+ | 70%+ |
| Mutation score | 70%+ | 50%+ | 30%+ |
| Contracts | Required | Required | Optional |
| Manual review | Required | Optional | Optional |

### Key Rules

1. **Stay in scope** -- only edit files listed in `scope.in`, never touch `scope.out`
2. **Respect change budgets** -- stay within `max_files` and `max_loc` limits
3. **No shadow files** -- edit in place, never create `*-enhanced.*`, `*-new.*`, `*-v2.*`, `*-final.*` copies
4. **Tests first** -- write failing tests before implementation
5. **Deterministic code** -- inject time, random, and UUID generators for testability
6. **No fake implementations** -- no placeholder stubs, no `TODO` in committed code, no in-memory arrays pretending to be persistence, no hardcoded mock responses
7. **Prove claims** -- never assert "production-ready", "complete", or "battle-tested" without passing all quality gates. Provide evidence, not assertions.
8. **No marketing language in docs** -- avoid "revolutionary", "cutting-edge", "state-of-the-art", "enterprise-grade"
9. **Ask first for risky changes** -- changes touching >10 files, >300 LOC, crossing package boundaries, or affecting security/infrastructure require discussion first
10. **Conventional commits** -- use `feat:`, `fix:`, `refactor:`, `docs:`, `chore:` prefixes

### Waivers

If you need to bypass a quality gate, create a waiver with justification:

```bash
caws waivers create --reason emergency_hotfix --gates coverage_threshold
```

Valid reasons: `emergency_hotfix`, `legacy_integration`, `experimental_feature`, `performance_critical`, `infrastructure_limitation`

## Project Structure

```
.caws/
  working-spec.yaml   # Project spec (risk tier, scope, acceptance criteria)
  policy.yaml         # Quality policy overrides (optional)
  waivers.yml         # Active waivers
```

## Hooks

This project has Claude Code hooks configured in `.claude/settings.json`:

- **PreToolUse**: Blocks dangerous commands, scans for secrets, enforces scope
- **PostToolUse**: Runs quality checks, validates spec, checks naming conventions
- **Session**: Audit logging for provenance tracking

See `.claude/README.md` for hook details.
