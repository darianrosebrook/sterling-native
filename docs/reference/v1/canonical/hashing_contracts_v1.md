> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Hashing Contracts and Canonical Serialization

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Version**: 1.2
**Date**: 2026-02-19
**Author**: @darianrosebrook
**Status**: Implemented (V1 contracts operational)

---

## 1. Thesis

Sterling's hash infrastructure provides deterministic, reproducible content addressing across three distinct subsystems: governance, canonicalization, and proofs. Each subsystem evolved independently and has different serialization conventions. Rather than migrate to a single convention (which would break all existing artifact hashes), the system defines three explicit **hash contracts** — each with specific, stable semantics that callers must select explicitly.

**Core invariant**: The same logical object, serialized under the same contract, produces the same hash on every platform, every run, forever. Changing hash output for a contract breaks all existing artifacts. If semantics must change, create a new contract version.

---

## 2. Hash Contracts

### 2.1 The HashContract Enum

Source: `core/hashing/contracts.py:51`

```python
class HashContract(str, Enum):
    GOVERNANCE_V1 = "governance_v1"
    CANONICALIZATION_V1 = "canonicalization_v1"
    PROOFS_V1 = "proofs_v1"
```

There are no silent defaults. Every call site must specify which contract it uses. This is enforced by making `contract` a required keyword argument on all public functions.

Serialization or hashing failures raise `HashContractError(Exception)` (e.g., non-serializable objects, NaN/Infinity floats).

A `CONTRACT_SEMANTICS` dictionary (keyed by `HashContract` enum values) provides introspectable metadata for each contract: `json_ensure_ascii`, `json_allow_nan`, `hash_prefix`, `domain_prefix_support`, `used_by`, and `persisted_artifacts`. Access via `get_contract_semantics(contract)`.

### 2.2 GOVERNANCE_V1

**Purpose**: Governance artifacts — witnesses, manifests, episode hashes, registry snapshots.

| Property | Value |
|----------|-------|
| JSON `sort_keys` | `True` |
| JSON `separators` | `(",", ":")` |
| JSON `ensure_ascii` | `False` (UTF-8 literals preserved) |
| JSON `allow_nan` | `False` (rejects NaN/Infinity) |
| Hash output format | `"sha256:<hex>"` (prefixed) |
| Domain prefix support | Yes — in both hash input AND output |
| Primary consumer | `core/governance/canonical_hash.py` |

**Persisted artifacts**: Episode content hashes, witness semantic hashes, manifest set_ids, registry snapshot hashes.

**UTF-8 behavior**: Non-ASCII characters are preserved as literal UTF-8 bytes, not escaped. `{"name": "José"}` serializes to `{"name":"José"}` (6 bytes for the name, where `é` is 2 UTF-8 bytes). This means GOVERNANCE_V1 hashes differ from CANONICALIZATION_V1/PROOFS_V1 hashes for any input containing non-ASCII characters.

**Domain prefix behavior**: When `domain_prefix="episode:"` is specified:
- Hash input: `b"episode:" + json_bytes`
- Hash output: `"sha256:episode:<hex>"`

### 2.3 CANONICALIZATION_V1

**Purpose**: Optimization cache keys, M4/M5 certificates, derivation manifests, determinism witnesses.

| Property | Value |
|----------|-------|
| JSON `sort_keys` | `True` |
| JSON `separators` | `(",", ":")` |
| JSON `ensure_ascii` | `True` (unicode escaped as `\uXXXX`) |
| JSON `allow_nan` | `False` |
| Hash output format | bare hex (no prefix) |
| Domain prefix support | No |
| Primary consumers | `core/optimization/hashing.py`, `core/features.py`, `core/memory/canonical.py` |

**Persisted artifacts**: Feature cache keys (ephemeral), M4/M5 certificates, derivation manifests, determinism witnesses.

### 2.4 PROOFS_V1

**Purpose**: Proof artifacts and certificates — certificate hashes, batch witness digests, TD-12 certificate IDs.

| Property | Value |
|----------|-------|
| JSON `sort_keys` | `True` |
| JSON `separators` | `(",", ":")` |
| JSON `ensure_ascii` | `True` (unicode escaped) |
| JSON `allow_nan` | `False` |
| Hash output format | bare hex (no prefix) |
| Domain prefix support | Yes — in hash INPUT only (output remains bare hex) |
| Primary consumers | `core/proofs/batch_hasher.py`, `core/proofs/signing.py` |

**Domain prefix behavior**: When `domain_prefix="witness:"` is specified:
- Hash input: `b"witness:" + json_bytes`
- Hash output: `"<hex>"` (bare, different value from non-prefixed hash due to different input)

### 2.5 Contract Comparison

