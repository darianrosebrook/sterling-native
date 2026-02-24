> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Operator Registry Contract v1

**Version**: 1.1
**Date**: 2026-02-17
**Status**: Canonical specification — sufficient to rebuild `core/operators/` from scratch.
**Scope**: Operator signatures, registry storage, applicability, execution, and enforcement.
**Layer**: 1 (Operators)
**Changelog**: 1.1 — Add UNKNOWN category, CANON_ONLY resolution mode, family_tag field, ParamSpec required/default, fix LabelConstraints field name/types, add missing precondition/effect entries, expand source file index.

---

## §1 Purpose

The Operator Registry is Sterling's central catalog of certified state transformations. Every operator that can modify a `StateNode` must be registered here with a signature declaring what it reads, writes, requires, and guarantees. The registry enforces these contracts at execution time.

---

## §2 OperatorSignature

The atomic unit of operator identity. Frozen dataclass — immutable after construction.

```
OperatorSignature (frozen)
├── name: str                        # Unique name (e.g., "APPLY_NEGATION")
├── category: str                    # One of: S, M, P, K, C
│   S = Structural (surface syntax)
│   M = Meaning (semantic transforms)
│   P = Pragmatic (discourse-level)
│   K = Knowledge (world/KG traversal)
│   C = Control (meta/orchestration)
├── reads: list[str]                 # Layers read (e.g., ["syntax", "semantics"])
├── writes: list[str]                # Layers modified (e.g., ["semantics"])
├── scope: str                       # "utterance" | "discourse" | "world"
├── preconditions: list[str]         # Named predicate keys (e.g., ["has_semantics"])
├── effects: list[str]               # Named effect assertion keys (e.g., ["polarity_flipped"])
├── operation_id: str                # Canonical URI: "sterling:op/<NAME>"
├── family_tag: str | None           # Optional grouping tag
├── worlds: set[str]                 # World memberships (e.g., {"pn", "text"})
├── fallback_priority: int           # Lower = higher priority (default 0)
├── label_constraints: LabelConstraints | None
├── params: list[ParamSpec]          # Parameter schema for parameterized operators
└── description: str | None          # Human-readable description
```

### §2.1 OperatorCategory enum

| Code | Name | Typical Reads | Typical Writes |
|------|------|--------------|----------------|
| `?` | Unknown | (unclassified) | (unclassified) |
| `S` | Structural | syntax | syntax |
| `M` | Meaning | semantics | semantics |
| `P` | Pragmatic | pragmatics | pragmatics |
| `K` | Knowledge | semantics | world_state |
| `C` | Control | (varies) | (varies) |

`OperatorCategory` is a `str, Enum` mixin — `OperatorCategory.MEANING == "M"` evaluates to `True`.

### §2.2 OperatorScope enum

| Value | Meaning |
|-------|---------|
| `utterance` | Operates on a single utterance's layers |
| `discourse` | Operates across multiple utterances |
| `world` | Operates on world state / knowledge graph |

### §2.3 LabelConstraints

Optional type-level constraints for label-aware operator filtering.

```
LabelConstraints (mutable dataclass)
├── allowed_subject_types: Set[str]   # e.g., {"schema:Person"}
├── allowed_object_types: Set[str]
├── allowed_properties: Set[str]      # e.g., {"sterling:hasAssignment"}
├── allowed_relations: Set[str]       # e.g., {"skos:broader"}
└── required_concepts: Set[str]
```

Methods: `has_constraints() -> bool`, `check(...) -> Tuple[bool, List[str]]`.

### §2.4 ParamSpec

Parameter schema for parameterized operators.

```
ParamSpec (frozen)
├── name: str             # e.g., "old_name"
├── type_hint: str        # e.g., "str", "int"
├── description: str      # Human-readable description (default "")
├── required: bool        # Whether the parameter is required (default True)
└── default: Any | None   # Default value if not required (default None)
```

### §2.5 operation_id derivation

If not explicitly provided, `operation_id` is computed as:
```
"sterling:op/" + name
```

Source: `core/operators/registry_types.py`

---

## §3 OperatorRegistry

