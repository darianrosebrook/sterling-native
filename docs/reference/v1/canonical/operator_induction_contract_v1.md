> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Operator Induction Contract v1

**Version**: 1.1
**Date**: 2026-02-17
**Status**: Canonical specification — sufficient to rebuild `core/induction/` from scratch.
**Scope**: Operator sketch lifecycle, synthesis, promotion, certification, and the 3-tier store.
**Layer**: 1 (Operators / Induction)

### Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2026-02-17 | Added HypothesisIR/PredictionIR/InvarianceWitnessIR section (§4A), added EpisodeSetV1 section (§6A), added Hypothesis Scoring section (§8A), added ArtifactRefV1 (§8.2), expanded source file index with 8 missing files |
| 1.0 | 2026-02-17 | Initial version |

---

## §1 Purpose

Sterling's induction system observes operator application patterns during reasoning episodes and proposes new operators. These induced operators progress through a 3-tier promotion pipeline — Shadow → Provisional → Production — with increasing evidence requirements at each gate. This document specifies the data structures, promotion contracts, and synthesis pipeline.

---

## §2 Core/Dossier Split (Footgun #1 Fix)

Every induced operator is split into two structures to prevent identity drift when accumulating evidence:

- **OperatorSketchCoreIR**: Hash-stable identity. Frozen. Hashed for identity.
- **OperatorSketchDossierIR**: Evolving evidence. Mutable. References core by hash.

**Invariant I-1**: Same core + different dossier evidence → same sketch_hash. The dossier never changes the operator's identity.

---

## §3 OperatorSketchCoreIR

Frozen dataclass — the canonical identity of an induced operator.

```
OperatorSketchCoreIR (frozen)
├── sketch_id: str                           # Content-derived: "sketch_" + hash[:12]
├── operator_name: str                       # Human-readable (e.g., "INDUCED_HYPERNYM_01")
├── preconditions: tuple[InferredClause, ...]  # Frozen tuple
├── effects: tuple[InferredClause, ...]        # Frozen tuple
├── parameter_slots: tuple[ParameterSlot, ...] # Frozen tuple
├── program_kind: str                        # "OPERATOR_PATTERN" | "TRANSFORMATION_RULE" | "ENTITY_RELATION"
├── program_body: dict[str, Any]             # Canonical program representation
├── parent_hypothesis_hash: str              # Provenance: hypothesis this came from
├── delta_pattern_hash: str                  # Provenance: pattern signature
└── _sketch_hash: str                        # Computed on __post_init__, cached
```

### §3.1 sketch_hash computation

```python
canonical_dict = {
    "operator_name": ...,
    "preconditions": [c.to_dict() for c in preconditions],
    "effects": [e.to_dict() for e in effects],
    "parameter_slots": [p.to_dict() for p in parameter_slots],
    "program_kind": ...,
    "program_body": canonicalize_program_body(program_kind, program_body),
    "parent_hypothesis_hash": ...,
    "delta_pattern_hash": ...,
}
sketch_hash = sha256(json.dumps(canonical_dict, sort_keys=True))
```

**Invariant I-2**: sketch_hash is deterministic — identical inputs produce identical hashes across runs.

### §3.2 sketch_id derivation

```python
sketch_id = f"sketch_{content_hash[:12]}"  # content_hash = same computation as sketch_hash
```

12 characters (not 8) to reduce collision probability in large runs.

### §3.3 Hash exclusions

The sketch_hash **never** includes: episode IDs, test results, witnesses, tier status, timestamps, or any dossier field.

Source: `core/induction/operator_sketch.py:102-212`

---

## §4 InferredClause

Precondition or effect clause inferred from delta observations.

```
InferredClause (frozen)
├── clause_type: str              # e.g., "requires_semantics", "modifies_polarity"
├── parameters: dict[str, Any]    # Clause-specific parameters
├── confidence: float             # 0.0–1.0 from cross-example validation
└── supporting_deltas: list[str]  # Delta hashes that support this clause
```

Canonical serialization: parameters are recursively sorted by key; floats rounded to 4 decimal places; lists sorted as strings.

Source: `core/induction/operator_sketch.py:34-70`

---

## §4A Hypothesis IR Types

Source: `core/induction/hypothesis.py`

### §4A.1 HypothesisStatus

```python
class HypothesisStatus(str, Enum):
    PROPOSED = "proposed"
    ELIGIBLE = "eligible"
    APPLIED = "applied"
    REJECTED = "rejected"
    REFINED = "refined"
```

