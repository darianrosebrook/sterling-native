---
authority: architecture
status: imported
source: tmp/capability_primitives_bundle
version: v0.4
---
# Sterling Rig Harness I/O Contract (Draft)

Status: v0.1-draft  
Last updated: 2026-01-31

This contract defines the *shape* of inputs/outputs between a rig harness (e.g., Minecraft bot, toy domains) and Sterling (reasoning service). The goal is: deterministic replay, typed artifacts, and audit-grade explanations.

## 1. SolveRequestV1 (minimum fields)

- request_id: stable unique ID
- primitive_id: integer (1..N)
- rig_id: string (e.g., "Rig A", "Rig I-ext")
- schema_version: semantic version string for this payload schema
- config:
  - budgets: node_cap, edge_cap, wall_clock_ms, max_depth
  - objective: scalar weights or declared preference model
  - determinism: seed policy, ordering keys, hash algorithm version
- domain_spec:
  - operators: typed operator definitions (validated)
  - invariants: typed constraints (validated)
  - canonicalization: bucketization/caps/symmetry rules
- state:
  - state_canon: canonical state payload (JSON)
  - state_digest: content hash of canonical state
- goal:
  - goal_predicate or objective vector, plus thresholds
- observations (optional):
  - evidence_batch_digest
  - evidence_batch payload for epistemic/diagnosis/entity-belief primitives
- context (optional):
  - agent condition (health, loadout), environment parameters, etc.

## 2. SolveResponseV1 (minimum fields)

- request_id
- status: success | no_solution | budget_exhausted | invalid_input | error
- solution:
  - path: ordered operator invocations (or policy graph for contingency)
  - cost: scalar and/or objective vector
- trace_bundle:
  - trace_hash
  - artifacts: list of content-addressed references (states, operators, evidence, explanations)
  - determinism_witness: ordering keys, seeds used, config digest
- explanation_bundle:
  - constraints_considered
  - legality_gates_triggered
  - top_alternatives_rejected (with reason codes)
  - evidence_citations (artifact refs)
- metrics:
  - nodes_expanded, edges_expanded, time_ms, replay_ok

## 3. Streaming / incremental updates (optional)

For partial observability rigs, a harness may send incremental evidence updates:
- EvidenceUpdateV1: request_id, t, evidence_item, sensor_meta, digest
Sterling may respond with:
- BeliefDeltaV1: track/belief deltas, saliency events, active_sense_requests

## 4. Compatibility rules

- Fail-closed: invalid operator schemas or missing required fields must return `invalid_input`.
- Determinism: all hashes computed over canonicalized, sorted representations.
- Version gating: schema_version and operator schema IDs must be checked before any solve.