Mutable container holding all registered operators.

### §3.1 Internal State

```
OperatorRegistry
├── _signatures: dict[str, OperatorSignature]          # name → signature
├── _implementations: dict[str, Callable]              # name → impl function
├── _by_operation_id: dict[str, str]                   # operation_id → name
├── _by_world: dict[str, list[OperatorSignature]]      # world → signatures
├── _applicability_index: ApplicabilityIndex | None     # Cached, invalidated on mutation
├── _shadow_store: ShadowOperatorStore                 # Tier 0 (induced sketches)
├── _certified_store: CertifiedOperatorStore           # Tier 1-2 (promoted operators)
└── _promotion_policy: PromotionPolicy                 # Promotion thresholds
```

### §3.2 Registration

```python
def register(signature: OperatorSignature, impl: Callable | None) -> None
```

1. Store signature by name
2. Map operation_id → name
3. Store implementation if provided
4. Index by world memberships
5. Invalidate applicability index

**Invariant R-1**: After `register(sig, impl)`, `get_signature(sig.name)` returns `sig`.

### §3.3 Lookup

| Method | Signature | Behavior |
|--------|-----------|----------|
| `get_signature(name_or_id)` | `str → OperatorSignature?` | Lookup by name, fallback to operation_id |
| `list_signatures()` | `→ list[OperatorSignature]` | All registered signatures |
| `list_for_world(world)` | `str → list[OperatorSignature]` | Signatures belonging to world |
| `is_registered(name_or_id)` | `str → bool` | Check existence |
| `iter_entries()` | `→ Iterable[(OperatorSignature, impl?)]` | Yield (sig, impl) pairs |

### §3.4 Applicability

```python
def applicable_operators(state: StateNode, world: str? = None,
                         governance_context: GovernanceContext? = None) -> list[str]
```

1. Build `ApplicabilityIndex` from signatures if missing (lazy, cached)
2. Extract available layers from state
3. If world provided, filter candidates to world membership
4. For each candidate, call `sig.can_apply(state, ...)` with governance context
5. Return names of applicable operators

**Invariant R-2**: `applicable_operators` is a pure function of (state, world, governance_context) — no side effects.

### §3.5 Label-Aware Queries

```python
def get_operators_for_entity_type(entity_type: str) -> list[str]
def get_operators_for_relation(relation: str) -> list[str]
def get_operators_for_property(property_id: str) -> list[str]
```

Filter operators by `LabelConstraints` fields. Used for type-directed operator selection.

Source: `core/operators/registry_store.py:394-416`

---

## §4 Precondition System

### §4.1 Precondition Object

```
Precondition (frozen)
├── name: str                        # Registry key (e.g., "has_semantics")
├── fn: (StateNode) → bool           # Predicate function
├── message: str                     # Human-readable failure message
└── required_layers: frozenset[str]  # Layers that must be present
```

### §4.2 Standard Preconditions

| Name | Required Layers | Checks |
|------|----------------|--------|
| `has_syntax` | {syntax} | `state.primary_utterance.syntax is not None` |
| `has_semantics` | {semantics} | `state.primary_utterance.semantics is not None` |
| `has_semiotics` | {semiotics} | `state.primary_utterance.semiotics is not None` |
| `has_pragmatics` | {pragmatics} | `state.primary_utterance.pragmatics is not None` |
| `has_latent` | {latent} | `state.primary_utterance.latent is not None` |
| `has_predication` | {semantics} | `len(propositions) > 0 or len(events) > 0` |
| `has_ambiguous_term` | {semiotics} | Any sense_id starts with `"ambiguous:"` |
| `has_entity_reference` | {semantics} | `len(entities) > 0` |
| `has_negation` | {semantics} | `polarity == "negative"` |
| `is_identity_pn` | {semantics} | `pn_type == "identity"` |
| `has_contraction` | {syntax} | Delegates to `core.operators.pn.linguistics` |
| `has_copula_variant` | {syntax} | Delegates to `core.operators.pn.linguistics` |
| `has_determiner_mismatch` | {syntax} | Delegates to `core.operators.pn.linguistics` |
| `has_swappable_subject_predicate` | {syntax, semantics} | Delegates to `core.operators.pn.linguistics` |
| `has_clitic_copula` | {syntax} | Delegates to `core.operators.pn.linguistics` |
| `has_symbol_reference` | {semantics} | `len(entities) > 0` (code domain) |
| `has_extractable_block` | {semantics} | `len(propositions) >= 2` (code domain) |
| `has_inlineable_variable` | {semantics} | Entity with `reference_count == 1` |
| `has_expression` | {semantics} | Expression node present (code domain) |

