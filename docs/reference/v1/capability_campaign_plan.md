# Capability Campaign Plan and Acceptance Gates

Status: active
Owner: TBD
Date: 2026-02-05
Updated: 2026-02-08
Version: 0.4

## Purpose

Define the next capability campaign for Sterling with a concrete, auditable path from EscapeGame-centric certification to multi-domain proof-carrying progress. The central thesis remains unchanged: progress is only real when carried by proof-carrying artifacts, deterministic replay, and promotion gates, not persuasive outputs.

This document translates that thesis into an implementable plan, acceptance gates, and a sequencing strategy that minimizes proof plumbing drift across domains.

## Scope

In scope
- WordNet Tiered integration into the unified benchmark (first non-EscapeGame certification-grade surface)
- Rome (All Links Lead to Rome) as a long-horizon graph reuse world with fixture-bound offline pass
- Phase C Tier-1 signal worlds (Mastermind, Slippery Gridworld, Poisoned Curriculum, Transactional KV Store)
- Unified World Harness as the cross-cutting engineering move to reduce drift

Out of scope
- New research directions not tied to certification evidence
- Tooling beyond what is required to meet acceptance gates

## Implementation Status (as of 2026-02-08)

This section tracks the current state of each campaign component against its acceptance gates. Updated after each audit pass.

### Core Artifact Schemas — COMPLETE

All five shared evidence schemas are implemented and production-ready in `core/proofs/benchmark_artifacts.py`:

| Schema | Status | Location |
|--------|--------|----------|
| `BoundInputsV1` | Done | `core/proofs/benchmark_artifacts.py` (lines 20-52) |
| `OutcomeRowsV1` | Done | `core/proofs/benchmark_artifacts.py` (lines 59-82) |
| `ReplayTraceV1` | Done | `core/proofs/benchmark_artifacts.py` (lines 90-106) |
| `MetricsBundleV1` | Done | `core/proofs/benchmark_artifacts.py` (lines 114-132) |
| `ToolTranscriptV1` | Done | `core/proofs/benchmark_artifacts.py` (lines 140-164) |

Supporting infrastructure:

| Schema | Status | Location |
|--------|--------|----------|
| `SolverContractIR` | Done | `core/proofs/fixture_ir.py` (lines 388-446) |
| `SolverContractBundleV1` | Done | `core/proofs/fixture_ir.py` (lines 457-520) |
| `FixtureIRV1` | Done | `core/proofs/fixture_ir.py` |
| `FixtureManifestV1` | Done | `core/induction/fixture_manifest.py` |

### A. WordNet Tiered — CERT-READY (was ~98%)

#### What exists

| Component | Status | Location |
|-----------|--------|----------|
| Fixture manifest with content hashes | Done | `benchmarks/wordnet_navigation_v2/canonical_tasks.json` (30 tasks) |
| Task pool (3000 tasks) with deterministic seeding | Done | `benchmarks/wordnet_navigation_v2/tasks_pool_3000.json` |
| `FixtureIRV1.from_task()` with content hashes | Done | `core/proofs/fixture_ir.py` |
| `FixtureManifestV1` with manifest hash | Done | `core/induction/fixture_manifest.py` |
| Per-task outcome fields in runner code | Done | `scripts/eval/benchmark/domains/wordnet_tiered.py` |
| WordNet kernel with deterministic neighbors | Done | `core/kernels/wordnet.py` |
| Task handler (goal predicate, success extractor) | Done | `core/tasks/wordnet_navigation.py` |
| Policy config with tiers and required proofs | Done | `configs/benchmark_domains_policy.yaml` (`domains.wordnet_tiered` section) |
| `OutcomeRowsV1` creation + deterministic `outcome_hash` | Done | `scripts/eval/benchmark/domains/wordnet_tiered.py` `run_wordnet_tiered_benchmark()` (latency excluded from normative rows) |
| `BoundInputsV1` artifact creation (suite-level) | Done | `scripts/eval/benchmark/types.py` `BenchmarkSuite.to_dict()` (requires `--wordnet-tier-max-steps`) |
| Solver contract bundle (per-tier budgets) | Done | `scripts/eval/benchmark/fixtures.py` `collect_domain_tasks()` (emitted when `--wordnet-tier-max-steps` provided) |
| Replay verification (inproc, `--verify-replay`) | Done | Verified 2026-02-08: 180 tasks, determinism confirmed |
| `GeneratorContractV1` sidecar with code hash | Done | `benchmarks/wordnet_navigation_v2/tasks_pool_3000_generator_contract.json` |
| `generator_contract_hash` wired into `BoundInputsV1` | Done | `collect_domain_tasks` loads sidecar, passes to suite `to_dict()` |
| Tier assignment from fixture spec (not recomputed) | Done | Runner now uses pool metadata `tier` field and `metadata.tiers` ranges |
| Contract bundle fail-closed in cert mode | Done | Raises `RuntimeError` if any domain lacks solver contract |

