---
authority: architecture
status: imported
source: tmp/capability_primitives_bundle
version: v0.4
---
# Capability Primitive 17: Credit assignment tied to execution, not plans

Status: v0.3-draft  
Primary proving rigs: Rig A, Rig C, Rig D, Rig E, Rig F, Rig G, Rig I, Rig J, Rig M  
Last updated: 2026-01-31
## 1. Problem shape

you already saw the trap: reinforcing “planned success” instead of “executed success.” This is a general capability: separating hypothesized plans from verified outcomes, then updating priors correctly.

Critical boundary: this spec defines semantics and certifiable gates. It does not mandate a particular algorithm; implementations may vary as long as determinism, boundedness, typed operator rules, and auditability constraints are satisfied.
## 2. Formal signature

- ExecutionReport(step_id, outcome, evidence_digest)
- CreditAssigner(trace, execution_reports) -> updated_priors
- Constraint: no update on unexecuted plans
## 3. What must be proven

- Priors update only from executed outcomes; failure attribution targets responsible segment
## 4. Minimal substrate requirements

4.1 Canonical state requirements
- Canonical hashing stable under irrelevant variation (ordering, micro-variance, symmetries as applicable).
- Bounded representation: caps, bucketization, eviction, or compression that preserves decision-relevant information.
- Deterministic iteration order for all collections used in hashing or expansion.

4.2 Operator requirements
- Typed operator schema with preconditions/effects and declared costs.
- Fail-closed legality: if legality cannot be proven from (state, operator), treat as illegal.
- Learning may reorder operators or update cost estimates only if modeled explicitly, and must not alter transition semantics.

4.3 Trace + audit requirements
- Trace bundle includes: canonical state digests, operator digests, legality decisions, and explanation bundle.
- Credit assignment is execution-grounded (updates require execution reports).
## 5. Certification gates

Signature gates
- Determinism: identical request payload + version + config -> identical trace hash.
- Boundedness: caps enforced; search stays within declared node/edge limits.
- Validity: all operator applications preserve invariants (no illegal transitions).

Performance gates (in-domain)
- Convergence: repeated episodes reduce expansions/time without regressing correctness.
- Stability: learning changes ordering/priors only; reachability and legality outcomes are unchanged.

Transfer gates (out-of-domain)
- Re-run the same primitive against a second, structurally different surface domain.
- Prove the same gates and preserve determinism/audit invariants.
## 6. Measurement and telemetry

Telemetry to log (minimum)

- request_id, primitive_id, rig_id, schema_version
- budgets (node/edge/time) and actual consumption
- trace_bundle hash, explanation bundle hash
- outcome status and failure category (if any)

Key metrics (primitive-specific)

- Determinism: identical request payload + version + config -> identical trace hash.
- Boundedness: caps enforced; search stays within declared node/edge limits.
- Validity: all operator applications preserve invariants (no illegal transitions).
- Convergence: repeated episodes reduce expansions/time without regressing correctness.
- Stability: learning changes ordering/priors only; reachability and legality outcomes are unchanged.
- Re-run the same primitive against a second, structurally different surface domain.
- Prove the same gates and preserve determinism/audit invariants.
## 7. Transfer envelope

- automation systems, agents operating with unreliable actuators/APIs, robotics, ops runbooks.
## 8. Known footguns to avoid

- Implicit state: legality or costs depend on external mutable facts not represented in the canonical state.
- Non-deterministic ordering: iteration over maps/sets changes results or hashes across runs.
- Unboundedness: state or operator sets can grow without caps, causing search blow-up and non-replayable traces.
- Learning changes semantics: priors/ordering leak into preconditions/effects and alter reachable set.
- Plan-success reinforcement: updating priors when a plan is found (rather than executed).
- Ambiguous objectives: implicit weights hidden in heuristics instead of explicit objective representation.
## 9. Rig interface notes (I/O contract fragments)

Inputs (minimum)
- `state_canon` + `state_digest`
- `goal` (typed goal predicate / objective vector)
- `operator_set` (typed, validated)
- `config` (budgets, objective weights, determinism settings)
- Optional: `observations/evidence` for epistemic or diagnosis primitives

Outputs (minimum)
- `solution_path` (operator invocations) or `policy` for contingency primitives
- `trace_bundle` (content-addressed artifacts, determinism witness)
- `explanation_bundle` (constraints, alternatives, evidence citations)
- `metrics` (expansions, cost, objective vector, success/failure reasons)
