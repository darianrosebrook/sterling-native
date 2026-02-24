> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Governance and Certification Contract

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Version**: 1.1
**Date**: 2026-02-17
**Author**: @darianrosebrook
**Status**: Implemented

### Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2026-02-17 | Added GovernanceContext properties/methods (§3.5), InMemoryWitnessStore/InMemoryArtifactStore (§3.6), fail_adapter_unimplemented factory (§4.3), expanded FailureWitness fields/functions/constants (§5), expanded ExecutionPolicy with enums/dataclasses (§6), added Canonical Hashing section (§7), added Adapter Strict Guard section (§8), added adapter_strict_guard.py to source index |
| 1.0 | 2026-02-17 | Initial version |

---

## 1. Thesis

Sterling's governance system provides typed, auditable certification infrastructure. Every reasoning episode runs under a declared **RunIntent** that determines strictness. A single **GovernanceContext** object is the authority for all governance decisions within an episode. Gates produce **GateVerdict** results (PASS/FAIL/SKIPPED), and failures are recorded as **FailureWitnesses** with content-addressed audit trails.

This document specifies the governance pipeline: intent declaration, context construction, gate evaluation, failure recording, and certification eligibility.

---

## 2. RunIntent: Strictness Authority

Source: `core/governance/run_intent.py:130-245`

```python
class RunIntent(str, Enum):
    DEV = "dev"           # Permissive, warnings allowed
    CERTIFYING = "certifying"  # Strict, warnings become errors
    PROMOTION = "promotion"    # Certifying-equivalent for promotion lane
    REPLAY = "replay"          # Certifying-equivalent for replay verification
```

### 2.1 Strictness Derivation

| Intent | `is_strict` | `is_permissive` | `requires_domain_id` | `requires_sealed_registry` |
|--------|------------|-----------------|---------------------|--------------------------|
| DEV | False | True | False | False |
| CERTIFYING | True | False | True | True |
| PROMOTION | True | False | True | True |
| REPLAY | True | False | True | True |

**Single source of truth**: `RunIntent.is_strict` is the canonical strictness check. All `strict: bool` parameters in helper functions should be derived from `run_intent.is_strict`.

### 2.2 Normalization

`RunIntent.normalize(value)` is the only permitted conversion point:
- `None` -> `RunIntent.DEV`
- `RunIntent` instance -> returned as-is
- `str` -> parsed via `from_string()` (case-insensitive)

### 2.3 Exception Taxonomy

Source: `core/governance/run_intent.py:58-127`

| Exception | Scope | When |
|-----------|-------|------|
| `CertifyingModeError` | Base class | Strict operation cannot complete |
| `BaselineNotFoundError` | Missing baseline | Strict mode, no baseline data |
| `EvidenceNotFoundError` | Missing evidence | Strict mode, empty evidence bundle |
| `ArtifactStoreRequiredError` | Missing store | Strict mode, no artifact store |

**Rule**: These exceptions are ONLY for strict code paths. Permissive code paths return `GateVerdict.SKIPPED`, never raise.

---

## 3. GovernanceContext: Single Authority

Source: `core/governance/governance_context.py:237-769`

GovernanceContext encapsulates all governance state for an episode. It replaces the anti-pattern of threading `run_intent`, `artifact_store`, `strict`, and `witness_recorder` as separate parameters.

### 3.1 Fields

```python
@dataclass(frozen=True)
class GovernanceContext:
    registry: DomainRegistry           # Isolated, sealed for certification
    run_intent: RunIntent              # Strictness authority
    session_manifest_hash: str         # Deterministic run identifier (no timestamps)
    artifact_store: Optional[Any]      # REQUIRED when strict + CERTIFIABLE
    witness_recorder: Optional[WitnessRecorder]  # REQUIRED when strict
    execution_class: ExecutionClass    # CERTIFIABLE / LOCAL_STRICT / DEVELOPMENT
```

### 3.2 ExecutionClass

Source: `core/governance/governance_context.py:191-234`

