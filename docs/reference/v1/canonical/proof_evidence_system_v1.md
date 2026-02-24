> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Proof and Evidence System Contract

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Version**: 1.1
**Date**: 2026-02-17
**Author**: @darianrosebrook
**Status**: Implemented

### Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2026-02-17 | Added missing ArtifactRole values (§3.6), added CommitmentDomain enum and quantization utilities (§9.4–9.6), added VerificationBundleV1/PromotedOperatorProjection/RevocationSnapshot (§9), added Stage K Projection section (§10), added Provenance Closure Projection (§11), added Pre-Certificate Reference (§12), added missing source files to index |
| 1.0 | 2026-02-17 | Initial version |

---

## 1. Thesis

Sterling's proof system provides cryptographic attestation that reasoning episodes produce valid, reproducible results. The system comprises: run manifests (environment pinning), evidence bundles (content-addressed dossiers), replay verification (determinism proofs), TD-12 certificates (signed attestations), provenance chains (artifact closure verification), and verification bundles (self-contained third-party verification packages).

Every proof artifact is content-addressed. Certificates bind sketch identity, policy snapshots, artifact closures, and evidence hashes into a signed, verifiable attestation. Replay verification proves determinism: re-running identical inputs produces identical claim-field outputs.

---

## 2. Run Manifest: Environment Pinning

Source: `core/proofs/run_manifest.py:41-122`

A run manifest pins the complete execution environment for reproducibility.

```python
manifest = {
    "schema_id": "sterling.run_manifest.v1",
    "version": "1.0",
    "run_id": str,
    "created_at": str,              # ISO8601 UTC
    "run_dir": str,                 # Repo-relative path
    "repo": {
        "git_commit": str,          # Full SHA
        "dirty": bool,              # Working tree dirty?
        "repo_root_rel": ".",
        "working_tree_diff": Optional[Dict]  # Diff hash if dirty
    },
    "environment": {
        "python_version": str,
        "platform": str,
        "deps_lock": {"path": str, "sha256": str}
    },
    "inputs": [{"id": str, "path": str, "sha256": str, "bytes": int}],
    "proof_config": Dict,           # Seed, budget, etc.
    "schemas": {
        "mode": "referenced-closure",
        "sha256": str,              # Hash of schema closure
        "entries": [{"id": str, "path": str, "sha256": str}]
    },
    "policies": Dict                # Network and evidence policies
}
```

**Atomic writes**: Manifests are written via temp file + rename (`write_run_manifest_atomic`) to prevent corruption.

---

## 3. Evidence Bundles (H2 Format)

Source: `core/proofs/h2_evidence_bundle.py`

### 3.1 Non-Negotiable Invariants

| Invariant | Code | Requirement |
|-----------|------|-------------|
| EB-1 | Content-addressed | `bundle_id = sha256(canonical_json(manifest))` |
| EB-2 | Self-describing | Verifiable without repo state |
| EB-3 | Replay-verifiable | Rehash + schema + consistency + optional replay |
| EB-4 | No silent not-measured | `measurement_status` explicit for every capability |
| EB-5 | Multi-regime | >=2 structurally different regimes (capability-specific) |

### 3.2 MeasurementStatus

Source: `core/proofs/h2_evidence_bundle.py:56-63`

```python
class MeasurementStatus(str, Enum):
    MEASURED = "MEASURED"
    NOT_MEASURED_INSUFFICIENT_POWER = "NOT_MEASURED_INSUFFICIENT_POWER"
    NOT_MEASURED_INVALID_SCOPE = "NOT_MEASURED_INVALID_SCOPE"
    NOT_MEASURED_SAFEGUARD_VIOLATION = "NOT_MEASURED_SAFEGUARD_VIOLATION"
    NOT_MEASURED_OTHER = "NOT_MEASURED_OTHER"
```

Every capability must have an explicit `MeasurementStatus`. "Not measured" is never silent.

### 3.3 Capability IDs

Source: `core/proofs/h2_evidence_bundle.py:66-78`