### §4A.2 HypothesisIR

Frozen dataclass — a hypothesis proposed by the induction system.

```
HypothesisIR (frozen)
├── hypothesis_id: str              # Volatile (excluded from canonical hash)
├── program_kind: str               # e.g., "OPERATOR_PATTERN"
├── program_body: Dict[str, Any]    # Executable representation (canonical)
├── program_hash: str               # Semantic hash of program_body
├── schema_version: str = "hypothesis-v1.0"
├── evidence_fingerprints: Optional[tuple[str, ...]]  # Semantic hashes of evidence
├── status: HypothesisStatus = PROPOSED
├── confidence: float = 0.0
└── mdl_cost: Optional[float] = None
```

Methods: `to_canonical_dict()` (excludes volatile fields), `compute_canonical_hash()`, `compute_program_hash()`.

Factory: `create_hypothesis(program_kind, program_body, ...)` — derives `hypothesis_id` from `program_hash` if not provided.

### §4A.3 PredictionIR

Frozen dataclass — what a hypothesis predicts will happen.

```
PredictionIR (frozen)
├── prediction_id: str              # Volatile
├── hypothesis_fingerprint: str     # Semantic hash of hypothesis (not node ID)
├── claim: Dict[str, Any]           # What the hypothesis predicts
├── claim_hash: str                 # Semantic hash of claim
├── schema_version: str = "prediction-v1.0"
└── target_state_hash: Optional[str]
```

Factory: `create_prediction(hypothesis_fingerprint, claim, ...)`.

### §4A.4 InvarianceWitnessIR

Frozen dataclass — evidence of hypothesis falsification.

```
InvarianceWitnessIR (frozen)
├── witness_id: str                 # Volatile
├── hypothesis_fingerprint: str
├── invariant_name: str
├── counterexample_state_hash: str
├── violation_details: Dict[str, Any]
├── witness_kind: str = "FALSIFICATION"   # 3-way: INFRA_ERROR, APPLICABILITY, FALSIFICATION
└── schema_version: str = "witness-v1.2"  # v1.2 includes witness_kind in hash
```

**witness_kind** classification:
- `INFRA_ERROR`: Infrastructure failure (PROGRAM_LOAD_ERROR, PROGRAM_EXECUTION_ERROR, ENGAGEMENT_ERROR)
- `APPLICABILITY`: Hypothesis not applicable (EMPTY_PREDICTIONS, INSUFFICIENT_COVERAGE)
- `FALSIFICATION`: Hypothesis tested and found false (default)

Factory: `create_witness(...)` — auto-infers `witness_kind` from `invariant_name` via `WITNESS_KIND_BY_INVARIANT` mapping.

---

## §5 ParameterSlot

```
ParameterSlot (frozen)
├── slot_name: str       # e.g., "target_entity", "relation_type"
├── slot_type: str       # e.g., "ENTITY", "RELATION", "SYNSET"
├── required: bool       # Whether parameter must be bound (default True)
└── default_value: Any?  # Default if not required
```

Source: `core/induction/operator_sketch.py:73-94`

---

## §6 OperatorSketchDossierIR

Mutable evidence envelope. References core by `sketch_hash`.

```
OperatorSketchDossierIR (mutable)
├── sketch_hash: str                          # Foreign key to OperatorSketchCoreIR
│
│  ── Evidence (grows over time) ──
├── supporting_episodes: list[str]            # Episode IDs
├── supporting_episode_hashes: list[str]      # Cryptographic commitments ("sha256:..." or fallback)
├── test_results: list[dict]                  # TestResultIR dicts
├── witnesses: list[dict]                     # InvarianceWitnessIR dicts
│
│  ── Committed Payload (set at synthesis time) ──
├── committed_payload_content_hash_v1: str?
├── committed_payload_bytecode_hash_v1: str?
├── committed_payload_implementation_hash: str?
├── committed_payload_dsl_version: str?
├── committed_payload_world_id: str?
│
│  ── Performance Tracking ──
├── scenario_improvements: list[ScenarioImprovement]
├── regression_checks: list[RegressionCheck]
│
│  ── Tier Progression ──
├── tier_transitions: list[TierTransition]
├── current_tier: "shadow" | "provisional" | "production"
├── certification_ref: str?                   # TD-12/MS artifact reference
├── sandbox_report_ref: str?                  # SandboxReport artifact reference (K4)
│
│  ── Metadata ──
├── created_at: datetime
└── last_updated: datetime
```