#### What is missing

| Gap | Priority | Notes |
|-----|----------|-------|
| (none — all acceptance gate items resolved) | — | — |

#### Acceptance gate checklist: WordNet-CERT v1

- [x] **Bound inputs**
  - [x] Fixture manifest hash present in run output
  - [x] Generator contract includes code hash, dataset artifact hash, extraction pipeline hash
  - [x] Solver contract bundle hash present (when `--wordnet-tier-max-steps` provided)
- [x] **Execution matches binding**
  - [x] Executed fixture IDs match manifest (tier assignment from fixture spec)
  - [x] Tier assignment in fixture spec (`difficulty` field in `canonical_tasks.json`)
- [x] **Normative outputs**
  - [x] Per-task outcomes include `task_id`, `fixture_content_hash`, `success`, `cost`
  - [x] Deterministic `outcome_hash` computed via `OutcomeRowsV1`
- [x] **Determinism**
  - [x] `replay_verification_hash` verified (2026-02-08)

### B. Rome Extended — CERT-READY (was ~98%)

All critical gaps from the v0.2 audit have been resolved. The previous "outcome_rows serialization bug" was a false positive — outcome_rows were present in output all along; the audit queried the wrong JSON path.

#### What exists

| Component | Status | Location |
|-----------|--------|----------|
| KG generator with deterministic seeding, opaque IDs | Done | `core/worlds/rome_kg.py` |
| Task fixtures (180 tasks, 18 configs x 10 episodes) | Done | `data/benchmarks/rome_extended_tasks_v1.json` |
| Per-task fields: task_id, kg_config_hash, kg_structure_hash, start/goal, budget | Done | In fixture file |
| Search runner with oracle separation | Done | `core/benchmarks/rome_demo.py` |
| Unified benchmark integration | Done | `scripts/eval/benchmark/domains/rome_extended.py` |
| `OutcomeRowsV1` creation + deterministic `outcome_hash` | Done | `scripts/eval/benchmark/domains/rome_extended.py` `run_rome_extended_benchmark()` (timing stripped from normative rows) |
| `BoundInputsV1` artifact creation (suite-level) | Done | `scripts/eval/benchmark/types.py` `BenchmarkSuite.to_dict()` |
| Fixture manifest hash in proofs section | Done | Present in output JSON |
| Unique fixture_id validation | Done | `scripts/eval/benchmark/domains/rome_extended.py` `run_rome_extended_benchmark()` |
| Policy config with required surfaces | Done | `configs/benchmark_domains_policy.yaml` (`domains.rome_extended` section) |
| Benchmark output artifact (360 tasks, 100% success) | Done | `output/benchmarks/unified_benchmark_rome_extended_cert_rerun1.json` |
| Replay verification (inproc, `--verify-replay`) | Done | Verified 2026-02-08: 360 tasks, determinism confirmed |
| `GeneratorContractV1` sidecar with code hash | Done | `data/benchmarks/rome_extended_tasks_v1_generator_contract.json` |
| `generator_contract_hash` wired into `BoundInputsV1` | Done | `collect_domain_tasks` loads sidecar, passes to suite `to_dict()` |
| Frozen graph edge-list snapshot | Done | `data/benchmarks/rome_extended_tasks_v1_graph_snapshot.json` (corridor + hub) |
| `kg_structure_hash` uses sorted edge pairs | Done | `core/worlds/rome_kg.py:get_structure_hash()` now hashes `(source, target)` pairs |
| Snapshot validation in cert mode runner | Done | Runner validates regenerated graph against frozen snapshot |
| Contract bundle fail-closed in cert mode | Done | Raises `RuntimeError` if any domain lacks solver contract |

#### What is missing

| Gap | Priority | Notes |
|-----|----------|-------|
| (none — all acceptance gate items resolved) | — | — |

#### Acceptance gate checklist: ROME-CERT v1

