---
authority: architecture
status: imported
source: tmp/capability_primitives_bundle
version: v0.4
---
# Capability Primitive Implementation Specs Bundle

Version: v0.4
Date: 2026-02-01

This bundle decomposes Sterling's reasoning work into domain-agnostic capability primitives. Each primitive has its own implementation spec and cert-grade gates.

## Contents

- `primitives/` One markdown file per capability primitive (P01..P21).
- `templates/` Standard rig template and authoring checklist.
- `schemas/` Minimal JSON Schemas for a solve request/response and evidence payloads (draft).

---

## Primitive Index

### P01: Deterministic Transformation Planning (Resource → Product)

**File**: `primitives/P01_deterministic_transformation_planning_resource_product.md`
**Proving Rigs**: Rig A

**What it solves**: Given state + operator set, find a valid minimal plan; return a traceable, executable path; repeat solves converge to stable shortcuts without breaking correctness.

**Formal signature**: Finite discrete state; typed operators with preconditions/effects; additive cost; goal predicate as subset/constraint satisfaction; search for minimal-cost path; optional learned edge ordering that does not change transition semantics.

**Key gates**: Determinism (identical inputs → identical trace hash), boundedness (caps enforced), validity (no illegal transitions), convergence (learning reduces expansions without regressions).

---

### P02: Capability Gating and Legality (What Actions Are Permitted)

**File**: `primitives/P02_capability_gating_and_legality_what_actions_are_permitted.md`
**Proving Rigs**: Rig B

**What it solves**: Sterling never proposes an illegal operator; it can reason about acquiring capabilities as first-class subgoals.

**Formal signature**: State includes a capability set; operators are enabled/disabled by capability predicates; monotone or partially monotone capability progression; legality checks are fail-closed.

**Key gates**: No illegal operator proposals, capability acquisition as subgoal planning, fail-closed legality checks.

---

### P03: Temporal Planning with Durations, Batching, and Capacity

**File**: `primitives/P03_temporal_planning_with_durations_batching_and_capacity.md`
**Proving Rigs**: Rig C

**What it solves**: Sterling can model time-consuming steps, prefer batch-efficient sequences, and avoid dead schedules.

**Formal signature**: Actions with duration and possible resource occupancy; objective includes time; state includes clocks or remaining-time fields; optionally parallel machines/slots; constraints on concurrency.

**Key gates**: Duration modeling, batch optimization, schedule feasibility, resource occupancy tracking.

---

### P04: Multi-Strategy Acquisition (Alternative Methods, Different Failure Modes)

**File**: `primitives/P04_multi_strategy_acquisition_alternative_methods_different_fai.md`
**Proving Rigs**: Rig D

**What it solves**: Sterling can choose among strategies, not just sequences, and adapt the strategy prior from experience.

**Formal signature**: Multiple operator families reach the same subgoal; costs differ; some operators have preconditions from external availability; learning updates "which strategy works here."

**Key gates**: Strategy selection over sequence selection, adaptive strategy priors, external availability handling.

---

### P05: Hierarchical Planning (Macro Policy over Micro Solvers)

**File**: `primitives/P05_hierarchical_planning_macro_policy_over_micro_solvers.md`
**Proving Rigs**: Rig E

**What it solves**: Sterling chooses the right high-level route/plan structure while delegating low-level details; failures feed back into macro costs.

**Formal signature**: Two (or more) abstraction layers; macro nodes represent regions/waypoints/contexts; micro controller handles local execution; macro edges invoke sub-solvers; costs incorporate execution feedback.

**Key gates**: Abstraction layer separation, failure feedback propagation, hierarchical cost integration.

---

### P06: Goal-Conditioned Valuation under Scarcity (Keep/Drop/Allocate)

**File**: `primitives/P06_goal_conditioned_valuation_under_scarcity_keep_drop_allocate.md`
**Proving Rigs**: Rig F

**What it solves**: Sterling's choices reflect priorities; it can explain what it sacrificed and why.

**Formal signature**: Constrained capacity (slots, budget, attention); objective is utility under current goals; value model can shift with goals; learning updates item/action valuations.

**Key gates**: Priority-reflective choices, sacrifice explanation, goal-conditioned value updates.

---

### P07: Feasibility under Constraints and Partial-Order Structure

