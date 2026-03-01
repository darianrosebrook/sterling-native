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
| Mastermind-like | -- | Planned | Epistemic | Partial observability, belief-state |
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

| Regime | Config | Exercises |
|--------|--------|-----------|
| truncation | N=8, V=4, cap=5 | Candidate truncation (32 candidates → 5) |
| duplicates | N=4, V=2, goal=Never | Duplicate suppression from permuted slot assignments |
| exhaustive_dead_end | N=4, V=3, trap=Slot0Eq(2) | Trap avoidance, dead-end detection |
| budget_limited | N=6, V=3, max_expansions=3 | Early budget termination |
| scale_1000 | N=10, V=4, 1000 expansions | Performance envelope, large state space |
| frontier_pressure | N=6, V=3, max_frontier=8 | Frontier pruning under pressure |

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

## Planned Worlds

### Mastermind-like (Partial Observability)

**Target axes:** (2) partial observability, (5) uncertainty calibration.

**Design sketch:** Hidden code of N pegs x K colors. Agent proposes guesses,
receives structured feedback (correct position count, correct color count).
Must maintain belief state over possible codes and choose
information-maximizing probes.

**Proof obligations:** Belief-state witness is content-addressed. Belief
updates are deterministic given observation sequence. Agent never claims
certainty without sufficient evidence.

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

| Regime | What it proves | Required artifacts |
|--------|---------------|-------------------|
| Carrier | Compile-trace-replay determinism | ByteState, ByteTrace, verification report |
| Search | Path-finding under budget + policy | SearchGraph, tape, scorer, operator registry |
| Epistemic | Belief-state reasoning under partial info | Belief witness, observation envelope |
| Stochastic | Robust planning under transition noise | Stochastic replay witness, degradation curve |

A world must declare its truth regime. Claims outside the regime are
inadmissible. See the [parity audit](../../architecture/v1_v2_parity_audit.md)
for which regimes are currently proven.
