---
authority: architecture
status: imported
source: tmp/capability_primitives_bundle
version: v0.4
---
# Capability Primitive 21: Entity belief maintenance and saliency under partial observability

Status: v0.3-draft  
Primary proving rigs: Rig I-ext, plus a second structurally different transfer surface  
Last updated: 2026-01-31
## 1. Problem shape

This primitive defines a domain-agnostic reasoning capability: maintaining a bounded, replayable belief model over persistent entities from intermittent, noisy observations, and surfacing only meaningful deltas to downstream cognition/planning.

Critical boundary: this primitive defines the perception → belief → delta contract. It does **not** mandate a particular filtering algorithm (Kalman, particle, heuristic). The requirements are determinism, boundedness, saliency semantics, and auditability.
## 2. Formal signature

Belief state (canonical, bounded)

- TrackSet: bounded set of track hypotheses. Each track includes:
  - track_id (internal stable ID; external IDs are treated as evidence, not ground truth)
  - class belief distribution (includes unknown)
  - kinematic belief (pose + uncertainty, represented as covariance or bounded error)
  - recency bucket (last_seen tier), visibility mode {visible, inferred, lost}
  - derived appraisal: threat_score / opportunity_score (continuous but bucketed for canonical state), with declared weights
- AttentionBudget: explicit compute + sensing budgets (modeled as state, not hidden heuristics)
- Context: agent condition relevant to appraisal (e.g., health, goals, inventory, risk budget)
- Optional derived summaries: compressed hazard/opportunity regions (bounded list), not full grids

Observations

- EvidenceBatch: time-stamped evidence items with sensor metadata (FOV/LOS flags, distance, occlusion markers) and association features.

Operators (typed, deterministic)

- TRACK_UPDATE(EvidenceBatch): deterministic association + fusion producing a new TrackSet
- DECAY(dt): deterministic belief decay (pose uncertainty grows; class confidence drifts toward unknown; transitions visible → inferred → lost)
- SALIENCY_DIFF(prev, next): emits a bounded set of typed deltas with hysteresis + cooldown semantics
- ACTIVE_SENSE_REQUEST(query): emits a request to perception/control (turn, scan sector, move to vantage, reacquire)
- FIELD_SYNTHESIZE(): optional; produces compressed hazard/opportunity regions from TrackSet

Objectives (explicit, multi-objective)

- Safety invariants: preserve risk budgets / survival constraints over time
- Task goals: navigation, acquisition, interaction objectives
- Resource minimization: sensing cost and cognitive load minimization (avoid per-tick “interpret everything”)
## 3. What must be proven

- Persistent identity under intermittent observations (occlusion → reappearance associates to the same track; no “novel spawn” spam)
- Saliency gating (stable scenes emit ~0 deltas after warmup; only meaningful changes propagate)
- Active sensing as first-class action (lost high-threat track triggers reacquire queries)
- Uncertainty honesty (unobserved tracks decay; no frozen certainty)
- Risk propagation without LLM-per-tick (navigation/planning consumes compressed risk summaries)
- Execution-grounded updates (penalize tracking only for preventable misses; avoid noisy blame for unobservable causes)
- Anti-ID reliance (tracking works under external ID noise/absence)
## 4. Minimal substrate requirements

4.1 Canonical state requirements

- Deterministic ordering and quantization: all collections iterated for hashing must be ordered; continuous values bucketed.
- Boundedness: TrackSet cap, bounded hazard region list, deterministic eviction policy.
- Separation of audit provenance from state equivalence:
  - Evidence/provenance ring buffers live in trace bundles.
  - Canonical planning state hashes must exclude raw evidence IDs and unbounded provenance (to avoid state explosion).

4.2 Operator requirements

- Typed operator schema with explicit preconditions/effects and declared costs.
- Fail-closed legality: if legality cannot be proven from (state, operator), treat as illegal.
- Saliency operators must include hysteresis and cooldown policy to prevent threshold jitter event storms.
- Learning may update ordering/prioritization/cost models only if modeled explicitly; transition semantics must remain unchanged.

4.3 Trace + audit requirements