**File**: `primitives/P07_feasibility_under_constraints_and_partial_order_structure.md`
**Proving Rigs**: Rig G

**What it solves**: Sterling avoids impossible sequences and learns stable partial orders that reduce rework.

**Formal signature**: Operators have nontrivial preconditions (support, dependency, reachability); some steps can commute; solution is a partially ordered plan; execution chooses a valid linearization.

**Key gates**: Impossible sequence avoidance, partial order learning, linearization validity.

---

### P08: Systems Synthesis (Compose Components to Satisfy a Behavioral Spec)

**File**: `primitives/P08_systems_synthesis_compose_components_to_satisfy_a_behavioral.md`
**Proving Rigs**: Rig H

**What it solves**: Sterling can search a design space, not just a trajectory space; it can reuse motifs and detect near-misses.

**Formal signature**: State is a partial design; operators add components; evaluation function checks behavior/spec satisfaction (deterministic simulator if possible); goal is "spec holds."

**Key gates**: Design space search, motif reuse, near-miss detection, spec satisfaction verification.

---

### P09: Contingency Planning with Exogenous Events

**File**: `primitives/P09_contingency_planning_with_exogenous_events.md`
**Proving Rigs**: Rig L

**What it solves**: Sterling anticipates predictable external transitions and chooses actions that remain safe under them.

**Formal signature**: Edges include chosen actions and forced transitions; state includes timeline/hazard triggers; goal includes survivability or invariant preservation; plan may be a policy (conditional branches).

**Key gates**: External transition anticipation, safety under forced transitions, conditional policy generation.

---

### P10: Risk-Aware Planning (Tail Risk, Not Just Expected Cost)

**File**: `primitives/P10_risk_aware_planning_tail_risk_not_just_expected_cost.md`
**Proving Rigs**: Rig D, Rig M

**What it solves**: Sterling prefers robust plans when stakes are high, and it can trade speed for safety in a principled way.

**Formal signature**: Stochastic outcomes; cost is distributional (e.g., chance constraints P(failure) < ε, CVaR); state includes risk budget; learning updates failure likelihoods.

**Key gates**: Robust plan preference, speed-safety tradeoffs, distributional cost handling, failure likelihood updates.

---

### P11: Epistemic Planning (Belief-State and Active Sensing)

**File**: `primitives/P11_epistemic_planning_belief_state_and_active_sensing.md`
**Proving Rigs**: Rig I, Rig N

**What it solves**: Sterling can decide what to measure next, not only what to do next.

**Formal signature**: Nodes represent beliefs (prob distributions or hypothesis sets); edges are probes/tests; transitions update beliefs; goal is confidence threshold or hypothesis collapse; cost is probe expense + risk.

**Key gates**: Belief representation, probe/test selection, belief updates, confidence thresholds, information-cost tradeoffs.

---

### P12: Invariant Maintenance (Non-Terminal Goals; Control-by-Receding-Horizon)

**File**: `primitives/P12_invariant_maintenance_non_terminal_goals_control_by_receding.md`
**Proving Rigs**: Rig J

**What it solves**: Sterling moves from reactive to proactive: it schedules upkeep to prevent emergencies.

**Formal signature**: State includes invariant metrics; drift dynamics; actions restore metrics; goal is to keep invariants within bounds over time (often solved repeatedly as MPC/receding horizon).

**Key gates**: Proactive scheduling, invariant metric maintenance, drift compensation, receding horizon control.

---

### P13: Irreversibility and Commitment Planning

**File**: `primitives/P13_irreversibility_and_commitment_planning.md`
**Proving Rigs**: Rig I

**What it solves**: Sterling avoids premature irreversible steps; it sequences "verify before commit" actions.

**Formal signature**: Some actions are irreversible or have large rollback cost; objective includes option value; planner must delay commitment until evidence threshold; constraints encode one-way doors.

**Key gates**: Premature commitment avoidance, verify-before-commit sequencing, option value preservation, evidence thresholds.

---

### P14: Program-Level Planning (Search over Compressed Representations)

**File**: `primitives/P14_program_level_planning_search_over_compressed_representation.md`
**Proving Rigs**: Rig H

**What it solves**: Sterling stays tractable by searching in a compressed space; it can justify template choice and parameterization.

**Formal signature**: Plan is a structured program (templates/modules/parameters); edges refine program; compilation maps program → concrete actions; correctness requires compilation validity + constraint satisfaction.