### §6.1 Episode Evidence

```python
def add_episode(episode_id: str, episode_hash: str? = None) -> None
```

If `episode_hash` is provided (preferred), normalized to `"sha256:<64-hex>"` format.
If not provided, fallback: `"fallback_episode_id_sha256:" + sha256(episode_id)`.

**Invariant I-3**: In certifying/production paths, each entry in `supporting_episode_hashes` MUST be a real content hash (`"sha256:<64-hex>"`). Fallback hashes are allowed only in dev/non-strict flows.

### §6.2 Committed Payload

Set at synthesis time (not install time). The loader verifies these fields against the actual payload — it does NOT recompile from sketch.

### §6.3 Pass Rate

```python
def get_pass_rate() -> float:
    passed = sum(1 for tr in test_results if tr["outcome"] == "PASS")
    return passed / len(test_results) if test_results else 0.0
```

### §6.4 Semantic Dict

`to_semantic_dict()` excludes timestamps and operational metadata, producing a hashable representation for closure hashing. Fields included: sketch_hash, sorted episodes/hashes, payload commitments, test results, witnesses, scenario improvements (no timestamps), regression checks (no timestamps), tier transitions (no timestamps), current_tier, certification_ref, sandbox_report_ref.

Source: `core/induction/operator_sketch.py:220-671`

---

## §6A Episode Set Infrastructure

Source: `core/induction/episode_set.py`

### §6A.1 EpisodeRecordV1

```
EpisodeRecordV1
├── episode_id: str
├── world_name: str
├── task_type: str
├── initial_state: Dict[str, Any]
├── goal_spec: Dict[str, Any]
├── expected_difficulty: str = "medium"
└── metadata: Dict[str, Any] = {}
```

`compute_episode_hash()` returns 16-character hex hash (excludes metadata).

### §6A.2 EpisodeSetV1

```
EpisodeSetV1
├── schema_version: str = "1.0"
├── episode_set_id: str
├── world_name: str
├── task_type: str
├── seed: int = 42
├── episode_count: int
├── generated_at: str
├── content_hash: str              # SHA-256 of content
├── train_episode_ids: List[str]   # 80% split
├── eval_episode_ids: List[str]    # 20% split
└── episodes: List[EpisodeRecordV1]
```

**Constants**: `DEFAULT_SEED = 42`, `TRAIN_SPLIT_RATIO = 0.80`.

Functions: `generate_episode_set()`, `save_episode_set()`, `load_episode_set(path, verify_hash=True)`, `compute_episode_set_hash()`.

### §6A.3 PoolBundle

Frozen dataclass for immutable episode pool references.

```
PoolBundle (frozen)
├── entries: tuple       # Frozen for hashability
├── digest: str          # SHA-256 hex of canonicalized entries
├── ref: str             # Logical name
└── version: str
```

---

## §7 Tier Progression Records

### §7.1 TierTransition

```
TierTransition
├── from_tier: "shadow" | "provisional" | "production"
├── to_tier: "shadow" | "provisional" | "production"
├── timestamp: datetime
├── reason: str
└── certification_ref: str?    # TD-12 cert if promotion
```

### §7.2 ScenarioImprovement

```
ScenarioImprovement
├── scenario_id: str
├── baseline_score: float
├── improved_score: float
├── episode_id: str
└── timestamp: datetime
```

### §7.3 RegressionCheck

```
RegressionCheck
├── episode_id: str
├── passed: bool
├── baseline_comparison: float
├── timestamp: datetime
└── details: dict?
```

Source: `core/induction/operator_sketch.py:221-250`

---

## §8 3-Tier Promotion System

### §8.1 Tier Definitions

| Tier | Name | Evidence Required | Execution Rights |
|------|------|-------------------|------------------|
| 0 | Shadow | Registered sketch + dossier | Influence search ranking only (TC-7A) |
| 1 | Provisional | 70%+ pass rate, invariance witnesses | Limited execution in sandbox |
| 2 | Production | 90%+ pass rate, TD-12 certificate, regression checks | Full execution as registered operator |

### §8.2 PromotionDecisionRecord

Frozen dataclass — immutable record of a promotion event. Certificates certify artifacts; decision records certify events (ID-Event-1).