| Capability | ID |
|-----------|-----|
| Route Convergence | `H2_ROUTE_CONVERGENCE` |
| Landmark Formation | `H2_LANDMARK_FORMATION` |
| Shadow Influence | `H2_SHADOW_INFLUENCE` |
| Multitask PN | `H2_MULTITASK_PN` |
| Linguistic I/O | `H3_LINGUISTIC_IO` |
| Transformer Light | `H3_TRANSFORMER_LIGHT` |
| Cross-Domain Transfer | `H3_CROSS_DOMAIN_TRANSFER` |

### 3.4 Regime Count Minimums

Source: `core/proofs/h2_evidence_bundle.py:87-104`

| Capability | Min Regimes | Rationale |
|-----------|-------------|-----------|
| H2 capabilities | 2 | Original EB-5 |
| H3.1 Linguistic I/O | 2 | >=2 structurally different texts |
| H3.2 Transformer Light | 2 | Token reduction generality |
| H3.3 Cross-Domain Transfer | 3 | Source + target + negative control |
| Default (unknown) | 2 | — |

### 3.5 Verification Phases

Source: `core/proofs/h2_evidence_bundle.py:120-135`

```python
class VerificationPhase(str, Enum):
    INTEGRITY = "integrity"      # Content hash verification
    SCHEMA = "schema"            # Schema validation
    CONSISTENCY = "consistency"  # Cross-reference consistency
    REPLAY = "replay"            # Full replay determinism
```

```python
class VerificationStatus(str, Enum):
    PASS = "PASS"
    FAIL = "FAIL"
    SKIP = "SKIP"
    PARTIAL = "PARTIAL"
```

### 3.6 Artifact Roles

Source: `core/proofs/h2_evidence_bundle.py:138-150`

```python
class ArtifactRole(str, Enum):
    REPORT = "report"
    PROJECTION = "projection"
    CONFIG = "config"
    EPISODE_LOG = "episode_log"
    EPISODE_MANIFEST = "episode_manifest"
    WEIGHTS = "weights"
    INFLUENCE_WITNESS = "influence_witness"
    LANDMARKS = "landmarks"
    INVARIANCE = "invariance"
    SAFEGUARDS = "safeguards"
    PLOT_DATA = "plot_data"
    REPLAY_INSTRUCTIONS = "replay_instructions"
    ENVIRONMENT = "environment"
```

---

## 4. Evidence Item Determinism

Source: `core/proofs/evidence_item_ir.py`

### 4.1 Contract

Evidence items must be "replay-grade": deterministic, no wall-clock timestamps, no randomized IDs, no unordered containers unless canonicalized.

### 4.2 Forbidden Fields

Source: `core/proofs/evidence_item_ir.py:18-30`

Patterns forbidden in evidence items: `timestamp`, `time`, `created_at`, `updated_at`, `uuid`, `guid`, `memory_address`, `object_id`, `repr`, and fields ending in `id$`.

### 4.3 Allowed ID Suffixes

Source: `core/proofs/evidence_item_ir.py:33-44`

Exceptions to the `id$` rule: `witness_id`, `test_id`, `schema_id`, `schema_version`, `details_hash`, `certificate_id`, `world_id`, `witness_ref`, `report_id`, `episode_id`.

### 4.4 Canonicalization

`canonicalize_evidence_item(evidence)` accepts either:
- IR objects with `to_canonical_dict()` method
- Plain dicts (validated for forbidden fields)

Returns a canonical dict suitable for deterministic hashing.

---

## 5. Replay Verification

Source: `core/proofs/replay_verification.py`

### 5.1 Protocol

Replay verification proves determinism of the measurement function:

1. **Capture** RNG state (Python `random`, NumPy, PyTorch).
2. **Run** benchmark with captured state.
3. **Restore** RNG state to captured snapshot.
4. **Re-run** identical benchmark.
5. **Compare** claim fields between runs.

### 5.2 RNG State Capture

Source: `core/proofs/replay_verification.py:77-112`

```python
@dataclass
class RNGStateV1:
    python_state: tuple
    numpy_state: Optional[Dict[str, Any]] = None
    torch_state: Optional[bytes] = None
    seed: Optional[int] = None
```

State hash is computed from individual source hashes: `sha256(canonical_json({python_state_hash, numpy_state_hash?, torch_state_hash?, seed}))`.