**Key gates**: Compressed space search, template justification, parameterization validity, compilation correctness.

---

### P15: Fault Diagnosis and Repair (Hypotheses → Tests → Fix)

**File**: `primitives/P15_fault_diagnosis_and_repair_hypotheses_tests_fix.md`
**Proving Rigs**: Rig N

**What it solves**: Sterling chooses discriminative tests first; minimizes time-to-isolation; validates fixes.

**Formal signature**: Hypothesis set; test operators reduce entropy; repair operators modify system; goal is "fault isolated + fix validated"; learning ranks tests by information gain.

**Key gates**: Discriminative test selection, time-to-isolation minimization, fix validation, information gain ranking.

---

### P16: Representation Invariance and State Canonicalization

**File**: `primitives/P16_representation_invariance_and_state_canonicalization.md`
**Proving Rigs**: Rig A, Rig B, Rig C, Rig E, Rig F, Rig G, Rig H

**What it solves**: Hash stability under permutations/symmetries and micro-variance; node expansion bounded by caps without losing reachability relevant to goals.

**Formal signature**: `Canonicalizer(state_raw, config) -> state_canon, state_digest`; `EquivalenceReducer(state_canon) -> eq_class_id`; `Hash(state_canon)` stable under irrelevant variation.

**Key gates**: Hash stability under symmetry, micro-variance tolerance, bounded expansion, reachability preservation.

**Why it matters**: Generalization hinges on canonical state hashing, count-capping, symmetry reduction, and "equivalence under irrelevant variation." Without this, every domain becomes brittle and memory-hungry.

---

### P17: Credit Assignment Tied to Execution, Not Plans

**File**: `primitives/P17_credit_assignment_tied_to_execution_not_plans.md`
**Proving Rigs**: Rig A, Rig C, Rig D, Rig E, Rig F, Rig G, Rig I, Rig J, Rig M

**What it solves**: Priors update only from executed outcomes; failure attribution targets responsible segment.

**Formal signature**: `ExecutionReport(step_id, outcome, evidence_digest)`; `CreditAssigner(trace, execution_reports) -> updated_priors`; Constraint: no update on unexecuted plans.

**Key gates**: Execution-only updates, failure attribution, no plan-success reinforcement.

**Why it matters**: Separating hypothesized plans from verified outcomes prevents reinforcing "planned success" instead of "executed success."

---

### P18: Multi-Objective Optimization and Preference Articulation

**File**: `primitives/P18_multi_objective_optimization_and_preference_articulation.md`
**Proving Rigs**: Rig C, Rig D, Rig F, Rig J, Rig L, Rig M

**What it solves**: Trade-offs are explicit and auditable; weight changes move choices predictably.

**Formal signature**: `ObjectiveVector: {time, risk, resource_burn, disruption, ...}`; `PreferenceModel: weights or Pareto policy`; Planner returns either scalarized optimum with declared weights or Pareto frontier subset.

**Key gates**: Explicit trade-off documentation, predictable weight sensitivity, Pareto handling.

**Why it matters**: Real-world planning is rarely single scalar cost. You need Pareto handling (time vs risk vs resource burn vs disruption) and a way to surface trade-offs.

---

### P19: Audit-Grade Explanations (Why This Plan, Why Not That Plan)

**File**: `primitives/P19_audit_grade_explanations_why_this_plan_why_not_that_plan.md`
**Proving Rigs**: Rig A, Rig B, Rig C, Rig D, Rig E, Rig F, Rig G, Rig H, Rig I, Rig J, Rig L, Rig M, Rig N

**What it solves**: A third party can replay and understand: why this plan, why not another, under what constraints.

**Formal signature**: `ExplanationBundle: constraints, legality checks, competing alternatives, evidence citations`; Deterministic rendering from trace + artifacts.

**Key gates**: Third-party replay, constraint documentation, alternative comparison, evidence citation.

**Why it matters**: If Sterling is a proving ground for trustworthy reasoning, you want structured rationales: which constraints bound the choice, which evidence updated beliefs, which alternatives were rejected and why.

---

### P20: Adversarial Robustness / "Rule Injection" Hardening

**File**: `primitives/P20_adversarial_robustness_rule_injection_hardening.md`
**Proving Rigs**: Rig A, Rig B, Rig D

