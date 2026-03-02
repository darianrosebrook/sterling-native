---
authority: reference
status: advisory
---

# World Catalog

**Advisory -- not normative.** This document describes the current and planned
world inventory for Sterling. Do not cite as canonical. See
[parity audit](../../architecture/v1_v2_parity_audit.md) for capability status.

## World Status Table

| World | Crate | Status | Truth Regime | Primary Axes |
|-------|-------|--------|-------------|-------------|
| RomeMini | harness | Built | Carrier | Deterministic compilation, replay |
| RomeMiniSearch | harness | Built | Search | Path-finding, bundle verification |
| SlotLatticeSearch | harness | Built (6 regimes) | Search | Budget governance, scorer advisory, trap/goal structure |
| TransactionalKvStore | harness | Built | Carrier | Write-once transactions, marker-based commit/rollback |
| ToolKvStore | harness | Built | Tool Safety | Stage/commit/rollback operators, tool transcript artifact |
| PartialObs (Mastermind) | harness | Built | Epistemic | Partial observability, belief monotonicity, winning-path replay |
| Slippery Grid | -- | Planned | Stochastic | Probabilistic transitions |

## Built Worlds

### RomeMini

Minimal carrier fixture: 1 layer, 2 slots, 1 SET_SLOT operator. Exercises
the full compile-trace-replay pipeline. Proves deterministic compilation,
canonical trace serialization, frame-by-frame replay verification, and bundle
artifact closure.

**Capability axes exercised:** (7) transfer (carrier contracts are
world-agnostic), (8) language grounding (no LLM), (10) compute governance
(single-frame budget).

**Rubric claims falsified:** None -- RomeMini is a fixture world, not a
search world. It cannot falsify search-level claims.

### RomeMiniSearch

Search fixture: 2 slots x 4 values, best-first search with goal checking.
Proves search determinism, graph construction, metadata bindings, tape
coherence, and bundle verification including scorer and operator registry
artifacts.

**Capability axes exercised:** (7) transfer (search contracts are
world-agnostic), (8) language grounding, (10) compute governance (budget
caps).

**Rubric claims falsified:** Determinism violations, metadata binding
tampering, scorer digest mismatch, operator registry bypass.

### SlotLatticeSearch (6 Regimes)

Parameterized N-slot x V-value world with configurable traps, goals, and
policy knobs. Six canonical regimes exercise distinct failure modes:

| Constructor | Config | Exercises |
|-------------|--------|-----------|
| `regime_truncation()` | N=8, V=4, cap=5 | Candidate truncation (32 candidates → 5) |
| `regime_duplicates()` | N=4, V=2, goal=Never | Duplicate suppression from permuted slot assignments |
| `regime_exhaustive_dead_end()` | N=4, V=3, trap=Slot0Eq(2) | Trap avoidance, dead-end detection |
| `regime_budget_limited()` | N=6, V=3, max_expansions=3 | Early budget termination |
| `regime_scale_1000()` | N=10, V=4, 1000 expansions | Performance envelope, large state space |
| `regime_frontier_pressure()` | N=6, V=3, max_frontier=8 | Frontier pruning under pressure |

Source: `harness/src/worlds/slot_lattice_regimes.rs`

**Capability axes exercised:** (4) adversarial robustness (trap regimes),
(7) transfer (same search contracts across regimes), (10) compute governance
(budget-sweep via regime variation).

**Rubric claims falsified:** Budget exhaustion, trap-induced dead ends,
scorer advisory violations, frontier pruning under pressure.

### TransactionalKvStore

Two-layer write-once KV store with marker-based transactions. Proves
commit/rollback semantics, write-once enforcement, and goal evaluation on
committed layer only.

**Capability axes exercised:** (8) language grounding (pure operator
semantics), (10) compute governance (single-step budget).

**Rubric claims falsified:** Write-once violation, rollback-as-truth,
uncommitted-goal acceptance.

### ToolKvStore

Two-layer KV store exercising tool-safety operators (OP_STAGE, OP_COMMIT,
OP_ROLLBACK) with distinct EffectKind variants (StagesOneSlot,
CommitsTransaction, RollsBackTransaction). Proves stage/commit/rollback
protocol compliance, tool transcript rendering, and derived artifact
downstream binding convention.

**Capability axes exercised:** (6) tool safety (stage/commit/rollback
protocol), (10) compute governance (transaction-bounded).

**Rubric claims falsified:** Tool transcript forgery (Cert equivalence
mismatch), layer-0 writes before commit, writes after rollback, tool
transcript obligation omission.

**CAWS spec:** TOOLSCRIPT-001 (closed)

Source: `harness/src/worlds/tool_kv_store.rs`

### PartialObs (Mastermind-style)

Mastermind-style hidden-truth world with K=2 positions, V=3 values (9
candidates). Two-layer ByteState: layer 0 (truth, write-protected after
compile), layer 1 (workspace for guesses, feedback, solved marker). Two-step
probe cycle: OP_GUESS (agent writes guess) → OP_FEEDBACK (environment writes
bulls-only feedback computed from truth). OP_DECLARE when belief uniquely
determines truth.

Belief is fully implicit — NOT stored in ByteState. Reconstructed from probe
history during winning-path replay. Belief monotonicity (non-increasing
cardinality) is verified by the replay invariant checker.

**Three-way authority division:**
- **Kernel**: bounds write surface (diff counts, layer constraints, Hole→Provisional)
- **World**: computes truth-dependent feedback in `enumerate_candidates` (harness privilege)
- **Verifier**: proves correspondence via winning-path replay (re-executes operators, checks feedback correctness, belief monotonicity, declare correctness)

**Capability axes exercised:** (2) partial observability, (5) uncertainty
calibration (belief converges to 1 before declare).

**Rubric claims falsified:** Truth layer writes, feedback incorrectness,
belief monotonicity violation, declare-truth mismatch, epistemic transcript
forgery (Cert equivalence via replay).

**CAWS spec:** POBS-001 (closed)

Source: `harness/src/worlds/partial_obs.rs`

## Planned Worlds

### Slippery Grid (Stochastic Transitions)

**Target axes:** (3) stochasticity, (10) compute governance under
uncertainty.

**Design sketch:** Grid world where move actions succeed with probability p
and slip to adjacent cell with probability 1-p. Agent must plan robust paths
that account for slip risk.

**Proof obligations:** Stochastic replay witness captures seed and transition
outcomes. Performance degrades monotonically as slip probability increases.
Same seed produces identical trace.

## Truth Regime Matrix

Each world operates in a truth regime that constrains what kinds of claims it
can support:

| Regime | What it proves | Required artifacts | Status |
|--------|---------------|-------------------|--------|
| Carrier | Compile-trace-replay determinism | ByteState, ByteTrace, verification report | Proven (RomeMini, TransactionalKvStore) |
| Search | Path-finding under budget + policy | SearchGraph, tape, scorer, operator registry | Proven (RomeMiniSearch, SlotLatticeSearch) |
| Tool Safety | Stage/commit/rollback protocol compliance | tool_transcript.json + Cert equivalence | Proven (ToolKvStore, TOOLSCRIPT-001) |
| Epistemic | Belief-state reasoning under partial info | epistemic_transcript.json + winning-path replay | Proven (PartialObs, POBS-001) |
| Stochastic | Robust planning under transition noise | Stochastic replay witness, degradation curve | Planned |

A world must declare its truth regime. Claims outside the regime are
inadmissible. See the [parity audit](../../architecture/v1_v2_parity_audit.md)
for which regimes are currently proven.