### 5.3 Verification Modes

| Mode | Description | Trade-off |
|------|-------------|-----------|
| `inproc` | In-process replay | Fast, good for frequent checks |
| `subproc` | Subprocess replay | Slower, better isolation, catches hidden state |

### 5.4 Gate S: Claim-Field Determinism

Source: `core/proofs/determinism.py`

Gate S defines which fields must match between runs:

**IDENTITY fields** (join key): `case_id`, `run_config_id`

**CLAIM fields** (must match):
- `goal_reached`, `cheated`, `forbidden_relations_used`
- `evidence_complete`, `evidence_missing_count`

**OBSERVATIONAL fields** (may vary): `expansions`, `path_length`, `traversed_relations`

Gate S proves determinism of the measurement function, not of search ordering.

---

## 6. TD-12 Certificate

### 6.1 Standard Certificate

Source: `core/proofs/certificate.py:34-106`

```python
certificate = {
    "schema_id": "sterling.certificate.v1",
    "version": "1.0",
    "certificate_id": str,
    "certificate_kind": str,     # "attestation" | "attempt"
    "created_at": str,
    "issuer": Dict,
    "run": Dict,                 # Run information
    "pins": Dict,                # Pinned inputs (repo, deps, schemas, datasets)
    "policies": Dict,
    "suites": List[Dict],        # Suite results
    "summary": Dict,
    "signatures": [{"kind": "ed25519"|"none", "payload_sha256": str, ...}]
}
```

**Payload hash**: `sha256(canonical_json(certificate - signatures))`. The `signatures` field is excluded from the payload hash to avoid circular dependency.

### 6.2 TD-12/MS Certificate (Operator Promotion)

Source: `core/proofs/td12_ms_certificate.py:39-74`

```python
@dataclass(frozen=True)
class TD12MSCertificateV1:
    schema_id: str                    # "sterling.td12_ms_cert"
    schema_version: int               # 1
    certificate_id: str               # sha256(canonical_payload)

    # Binding inputs
    sketch_hash: str
    target_tier: str                  # "provisional" | "production"
    policy_hash: str
    closure_hash: str

    # Proof bindings (hashes only)
    dossier_snapshot_hash: str
    evidence_hashes: List[str]
    validation_report_hash: str

    # Optional reproducibility anchors
    code_version_hash: Optional[str] = None
    operator_vocab_version: Optional[str] = None
    domain_id: Optional[str] = None   # Required for D1.1 action surface verification
```

**Non-negotiable invariants**:
1. **Deterministic identity**: `certificate_id = sha256(canonical_payload)` — no wall-clock in signing payload.
2. **Fail-closed verification**: Promotion to Tier>=1 requires `verify(certificate) == OK`.
3. **Closure-bound**: References `ArtifactClosure.content_hash`.
4. **Policy-bound**: References `PromotionPolicy.policy_hash`.
5. **Evidence-bound**: References dossier snapshot hash + evidence hashes (not raw objects).
6. **Domain commitment**: `domain_id` is REQUIRED (enforced by construction); empty string raises `CertificationBlockerError`.

---

## 7. Cryptographic Signing

Source: `core/proofs/signing.py`

### 7.1 SterlingSigner

Uses Ed25519 for certificate signing:

1. Load or generate private key (PEM format, PKCS8).
2. Canonicalize payload: `json.dumps(payload, sort_keys=True, separators=(",", ":"))`.
3. Sign with Ed25519: `private_key.sign(canonical_bytes)`.
4. Encode signature as base64.

### 7.2 Verification

`verify_signature(payload, signature_b64, public_key_raw)`:
1. Reconstruct canonical JSON from payload.
2. Decode base64 signature.
3. Verify with Ed25519 public key.

### 7.3 Signature Block

```python
{
    "kind": "ed25519",
    "payload_sha256": str,      # Hash of canonical payload
    "signature": str,           # Base64-encoded Ed25519 signature
    "public_key": str           # Base64-encoded raw public key
}
```

When no signer is available: `{"kind": "none", "payload_sha256": str}`.

---

## 8. Provenance Chain