```
PromotionDecisionRecord (frozen)
├── certificate_ref: ArtifactRefV1      # Reference to certificate (not embedded)
├── certificate_hash: str
├── sketch_hash: str
├── policy_hash: str
├── closure_hash: str
├── from_tier: "shadow" | "provisional"
├── to_tier: "provisional" | "production"
├── reason_code: str                    # e.g., "ORCHESTRATOR_PROMOTION"
├── timestamp: str                      # ISO timestamp (event time)
├── threshold_checks: dict[str, Any]    # episodes_count, pass_rate, etc.
├── mdl_cost: float                     # MDL cost at promotion time
├── schema_id: str                      # "sterling.promotion_decision.v1"
├── schema_version: str                 # "1"
└── promotion_token_hash: str?          # Audit only
```

Source: `core/induction/promotion_decision.py`

### §8.2 ArtifactRefV1

Source: `core/induction/artifact_closure.py`

```python
@dataclass(frozen=True)
class ArtifactRefV1:
    schema_id: str          # e.g., "sterling.delta_pack.v1"
    schema_version: str     # Extracted from schema_id
    content_hash: str       # e.g., "sha256:abc123..."
    locator: Optional[str] = None  # Backend-specific path (excluded from identity)
```

Identity is `(schema_id, schema_version, content_hash)` — `locator` is not part of identity.

---

## §8A Hypothesis Scoring

Source: `core/induction/hypothesis_scoring.py`

The scoring system evaluates hypotheses across multiple dimensions, producing a transparent `HypothesisScorecardV1`.

### §8A.1 Component Summaries

| Summary | Key Fields | Purpose |
|---------|-----------|---------|
| `MDLBreakdownV1` | `L_struct`, `L_params`, `L_ex`, `mdl_total` | Minimum Description Length cost |
| `FitSummaryV1` | `n_pass`, `n_fail`, `n_unknown`, `pass_rate`, `fit_score` | Evidence fit quality |
| `StabilitySummaryV1` | `k_window`, `pass_rate_recent`, `pass_rate_drop`, `unstable` | Rolling stability |
| `UtilitySummaryV1` | `estimated_search_cost_reduction`, `estimated_solution_rate_gain` | Expected value |
| `NoveltySummaryV1` | `explained_observation_fraction`, `redundancy_penalty` | Novelty vs redundancy |
| `EligibilitySummaryV1` | Hard gates: `has_tested_prediction`, `invariance_passed`, `determinism_ok`, `budget_ok`, `interpreter_available`, `safe_to_apply` | Go/no-go |

### §8A.2 DecisionV1

```
DecisionV1 (frozen)
├── kind: "REJECT" | "HOLD" | "TEST_NEXT" | "REFINE" | "PROMOTE" | "DEPLOY"
├── level: "INFO" | "WARN" | "ERROR"
├── reason_codes: List[str]      # IND/<CATEGORY>/<CODE> taxonomy
├── short_message: str
└── policy_version: str
```

### §8A.3 HypothesisScorecardV1

Derived artifact combining all scoring dimensions:

```
HypothesisScorecardV1 (frozen)
├── schema_version: "hyp_scorecard_v1.0"
├── hypothesis_program_hash: str
├── hypothesis_kind: str
├── evidence_fingerprints: List[str]
├── test_fingerprints: List[str]
├── witness_fingerprints: List[str]
├── eligibility: EligibilitySummaryV1
├── fit: FitSummaryV1
├── mdl: MDLBreakdownV1
├── stability: StabilitySummaryV1
├── novelty: NoveltySummaryV1
├── utility: UtilitySummaryV1
├── rank_score: float
├── rank_components: Dict[str, float]
├── decision: DecisionV1
└── debug: Optional[Dict[str, Any]]
```

### §8A.4 Reason Code Taxonomy

Format: `IND/<CATEGORY>/<CODE>`. Categories: `GATE` (eligibility), `FIT` (evidence quality), `STAB` (stability), `MDL` (parsimony), `NOV` (novelty), `UTIL` (utility), `LIFE` (lifecycle actions).

---

## §9 Promotion Gate

### §9.1 PromotionGateV1

Version-gated promotion enforcer. Default: v2 gate (fail-closed). Legacy v1 allowed only with explicit policy flag.

```python
class PromotionGateV1:
    def __init__(self, provenance_schema_version_required: int = 2,
                 legacy_v1_allowed: bool = False)
    def validate(core, dossier, certificate, *, policy_hash?, verify_pre_cert_ref?) -> PromotionGateResult
```

### §9.2 V2 Gate Requirements (fail-closed)