| Class | `is_strict` | `is_certifiable` | `requires_durable_persistence` | Use Case |
|-------|------------|------------------|-------------------------------|----------|
| CERTIFIABLE | True | True | True | Production certification |
| LOCAL_STRICT | True | False | False | Local testing of strict paths |
| DEVELOPMENT | False | False | False | Development mode |

### 3.3 Construction-Time Validation

GovernanceContext validates invariants at construction (fail-closed):

1. **CERTIFIABLE + strict**: Requires `artifact_store`, `witness_recorder`, and `witness_recorder.has_durable_store`.
2. **LOCAL_STRICT**: Requires `witness_recorder` (may be non-durable). Uses `InMemoryWitnessStore` and `InMemoryArtifactStore`.
3. **DEVELOPMENT + strict intent**: Configuration error — must explicitly choose CERTIFIABLE or LOCAL_STRICT.

### 3.4 Factory Methods

| Factory | Intent | Execution Class | Registry |
|---------|--------|----------------|----------|
| `for_certification(manifest, store, recorder)` | CERTIFYING | CERTIFIABLE | Isolated, sealed |
| `for_promotion(manifest, store, recorder)` | PROMOTION | CERTIFIABLE | Isolated, sealed |
| `for_replay(manifest, store, recorder)` | REPLAY | CERTIFIABLE | Isolated, sealed |
| `for_development(registry?)` | DEV | DEVELOPMENT | Provided or singleton |
| `for_local_strict(intent?, registry?)` | CERTIFYING | LOCAL_STRICT | Provided or singleton |

### 3.5 Key Properties and Methods

**Properties**:
- `is_strict`: True if `execution_class.is_strict` or `run_intent.is_strict`.
- `is_certifiable`: True only for CERTIFIABLE execution class.
- `is_certifying`: Alias for `is_strict` (backward compatibility).

**Methods**:
- `require_pass(verdict, gate_name)`: Raises `GovernanceContextError` if verdict is not PASS in strict mode.
- `get_domain_by_id(domain_id)`: Look up a domain in the registry by ID.
- `get_domain_by_kernel(kernel_type, kernel_version=None)`: Look up a domain by kernel type.
- `record_witness(witness)`: Delegate to `witness_recorder.record()`, returns witness ID.
- `to_dict()`: Serialize context to dictionary representation.

**Exception**: `GovernanceContextError` — raised for governance invariant violations.

### 3.6 In-Memory Stores (S3.0i)

Used by `for_local_strict()` to provide strict-mode validation without durable persistence.

**InMemoryWitnessStore**:
- `put(key, value)` / `store_witness(key, value)`: Store a witness by key.
- `get(key)`: Retrieve witness by key.
- `get_all()`: Return all stored witnesses.
- `clear()`: Remove all witnesses.

**InMemoryArtifactStore**:
- `put(schema_id, content, *, key=None)`: Store artifact, returns `SimpleArtifactRef`.
- `get_by_key(key, *, verify=True)`: Retrieve by key.
- `get_by_hash(content_hash, *, verify=True)`: Retrieve by content hash.
- `clear()`: Remove all artifacts.

**SimpleArtifactRef** (frozen dataclass): `schema_id`, `content_hash`, `locator`, `schema_version`.

---

## 4. GateVerdict: Tri-State Result

Source: `core/governance/gate_verdict.py:80-116`

```python
class GateVerdict(str, Enum):
    PASS = "PASS"       # Gate evaluated, requirements met
    FAIL = "FAIL"       # Gate evaluated, requirements NOT met
    SKIPPED = "SKIPPED" # Gate could not be evaluated
```

**Critical**: SKIPPED != PASS. Code that treats SKIPPED as PASS introduces pass-by-default semantics.

| Property | PASS | FAIL | SKIPPED |
|----------|------|------|---------|
| `is_success` | True | False | False |
| `is_failure` | False | True | False |
| `is_indeterminate` | False | False | True |
| `requires_action_in_strict` | False | True | True |

### 4.1 SkipReason

Source: `core/governance/gate_verdict.py:118-174`

Two categories:

**Missing prerequisite** (must raise in strict mode):
- `BASELINE_NOT_FOUND`, `EVIDENCE_NOT_FOUND`, `CERTIFICATE_NOT_FOUND`
- `ARTIFACT_STORE_MISSING`, `INSUFFICIENT_DATA`, `DEPENDENCY_FAILED`

**Not applicable** (may remain SKIPPED in strict mode):
- `NOT_APPLICABLE`, `WORLD_NOT_REGISTERED`, `OPERATOR_NOT_CERTIFIED`

### 4.2 FailReasonCode

Source: `core/governance/gate_verdict.py:177-228`

Every FAIL verdict in strict mode MUST have a typed reason code:

| Code | Meaning |
|------|---------|
| `PREREQ_MISSING` | Cannot evaluate — prerequisites absent |
| `REGRESSION_DETECTED` | Gate found a regression |
| `INVARIANT_VIOLATED` | Invariant check failed |
| `EVIDENCE_INVALID` | Evidence exists but corrupt/malformed |
| `THRESHOLD_NOT_MET` | Metric below required threshold |
| `ADAPTER_UNIMPLEMENTED` | Adapter hook not implemented |
| `UNKNOWN_CONVERTED` | UNKNOWN outcome converted to FAIL in strict mode |
| `VERIFICATION_FAILED` | Prediction verification failed |
| `SCHEMA_UNSUPPORTED` | Unknown/unsupported schema version |

**PREREQ_MISSING** is special: it indicates "cannot evaluate" but in strict mode must still be FAIL (not SKIPPED). Use `is_prereq_missing` to detect for diagnostics.

### 4.3 GateResult

Source: `core/governance/gate_verdict.py:270-903`

```python
@dataclass
class GateResult:
    verdict: GateVerdict
    gate_id: str
    reason: str
    skip_reason: Optional[SkipReason] = None
    fail_reason_code: Optional[FailReasonCode] = None
    missing_prerequisites: Optional[List[str]] = None
    details: Dict[str, Any] = {}
    witness_hash: Optional[str] = None
    _strict_validation: bool = False
```

**Post-init validation**:
1. SKIPPED verdict requires `skip_reason`.
2. In strict mode, FAIL verdict requires `fail_reason_code` (raises `UntypedFailInStrictModeError`).

**Factory methods** (preferred over direct construction):

| Factory | Verdict | Reason Code |
|---------|---------|-------------|
| `pass_verdict(gate_id, reason)` | PASS | — |
| `fail_verdict(gate_id, reason)` | FAIL | (untyped — avoid in strict) |
| `skip_verdict(gate_id, skip_reason, reason)` | SKIPPED | — |
| `fail_prereq_missing(gate_id, missing, reason)` | FAIL | PREREQ_MISSING |
| `fail_regression(gate_id, reason)` | FAIL | REGRESSION_DETECTED |
| `fail_invariant_violated(gate_id, reason)` | FAIL | INVARIANT_VIOLATED |
| `fail_evidence_invalid(gate_id, reason)` | FAIL | EVIDENCE_INVALID |
| `fail_threshold_not_met(gate_id, reason)` | FAIL | THRESHOLD_NOT_MET |
| `fail_verification(gate_id, reason)` | FAIL | VERIFICATION_FAILED |
| `fail_unknown_converted(gate_id, reason)` | FAIL | UNKNOWN_CONVERTED |
| `fail_adapter_unimplemented(gate_id, reason)` | FAIL | ADAPTER_UNIMPLEMENTED |
| `fail_strict(gate_id, reason, code)` | FAIL | (required param) |
| `for_context(context, verdict, gate_id, ...)` | Any | Auto-validates from context |

### 4.4 Schema Versioning

GateResult serializes with `schema_id`:
- `sterling.gate_result.v1`: Legacy (read-only, best-effort)
- `sterling.gate_result.v2`: Current (full S3.0c support)

In strict mode, unknown schemas fail-closed (`SchemaValidationError`).

---

## 5. Failure Witnesses

Source: `core/governance/failure_witness.py`

When a governance gate FAILs, a **FailureWitness** records what failed, why, and the content-addressed evidence. Witnesses form an append-only audit trail.