- [x] **Bound inputs**
  - [x] Graph snapshot hash per task (`kg_structure_hash` in fixtures)
  - [x] Task specs hash (fixture_content_hash computed)
  - [x] Generator contract with code hash
- [x] **Normative outputs**
  - [x] Per-task outcome rows exist in code and output
  - [x] `outcome_hash` persisted in output (deterministic, timing excluded)
- [x] **Determinism**
  - [x] Replay determinism verified (2026-02-08)

### C. Unified World Harness — 0% (Design Only)

The campaign plan defines two minimal protocols (`WorldStep`, `EvidenceEmitter`) and a toggle-based configuration system. None of this has been implemented.

| Component | Status | Notes |
|-----------|--------|-------|
| `WorldStep` protocol (observation, action, transition witness) | Not started | Documented in this plan only |
| `EvidenceEmitter` protocol | Not started | Documented in this plan only |
| Toggle system (observability, stochasticity, poisoned data, tools) | Not started | |
| Existing domains use `WorldAdapter` protocol | Existing | `core/worlds/base.py` — different pattern from proposed harness |

### D. Phase C Tier-1 Signal Worlds — 25% (Mastermind complete)

| World | Axis | Status | Notes |
|-------|------|--------|-------|
| Mastermind | #2 Partial Observability | **Done** | Kernel, world adapter, runner, fixtures, policy entry. Bitset belief (1296-bit int), atomic APPLY_GUESS, minimax policy. |
| Slippery Gridworld | #3 Stochasticity | Not started | No stochastic transition kernel or seed set binding |
| Poisoned Curriculum | #4 Adversarial Robustness | Not started | No adversarial detection or quarantine mechanism |
| Transactional KV Store | #9 Safe Tool Execution | Not started | No plan/apply/verify lifecycle or tool transcript emission |

### Footguns and Pivots — Implementation Status

| Decision | Schema done? | Wired into runners? | Enforced? |
|----------|:---:|:---:|:---:|
| 1) Contract bundle instead of single suite contract | Yes | No | No |
| 2) Generator contracts bind code + datasets | Yes | Yes (sidecar) | Yes (fail-closed in cert mode; warn+degrade in dev) |
| 3) Stochastic evidence binds seed sets | Schema field exists | No consumers | No |
| 4) Quantize belief weights | N/A (bitset) | N/A (bitset) | Resolved by design (integer bitset, no floats) |
| 5) Unified World Harness as minimal contracts | Design only | No | No |
| 6) Standardize ToolTranscriptV1 | Yes | No domain emits it | No |
| 7) Declarative policy/promotion semantics | Policy YAML exists | MetricsBundleV1 exists | Partial |

### Priority Actions (updated 2026-02-08)

1. ~~**Critical (unblocks both WordNet + Rome certification)**~~ — ALL RESOLVED
   - ~~Wire `OutcomeRowsV1` creation into WordNet tiered runner~~ ✓
   - ~~Wire `BoundInputsV1` creation into both runners~~ ✓
   - ~~Wire replay verification protocol into both runners~~ ✓
   - ~~Fix Rome outcome_rows serialization bug~~ ✓ (was false positive)

2. ~~**High (certification quality)**~~ — ALL RESOLVED
   - ~~Emit generator contract from `generate_wordnet_tiered_pool.py` with code hash~~ ✓
   - ~~Emit generator contract from `generate_rome_extended_tasks_v1.py` with code hash~~ ✓
   - ~~Add frozen graph edge-list snapshot for Rome fixtures~~ ✓
   - ~~Add contract bundle fail-closed check in certification mode~~ ✓

3. ~~**Medium (robustness)**~~ — ALL RESOLVED
   - ~~Audit WordNet tier assignment: verify derivation from fixture spec, not recomputed~~ ✓ (was mismatched; fixed)
   - ~~Upgrade Rome `kg_structure_hash` to use sorted edge pairs, not degree sequence~~ ✓
   - ~~Add test coverage for WordNet and Rome certification paths~~ ✓ (22 new tests)

4. **Future (Phase C prerequisites)**
   - Implement Unified World Harness (WorldStep + EvidenceEmitter protocols)
   - Build Phase C worlds as harness configurations

---

## Principles

- Proof-carrying artifacts are mandatory. Every claim must be replayable and independently verifiable.
- Deterministic replay is non-negotiable for certification surfaces.
- Promotion gates must be explicit and fail-closed.
- Domains are proving grounds, not destinations.
- Avoid bespoke proof plumbing. Prefer a single, unified harness where possible.

