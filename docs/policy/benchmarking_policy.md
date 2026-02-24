---
status: "Draft (Sterling Native)"
authority: policy
scope: All benchmarks and performance claims
---
# Benchmarking Policy

## Purpose

Benchmarks in Sterling Native exist to validate specific architectural claims with evidence, not to generate marketing numbers.

Benchmarks must answer, with artifacts:
1. Do hard invariants hold under load (deterministic replay, trace completeness, sealed interfaces, fail‑closed legality)?
2. Does the carrier/substrate make search economically viable without changing semantics?
3. Do improvements generalize across worlds (transfer), not just within one world?

## Non-goals

- Benchmarks are not leaderboard submissions.
- Benchmarks are not “LLM vs Sterling” contests without equivalence definitions.
- No published metric is valid without an artifact bundle and eligibility checks.

## Benchmark classes

### A. Microbenchmarks (kernel primitives)
Measure single operation families: compile/decompile, hashing, precondition checks, frontier selection, dedup/equivalence reduction.

### B. Scenario benchmarks (single-world workloads)
End-to-end episodes through the Unified World Harness within one world.

### C. Transfer benchmarks (cross-world invariance)
The same claim catalog tested across multiple worlds, under comparable budgets and governance mode.

## Required artifact bundle (for any cited result)

A benchmark run MUST produce an immutable, content-addressed bundle that includes:

1) Inputs bundle (sealed)
- policy snapshot (mode, gate set, budgets)
- payload bytes / fixtures
- schema descriptor + registry snapshot
- engine version + epoch ID
- environment record (OS/CPU, build flags; optional GPU)

2) Trace bundle (canonical)
- ByteTrace (canonical trace format)
- outcome rows (per task)
- metrics bundle (timings, counts, memory)
- tool transcripts (if any tool operators executed)

3) Verification bundle
- replay verification result + hash
- gate verdict bundle (explicit pass/fail per required gate)
- negative control status (where applicable)

## Eligibility

Benchmarks have an explicit eligibility state. Only eligible runs may be cited.

Minimum eligibility requirements:
- deterministic replay verified for the run’s version/epoch
- trace completeness satisfied (no missing obligations)
- required gate verdicts present (no “unknown” state)
- task count meets the benchmark class floor
- environment record present

## DEV vs CERTIFIED rules

- DEV runs may proceed after failures, but remain ineligible for published claims.
- CERTIFIED runs are fail-closed and are the only mode eligible for promotion-grade benchmark artifacts.

## Comparison policy (when comparing to transformer-centric systems)

Comparisons are allowed only if:
- the task spec is machine-checkable (input/output/verifier)
- resource budgets are explicit (time, calls, retries, tokens)
- all costs are recorded (wall time, tokens, tool I/O)
- asymmetries are stated (e.g., “verified trace” vs “unverified final answer”)

## Regression policy

- Microbench regressions: fail CI if a hot-path primitive regresses beyond a versioned threshold.
- Scenario regressions: block promotion if eligible performance regresses while semantics are unchanged.
- Any optimization that changes canonical bytes is a semantic change and requires a version/epoch bump.