| Feature | GOVERNANCE_V1 | CANONICALIZATION_V1 | PROOFS_V1 |
|---------|--------------|---------------------|-----------|
| `ensure_ascii` | `False` | `True` | `True` |
| Hash prefix | `sha256:` | None | None |
| Domain prefix | In input + output | Not supported | In input only |
| Non-ASCII | UTF-8 literal | `\uXXXX` escaped | `\uXXXX` escaped |

**Consequence**: For ASCII-only inputs without domain prefixes, CANONICALIZATION_V1 and PROOFS_V1 produce identical hex digests (but GOVERNANCE_V1 produces a different string due to the `sha256:` prefix, even though the underlying hex is the same). For non-ASCII inputs, all three may differ.

---

## 3. Canonical JSON Serialization

### 3.1 Contract-Aware Serialization

Source: `core/hashing/contracts.py:96`

```python
def canonical_json_bytes(obj: Any, *, contract: HashContract) -> bytes
def canonical_json_str(obj: Any, *, contract: HashContract) -> str
```

Both use `json.dumps()` with contract-specific `ensure_ascii` setting. All other parameters are shared: `sort_keys=True`, `separators=(",", ":")`, `allow_nan=False`. Output is UTF-8 encoded bytes.

### 3.2 Standalone Canonical JSON

Source: `core/canonicalization/json.py:11`

```python
def canonical_json_dumps(obj: Any) -> str
def canonical_json_bytes(obj: Any) -> bytes
```

These use `ensure_ascii=True` (matching CANONICALIZATION_V1 semantics). They predate the explicit contract system and are used by the `VersionedHasher`, `RealizationSpec.compute_digest()`, and other modules that need canonical serialization without governance context.

### 3.3 Serialization Invariants