Required fields:
1. `parent_hypothesis_hash` (from core) — must be non-empty
2. `delta_pattern_hash` (from core) — must be non-empty
3. `supporting_episode_hashes` (from dossier) — must be non-empty, all `"sha256:"` prefixed
4. `pre_certificate_ref_v1` or `certificate_id` (from certificate) — must be `"sha256:"` prefixed; `"pending"` rejected
5. `closure_hash` (from certificate) — must be non-empty

### §9.3 PromotionGateResult

```
PromotionGateResult
├── ok: bool
├── error_code: str?        # V2_PROVENANCE_INCOMPLETE | V2_HASH_FORMAT | V1_REJECTED | ...
├── reasons: tuple[str, ...]
├── missing_fields: list[str]
└── invalid_fields: list[str]
```

### §9.4 V2 Ratchet

Once `provenance_schema_version_required >= 2` is deployed, v1 certificates are rejected unless `legacy_v1_allowed=True` during migration. This is an irreversible ratchet — once deployed, v1 cannot spread.

### §9.5 Verifier Integration

`validate_promotion_gate_v2_with_verifier()` uses the same `ProvenanceVerifier` as the loader (belt-and-suspenders — no duplicated logic paths).

In STRICT mode: if provenance chain construction fails, fail closed (no silent downgrade to fields-only).
In PERMISSIVE mode: fallback to fields-only verification.

Source: `core/induction/promotion_gate.py`

---

## §10 Operator Synthesis

### §10.1 Architecture

Synthesis converts `OperatorSketchCoreIR` into executable bytecode at **promotion time** (not install time). The certified bundle commits to the executable payload; the loader verifies and installs it without recompilation.

### §10.2 Bytecode DSL v1

**DSL Version**: `"operator_dsl/v1"`

#### Opcodes

| Opcode | Category | Operands | Semantics |
|--------|----------|----------|-----------|
| GRAPH_TRAVERSE | Graph | (edge_type, direction) | Traverse KG edge → neighbor nodes |
| GRAPH_TRAVERSE_SIBLING | Graph | () | Traverse to sibling via shared parent |
| APPLY_TRANSFORM | State | (transform_type, params) | Apply semantic transformation |
| SET_CURRENT_NODE | State | (node_id_source) | Set current KG node |
| CHECK_CONDITION | Condition | (condition_type, params...) | Check precondition → bool |
| CHECK_GOAL_REACHED | Condition | () | Check if goal node reached |
| EMIT_RESULT | Result | (state_ref) | Emit successor state |
| EMIT_EMPTY | Result | () | Emit empty result (precondition failed) |

New opcodes require a DSL version bump.

### §10.3 BytecodeInstruction

```
BytecodeInstruction (frozen)
├── opcode: Opcode
├── operands: tuple[Any, ...]
└── label: str?               # For jump targets (future extension)
```

### §10.4 OperatorBytecodeV1

```
OperatorBytecodeV1 (frozen)
├── dsl_version: str                          # Must equal "operator_dsl/v1"
├── operator_name: str
├── instructions: tuple[BytecodeInstruction, ...]
└── metadata: dict[str, Any]
```

`canonical_hash()` = `sha256(json.dumps(to_dict(), sort_keys=True))`

**Invariant I-4**: `dsl_version` must equal `DSL_VERSION` constant. Construction with a different version raises `ValueError`.

### §10.5 BytecodeCompiler

Compiles `OperatorSketchCoreIR.program_body` to `OperatorBytecodeV1`.

Supported program kinds:
- `OPERATOR_PATTERN`: Check conditions → traverse graph → emit result
- `TRANSFORMATION_RULE`: Check precondition → apply transformation → emit result
- `ENTITY_RELATION`: Check entity constraints → traverse relation → emit result

Unknown program kinds raise `UnsupportedConstructError` (fail-closed).

### §10.6 BytecodeInterpreter

Deterministic interpreter for `OperatorBytecodeV1`.

**Key invariants**:
- No ambient state (clock, RNG, filesystem, environment, global registries)
- Pure function: same bytecode + state + inputs → same output
- Fail-closed on unknown opcodes
- Strict mode for certifying/promotion paths (runtime errors are fatal)

Execution: sequential instruction processing with `ExecutionContext`:

```
ExecutionContext
├── state: StateNode           # Input state
├── args: dict[str, Any]       # Operator arguments
├── kernel: KernelProvider?    # World-specific execution seam
├── successors: list[StateNode]  # Output states
├── metadata: dict[str, Any]   # Execution metadata
└── condition_failed: bool     # Short-circuit flag
```