Source: `core/proofs/provenance_chain.py:137-169`

### 8.1 ProvenanceChainV1

```python
@dataclass
class ProvenanceChainV1:
    sketch_hash: str
    parent_hypothesis_hash: str
    delta_pattern_hash: str
    supporting_episode_hashes: List[str]  # sorted, sha256: prefixed
    certificate_ref: str
    closure_hash: str
    evidence_bindings: List[EvidenceBinding] = []
```

All fields are REQUIRED for v2 certificates. `supporting_episode_hashes` must be sorted for determinism.

### 8.2 EvidenceBinding

Source: `core/proofs/provenance_chain.py:88-134`

```python
@dataclass(frozen=True)
class EvidenceBinding:
    episode_hash: str           # sha256: prefixed
    selection_rule_hash: str    # sha256: prefixed
    evidence_slice_digest: str  # sha256: prefixed
```

Each episode hash is paired with a selection rule hash, pinning which semantics were consumed. This prevents "these 100 episodes" without verifiable evidence extraction.

### 8.3 Chain Hash

Source: `core/proofs/provenance_chain.py:380-393`

```python
PROVENANCE_CHAIN_DOMAIN = "PROVCHAIN_V1"

def compute_chain_hash(self) -> str:
    canonical = canonical_json_dumps(self.to_dict())
    domain_separated = f"{PROVENANCE_CHAIN_DOMAIN}|{canonical}"
    digest = hashlib.sha256(domain_separated.encode("utf-8")).hexdigest()
    return f"sha256:{digest}"
```

Domain separation prevents hash collisions across different schema types.

### 8.4 Closure Verification

Verification checks (P0 boundary):
1. All `supporting_episode_hashes` resolve in artifact store.
2. `closure_hash` matches recomputed closure from artifacts.
3. All hashes use `sha256:` prefix.

**Deferred to Horizon 2**: Transitive hypothesis verification, parent hypothesis artifact existence.

---

## 9. Verification Bundle

Source: `core/proofs/verification_bundle.py`

### 9.1 Purpose

A self-contained bundle for third-party verification. A verifier with NO privileged access can validate provenance closure, chain binding, certificate-rooted dependencies, and Stage K report reproducibility.

### 9.2 Bundle Contents

| File | Purpose |
|------|---------|
| `manifest.json` | Bundle identity and hash references |
| `stage_k_projection.json` | Canonical Stage K report projection |
| `provenance_closure_projection.json` | Canonical provenance closure |
| `certificate_payload.json` | Certificate payload or projection |
| `promoted_operator_ir.json` | Promoted operator IR (if promotion succeeded) |
| `revocation_snapshot.json` | Revocation list at bundle creation time |

### 9.3 VerificationBundleV1

```python
@dataclass
class VerificationBundleV1:
    schema_id: str = "sterling.verification_bundle.v1"
    schema_version: str = "1"
    stage_k_projection_json: str           # Canonical JSON string
    stage_k_stable_digest: str             # sha256 hex
    provenance_closure_projection_json: str # Canonical JSON string
    provenance_closure_hash: str           # sha256 hex
    certificate_projection: Optional[CertificateProjection] = None
    promoted_operator_projection: Optional[PromotedOperatorProjection] = None
    revocation_snapshot: Optional[RevocationSnapshot] = None
    bundle_hash: str                       # sha256 of bundle manifest
```

Methods: `compute_bundle_hash()`, `to_bundle_dir(output_dir)`, `from_bundle_dir(bundle_dir)`.

### 9.4 CertificateProjection

Source: `core/proofs/verification_bundle.py:55-91`

```python
@dataclass
class CertificateProjection:
    certificate_id: str       # sha256: prefixed
    certificate_kind: str     # "production" | "k1_evaluation"
    closure_hash: str         # TD-12 hex64
    policy_hash: str          # sha256: prefixed
    sketch_hash: Optional[str] = None
    evidence_count: int = 0
```

Contains only fields needed for third-party verification.

### 9.5 PromotedOperatorProjection

```python
@dataclass
class PromotedOperatorProjection:
    operator_id: str           # sha256: content-addressed
    operator_kind: str         # "synthesized" | "manual"
    signature_hash: str        # sha256: prefixed
    certificate_id: str
```