### §4.3 Precondition Resolution Modes

```
PreconditionResolutionMode
├── STRICT      # Canonical IDs only, fail if not in PreconditionRegistry
├── CANON_ONLY  # Skip unresolvable preconditions silently
└── COMPAT      # Fallback to base name lookup in PRECONDITION_PREDICATES
```

Governance-aware preconditions accept optional `governance_context` parameter via signature introspection.

Source: `core/operators/registry_types.py:97-102`

---

## §5 Effect System

### §5.1 Effect Assertions

| Name | Checks (pre_state, post_state) |
|------|-------------------------------|
| `syntax_modified` | Syntax tokens differ between pre and post |
| `semantics_modified` | Semantic layer differs between pre and post |
| `polarity_flipped` | Polarity changed between pre and post |
| `entity_activated` | More activated entities in post than pre |
| `sense_resolved` | Ambiguous sense in pre resolved to non-ambiguous in post |

### §5.2 Effect Resolution Modes

```
EffectResolutionMode
├── STRICT      # Canonical IDs only, fail if not in EffectRegistry
├── CANON_ONLY  # Skip unresolvable effects silently
└── COMPAT      # Fallback to base name lookup in EFFECT_ASSERTIONS
```

Source: `core/operators/registry_types.py:105-110`

---

## §6 Execution Contract

### §6.1 Gated Execution (K6-A)

```python
def apply_gated(name, state, gate, verification_receipt, args?, enforcement_mode?,
                apply_fence?, packet?) -> (list[StateNode], ClaimDelta?)
```

**K6-A SEALED**: Direct `apply()` is permanently disabled. All operator execution goes through `apply_gated()`.

Execution sequence:
1. Verify receipt matches gate
2. Assert state matches gate's recorded state
3. Assert operator is in gate's applicable set
4. Assert operator is in universe
5. Check universe hash if expected
6. Resolve RC-9 implementation digest
7. Check preconditions (in STRICT/WARN mode)
8. Call implementation via `_call_operator_impl()`
9. Validate effects on successors (in STRICT/WARN mode)
10. Return (successors, claim_delta)

### §6.2 Enforcement Modes

```
EnforcementMode
├── STRICT   # Violations raise OperatorContractViolation
├── WARN     # Violations logged as warnings
└── OFF      # No enforcement (testing only)
```

### §6.3 Implementation Calling Convention

Operator implementations are called via `_call_operator_impl()` which introspects the function signature to pass:

- `state: StateNode` — always passed
- `args: dict[str, Any]` — always passed (positional)
- `governance_context: GovernanceContext?` — passed if function accepts it
- `packet: DecisionPacket?` — passed if function accepts it (Milestone 3.1.7)

Source: `core/operators/registry_resolve.py:56-114`

---

## §7 Registry Hash and Drift Detection

### §7.1 Registry Hash

```python
def compute_registry_hash() -> str
```

Deterministic hash of all operator signatures:
1. For each name in sorted order, build canonical entry dict
2. Serialize with `canonical_json_dumps()`
3. SHA-256 hash with `"sha256:"` prefix

Fields included: name, category, fallback_priority, reads (sorted), writes (sorted), scope, preconditions (sorted), effects (sorted), operation_id, worlds (sorted).

### §7.2 RegistrySnapshot

```
RegistrySnapshot (frozen)
├── registry_hash: str           # "sha256:..." digest
├── operator_count: int          # Number of registered operators
└── operator_names: tuple[str]   # Sorted operator names
```

### §7.3 Drift Detection