- Trace bundle includes: canonical state digests, operator digests, legality decisions, emitted deltas, and explanation bundle.
- Credit assignment is execution-grounded:
  - negative updates require post-hoc execution evidence (e.g., death/damage plus pre-death belief state showing preventability).
  - survival is not treated as positive reinforcement by default.
## 5. Certification gates

Signature gates

- Determinism: identical evidence stream + config + version → identical (TrackSet hash, delta stream, trace hash).
- Boundedness: TrackSet.size <= TRACK_CAP; deltas/tick <= MAX_EVENTS_PER_TICK after warmup; hazard region list bounded.
- Separation: raw detections never directly create tasks; only SALIENCY_DIFF outputs can trigger cognition/planning.
- Uncertainty honesty: when unobserved beyond threshold, unknown confidence increases and pose uncertainty grows monotonically.

Anti-cheat / discriminative gates

- Anti-ID reliance: under ID perturbation/absence harness, association accuracy remains above baseline.
- Event stability: near-threshold oscillations do not cause repeated flip-flop deltas (hysteresis required).

Execution / learning gates

- Preventability guard: penalize tracking only when pre-death evidence supports a preventable miss (track existed + high confidence + risk budget violation).
- Learning stability: learning may adjust prioritization/costs only; legality and reachability outcomes must not change.

Transfer gates

- Re-run the same primitive on a second, structurally different surface domain and preserve determinism, boundedness, sparsity, and honesty gates.
## 6. Measurement and telemetry

Telemetry to log (minimum)

- request_id, primitive_id, rig_id, schema_version
- budgets (node/edge/time) and actual consumption
- trace_bundle hash, explanation bundle hash
- outcome status and failure category (if any)

Key metrics (primitive-specific)

- Determinism: identical evidence stream + config + version → identical (TrackSet hash, delta stream, trace hash).
- Boundedness: TrackSet.size <= TRACK_CAP; deltas/tick <= MAX_EVENTS_PER_TICK after warmup; hazard region list bounded.
- Separation: raw detections never directly create tasks; only SALIENCY_DIFF outputs can trigger cognition/planning.
- Uncertainty honesty: when unobserved beyond threshold, unknown confidence increases and pose uncertainty grows monotonically.
- Anti-ID reliance: under ID perturbation/absence harness, association accuracy remains above baseline.
- Event stability: near-threshold oscillations do not cause repeated flip-flop deltas (hysteresis required).
- Preventability guard: penalize tracking only when pre-death evidence supports a preventable miss (track existed + high confidence + risk budget violation).
- Learning stability: learning may adjust prioritization/costs only; legality and reachability outcomes must not change.
## 7. Transfer envelope

- Robotics/drones: multi-target tracking with active sensing and hazard fields (compressed regions)
- Security monitoring: actor tracking across intermittent camera coverage (stable scenes emit ~0 deltas)
- Interactive systems: session/user-intent tracking with sparse event gating
- Infrastructure diagnosis: services/incidents as entities; probes as evidence; decay by staleness; diagnostic queries as active sensing
## 8. Known footguns to avoid

- Treating external entity IDs as truth (makes the rig non-discriminative).
- Mixing audit provenance into canonical state hashes (state-space blow-up; replay equivalence breaks).
- Full-grid risk fields in planning state (hash churn; transfer friction). Prefer compressed regions.
- Implicit attention (hidden heuristics) instead of explicit AttentionBudget.
- Event storms from jitter near thresholds (omit hysteresis/cooldowns and you reintroduce spam via deltas).
- Rewarding “lucky survival” as positive signal (reinforces accidental behavior).
- Hidden routers/bypass paths where raw detections become tasks (recreates observation spam under a new name).
## 9. Rig interface notes (I/O contract fragments)

SolveRequestV1 (typical)

- primitive_id: P21
- rig_id: (see primary proving rigs)
- domain_spec: operator schemas + legality rules (fail-closed)
- state: canonical state payload for this primitive
- goal: goal predicate / constraints
- observations: optional; include only if the primitive requires it
- context: objective weights / risk budgets / resource caps

SolveResponseV1 (typical)

- solution: plan / policy / query set (as applicable)
- trace_bundle: content-addressed trace with state/operator digests
- explanation_bundle: why-this/why-not-that deltas and invariants
- metrics: expansions, costs, budgets, gate results