When `condition_failed` is set, subsequent instructions are skipped until `EMIT_RESULT`.

### §10.7 KernelProvider Protocol

World-specific execution seam for graph operations:

```python
class KernelProvider(Protocol):
    @property
    def world_id(self) -> str: ...
    def apply_operator(state, operator_name, args) -> StateNode?: ...
    def apply_transform(state, transform_type, params, args) -> StateNode?: ...
    def get_neighbors(state, edge_type) -> list[str]: ...
```

v1 implementation: `PNKernelProvider` (PN world only).

### §10.8 OperatorSynthesizer

Main entry point for Stage K operator promotion.

```python
class OperatorSynthesizer:
    def synthesize(sketch_core, sketch_dossier?, config?) -> SynthesisResult
```

Steps:
1. Compile sketch to bytecode via `BytecodeCompiler`
2. Compile InferredClauses to `Precondition` and `EffectAssertion` objects
3. Create `OperatorSignature`
4. Compute `implementation_hash` = `sha256(semantic_bytecode | dsl_version | signature_digest)`
5. Compute `bytecode_hash` = `bytecode.canonical_hash()`
6. Compute deterministic `operator_id` = `sha256(sketch_hash + config_hash + engine_version)`
7. Return `SynthesisResult`

### §10.9 SynthesisResult

```
SynthesisResult
├── success: bool
├── operator_id: str?
├── signature: OperatorSignature?
├── bytecode: OperatorBytecodeV1?
├── implementation_hash: str?
├── bytecode_hash: str?
├── dsl_version: str
├── world_id: str?
└── error: SynthesisError?
```

`to_committed_payload()` produces the dict stored in certified bundles:
```python
{
    "bytecode_v1": bytecode.to_dict(),
    "bytecode_hash_v1": bytecode_hash,
    "implementation_hash": implementation_hash,
    "dsl_version": dsl_version,
    "world_id": world_id,
}
```

### §10.10 Implementation Hash

The semantic implementation hash excludes non-deterministic metadata (sketch_id) to ensure stability across runs with identical semantic content:

```
implementation_hash = sha256(semantic_bytecode_json + "|" + DSL_VERSION + "|" + signature_json)
```

Where `semantic_bytecode_json` includes: dsl_version, operator_name, instructions, and deterministic metadata (program_kind, sketch_hash). Explicitly excludes sketch_id.

Source: `core/induction/operator_synthesizer.py`

---

## §11 Invariants Summary

1. **I-1**: Core/Dossier split — dossier changes never affect sketch_hash.
2. **I-2**: sketch_hash is deterministic across runs for identical inputs.
3. **I-3**: Production-path episode hashes must be real content hashes (`"sha256:<64-hex>"`), not fallbacks.
4. **I-4**: OperatorBytecodeV1 rejects mismatched DSL versions at construction.
5. **I-5**: BytecodeInterpreter has no ambient state — purely functional.
6. **I-6**: Synthesis occurs at promotion time. Loader installs committed payload, does not recompile.
7. **I-7**: V2 promotion gate is fail-closed — missing provenance fields block promotion.
8. **I-8**: V2 ratchet is irreversible — once deployed, v1 certificates cannot be created.
9. **I-9**: Unknown bytecode opcodes cause fail-closed behavior (empty result or RuntimeError in strict mode).

---

## §12 Related Documents

- [Operator Registry Contract](operator_registry_contract_v1.md) — Where promoted operators are registered
- [Governance & Certification](governance_certification_contract_v1.md) — Gate/verdict and TD-12 certificates
- [Proof & Evidence System](proof_evidence_system_v1.md) — Evidence bundles and replay verification
- [Hashing Contracts](hashing_contracts_v1.md) — Canonical JSON and hash conventions

---

## §13 Source File Index