```python
def assert_registry_unchanged(before_hash: str) -> None
```

**Invariant R-3**: During promotion (Config C), the base registry must remain immutable. `assert_registry_unchanged()` raises `RegistryMutationError` if the hash changed — detecting ephemeral state leaks or improper overlay isolation.

Source: `core/operators/registry_store.py:315-388`

---

## §8 Built-in Operators

### §8.1 PN Operators (world: pn, text)

| Name | Category | Reads | Writes | Preconditions |
|------|----------|-------|--------|---------------|
| APPLY_NEGATION | M | semantics | semantics | has_semantics, has_predication |
| SWAP_SUBJECT_PREDICATE | M | syntax, semantics | syntax, semantics | has_syntax, has_semantics, is_identity_pn, has_swappable_subject_predicate |
| EXPAND_CONTRACTION | S | syntax | syntax | has_syntax, has_contraction |
| NORMALIZE_COPULA | S | syntax | syntax | has_syntax, has_copula_variant |
| STANDARDIZE_DETERMINER | S | syntax | syntax | has_syntax, has_determiner_mismatch |
| NORMALIZE_CLITIC_COPULA | S | syntax | syntax | has_syntax, has_clitic_copula |

### §8.2 WordNet Operators (world: wordnet, kg)

| Name | Category | Reads | Writes | Preconditions |
|------|----------|-------|--------|---------------|
| HYPERNYM_OF | K | semantics | world_state | has_entity_reference |
| HYPONYM_OF | K | semantics | world_state | has_entity_reference |
| MERONYM_OF | K | semantics | world_state | has_entity_reference |
| HOLONYM_OF | K | semantics | world_state | has_entity_reference |
| SIMILAR_TO | K | semantics | world_state | has_entity_reference |
| ANTONYM_OF | K | semantics | world_state | has_entity_reference |

### §8.3 Code Refactoring Operators (world: code_refactoring)

| Name | Category | Params | Preconditions |
|------|----------|--------|---------------|
| RENAME_SYMBOL | M | old_name: str, new_name: str | has_syntax, has_semantics, has_symbol_reference |
| EXTRACT_FUNCTION | S | start_line: int, end_line: int, new_function_name: str | has_syntax, has_semantics, has_extractable_block |
| INLINE_VARIABLE | S | variable_name: str | has_syntax, has_semantics, has_inlineable_variable |
| EXTRACT_VARIABLE | S | expression: str, variable_name: str | has_syntax, has_semantics, has_expression |

### §8.4 Factory

```python
def create_default_registry() -> OperatorRegistry
```

Iterates `iter_builtin_operator_entries()` which yields all PN, WordNet, and Code Refactoring operator (signature, implementation) pairs in sorted name order.

Source: `core/operators/registry_resolve.py:1078-1333`

---

## §9 Shadow Operator Integration

The registry integrates with the induction system's 3-tier operator store:

| Tier | Store | Purpose |
|------|-------|---------|
| 0 (Shadow) | `ShadowOperatorStore` | Uncertified induced sketches — influence search ranking only |
| 1 (Provisional) | `CertifiedOperatorStore` | Partially certified — limited execution |
| 2 (Production) | `CertifiedOperatorStore` | Fully certified — registered as normal operators |

### §9.1 Shadow Influence

```python
def get_shadow_influence(candidate_ids: list[str], world_id: str?) -> HypothesisInfluence?
```

Returns ranking hints from shadow store without granting execution rights. Shadow operators influence search priority but never directly modify state (TC-7A compliance).

### §9.2 Sketch Registration

```python
def register_sketch(core: OperatorSketchCoreIR, dossier: OperatorSketchDossierIR) -> None
```

Registers induced operator sketch in shadow store for influence tracking.

Source: `core/operators/registry_store.py:530-571`

---

## §10 Invariants

