# CLAUDE.md

This project uses CAWS (Coding Agent Working Standard) for quality-assured AI-assisted development.

## Build & Test

```bash
# Run tests (all crates, lib + bin + integration test targets)
cargo test --workspace

# Run tests including lock tests with cross-proc binaries
cargo test --workspace --lib --bins --tests

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Run all quality gates
caws validate
```

**Do NOT use `cargo test --workspace --all-targets`.** The `--all-targets` flag
includes bench targets, which pulls in Criterion + plotters in debug mode. That
adds ~13 minutes of compilation for zero tests. The benchmarks crate is
intentionally excluded from `default-members` in the workspace `Cargo.toml`.
Use `--lib --bins --tests` if you need explicit target selection.

To run benchmarks specifically:
```bash
cargo bench -p sterling-benchmarks
```

## How to present results summaries

It's great if tests pass, but the user needs a clearer signal than quantity of green checks. Do tests pass or fail in any meaningful way? Provide a critical evaluation of the session's results and progress, along with next steps laid out for the next stretch of work. If more work is warranted, explain what you would want to investigate/implement/change, where, and why.

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