### 9.6 RevocationSnapshot

```python
@dataclass
class RevocationSnapshot:
    snapshot_timestamp: str            # ISO-8601 UTC
    revoked_certificate_ids: List[str]
    revoked_operator_ids: List[str]
```

Methods: `is_certificate_revoked(id)`, `is_operator_revoked(id)`.

### 9.7 Commitment Hashing

Source: `core/proofs/commitment_hash.py`

All bundle hashes use domain-separated commitment hashing: `sha256(DOMAIN_PREFIX + "|" + canonical_json(payload))`.

**CommitmentDomain** enum defines domain separation prefixes:

| Domain | Purpose |
|--------|---------|
| `STAGEK_PROJ_V1` | Stage K report projection |
| `VERIFICATION_BUNDLE_V1` | Verification bundle manifest |
| `PROVCLOSURE_V1` | Provenance closure projection |
| `PROVCHAIN_V1` | Provenance chain |
| `CERTIFICATE_V1` | Certificate payload |
| `OPERATOR_IR_V1` | Operator IR |
| `STAGEK_EPISODE_HASHLIST_V1` | Episode hash list |
| `PROPOSED_PROGRAM_HASHLIST_V1` | Proposed program hashes |
| `PROPOSAL_INPUTS_V1` | Proposal inputs |
| `PROPOSER_CONFIG_V1` | Proposer configuration |
| `ORACLE_SEPARATION_CONFIG_V1` | Oracle separation config |
| `WORLD_QUARANTINE_V1` | World quarantine status |
| `SAFEGUARD_CONFIG_V1` | Safeguard configuration |
| `H2_EVIDENCE_BUNDLE_V1` | H2 evidence bundle |
| `H2_CAPABILITY_SUMMARY_V1` | H2 capability summary |
| `H2_INVARIANCE_RESULTS_V1` | H2 invariance results |
| `H2_SAFEGUARD_ATTESTATIONS_V1` | H2 safeguard attestations |

**CommitmentHash** (frozen dataclass): `domain`, `hex_digest` (64-char lowercase hex). Post-init validates length, case, and hex characters.

### 9.8 Quantization Utilities

Commitment hashing forbids floating-point values. Utilities for deterministic conversion:

| Function | Purpose |
|----------|---------|
| `quantize_rate(float)` | Rate to basis points (int, `value * 10000`) |
| `dequantize_rate(int)` | Basis points to rate |
| `quantize_cost(float)` | Cost to milli-units (int, `value * 1000`) |
| `dequantize_cost(int)` | Milli-units to cost |

**RateFraction** (frozen dataclass): `numerator`, `denominator`. Represents rates without floating-point. Factory: `from_rate_and_count(rate, total)`.

### 9.9 Volatile Field Filtering

`VOLATILE_FIELD_PATHS` (frozenset) defines ~20 field paths (timestamps, PIDs, paths) excluded from commitment hashes. `filter_volatile_fields(data, prefix)` recursively removes these.

---

## 10. Stage K Report Projection

Source: `core/proofs/stage_k_projection.py`

### 10.1 StageKReportProjectionV1

A deterministic projection of a Stage K report for commitment hashing. All floating-point values are converted to integers or `RateFraction`.

**Certification outcome** (deterministic): `scenario_id`, `pipeline_id`, `stage_k_complete`, `k0_satisfied`, `k1_satisfied`, `certification_passed`.

**Certificate binding** (deterministic): `certificate_id` (sha256: prefixed), `certificate_kind`, `closure_hash`, `policy_hash`.

**Candidate identity**: `candidate_program_hash`, `candidate_mdl_cost_milliunits` (integer, not float).

**Promotion outcome**: `promotion_lane_attempted`, `promotion_lane_ready`, `promoted_operator_id` (sha256: content-addressed), `promotion_failure_codes` (sorted tuple).

**Success rates as fractions** (NO FLOATS): `baseline_success: Optional[RateFraction]`, `validation_success: Optional[RateFraction]`.

**Evidence binding**: `supporting_episode_hashes_digest` (sha256 of sorted hashes), `evidence_count`.

