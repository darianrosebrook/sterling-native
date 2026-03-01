---
authority: reference
status: advisory
---

# Operator Policy

**Advisory -- not normative.** This document describes design rationale for
treating operators as policies rather than classifiers. Do not cite as
canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for
capability status.

## Thesis

The operator head in Sterling is a **policy**, not a classifier. Its job is:
given the current state, choose an operation whose execution moves the system
closer to a state from which the remaining problem is tractable.

This implies:

- Operators are actions with consequences, not labels.
- Learning is about expected downstream value, not surface correctness.
- Supervision must reflect choice among alternatives, not confirmation of a
  single allowed option.

## Operator Semantics

An operator represents a typed state transition (S -> S') with a known
affordance (what it enables next) and a learned expected value (whether this
is a good move now).

Operators are not relation names, syntactic tags, intent labels, or teacher
logits to be mimicked.

### Landmarks

Sterling learns landmarks, not full plans. A landmark is a state from which
the remaining problem is easy, known, or low-entropy. Operator learning is
therefore implicitly learning: "If I apply operator o in state s, I am likely
to land in a region where future progress is reliable."

## Proven by Existing Code

- **Operator registry with effect contracts** — `kernel/src/operators/operator_registry.rs`
  `OperatorRegistryV1` enforces three-phase apply: registry lookup →
  dispatch → effect validation
- **Operator masks via preconditions** — `kernel/src/operators/apply.rs`
  `apply()` checks operator legality before execution (masks are structural,
  not learned)
- **ValueScorer trait** — `search/src/scorer.rs` provides `UniformScorer`
  (mandatory baseline) and `TableScorer` (per-action scoring via
  content-addressed lookup tables)
- **Advisory-only invariant** — scorer reorders candidates but cannot add,
  remove, or change legality (`search/src/scorer.rs`)

## Future Proof Obligations

The loss function, baseline requirements, evaluation contract, and landmark
learning described below are design blueprints for a future learned operator
policy. No ML training pipeline exists in the codebase. The connection point
is the `ValueScorer` trait — a learned policy would produce `TableScorer`
entries.

## Masked Gold Cross-Entropy Loss

The primary loss for operator-supervised examples:

```
L_op = CE(logits + prior_bias, gold_operator_id, restricted to operator_mask)
```

Key properties:

- Masked logits (negative infinity outside mask)
- Gold labels only (no teacher KL divergence)
- One decision per state
- Mask size must be >= 2 for policy learning; singleton masks are not
  operator learning examples

**Explicitly forbidden:**

- Distilling operator probabilities from teacher
- Cross-entropy over full operator vocabulary
- Cross-entropy when mask size = 1
- Treating operator prediction as intent classification

## Mandatory Baselines

Every operator dataset must report four baselines before training is
permitted:

| Baseline | Meaning |
|----------|---------|
| Majority | Always choose most common operator |
| Uniform(mask) | Random valid operator |
| Prior-counts | Argmax neighbor counts from graph adjacency |
| Linear(features) | Logistic regression on provided features |

**Identifiability gate:** If linear(features) <= majority, features are
insufficient -- stop. This gate is not optional.

If prior-counts > majority, the task is potentially learnable. If
(model + prior) approximates prior, learning has failed.

## Formal Evaluation Contract

An operator evaluation set must:

- Include the same feature vector as training
- Include operator masks
- Have >= 2 valid operators per example
- Be split by state identity (no leakage)

Benchmarks without features are invalid.

**Primary metrics:**

- Accuracy vs prior-only
- Accuracy vs uniform(mask)

**Secondary metrics:**

- Entropy of predictions under mask
- Calibration vs expected downstream value

## Connection to ValueScorer

The existing ValueScorer trait (UniformScorer, TableScorer) operates at the
search level: it scores candidate actions during expansion. The operator
policy described here operates at the learning level: it trains the model
that ultimately produces those scores.

The design space for future work:

- **TableScorer** already provides per-action scoring via content-addressed
  lookup tables. A learned operator policy would produce these tables.
- **UniformScorer** is the mandatory baseline -- equivalent to the
  Uniform(mask) baseline above.
- The identifiability gate ensures that any learned scorer demonstrably
  outperforms uniform scoring before deployment.

## Architectural Consequences

1. Operator head is a policy head, not a classifier.
2. Data generation must create choice, not single-path episodes.
3. Goal signal must exist in features, or learning is impossible.
4. Masks are part of the state, not metadata.
5. Teacher logits are untrusted for action learning.

This aligns Sterling with RL-style decision making, landmark-based
navigation, and hierarchical reasoning (operator -> planner -> value), and
prevents the system from silently collapsing into "predict the most common
relation."