**What it solves**: Malformed or malicious rules are rejected; no silent semantics change.

**Formal signature**: `UntrustedDomainInput -> ValidatedDomainSpec (schema/version gated)`; Boundedness constraints (caps, cost bounds, operator count); SecurityPolicy: deny-by-default; allowlist semantics.

**Key gates**: Malformed input rejection, silent semantics prevention, deny-by-default policy, boundedness enforcement.

**Why it matters**: Many domains are client-defined at solve time. You need input validation, boundedness, and "no untrusted semantics" guarantees.

---

### P21: Entity Belief Maintenance and Saliency under Partial Observability

**File**: `primitives/P21_entity_belief_maintenance_and_saliency.md`
**Proving Rigs**: Rig I-ext, plus a second structurally different transfer surface

**What it solves**: Maintaining a bounded, replayable belief model over persistent entities from intermittent, noisy observations, and surfacing only meaningful deltas to downstream cognition/planning.

**Formal signature**: TrackSet with track_id, class belief distribution, kinematic belief (pose + uncertainty), recency bucket, visibility mode, and derived appraisal scores.

**Key gates**: Bounded belief maintenance, noisy observation handling, meaningful delta surfacing, replayability.

**Why it matters**: This defines the perception → belief → delta contract for environments with partial observability and entity persistence.

---

### P22: Perceptual Substrate + Visual Realization

**File**: `primitives/P22_perceptual_substrate_and_visual_realization.md`
**Proving Rigs**: Rig I-ext, Rig N, transfer validation lane

**What it solves**: Makes multimodal perception and media realization first-class governed substrates with strict two-stage authority and verify-then-commit.

**Formal signature**: `sense -> observation evidence`; `commit -> percept state`; `derive intent -> realize candidate -> verify -> commit`.

**Key gates**: Contract separation, evidence binding, replay determinism from recorded evidence, transfer across structural fixture domains, fail-closed verification.

**Why it matters**: Enables perception/generation capabilities without importing model ontology into Sterling's authoritative state semantics.

---

## Quick Reference: Primitive Categories

### Core Planning (P01-P03)
| ID | Name | Key Capability |
|----|------|----------------|
| P01 | Deterministic Transformation | Minimal-cost planning with learning |
| P02 | Capability Gating | Fail-closed legality, capability subgoals |
| P03 | Temporal Planning | Duration, batching, resource occupancy |

### Strategy & Structure (P04-P08)
| ID | Name | Key Capability |
|----|------|----------------|
| P04 | Multi-Strategy Acquisition | Strategy selection, adaptive priors |
| P05 | Hierarchical Planning | Macro-micro decomposition |
| P06 | Goal-Conditioned Valuation | Priority under scarcity |
| P07 | Feasibility & Partial Order | Dependency handling, linearization |
| P08 | Systems Synthesis | Design space search |

### Uncertainty & Risk (P09-P13)
| ID | Name | Key Capability |
|----|------|----------------|
| P09 | Contingency Planning | Exogenous event handling |
| P10 | Risk-Aware Planning | Tail risk, distributional cost |
| P11 | Epistemic Planning | Belief state, active sensing |
| P12 | Invariant Maintenance | Proactive upkeep scheduling |
| P13 | Irreversibility Planning | Verify-before-commit |

### Abstraction & Diagnosis (P14-P15)
| ID | Name | Key Capability |
|----|------|----------------|
| P14 | Program-Level Planning | Compressed representation search |
| P15 | Fault Diagnosis | Discriminative testing, fix validation |

### Governance & Trust (P16-P22)
| ID | Name | Key Capability |
|----|------|----------------|
| P16 | Representation Invariance | Canonical hashing, symmetry |
| P17 | Credit Assignment | Execution-grounded updates |
| P18 | Multi-Objective Optimization | Pareto handling, trade-offs |
| P19 | Audit-Grade Explanations | Replayable rationales |
| P20 | Adversarial Robustness | Input validation, deny-by-default |
| P21 | Entity Belief Maintenance | Partial observability tracking |
| P22 | Perceptual Substrate + Visual Realization | Governed perception + verify-then-commit realization |

---

## See Also

- [I/O Contract](00_IO_CONTRACT.md) - Rig input/output schemas
- [Capability Axes](../../reference/world_design/capability_axes.md) - 10 orthogonal axes for world design *(advisory)*