## Governance-First Rebuild Constitution

This section restates Sterling as a governance-first reasoning system and captures the hard-earned lessons as non-negotiables, architectural invariants, and early sequencing so domain-agnostic, long-horizon properties are native rather than retrofitted.

### First-Class Choices

- Governance kernel as the center of gravity. The primary output is governed artifacts, not model text.
- Explicit state hierarchy. Utterance, world, and search position are distinct types with explicit contracts.
- Append-only episode graph. Every decision is reconstructible from immutable nodes and edges.
- Typed operator contracts. Preconditions and effects are executable, typed, and enforced.
- World-task separation. Worlds own parsing/state/operators; tasks own goals/success criteria.
- Canonical feature extraction. One source of truth with versioned modes used across training and inference.
- Deterministic identity. Hash-based identities for artifacts, operators, and certificates.
- Explicit bridge costs. Cross-domain transitions are computed, not looked up.
- Strict invariants with modes. Fail-closed in production, warn in shadow, off only for perf-critical.
- Value target contract. Targets are versioned and hash-verified, not quietly mutable.

### Non-Negotiable Capabilities

- Full trace auditability. Every decision is reconstructible from the episode graph.
- Deterministic replay. Same inputs and versions yield identical outcomes.
- Governance tests as gates. No hidden routers, oracle separation, and value target integrity are enforced by tests.
- Oracle separation. Admissible features are explicit and validated before inference.
- State invariants. Phrase structure only in syntax, latent frozen, semantic IR DAG validity enforced.
- KG registry isolation. State copy cost is O(semantic_delta), KG access is hash-verified.
- Backward-compatible contracts. Core schemas are stable; breaking changes require major versions.
- Evidence-based policy. Prior influence artifacts are recorded, hashed, and replay-verifiable.

### Architectural Invariants

- Reducer purity. Reducers are pure with respect to declared inputs; externalities are explicit artifacts or forbidden.
- Determinism at commit boundaries. Stochastic exploration is allowed only before artifact commitment.
- Fail-closed by default. Missing or unverifiable inputs produce governed failure artifacts.
- Operator identity is hashable. Operator code version, args, and contract digest are captured.
- Bounded search is mandatory. Every run produces a budget-consumption artifact.
- Memory is governed evidence. Retrieval and summarization are operators with traceable sources.

### Hard-Earned Lessons

- Hidden routing appears unless routing is a first-class artifact in the episode graph.
- Invariants prevent policy drift more than any planner improvement.
- Feature drift destroys cross-domain transfer; canonical extraction is mandatory.
- Performance issues concentrate in state copy and KG access, not just search.
- Bridges are decisions under cost, not lookups.
- Determinism is a prerequisite for governance, not a nice-to-have.
- Tasks are governance objects; goals and success criteria must be explicit functions.
- Value learning without contracts yields untrustworthy policies.
- Replay verification catches drift that metrics hide.
- Domain agnosticism comes from interfaces, not content.

### Early Sequencing (Avoid Retrofits)

Phase 0: Kernel invariants before capability
- Artifact model and content addressing
- Deterministic reducer contract
- Normative vs debug surfaces
- Failure taxonomy
- Budget model

Phase 1: Operator calculus and state graph substrate
- Typed operator interface and digests
- State graph representation with strict reducer semantics
- Minimal planner loop under budgets

Phase 2: Certification and replay as definition of done
- Evidence packet format
- Replay verifier
- Promotion rules

Phase 3: Two intentionally different domains
- One symbolic graph navigation domain
- One execution or embodied domain

## Footguns and Pivots (Lock These Now)

This section captures the highest-leverage interface decisions that will prevent EscapeGame-shaped refactors when the system goes multi-domain, stochastic, and tool-realistic.

### 1) Contract bundle instead of a single suite contract

Risk
- A single suite-level solver contract becomes invalid as soon as multiple domains or tier-specific budgets exist.

Pivot
- Define and bind a contract bundle as a first-class object.
- `suite_contract_bundle = {domain_id -> contract_hash_or_contract_object}`
- With tiers: `{domain_id: {tier_id -> contract_hash}}`
- Certification mode must fail-closed if a domain emits results without a contract entry.