### 5.1 GovernanceFailureWitness

```python
@dataclass
class GovernanceFailureWitness:
    record_id: str                              # Unique per occurrence (UUID)
    semantic_hash: str                          # Reproducible from deterministic fields
    failure_type: str                           # Typed failure category
    gate_id: str                                # Which gate failed
    verdict: GateVerdict                        # The verdict that triggered the witness
    required_artifact: str                      # What artifact was needed
    search_keys: List[str] = []                 # Keys searched during resolution
    timestamp: str = ""                         # Run-local (NOT in semantic hash)
    artifact_ids_checked: List[str] = []        # Artifacts examined
    context: Dict[str, Any] = {}                # Additional context
    stack_summary: Optional[str] = None         # Stack trace (NOT in semantic hash)
```

**Property**: `witness_id` — returns `semantic_hash` (backward compatibility alias).

**Dual-hash design**: Both `semantic_hash` (for deduplication) and `record_id` (for ordering) are tracked. The semantic hash is computed from allowlisted fields only (no timestamps, stack traces, hostnames).

### 5.2 Witness Hash Constants

| Constant | Type | Purpose |
|----------|------|---------|
| `WITNESS_SEMANTIC_FIELDS` | frozenset | Deterministic fields included in semantic hash |
| `WITNESS_METADATA_FIELDS` | frozenset | Run-local entropy fields (NOT in hash) |
| `WITNESS_HASH_DENYLIST` | frozenset | Fields NEVER included in hash-critical digests |

### 5.3 Witness Functions

| Function | Purpose |
|----------|---------|
| `compute_semantic_hash(failure_type, gate_id, verdict, required_artifact, search_keys, context=None)` | Reproducible hash from deterministic fields only |
| `compute_witness_hash(witness_data)` | Legacy backward-compat hash (v1) |
| `create_failure_witness(...)` | Factory for creating witnesses |
| `record_and_raise(exception, failure_type, gate_id, verdict, required_artifact, ..., strict=True, recorder=None)` | Witness-first pattern: records witness THEN raises exception |

**Exception**: `WitnessNotDurableError` — raised when strict mode requires durable storage but store is not durable.

### 5.4 WitnessRecorder

The WitnessRecorder is an append-only recorder that persists witnesses to a durable store:

- `record(witness, strict)`: Persist witness, return witness_id. Deduplicates by `semantic_hash` within a single run (S3.0l).
- `has_durable_store`: True if the recorder has a functioning store.
- `get_witnesses()`: Return all recorded witnesses.
- `get_seen_semantic_hashes()`: Return set of deduplicated semantic hashes.
- `clear()`: Remove all witnesses.
- `reset_dedupe()`: Reset deduplication state (for test isolation).

In strict mode, recording failure raises `WitnessNotDurableError` if the store is not durable.

---

## 6. Execution Policy

Source: `core/governance/execution_policy.py`

ExecutionPolicy controls which operators may execute under a given RunIntent, based on their origin (builtin, certified, learned, external).

### 6.1 OperatorOrigin

```python
class OperatorOrigin(str, Enum):
    BUILTIN = "builtin"       # Built-in operator from core Sterling (trusted)
    CERTIFIED = "certified"   # Operator certified through TD-12/MS pipeline (verified)
    LEARNED = "learned"       # Learned operator from induction (unverified at runtime)
    EXTERNAL = "external"     # External operator from plugin/extension (untrusted)
```

### 6.2 ExecutionPolicyMode

```python
class ExecutionPolicyMode(str, Enum):
    STRICT = "strict"         # Learned operators banned in strict contexts (default)
    MONITORED = "monitored"   # Learned operators allowed only with monitored execution substrate
    PERMISSIVE = "permissive" # All operators allowed (dev mode only)
```

### 6.3 ExecutionPolicy

```python
@dataclass(frozen=True)
class ExecutionPolicy:
    mode: ExecutionPolicyMode
    allow_learned_in_strict: bool = False
    require_attestation: bool = True
    require_monitored_vm: bool = False
```

**Factory methods**:

| Factory | Mode | Use Case |
|---------|------|----------|
| `strict()` | STRICT | Default for CERTIFYING/PROMOTION/REPLAY intents |
| `monitored()` | MONITORED | Learned operators with execution substrate |
| `permissive()` | PERMISSIVE | DEV intent only |
| `for_run_intent(run_intent)` | Auto | Derives mode from RunIntent |

**Key method**: `check_allowed(run_intent, operator_origin, operator_name, has_attestation=False, has_monitored_vm=False)` — raises `ExecutionPolicyError` with a typed `ExecutionPolicyViolation` if the operator is not permitted.

### 6.4 ExecutionPolicyViolation

```python
@dataclass(frozen=True)
class ExecutionPolicyViolation:
    policy_mode: ExecutionPolicyMode
    run_intent: RunIntent
    operator_origin: OperatorOrigin
    operator_name: str
    message: str
    remediation: str
```

**Exception**: `ExecutionPolicyError` — stores `violation: ExecutionPolicyViolation`.

**Module-level constants**:
- `CERTIFYING_EXECUTION_POLICY = ExecutionPolicy.strict()`
- `DEV_EXECUTION_POLICY = ExecutionPolicy.permissive()`

---

## 7. Canonical Hashing

Source: `core/governance/canonical_hash.py`

Provides content-addressed hashing for governance artifacts, episode inputs, and puzzle identity.

### 7.1 Core Functions

- `canonical_json_serialize(obj)`: Canonical JSON with sorted keys, compact separators.
- `canonical_json_hash(obj, prefix="")`: SHA-256 hash with domain separation (prefix in preimage).

### 7.2 Semantic Payload Extraction

- `extract_semantic_payload(data, allowlist, denylist, normalize_lists=True)`: Extract deterministic fields for hashing.
- `compute_semantic_hash(data, allowlist, prefix="")`: Hash only allowlisted deterministic fields.

### 7.3 Episode Input Extraction (S3.0d)

Single authority for extracting episode inputs for different contract types:

| Constant | Fields |
|----------|--------|
| `PUZZLE_INPUT_FIELDS` | puzzle_id, seed, initial_state, board_width, board_height, num_pieces, goal_state |
| `DISCRIMINATIVE_METADATA_FIELDS` | episode_id, expected_difficulty, baseline_outcome |
| `REGRESSION_METADATA_FIELDS` | episode_id, expected_success |

- `extract_episode_input(episode_data, contract, context=None)`: Strict mode raises on missing fields.
- `hash_episode_input(episode_data, contract, context=None)`: Hash the extracted input.

### 7.4 Puzzle vs Contract Hash (S3.0m)

Two complementary hash identities:

| Function | Stability | Use |
|----------|-----------|-----|
| `compute_puzzle_hash(episode_data)` | Stable across contracts | Puzzle identity |
| `compute_contract_hash(episode_data, contract)` | Contract-specific | Episode-in-contract identity |

### 7.5 Exceptions

| Exception | When |
|-----------|------|
| `CanonicalHashError` | Base class for hashing errors |
| `EpisodeHashingError` | Episode content hash failure |
| `EpisodeInputExtractionError` | Missing/invalid episode fields |
| `MissingPuzzleFieldsError` | Required puzzle fields absent |

---

## 8. Adapter Strict Guard

Source: `core/governance/adapter_strict_guard.py`

Validates that domain adapters comply with strict-mode requirements. Implements the witness-first pattern: record failure witness BEFORE raising exceptions.

### 8.1 Guard Functions

| Function | Purpose |
|----------|---------|
| `check_emit_observations_strict(adapter_name, observations, strict=False)` | Fails if strict and observations empty or all UNIMPLEMENTED |
| `check_verify_prediction_strict(adapter_name, result, strict=False)` | Fails if strict and outcome is UNKNOWN |
| `validate_adapter_for_strict_mode(adapter, context=None)` | Checks adapter has required methods (emit_observations, verify_prediction) |

### 8.2 Wrapper Functions