1. **R-1**: Registration is idempotent — re-registering the same signature overwrites silently.
2. **R-2**: `applicable_operators()` is pure — no side effects, deterministic for same inputs.
3. **R-3**: Registry hash must not change during promotion (Config C immutability).
4. **R-4**: `apply()` is permanently sealed (K6-A). Only `apply_gated()` executes operators.
5. **R-5**: Precondition predicates are pure functions of state (and optional governance context).
6. **R-6**: Effect assertions are pure functions of (pre_state, post_state).
7. **R-7**: Applicability index is invalidated on any registration. Stale index never used.
8. **R-8**: operation_id is unique per operator. Lookup by operation_id returns the same signature as lookup by name.

---

## §11 Related Documents

- [State Model Contract](state_model_contract_v1.md) — StateNode that operators transform
- [Governance & Certification](governance_certification_contract_v1.md) — Gate/verdict system operators execute through
- [Operator Induction Contract](operator_induction_contract_v1.md) — How new operators are induced and promoted
- [Hashing Contracts](hashing_contracts_v1.md) — Canonical JSON serialization used for registry hash

---

## §12 Source File Index

| File | Defines |
|------|---------|
| `core/operators/registry_types.py` | OperatorSignature, OperatorCategory, OperatorScope, RegistrySnapshot, EnforcementMode, LabelConstraints, ParamSpec, Precondition, OperatorCall, PreconditionResolutionMode, EffectResolutionMode, exceptions |
| `core/operators/registry_store.py` | OperatorRegistry, create_default_registry, registry hash computation |
| `core/operators/registry_resolve.py` | Precondition predicates, effect assertions, operator implementations, built-in signatures |
| `core/operators/registry_view.py` | Read-only registry views |
| `core/operators/registry_protocol.py` | Registry protocol interface |
| `core/operators/registry_profiles.py` | Pre-configured registry profiles, WordNet operator factory |
| `core/operators/registry.py` | Public facade (re-exports) |
| `core/operators/preconditions.py` | PreconditionID, PreconditionRegistry, standard precondition constants |
| `core/operators/effects.py` | EffectID, EffectRegistry, EffectAssertion, standard effect constants |
| `core/operators/promotion_policy.py` | PromotionPolicy (tier thresholds, MDL coefficients) |
| `core/operators/promotion_service.py` | PromotionService (tier transition orchestration) |
| `core/operators/promoted_operator.py` | PromotedOperatorIR, CommittedPayload, ProvenanceBinding |
| `core/operators/promotion_token.py` | Promotion token types |
| `core/operators/inventory.py` | OperatorInventory, OperatorEntry, global singleton |
| `core/operators/execution_context.py` | GovernanceContext (operator execution), ExecutionContextBuilder |
| `core/operators/bundle.py` | Operator bundle assembly |
| `core/operators/bundle_schemas.py` | Bundle schema definitions |
| `core/operators/canonicalization.py` | Operator canonicalization utilities |
| `core/operators/certified_bundle.py` | Certified operator bundles |
| `core/operators/certified_loader.py` | Certified bundle loader |
| `core/operators/certified_store.py` | CertifiedOperatorStore (Tier 1-2) |
| `core/operators/contract.py` | Operator contract definitions |
| `core/operators/episode_chain.py` | Episode chain tracking |
| `core/operators/errors.py` | Operator error hierarchy |
| `core/operators/evidence_contract.py` | Operator evidence contracts |
| `core/operators/execution_attestation.py` | Execution attestation records |
| `core/operators/fence.py` | Operator fence/boundary enforcement |
| `core/operators/gate.py` | Operator gate checks |
| `core/operators/governance_runtime.py` | Runtime governance integration |
| `core/operators/operator_ir_v1.py` | Operator IR v1 format |
| `core/operators/operator_ir_verifier.py` | IR verification |
| `core/operators/scope_allowlist.py` | Scope allowlist enforcement |
| `core/operators/shadow_store.py` | ShadowOperatorStore (Tier 0) |
| `core/operators/text_semantic_ops.py` | Text semantic operator implementations |
| `core/operators/universe.py` | Operator universe enumeration |
| `core/operators/pn/linguistics.py` | PN domain linguistic operators |
| `core/operators/pn/operators.py` | PN domain operators |
| `core/optimization/applicability_index.py` | ApplicabilityIndex for fast operator filtering |
