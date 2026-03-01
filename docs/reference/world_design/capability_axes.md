---
authority: reference
status: advisory
---

# Capability Axes

**Advisory -- not normative.** This document describes proof obligations for
future v2 work along ten orthogonal capability axes. Do not cite as canonical.
See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability
status.

## Ten Orthogonal Capability Axes

A deployable, domain-agnostic reasoning engine must close proof obligations
along axes that existing worlds only partially touch. Each axis below defines
what must be true in deployment, how to get a clean pass/fail signal, and
cert-grade acceptance criteria.

### 1. Domain Induction

**Deployment obligation:** The system can construct a domain model (state
schema, invariants, affordances) from raw interactions and operate over it.

**Signal:** Two independent induction runs on the same evidence produce the
same content-addressed DomainSpecIR. A new task family produces a new domain
spec without new code.

**Acceptance:** DomainSpecIR digest stable across reruns; end-to-end
deterministic from induction through search.

### 2. Partial Observability

**Deployment obligation:** The system reasons correctly when state is not
fully visible, maintaining belief states and choosing information-gathering
actions.

**Signal:** Mastermind-like world where the agent must probe hidden state.
Correct belief updates after each observation; no hallucinated certainty.

**Acceptance:** Belief-state witness is content-addressed and replayable.

### 3. Stochasticity

**Deployment obligation:** The system produces robust plans when transitions
are probabilistic.

**Signal:** Slippery-grid world where moves succeed with known probability.
Performance degrades monotonically as slip probability increases.

**Acceptance:** Stochastic replay witness captures random seed and transition
outcomes; deterministic given seed.

### 4. Adversarial Robustness

**Deployment obligation:** The system resists adversarial perturbation of
inputs, operators, or scoring without silent degradation.

**Signal:** Mutation campaign (>=10 adversarial mutations) with classified
outcomes. Refusal digests stable across reruns.

**Acceptance:** Typed failure families; no untyped exceptions on claim surface.

### 5. Uncertainty Calibration

**Deployment obligation:** Confidence scores reflect actual outcome
distributions. **Signal:** Calibration curve; ECE below threshold.
**Acceptance:** Calibration witness artifact with per-bin statistics.

### 6. Continual Learning

**Deployment obligation:** New evidence does not cause catastrophic forgetting.
**Signal:** Interleaved training on domain A then B then A; accuracy on A
does not collapse. **Acceptance:** Regression sweep showing per-capability
accuracy before and after each learning phase.

### 7. Transfer and Compositionality

**Deployment obligation:** Capabilities proven in one domain transfer to a
second domain with zero capsule code changes.

**Signal:** Same conformance suite passes on two structurally different
domains. Transfer failure on a near-miss domain (same structure, disjoint
vocabulary) proves the check is not vacuous.

**Acceptance:** Transfer verdict artifact binding source and target domains.

### 8. Language Grounding

**Deployment obligation:** The system connects language I/O to structured
state without relying on transformer chain-of-thought for reasoning.

**Signal:** Transformer calls occur only at I/O boundaries (parse input,
render output). All mid-episode state transitions use operators, not LLM
generation.

**Acceptance:** Call-count witness showing zero LLM calls during reasoning.

### 9. Tool Integration

**Deployment obligation:** The system invokes external tools through
declared, versioned adapter interfaces without domain coupling.

**Signal:** Tool adapter conformance suite passes; tool failures produce
typed refusals, not unhandled exceptions.

**Acceptance:** Adapter certification artifact per tool.

### 10. Compute Governance

**Deployment obligation:** The system operates within declared budgets and
degrades predictably under resource pressure.

**Signal:** Monotonic degradation curve: reducing budget reduces quality but
never causes chaotic failure. Budget sensitivity demonstrated (changing
budget changes all downstream digests).

**Acceptance:** Budget-sweep artifact with cost-per-solve metrics.

## Unified World Harness Template

Future worlds should be parameterized along four toggles:

| Toggle | Off | On |
|--------|-----|-----|
| Observability | Fully observed | Partial (hidden state, belief updates) |
| Transition stochasticity | Deterministic | Probabilistic (known distribution) |
| Data cleanliness | Clean inputs | Noisy / adversarial perturbations |
| Tool type | Pure operators | External tool calls via adapter |

A world's toggle configuration determines which capability axes it exercises.

## Sterling Capability Matrix

| Axis | Status | Exercised by |
|------|--------|--------------|
| 1. Domain induction | Unproven | -- |
| 2. Partial observability | Unproven | -- |
| 3. Stochasticity | Unproven | -- |
| 4. Adversarial robustness | Partial | Lock test mutation campaigns |
| 5. Uncertainty calibration | Unproven | -- |
| 6. Continual learning | Unproven | -- |
| 7. Transfer / compositionality | Proven (structural) | SlotLatticeSearch 6 regimes; RomeMiniSearch |
| 8. Language grounding | Proven (structural) | All worlds (no LLM in search loop) |
| 9. Tool integration | Unproven | -- |
| 10. Compute governance | Proven | SearchPolicyV1 budget caps; SlotLatticeSearch scale_1000 |

Axes 1-3, 5-6, and 9 represent the primary unproven surface area. New worlds
should target these axes before broadening existing ones.