| File | Defines |
|------|---------|
| `core/induction/operator_sketch.py` | OperatorSketchCoreIR, OperatorSketchDossierIR, InferredClause, ParameterSlot, TierTransition, ScenarioImprovement, RegressionCheck |
| `core/induction/operator_synthesizer.py` | OperatorSynthesizer, BytecodeCompiler, BytecodeInterpreter, OperatorBytecodeV1, BytecodeInstruction, Opcode, KernelProvider, SynthesisResult, ExecutionContext |
| `core/induction/promotion_gate.py` | PromotionGateV1, PromotionGateResult, validate_promotion_gate_v2 |
| `core/induction/promotion_decision.py` | PromotionDecisionRecord |
| `core/induction/program_canonicalization.py` | canonicalize_program_body |
| `core/induction/synthesis_config.py` | SynthesisConfigV1, compute_operator_id |
| `core/induction/hypothesis.py` | InvarianceWitnessIR |
| `core/operators/shadow_store.py` | ShadowOperatorStore (Tier 0) |
| `core/operators/certified_store.py` | CertifiedOperatorStore (Tier 1-2) |
| `core/operators/promotion_policy.py` | PromotionPolicy |
| `core/operators/promotion_service.py` | PromotionService |
| `core/induction/hypothesis_state.py` | HypothesisState container |
| `core/induction/hypothesis_scoring.py` | HypothesisScorecardV1, MDLBreakdownV1, FitSummaryV1, DecisionV1 |
| `core/induction/episode_set.py` | EpisodeSetV1, EpisodeRecordV1, PoolBundle |
| `core/induction/artifact_closure.py` | ArtifactRefV1, ArtifactClosureV1 |
| `core/induction/promotion_lane.py` | PromotionLane, FenceWitness |
| `core/induction/promotion_lane_events.py` | PromotionResult, lifecycle event types |
| `core/induction/promotion_blocker.py` | Promotion blocker detection |
| `core/induction/induction_session.py` | InductionSession (cross-episode state) |
| `core/induction/hypothesis_program.py` | HypothesisProgramError, program execution |
| `core/induction/hypothesis_proposer.py` | Hypothesis proposer pipeline |
| `core/induction/hypothesis_refiner.py` | Hypothesis refinement |
| `core/induction/hypothesis_selection.py` | Hypothesis selection policy |
| `core/induction/hypothesis_policy_translator.py` | Policy-to-hypothesis translation |
| `core/induction/hypothesis_build_failure.py` | Hypothesis build failure tracking |
| `core/induction/abc_comparison.py` | ABC comparison metrics |
| `core/induction/aging_policy.py` | Hypothesis aging and expiry |
| `core/induction/artifact_store.py` | Abstract artifact store interface |
| `core/induction/file_artifact_store.py` | File-backed artifact store |
| `core/induction/ms_artifact_store.py` | Memory-substrate artifact store |
| `core/induction/baseline_mode.py` | Baseline mode definitions |
| `core/induction/baseline_runner.py` | Baseline runner for evaluation |
| `core/induction/batch_induction_helpers.py` | Batch induction utilities |
| `core/induction/certificate_builder.py` | Certificate construction |
| `core/induction/certificate_verifier.py` | Certificate verification |
| `core/induction/certificates.py` | Certificate type definitions |
| `core/induction/closure_allowlist.py` | Closure allowlist enforcement |
| `core/induction/counting_prior_store.py` | Counting-based prior store |
| `core/induction/decision_policy.py` | Decision policy evaluation |
| `core/induction/delta_clustering.py` | Semantic delta clustering |
| `core/induction/delta_pack.py` | Delta packing for episodes |
| `core/induction/derivation_run_manifest.py` | Derivation run manifests |
| `core/induction/determinism_witness.py` | Determinism witness recording |
| `core/induction/dialogue_rollout_scenario.py` | Dialogue rollout scenario |
| `core/induction/e2e_certification_pipeline.py` | End-to-end certification pipeline |
| `core/induction/episode_attestation.py` | Episode attestation records |
| `core/induction/episode_commitment.py` | Episode commitment types |
| `core/induction/episode_induction_input.py` | Episode induction input format |
| `core/induction/episode_loader_helpers.py` | Episode loader helpers |
| `core/induction/episode_loader_induction.py` | Induction-specific episode loader |
| `core/induction/episode_manifest.py` | Episode manifest types |
| `core/induction/episode_split.py` | Episode splitting for train/test |
| `core/induction/evidence_weighting.py` | Evidence weighting policy |
| `core/induction/fixture_manifest.py` | Test fixture manifests |
| `core/induction/generalization_bar.py` | Generalization threshold logic |
| `core/induction/goal_outcome_schema.py` | Goal outcome schema definitions |
| `core/induction/golden_corpus_provider.py` | Golden corpus data provider |
| `core/induction/golden_fixture_descriptor.py` | Golden fixture descriptor types |
| `core/induction/hashing.py` | Induction-specific hashing |
| `core/induction/heldout_manifest.py` | Held-out set manifest |
| `core/induction/induction_readiness_tripwires.py` | Readiness tripwire checks |
| `core/induction/invariance_checker.py` | Invariance checking for hypotheses |
| `core/induction/k1_evaluation_ir.py` | K1 evaluation IR types |
| `core/induction/k1_metrics.py` | K1 evaluation metrics |
| `core/induction/k1_substrate_extractor.py` | K1 substrate extraction |
| `core/induction/k1_test_substrate.py` | K1 test substrate definitions |
| `core/induction/lifecycle_controller.py` | Induction lifecycle controller |
| `core/induction/minimal_test_adapter.py` | Minimal adapter for testing |
| `core/induction/nontriviality_gates.py` | Non-triviality gate checks |
| `core/induction/orchestrator.py` | Induction orchestrator |
| `core/induction/outcome_witness.py` | Outcome witness records |
| `core/induction/overlay_registry.py` | Overlay registry for sandboxing |
| `core/induction/pn_rollout_scenario.py` | PN domain rollout scenario |
| `core/induction/pn_semantic_metrics.py` | PN semantic evaluation metrics |
| `core/induction/policy_ir.py` | Policy IR types |
| `core/induction/policy_scope.py` | Policy scope definitions |
| `core/induction/policy_snapshot.py` | Policy snapshot capture |
| `core/induction/policy_to_prior.py` | Policy-to-prior conversion |
| `core/induction/policy_weights_artifact.py` | Policy weights artifact format |
| `core/induction/prediction_spec_registry.py` | Prediction spec registration |
| `core/induction/prediction_specs/pn_operator_pattern.py` | PN operator prediction patterns |
| `core/induction/prediction_specs/wordnet_operator_pattern.py` | WordNet operator prediction patterns |
| `core/induction/prior_artifact_envelope.py` | Prior artifact envelope format |
| `core/induction/prior_derivation.py` | Prior derivation tracking |
| `core/induction/prior_index.py` | Prior index structure |
| `core/induction/prior_influence_gate.py` | Prior influence gating |
| `core/induction/prior_ir.py` | Prior IR types |
| `core/induction/prior_key_types.py` | Prior key type definitions |
| `core/induction/prior_store_errors.py` | Prior store error hierarchy |
| `core/induction/prior_store.py` | Prior store interface and implementation |
| `core/induction/prior_training_loop.py` | Prior training loop |
| `core/induction/prior_verifier.py` | Prior verification |
| `core/induction/reason_emitter.py` | Reason emitter for induction decisions |
| `core/induction/sandbox_report.py` | Sandbox execution reports |
| `core/induction/sandbox_run_manifest.py` | Sandbox run manifests |
| `core/induction/sandbox_runner.py` | Sandbox execution environment |
| `core/induction/sandbox_store.py` | Sandbox artifact store |
| `core/induction/scenario_suite.py` | Scenario suite orchestration |
| `core/induction/scoped_k1_comparison.py` | Scoped K1 comparison |
| `core/induction/scorecard_computation.py` | Scorecard computation logic |
| `core/induction/semantic_delta_observation_bridge.py` | Semantic delta to observation bridge |
| `core/induction/session_scoring.py` | Session-level scoring |
| `core/induction/signature_separation.py` | Signature separation for operators |
| `core/induction/sketch_compiler.py` | Sketch-to-bytecode compilation |
| `core/induction/stage_k_report.py` | Stage K reporting |
| `core/induction/state_snapshot.py` | Induction state snapshots |
| `core/induction/synthesis_replay_check.py` | Synthesis replay verification |
| `core/induction/tc2_validator.py` | TC2 validation |
| `core/induction/tier_gate_reason.py` | Tier gate reason codes |
| `core/induction/validation_fixtures.py` | Validation fixture utilities |
| `core/induction/wordnet_rollout_scenario.py` | WordNet domain rollout scenario |
| `core/induction/gates/efficiency_delta_gate.py` | Efficiency delta promotion gate |
| `core/induction/proposer/base.py` | Base hypothesis proposer |
| `core/induction/proposer/budget.py` | Proposer budget management |
| `core/induction/proposer/manager.py` | Proposer strategy manager |
| `core/induction/proposer/strategies/delta_generalization.py` | Delta generalization strategy |
| `core/induction/proposer/strategies/entity_relation.py` | Entity-relation strategy |
| `core/induction/proposer/strategies/operator_induction.py` | Operator induction strategy |
| `core/induction/proposer/strategies/operator_sequence.py` | Operator sequence strategy |
| `core/induction/proposer/strategies/semantic_delta.py` | Semantic delta strategy |