Implementation status (2026-02-07)
- `SolverContractBundleV1` schema: DONE (`core/proofs/fixture_ir.py`)
- Fail-closed enforcement in runners: NOT DONE

### 2) Generator contracts must bind code and datasets, not names

Risk
- Hashing "generator_version" and params is insufficient; code or dependency drift will silently break replay.

Pivot
- Generator identity must include a content hash of generator source or a repo tree hash with paths.
- Dataset identity must bind extracted artifacts (adjacency snapshots), not just a label like "WordNet 3.1."
- Extraction pipeline identity must be hashed (script plus pinned dependencies), or vendor the extraction output.
- Prefer fixture manifests for WordNet and Rome; reserve generator contracts for Phase C harness worlds.

Implementation status (2026-02-08)
- Fixture manifests with content hashes: DONE
- Generator contract with code hash: DONE (`GeneratorContractV1` schema + sidecar files for both WordNet and Rome)
- Fail-closed enforcement in certification mode: DONE (`_load_generator_contract_hash()` in `scripts/eval/benchmark/fixtures.py`; orchestrator re-raises in cert mode)

### 3) Stochastic evidence must bind seed sets, not single seeds

Risk
- "Distributional evidence" is meaningless unless the seed set is cryptographically bound.

Pivot
- Fixtures include `scenario_id`, `seed_set_id`, explicit seed lists, repetitions, and aggregation function.
- Outcomes include per-seed results or a hash of per-seed logs.
- Certification mode fails closed if seed set binding is missing.

Implementation status (2026-02-07)
- `execution_seed_set_id` field exists in `ReplayTraceV1`: DONE
- No domain currently emits stochastic evidence with bound seed sets: NOT DONE

### 4) Quantize belief weights to avoid cross-machine drift

Risk
- Entropy and information gain computed with floats will drift across platforms.

Pivot
- Use fixed-point or quantized belief weights in the belief ledger.
- Compute entropy and IG on the quantized representation.
- Record quantization scheme in the solver or belief-ledger contract.

Implementation status (2026-02-08)
- RESOLVED by design: Mastermind uses integer bitset belief (no floats). Belief hash is `sha256(int.to_bytes(...))`, fully deterministic and cross-platform stable. No quantization needed.

### 5) Unified World Harness should be minimal contracts, not a god-object

Risk
- Over-abstracting creates a framework that makes simple domains hard and locks in wrong semantics.

Pivot
- Define minimal protocols:
  - `WorldStep`: observation, action, transition witness.
  - `EvidenceEmitter`: produces BoundInputs, OutcomeRows, ReplayTrace, MetricsBundle, and optional ToolTranscript.
- Toggling is configuration layered on top, not separate code paths.
- Test: Mastermind should be implementable in roughly one file without harness core edits.

Implementation status (2026-02-07)
- Design documented here: DONE
- Implementation: NOT STARTED
- Existing domains use `WorldAdapter` protocol (different pattern)

### 6) Standardize tool transcripts once

Risk
- Rome Pass 2, KV Store, and HTTP sandbox worlds diverge into incompatible tool evidence.

Pivot
- Define `ToolTranscriptV1` once and require all tool-using domains to emit it.
- Include canonical request/response fields, stable error taxonomy, retry metadata, deterministic ordering, and payload witness hashes.

Implementation status (2026-02-07)
- `ToolTranscriptV1` schema: DONE (`core/proofs/benchmark_artifacts.py`)
- Domain emission: NOT DONE (no domain currently produces tool transcripts)

### 7) Keep policy and promotion semantics declarative and uniform

Risk
- Each domain invents custom validators and subtly redefines "targets," fragmenting governance.

Pivot
- Use a declarative policy schema where targets map to canonical metrics fields.
- Require a `MetricsBundleV1` with a stable schema version for every domain.
- Track Evaluation Gates (EVAL-01/02/03) alongside domain gates in every benchmark artifact.

