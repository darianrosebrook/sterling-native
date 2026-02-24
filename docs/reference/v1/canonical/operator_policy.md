> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Sterling Operator Learning v1

**Policy-First Operator Selection Specification**

**Status:** Draft → Candidate for “Architectural Commitment”
**Applies to:** WordNet, KG navigation domains, future symbolic planners
**Explicitly NOT:** intent classification, label prediction, or teacher imitation

---

## 1. Purpose (Non-Negotiable)

The operator head in Sterling is a **policy**, not a classifier.

Its job is:

> **Given the current state, choose an operation whose execution moves the system closer to a goal state that Sterling knows how to solve from there.**

This implies:

- Operators are **actions** with consequences, not labels.
- Learning is about **expected downstream value**, not surface correctness.
- Supervision must reflect **choice among alternatives**, not confirmation of a single allowed option.

Anything that violates this premise is out of scope for operator learning.

---

## 2. Operator Semantics

### 2.1 What an Operator Represents

An operator represents:

- a **typed state transition** (`S → S'`)
- with a **known affordance** (“what it enables next”)
- and a **learned expected value** (“is this a good move _now_?”)

Operators are not:

- relation names
- syntactic tags
- intent labels
- teacher logits to be mimicked

### 2.2 Landmarks (Critical Concept)

Sterling learns **landmarks**, not full plans.

A landmark is:

- a state from which the remaining problem is easy / known / low-entropy

Operator learning is therefore implicitly learning:

> “If I apply operator _o_ in state _s_, I am likely to land in a region of the state space where future progress is reliable.”

This is why state-level masking and neighbor priors worked: they approximate landmark affordances.

---

## 3. Inputs Required for Operator Learning

Operator learning is **ill-posed** unless _all_ of the following exist.

### 3.1 Required Fields per Training Example

```json
{
  "features": <state_features>,          // must include goal-relevant signal
  "src_state_id": "...",                  // e.g., src_synset
  "goal_state_id": "...",                 // explicit or implicit
  "operator_id": <int>,                   // gold action taken
  "operator_mask": <bool[N_ops]>,         // feasible actions at this state
  "domain_id": <int>
}
```

If any of these are missing, operator CE **must not** be applied.

---

### 3.2 Mask Semantics (Hard Invariant)

- `operator_mask[i] = True` ⇔ operator _i_ is executable **from this state**
- Gold operator **must be inside the mask**
- Mask size **must be ≥ 2** for policy learning

Singleton masks are **not operator learning examples**.

---

## 4. Feature Requirements (Identifiability Gate)

Operator learning is allowed _only if_ the following test passes:

> **There exists a baseline policy using the provided inputs that beats the majority operator by ≥ X%.**

### 4.1 Mandatory Baselines

Every operator dataset must report:

| Baseline         | Meaning                            |
| ---------------- | ---------------------------------- |
| Majority         | Always choose most common operator |
| Uniform(mask)    | Random valid operator              |
| Prior-counts     | Argmax neighbor counts from KG     |
| Linear(features) | Logistic regression                |

If **prior-counts > majority**, the task is _potentially learnable_.

If **linear(features) ≤ majority**, features are insufficient — stop.

This gate is **not optional**.

---

## 5. Loss Function (Gold Policy Learning)

### 5.1 Primary Loss: Masked Gold Cross-Entropy

For operator-supervised examples:

```
L_op = CE(
  logits + prior_bias,
  gold_operator_id,
  restricted to operator_mask
)
```

Key properties:

- Masked logits (`−∞` outside mask)
- Gold labels only (no teacher KL)
- One decision per state

### 5.2 Explicitly Forbidden

- Distilling operator probabilities from teacher
- CE over full operator vocabulary
- CE when mask size = 1
- Treating operator prediction as intent classification

---

## 6. Role of Priors

Priors are **biases**, not answers.

### 6.1 Allowed Prior

- `log1p(neighbor_counts)` from KG adjacency

### 6.2 Required Controls

Every experiment must report:

- accuracy with **prior only**
- accuracy with **model only**
- accuracy with **model + prior**

If `(model + prior) ≈ prior`, learning has failed.

---

## 7. Evaluation Contract

### 7.1 Evaluation Dataset Requirements

An operator eval set **must**:

- include the same feature vector as training
- include operator masks
- have ≥ 2 valid operators per example
- be split by **state identity** (no leakage)

Benchmarks without features are invalid.

---

### 7.2 Metrics That Matter

Primary:

- Accuracy vs **prior-only**
- Accuracy vs **uniform(mask)**

Secondary:

- Entropy of predictions under mask
- Calibration vs expected downstream value (future v2)

---

## 8. Explicit Non-Goals (v1)

Operator Learning v1 does **not** attempt:

- multi-step planning
- search tree optimization
- operator composition
- symbolic proofs of correctness

Those belong to **planner/search layers**, not the operator head.

---

## 9. Architectural Consequences (Why This Matters)

This spec implies:

1. **Operator head is a policy head**, not a classifier
2. **Data generation must create choice**, not single-path episodes
3. **Goal signal must exist in features**, or learning is impossible
4. **Masks are part of the state**, not metadata
5. **Teacher logits are untrusted** for action learning

This aligns Sterling with:

- RL-style decision making
- landmark-based navigation
- hierarchical reasoning (operator → planner → value)

…and prevents the system from silently collapsing into “predict the most common relation”.

---

## 10. Immediate Next Steps (Concrete)

To operationalize this spec:

1. **Regenerate WordNet episodes**

   - `allowed_edge_types` size ≥ 3
   - same start, same goal, multiple viable relations

2. **Add minimal goal-conditioned features**

   - per-relation depth estimates
   - distance-to-goal heuristics
   - synset depth deltas

3. **Lock an Operator Learning v1 benchmark**

   - with features
   - with masks
   - with published baselines

4. **Promote this spec to an Architectural Commitment**

   - violations fail CI
   - identifiability gate required before training

---

### Final assessment

This spec is **policy first, classification second**.

If Sterling learns operators as policies tied to landmarks, everything else you’re building (value heads, search, long-horizon reasoning) has something solid to stand on.