**Safeguard hardening**: `certification_intent`, `world_id`, `world_quarantine_status`, `world_quarantine_reasons` (sorted), `oracle_policy_config_digest`, `safeguard_version`.

Methods: `to_dict()` (sorted keys, NO floats), `compute_stable_digest()` → `CommitmentHash`, `from_dict()`.

Factory: `build_projection_from_report(report)` — converts floats to fractions/milli-units.

---

## 11. Provenance Closure Projection

Source: `core/proofs/provenance_closure_projection.py`

```python
@dataclass(frozen=True)
class ProvenanceClosureProjectionV1:
    schema_id: str = "sterling.provenance_closure_projection.v1"
    schema_version: str = "1"
    episode_digests: List[str]                          # sha256: prefixed, sorted
    evidence_slice_digests: List[EvidenceSliceDigest]
    policy_engagement_witness_digest: Optional[str]
    semantic_delta_pattern_digest: str
    sketch_hash: str
    certificate_ref: str
```

**EvidenceSliceDigest** (frozen): `episode_hash`, `slice_hash`, `selection_rule_hash` (all sha256: prefixed).

Methods: `to_dict()` (sorted keys), `compute_hash()` (sha256: prefixed, using `PROVCLOSURE_V1` domain), `validate()`.

---

## 12. Pre-Certificate Reference

Source: `core/proofs/pre_certificate_ref.py`

Binds gate inputs to an eventual certificate before the certificate exists. Used for binding seam closure.

```python
@dataclass(frozen=True)
class PreCertificateRefV1:
    schema_version_required: int
    sketch_hash: str
    closure_hash: str
    policy_hash: str
    parent_hypothesis_hash: str
    delta_pattern_hash: str
    supporting_episode_hashes_digest_v1: str
    committed_payload_content_hash_v1: Optional[str]
    pinned_env_hash_v1: Optional[str]
```

Methods: `to_canonical_dict()` (sorted keys, None excluded), `compute_ref()` (sha256: prefixed).

Functions: `build_pre_certificate_ref_v1(...)`, `verify_pre_certificate_ref_v1(...)`.

---

## 13. Batch Hashing

Source: `core/proofs/batch_hasher.py`

### 10.1 Contract

Uses `HashContract.PROOFS_V1` for all hashing. Produces bare hex SHA-256 digests.

### 10.2 Parallelization

Tier 1 (pure + safe) per parallelization policy:
- **Map**: artifact → digest (embarrassingly parallel)
- **Reduce**: stable map `{artifact_id: digest}` + sorted digest list
- No shared mutable state in workers

### 10.3 HashResult

Source: `core/proofs/batch_hasher.py:45-72`

```python
@dataclass(frozen=True)
class HashResult:
    artifact_id: str
    digest: str
    algorithm: str = "sha256"
    error: Optional[str] = None
```

Frozen for thread safety. `is_ok` property: True when error is None and digest is non-empty.

---

## 14. Structural Verification

Source: `core/proofs/byte_replay_verification.py`

### 11.1 VerificationFailureCode

| Code | Meaning |
|------|---------|
| `MISSING_ARTIFACT` | Referenced artifact not found |
| `ARTIFACT_HASH_MISMATCH` | Artifact content doesn't match declared hash |
| `PROOF_HASH_MISMATCH` | Recomputed proof hash differs from declared |
| `PROOF_DICT_DIVERGENCE` | Proof dict structure differs |
| `PROOF_SPEC_DRIFT` | Proof algorithm/spec version changed |
| `SCHEMA_AMBIGUITY` | Multiple schemas for hash |
| `SCHEMA_MISMATCH` | Wrong schema type |
| `SCHEMA_VERSION_MISMATCH` | Correct type but wrong version |
| `IMPLEMENTATION_DRIFT` | Operator/engine changed |
| `INCOMPLETE_BUNDLE` | Bundle missing required commitments |

**Precedence** (mutual exclusivity):
1. Implementation commitments differ → `IMPLEMENTATION_DRIFT`
2. Proof spec differs → `PROOF_SPEC_DRIFT`
3. Proof dict structure differs → `PROOF_DICT_DIVERGENCE`
4. Proof hash differs → `PROOF_HASH_MISMATCH`
5. Artifact integrity fails → relevant artifact failure