Implementation status (2026-02-07)
- Declarative policy YAML: DONE (`configs/benchmark_domains_policy.yaml`)
- `MetricsBundleV1` schema: DONE
- Evaluation gate tracking in benchmark artifacts: PARTIAL (policy defines gates, runner doesn't emit them consistently)

## Core Artifact Schemas (Lock This Week)

These schemas are the shared evidence surface for all upcoming domains. Each domain implements producers, and runners only aggregate.

- `BoundInputsV1` (manifest hash or generator plus dataset hash or contract bundle hash) — DONE
- `OutcomeRowsV1` plus deterministic `outcome_hash` — DONE
- `ReplayTraceV1` plus `replay_verification_hash` — DONE
- `MetricsBundleV1` (domain-agnostic keys plus optional domain extension block) — DONE
- `ToolTranscriptV1` (even if unused by a domain) — DONE

## Campaign Overview

### A. WordNet Tiered (WordNet-CERT v1)

Objective
- Prove the unified pipeline is truly multi-domain, not EscapeGame-shaped.

Rationale
- WordNet is structurally aligned with current strengths (deterministic graph search and audit primitives).
- Policy already defines a "WordNet Tiered Navigation" suite with tiers and explicit targets.

Minimum implementation targets
- Fixture binding
  - Preferred: emit fixture specs into a manifest and bind the manifest hash.
  - Alternative: bind a generator contract that includes generator code hash, dataset artifact hash, and extraction pipeline hash.
- Per-task outcomes and deterministic `outcome_hash`
  - Mirror EscapeGame's per-fixture outcome logging.
  - Aggregate metrics are insufficient without falsifiable per-task rows.
- Replay verification
  - Produce `suite_identity`, `result_hash`, `replay_verification_hash` (policy minimum).
  - Still produce per-task normative surface for audits.
- Contract bundle
  - Bind a per-domain or per-tier contract bundle hash, not a single suite contract.
  - Fail closed if a domain emits results without a contract entry.

Key files
- `benchmarks/wordnet_navigation_v2/canonical_tasks.json` — 30 validated tasks (10 easy, 10 medium, 10 hard)
- `scripts/data/generate_wordnet_tiered_pool.py` — deterministic task generator
- `core/kernels/wordnet.py` — WordNet kernel with sorted neighbors and distance caching
- `core/tasks/wordnet_navigation.py` — task handler
- `scripts/eval/benchmark/domains/wordnet_tiered.py` — runner
- `configs/benchmark_domains_policy.yaml` — policy (lines 461-523)

Acceptance gate: WordNet-CERT v1
- Bound inputs
  - Fixture manifest (or generator contract) hash present.
  - Generator contract includes code hash, dataset artifact hash, and extraction pipeline hash.
  - Solver contract bundle hash present if budgets or tie-breaks materially affect outcomes.
- Execution matches binding
  - Executed fixture IDs exactly match the manifest.
  - Tier assignment derivable from fixture spec, not recomputed ad hoc.
- Normative outputs
  - Per-task outcomes include `task_id`, `fixture_content_hash`, `success`, `cost`.
  - Deterministic `outcome_hash` computed over per-task rows.
- Determinism
  - `replay_verification_hash` verified.

### B. Rome (ROME-CERT v1)

Objective
- Establish a long-horizon graph reuse world with fixture-bound evidence.

Rationale
- Rome/Wikipedia is valuable for route learning and landmark formation if instrumented.
- It should be staged to preserve certification integrity before adding tool-call realism.

Pass 1 (offline, cert-friendly)
- World: frozen link graph snapshot as a fixture artifact.
- Task: start node, target node ("Rome"), constraints (max hops, banned nodes).
- Evidence: path, expansions, failure_reason, plus graph snapshot hash commitment.

Pass 2 (tool realism, not necessarily certification reference yet)
- Treat "fetch outgoing links" as an external tool call with transcripts.
- Use this to bridge toward axis #9 semantics, but do not make it the primary #9 proving ground.

Key files
- `core/worlds/rome_kg.py` — KG generator (RomeKG, RomeRegime, RomeKGConfig)
- `core/benchmarks/rome_demo.py` — search runner with oracle separation
- `data/benchmarks/rome_extended_tasks_v1.json` — 180 tasks (18 configs x 10 episodes)
- `scripts/eval/benchmark/domains/rome_extended.py` — runner
- `scripts/data/generate_rome_extended_tasks_v1.py` — task generator
- `configs/benchmark_domains_policy.yaml` — policy (lines 791-902)
- `output/benchmarks/unified_benchmark_rome_extended_cert_rerun1.json` — latest run (360 tasks, 100% success)

Known bugs
- ~~Outcome rows serialization~~ (RESOLVED 2026-02-08): False positive from v0.2 audit. `OutcomeRowsV1` is created and serialized correctly. The audit queried the wrong JSON path.

Acceptance gate: ROME-CERT v1
- Bound inputs
  - Graph snapshot hash and task specs hash.
  - If generator contracts are used, bind generator code hash and extraction pipeline hash.
- Normative outputs
  - Per-task outcome rows and deterministic `outcome_hash`.
- Determinism
  - Replay determinism verified.
  - For tool pass, transcripts must be replayable.

### C. Phase C Tier-1 Signal Worlds

Objective
- Close capability-axis gaps with minimal proving grounds and hard pass/fail validators.

Rationale
- Phase C is the next capability push per roadmap.
- Tier-1 signal worlds lock axes #2, #3, #4, #9.

Cross-cutting move: Unified World Harness
- Implement the harness with four toggles
  - Observability
  - Stochasticity
  - Poisoned data
  - External tools
- Each Tier-1 world is a configuration of the harness.
- Certification machinery remains uniform and auditable.

#### C.1 Mastermind (Axis #2 Partial Observability) — IMPLEMENTED

Key files
- `core/kernels/mastermind.py` — Kernel with bitset belief (1296-bit int bitmask), feedback, prune, minimax/expected-size guess selection
- `core/worlds/mastermind.py` — WorldAdapter (zero base protocol mutations), DeltaObservationIR emission, prediction verification
- `scripts/eval/benchmark/domains/mastermind.py` — Runner with generic step loop (future harness extraction point), cert-mode fail-closed
- `data/benchmarks/mastermind_fixtures_v1.json` — File-backed fixtures (20 tasks, seed=42, 4p×6c). Returns `BOUND` binding verdict in cert mode.
- `tests/unit/test_mastermind_kernel.py` — 43 tests (enumeration, feedback, belief, prune, determinism, guess selection, belief-never-zero)
- `tests/unit/test_mastermind_world_adapter.py` — 25 tests (protocol conformance, observations, predictions, capabilities)
- `configs/benchmark_domains_policy.yaml` — Policy entry (`domains.mastermind`)

Design decisions
- Single atomic operator (APPLY_GUESS): feedback + prune in one transition. Avoids coupling hazard of separate MAKE_GUESS + PRUNE.
- Bitset belief representation: `int` bitmask over all possible codes, hashed via `sha256(int.to_bytes(...))`. No float drift.
- Policy selection (minimax/expected-size) is a policy decision, not an operator edge. Recorded in evidence, not as StateGraph edge.
- Belief ledger as per-step rows embedded in existing runner output, not a new top-level artifact type.
- Runner uses generic step loop pattern: `state = kernel.initial_state(input); while not goal and budget: action = policy(state); transition = kernel.apply(state, op, action); state = transition.post_state`. This is the extraction point for the future Unified World Harness.

P11 (Epistemic Planning) contract — artifact-checkable, narrow scope
- **Invariant**: Belief set size is non-increasing after each probe. (NOT "entropy must decrease" — uninformative probes are valid and expected.)
- **Goal**: Secret identified (belief is singleton) within budget.
- **Checkable artifacts**: Per-step belief_size in outcome rows; runner validates non-increasing invariant in certification mode.
- **v1 does NOT require**: optimal guess counts, information-gain citations, or entropy calculations. Those are future extensions.

Acceptance gate (non-negotiable)
- Belief ledger exists as per-step rows in runner output (belief_size per step).
- Belief set size is non-increasing after probes (validated in certification mode; raises RuntimeError on violation).
- Belief never prunes to zero (contradiction detection raises RuntimeError in kernel).
- Goal reached within budget (success = belief is singleton).
- Cert mode uses file-backed fixtures (`data/benchmarks/mastermind_fixtures_v1.json`) returning `binding_verdict="BOUND"` with `dataset_artifact_hash`. Implicit generation in cert mode is fail-closed.
- Runner propagates governance errors in cert mode (no swallowing into `result.errors`).
- Bitset representation eliminates float drift; guess scoring uses integer sum-of-squares. No quantization scheme needed.

#### C.2 Slippery Gridworld (Axis #3 Stochasticity)

Acceptance gate
- Certification commits to record, not environment.
- Replay reproduces observed transitions from seeds and witnesses.
- Certificates commit to distributional evidence over a seed set, not a single run.
- Seed sets are bound fixtures with explicit lists and aggregation definitions.

#### C.3 Poisoned Curriculum (Axis #4 Adversarial Robustness)

Acceptance gate
- Learning loop is treated as the attack surface.
- System refuses to promote poisoned heuristics or only promotes with explicit scoped validity that blocks adversarial slices.
- Quarantine or revocation is demonstrable in artifacts.

#### C.4 Transactional KV Store (Axis #9 Safe Tool Execution)

Acceptance gate
- Explicit plan/apply/verify/rollback lifecycle.
- No phantom success.
- Tool transcripts are evidence artifacts.
- Failure modes are classified in stable, hash-distinct categories (timeout, conflict, permission denial).

## Policy and Promotion Semantics

- EscapeGame remains the certification reference world.
- WordNet and Rome are stable domains with meaningful audit surfaces.
- Phase C worlds must yield hard pass or fail signals for their target axes.
- Eligibility and targets-met remain distinct; promotion gates should enforce this separation.
- Use a declarative policy schema that maps targets to canonical `MetricsBundleV1` fields.
- Track Evaluation Gates (EVAL-01/02/03) alongside domain gates in every benchmark artifact.

## Sequencing and Dependencies

Ordered by leverage
1. WordNet Tiered integration
  - Fixture provider or generator-contract manifest with code and dataset binding
  - Per-task outcome rows and deterministic `outcome_hash`
  - Replay verification in unified benchmark
  - Contract bundle hash and fail-closed enforcement
2. Rome offline fixture-bound world
  - Frozen graph snapshot and task fixtures
  - Per-task outcomes and deterministic `outcome_hash`
  - Replay determinism
  - ToolTranscriptV1 for Pass 2
3. Unified World Harness
  - Implement toggles first
  - Configure Mastermind
  - Then Slippery Gridworld, Poisoned Curriculum, Transactional KV Store

## Deliverables

- WordNet-CERT v1
  - Fixture manifest or generator contract hash
  - Per-task outcomes and `outcome_hash`
  - Replay verification
- ROME-CERT v1
  - Frozen graph snapshot and task fixtures
  - Per-task outcomes and `outcome_hash`
  - Replay verification
- Unified World Harness
  - Toggle-based configuration with consistent certification artifacts
- Phase C Tier-1 Worlds
  - Mastermind, Slippery Gridworld, Poisoned Curriculum, Transactional KV Store
  - Each with acceptance gates above and cert-grade artifacts

## Open Questions

- Where should fixture manifests and generator contracts live, and what schema should they use?
  - Partial answer (2026-02-07): `FixtureManifestV1` exists in `core/induction/fixture_manifest.py`. Generator contracts need a new `GeneratorContractV1` schema.
- What minimal solver contract hash is required for WordNet and Rome to be considered bound?
  - Partial answer (2026-02-07): `SolverContractIR` and `SolverContractBundleV1` exist. Need to define which budget/tie-break fields are material per domain.
- Which evidence fields are mandatory for each acceptance gate to be verifiable by the validator?
  - See acceptance gate checklists above (updated 2026-02-07).
- How will promotion gates be encoded in policy to enforce "eligibility vs targets met" across all domains?
  - Open. Policy YAML defines gates per domain but runner doesn't emit evaluation gate pass/fail consistently.

## Schema Checklist (Appendix)

This is a minimum field checklist for shared artifact schemas. It is intentionally small and stable; domain-specific extensions should live under a namespaced extension block.

All schemas below are IMPLEMENTED in `core/proofs/benchmark_artifacts.py`.

BoundInputsV1
- `suite_id`
- `suite_identity_hash`
- `contract_bundle_hash`
- `fixtures_manifest_hash` or `generator_contract_hash`
- `dataset_artifact_hash` (if generator contract used)
- `extraction_pipeline_hash` (if generator contract used)
- `schema_version`

OutcomeRowsV1
- `task_id`
- `fixture_id`
- `fixture_content_hash`
- `success`
- `cost`
- `outcome_hash` (computed over all rows)
- `schema_version`

ReplayTraceV1
- `replay_verification_hash`
- `trace_log_hash`
- `execution_seed_set_id` (if stochastic)
- `schema_version`

MetricsBundleV1
- `metrics_schema_version`
- `domain_id`
- `summary_metrics` (domain-agnostic keys)
- `domain_extension` (optional, namespaced)

ToolTranscriptV1
- `tool_call_id`
- `request` (canonicalized)
- `response` (canonicalized)
- `error` (taxonomy, stable codes)
- `attempt_index`
- `backoff_reason` (if retry)
- `payload_witness_hash` (for large payloads)
- `schema_version`