- `wrap_emit_observations_strict(adapter, emit_fn, context=None)`: Wraps `emit_observations()` with strict-mode guard and witness recording.
- `wrap_verify_prediction_strict(adapter, verify_fn, context=None)`: Wraps `verify_prediction()` with strict-mode guard and witness recording.

### 8.3 Result and Exceptions

**AdapterGuardResult** (dataclass): `passed`, `adapter_name`, `method_name`, `violation`, `return_value`.

| Exception | When |
|-----------|------|
| `AdapterStrictModeError` | Adapter violates strict-mode requirements (raised AFTER witness recorded) |
| `StrictModeConfigurationError` | Strict mode requested but requirements not met (NO witness — configuration error) |

---

## 9. Adapter Outcome Mapping

Source: `core/governance/gate_verdict.py:977-1069`

`map_test_result_to_gate_result()` is the single authority for converting adapter `TestResultIR` outcomes to `GateResult`:

| Adapter Outcome | Non-Strict | Strict |
|-----------------|-----------|--------|
| `"PASS"` | GateResult.PASS | GateResult.PASS |
| `"FAIL"` | GateResult.FAIL (VERIFICATION_FAILED) | GateResult.FAIL (VERIFICATION_FAILED) |
| `"UNKNOWN"` | GateResult.SKIPPED (INSUFFICIENT_DATA) | GateResult.FAIL (UNKNOWN_CONVERTED) |
| Unexpected | GateResult.SKIPPED | Raises `UntypedFailInStrictModeError` |

Adapters must NEVER create GateResult directly — always use this mapper.

---

## 10. Invariants Summary

1. **Single authority**: GovernanceContext is the only source of strictness decisions within an episode.
2. **Construction-time validation**: Misconfigured contexts fail immediately, not at first use.
3. **No ambient state**: No global variables for governance decisions — all state flows through GovernanceContext.
4. **Tri-state verdicts**: SKIPPED is never silently treated as PASS.
5. **Typed failures**: Every FAIL in strict mode has a `FailReasonCode` — no untyped failures.
6. **Witness-first**: Failure witnesses are recorded before exceptions are raised.
7. **Deterministic sessions**: Session identity uses manifest hashes, never timestamps.
8. **Schema fail-closed**: Unknown schemas are rejected in strict mode.
9. **Certifiability gate**: Only CERTIFIABLE execution class can produce promotion-grade certificates. LOCAL_STRICT is strict but not certifiable.

---

## 11. Source File Index

| File | Purpose |
|------|---------|
| `core/governance/run_intent.py` | RunIntent enum, CertifyingModeError hierarchy |
| `core/governance/governance_context.py` | GovernanceContext, ExecutionClass, in-memory stores |
| `core/governance/gate_verdict.py` | GateVerdict, SkipReason, FailReasonCode, GateResult |
| `core/governance/failure_witness.py` | FailureWitness creation, WitnessRecorder |
| `core/governance/execution_policy.py` | ExecutionPolicy configuration |
| `core/governance/canonical_hash.py` | GOVERNANCE_V1 hashing reference implementation |
| `core/governance/adapter_strict_guard.py` | Adapter strict-mode validation, guard wrappers |
| `core/governance/legacy_deprecation.py` | Legacy API deprecation helpers |
| `core/governance/replay/__init__.py` | Replay verification subpackage |
| `core/governance/replay/artifact_resolver.py` | Replay artifact resolution |
| `core/governance/replay/structural_verifier.py` | Structural replay verification |
| `core/governance/replay/verification_result.py` | Replay verification result types |
| `core/governance/__init__.py` | Package exports |

---

## 12. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [Hashing Contracts](hashing_contracts_v1.md) | GOVERNANCE_V1 hash contract is used for all governance artifact hashing |
| [Proof Evidence System](proof_evidence_system_v1.md) | Proofs consume GateResults and produce certificates |
| [Evaluation Gates](evaluation_gates_v1.md) | Gate definitions that produce GateResult instances |
| [Conformance](conformance.md) | Conformance suites run under CERTIFYING intent |
| [State Model](state_model_contract_v1.md) | StateGraph validation produces governance witnesses |