---

## 15. Invariants Summary

1. **Content-addressed**: Every proof artifact is identified by its content hash.
2. **Deterministic identity**: Certificate IDs are `sha256(canonical_payload)` — no wall-clock in signing payload.
3. **Fail-closed verification**: Promotion requires passing verification.
4. **Domain separation**: Provenance chain hashes use `PROVCHAIN_V1|` domain prefix.
5. **No silent not-measured**: Every capability has explicit `MeasurementStatus` (EB-4).
6. **Multi-regime**: Evidence bundles require capability-specific minimum regime counts (EB-5).
7. **Replay-grade evidence**: Evidence items contain no timestamps, UUIDs, or non-deterministic fields.
8. **Atomic writes**: Manifests use temp-file-then-rename to prevent corruption.
9. **Closure binding**: Certificates reference `closure_hash`, not raw artifacts.
10. **Evidence binding**: Episode hashes paired with selection rule hashes to pin consumed semantics.
11. **Hash contract compliance**: Batch hashing uses `PROOFS_V1` (bare hex, `ensure_ascii=True`).
12. **No floats in commitments**: All commitment hashes use integer representations (basis points, milli-units, `RateFraction`) — never floating-point.
13. **Volatile field exclusion**: Timestamps, PIDs, file paths are excluded from commitment hashes via `VOLATILE_FIELD_PATHS`.

---

## 16. Source File Index

| File | Purpose |
|------|---------|
| `core/proofs/run_manifest.py` | Run manifest construction, environment pinning |
| `core/proofs/h2_evidence_bundle.py` | H2 evidence bundle, MeasurementStatus, capabilities |
| `core/proofs/evidence_item_ir.py` | Evidence item determinism, forbidden fields |
| `core/proofs/replay_verification.py` | RNG state capture, replay determinism protocol |
| `core/proofs/determinism.py` | Gate S claim-field determinism, CaseRecord |
| `core/proofs/certificate.py` | TD-12 certificate construction, verdict hashing |
| `core/proofs/td12_ms_certificate.py` | TD-12/MS certificate for operator promotion |
| `core/proofs/signing.py` | Ed25519 signing and verification |
| `core/proofs/provenance_chain.py` | ProvenanceChainV1, EvidenceBinding, closure verification |
| `core/proofs/verification_bundle.py` | Self-contained third-party verification bundles |
| `core/proofs/batch_hasher.py` | Parallel witness/artifact hash computation |
| `core/proofs/byte_replay_verification.py` | Structural verification, VerificationFailureCode |
| `core/proofs/artifact_hashing.py` | File and claim hash computation |
| `core/proofs/commitment_hash.py` | Domain-separated commitment hashing |
| `core/proofs/provenance_verifier.py` | ProvenanceVerifier (current API) |
| `core/proofs/cold_start_verifier.py` | Cold-start verification without runtime state |
| `core/proofs/stage_k_projection.py` | StageKReportProjectionV1, deterministic projection |
| `core/proofs/provenance_closure_projection.py` | ProvenanceClosureProjectionV1, EvidenceSliceDigest |
| `core/proofs/pre_certificate_ref.py` | PreCertificateRefV1, binding seam closure |
| `core/proofs/td12_policy.py` | TD12Policy, verification level configuration |
| `core/proofs/evidence_schema_registry.py` | EvidenceSchemaV0, EvidenceSchemaRegistry |

---

## 17. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [Hashing Contracts](hashing_contracts_v1.md) | PROOFS_V1 contract used for batch hashing; GOVERNANCE_V1 for provenance chain hashes |
| [Governance Certification](governance_certification_contract_v1.md) | GovernanceContext determines verification strictness; GateResult consumed by proof pipeline |
| [Claim Schema System](claim_schema_system_v1.md) | Claims provide the semantic content that evidence bundles attest to |
| [Code32 and ByteState](code32_bytestate.md) | ByteTrace provides execution traces for structural replay verification |
| [Conformance](conformance.md) | Conformance suites produce TD-12 certificates |