1. **Key ordering**: `sort_keys=True` — keys are sorted lexicographically at every nesting level.
2. **No whitespace**: `separators=(",", ":")` — no spaces after colons or commas.
3. **No trailing newline**: `json.dumps()` does not append newline.
4. **No NaN/Infinity**: `allow_nan=False` — raises on non-finite floats.
5. **Deterministic**: Given the same Python object, produces identical bytes on every call, every platform (assuming CPython's `json` module, which is stable).
6. **No `default=str` on identity paths (DET-1A)**: Callers that produce content hashes for identity surfaces (loop detection, certification, replay) must use `json.dumps()` **without** `default=str`. Non-serializable values must be excluded before reaching the serialization boundary (e.g., via `WorldState.public_assumptions`). This is enforced in `core/contracts/semantic_edits.py` for `compute_content_hash` and `compute_trace_hash`.

**Platform note**: These invariants hold for CPython's `json` module. If Sterling is ever ported to a different Python implementation, the canonical JSON output must be verified against golden vectors.

---

## 4. SHA-256 Hashing

### 4.1 Hash Computation

Source: `core/hashing/contracts.py:174`

```python
def sha256_hash(data: bytes, *, contract: HashContract, domain_prefix: str = "") -> str
```

Steps:
1. If `domain_prefix` is set, prepend `domain_prefix.encode("utf-8")` to the data.
2. Compute `hashlib.sha256(hash_input).hexdigest()`.
3. Format per contract: GOVERNANCE_V1 prepends `"sha256:"` (and optionally the domain prefix); others return bare hex.

**Domain prefix restriction**: `domain_prefix` is only supported for PROOFS_V1 and GOVERNANCE_V1. Using it with CANONICALIZATION_V1 raises `ValueError`.

### 4.2 Content Hash (Convenience)

Source: `core/hashing/contracts.py:226`

```python
def compute_content_hash(obj: Any, *, contract: HashContract, domain_prefix: str = "") -> str
```

Combines `canonical_json_bytes()` and `sha256_hash()` in one call. This is the recommended entry point for hashing JSON-serializable objects.

---

## 5. Hash Normalization and Validation

Source: `core/canonicalization/hashes.py`

The hash normalization layer handles parsing, validating, and converting between hash string formats. It defines two strictness tiers:

### 5.1 Strict Validators (Certification Path)

| Function | Input | Output | Use Case |
|----------|-------|--------|----------|
| `validate_sha256_prefixed(value)` | Must have `sha256:` prefix | `sha256:<64-hex-lowercase>` | New binding seams (evidence, payload, env) |
| `validate_sha256_hex64(value)` | Bare hex or prefixed | `<64-hex-lowercase>` (no prefix) | TD-12 v1 identity fields |

`validate_sha256_prefixed` rejects bare 64-hex strings in strict mode. This enforces that new code always uses the prefixed form.

### 5.2 Normalization (Dev Path)

| Function | Input | Output | Use Case |
|----------|-------|--------|----------|
| `normalize_hash_id(value)` | Hex or prefixed | `sha256:<64-hex-lowercase>` | General normalization |
| `normalize_sha256_dev(value)` | Hex or prefixed | `sha256:<64-hex-lowercase>` | Dev tooling ingestion |
| `normalize_hash_permissive(value)` | Anything | `sha256:<value>` or `sha256:UNKNOWN` | Reporting/bundling |

### 5.3 Extraction

| Function | Input | Output |
|----------|-------|--------|
| `extract_hash_value(value)` | Hex or prefixed | `<64-hex-lowercase>` |
| `parse_hash_id(value)` | Hex or prefixed | `("sha256", "<hex>")` |
| `is_sha256_prefixed(value)` | Any string | `bool` |

### 5.4 Invariants

1. **Single prefix**: Double prefixes (`sha256:sha256:...`) are rejected by strict validators.
2. **Case sensitivity**: Prefix must be lowercase `sha256:` — mixed case is rejected.
3. **Hex length**: Digest must be exactly 64 hex characters.
4. **Lowercase output**: All hex output is normalized to lowercase.
5. **Idempotence**: `normalize_hash_id(normalize_hash_id(x)) == normalize_hash_id(x)`.

---

## 6. Version-Gated Hashing

Source: `core/canonicalization/versioned_hash.py`

For artifact identity migration, Sterling provides version-gated hashing. This allows hash scheme evolution without breaking existing artifact references.

### 6.1 Hash Versions

| Version | Function | Format | Semantics |
|---------|----------|--------|-----------|
| `v1` | `hash_v1(content)` | `v1:sha256:<hex>` | SHA-256 of `canonical_json_dumps(content)` |
| `v2` | `hash_v2_incremental(content, base_hash?)` | `v2:sha256:<hex>` | SHA-256 with optional base hash chaining |

**V2 chaining**: If `base_hash` is provided, the raw hex is extracted and prepended to the canonical JSON: `f"{raw_base}:{canonical}"`. This enables efficient re-hashing when only part of the content changes.

### 6.2 VersionedHasher

Source: `core/canonicalization/versioned_hash.py:99`

```python
class VersionedHasher:
    def hash(self, content: Any) -> str           # Uses current version
    def hash_with_version(self, content, version)  # Uses specified version
    def verify(self, content, stored_hash) -> bool  # Auto-detects version from prefix
```

**Version detection**: Extracts version from the first colon-delimited segment (e.g., `v1` from `v1:sha256:<hex>`). Falls back to `v1` for legacy hashes without version prefix.

**Default instance**: `_default_hasher = VersionedHasher()` uses `v1` as current version. Convenience functions `versioned_hash()` and `verify_versioned_hash()` delegate to it.

### 6.3 Migration Protocol

1. Define new hash function under new version key.
2. New artifacts automatically use the new version.
3. Existing artifacts remain verifiable via legacy version lookup.
4. After migration period, optionally rehash and update references.

---

## 7. Semantic Canonicalization

Source: `core/canonicalization/semantic.py`

Beyond JSON serialization, Sterling provides semantic canonicalization for complex Python objects that need stable representations.

### 7.1 Stable Sort Keys

`_stable_sort_key(obj)` produces a deterministic sort key for heterogeneous collections. The sort key is a tuple of `(type_name, *value_components)`, ensuring consistent ordering across types.

### 7.2 Semantic Dict Canonicalize

```python
def semantic_dict_canonicalize(
    obj: Any,
    value_transform: Optional[Callable[[Any], Any]] = None,
    skip_none: bool = False,
) -> Any
```

Produces a canonical representation of a Python object by:
1. Applying `value_transform` to the input if provided.
2. For dicts: stringifying keys (`str(key)`), sorting lexicographically, recursively canonicalizing values. Raises `CanonicalizationError` on duplicate string-coerced keys.
3. For lists/tuples: converting to list, recursively canonicalizing items.
4. For sets/frozensets: canonicalizing items, then sorting by `_stable_sort_key`.
5. Primitives (`str`, `int`, `float`, `bool`, `None`) are returned as-is.

### 7.3 Stable Repr

`stable_repr(v: Any) -> str` produces a deterministic string representation for logging/debugging. Uses `"∅"` for `None`, bespoke formatting for collections.

---

## 8. Witness Canonicalization

Source: `core/canonicalization/witness.py`

For evaluation evidence, Sterling defines versioned canonicalization functions for witness comparison:

| Function | Version Constant | Purpose |
|----------|-----------------|---------|
| `normalize_surface_for_match_v1()` | `SURFACE_NORM_VERSION = "surface_norm_v1"` | Normalize text surface for exact match |
| `surface_exact_match_v1()` | — | Compare two surfaces after normalization |
| `compute_goal_signature_v1()` | `GOAL_SIGNATURE_VERSION = "goal_signature_v1"` | Content-address a goal for deduplication |
| `goal_signature_match_v1()` | — | Compare two goal signatures |
| `canonicalize_witness_v1()` | `CANONICALIZE_WITNESS_VERSION = "canonicalize_witness_v1"` | Canonical form of an evaluation witness |

All version constants are embedded in output artifacts so that future changes to normalization rules can be detected and handled.

---

## 9. Golden Vectors

These hashes are canonical and MUST NOT change. They serve as the ABI contract.

### GOVERNANCE_V1

| Input | Hash |
|-------|------|
| `{"name": "test"}` | `sha256:7d9fd2051fc32b32feab10946fab6bb91426ab7e39aa5439289ed892864aa91d` |
| `{"count": 42}` | `sha256:b35539ce83f07b2fe8fb4bdce27fb9666c2122af4ad9a5850a711d906fb58998` |
| `{"name": "José"}` | `sha256:0264c9b67e4687fb6eb775c9517fa029a75fa2b3e041da81bd8703885d13647b` |

### CANONICALIZATION_V1

| Input | Hash |
|-------|------|
| `{"name": "test"}` | `7d9fd2051fc32b32feab10946fab6bb91426ab7e39aa5439289ed892864aa91d` |
| `{"name": "José"}` | `782f7fb6e7349477ad0878467428033420f78fc728c94d07ebb1d49d7cbae82e` |

**Test file**: `tests/unit/test_hashing_golden_vectors.py`

---

## 10. Invariants Summary

1. **No silent defaults**: Every hashing call site specifies its contract explicitly.
2. **Contract stability**: Changing hash output for an existing contract is forbidden. Create a new version instead.
3. **Serialization determinism**: `canonical_json_bytes(obj, contract=C)` produces identical bytes for identical objects, every time.
4. **Prefix consistency**: GOVERNANCE_V1 always returns `sha256:` prefixed output. CANONICALIZATION_V1 and PROOFS_V1 always return bare hex.
5. **Domain separation**: Domain prefixes are included in the hash INPUT, providing true cryptographic domain separation.
6. **Hash length**: All SHA-256 hex digests are exactly 64 lowercase characters.
7. **No NaN/Infinity**: Non-finite floats cause serialization failure (not silent corruption).
8. **Version gating**: Hash scheme changes are version-gated so old artifacts remain verifiable.
9. **DET-1A (no `default=str`)**: Identity-path serialization must not use `default=str` or any fallback that silently coerces non-serializable objects. Filtering happens before serialization (via `public_assumptions`), not during it.

---

## 11. Source File Index

| File | Purpose |
|------|---------|
| `core/hashing/__init__.py` | Package exports |
| `core/hashing/contracts.py` | HashContract enum, serialization, hashing functions |
| `core/canonicalization/__init__.py` | Canonicalization package exports |
| `core/canonicalization/json.py` | Standalone canonical JSON (ensure_ascii=True) |
| `core/canonicalization/hashes.py` | Hash parsing, validation, normalization |
| `core/canonicalization/versioned_hash.py` | Version-gated hashing for artifact migration |
| `core/canonicalization/semantic.py` | Semantic dict canonicalization |
| `core/canonicalization/witness.py` | Witness canonicalization for evaluation evidence |
| `core/canonicalization/errors.py` | CanonicalizationError, StrictCanonicalizationError |
| `tests/unit/test_hashing_golden_vectors.py` | Hash ABI regression tests |
| `tests/unit/test_hash_normalization.py` | Normalization behavior tests |
| `tests/unit/test_hash_validators.py` | Validator strictness tests |

---

## 12. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [Code32 and ByteState](code32_bytestate.md) | ByteState evidence hashing uses `sha256(payload_bytes)` directly on raw bytes — no JSON serialization. The hash contract applies only to JSON-serializable metadata in headers/footers. |
| [Governance Certification](governance_certification_contract_v1.md) | GOVERNANCE_V1 contract is the primary hash contract for all governance artifacts. |
| [Proof Evidence System](proof_evidence_system_v1.md) | PROOFS_V1 contract is used for certificate hashes and batch witness digests. |
| [Conformance](conformance.md) | Golden hash vectors are conformance-level gates. |
| [State Model Contract](state_model_contract_v1.md) | `CANONICAL_VERSION = 2` and `public_assumptions` define the pre-serialization filtering boundary. |
| [Rust Parity Audit](../../rust_parity_audit.md) | Documents that Rust's `compute_node_key` (SHA-256) is internal-only and not a certification identity surface. |
