---
version: "1.0"
authority: canonical
date: 2026-02-15
author: "@darianrosebrook"
status: "Partially implemented (core compilation boundary operational; epoch transitions and dynamic domain handshake not yet wired)"
notice: "This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label."
---
# ByteState Compilation Boundary

---

## 1. Purpose

This document governs how **external domain payloads become ByteState** (and back). It does not define what Code32 is, how ByteState is laid out, or how evidence is hashed — those belong to [code32_bytestate.md](code32_bytestate.md).

The compilation boundary is the interface between:

- **(a) Versioned, dynamic artifact streams** — domain specs, operator catalogs, schema proposals, solve requests — that evolve over time through governed promotion.
- **(b) The frozen inner-loop tensor** — ByteStateV1 with a fixed schema, frozen registry, and static operator masks — that executes at millisecond scale.

The boundary exists because dynamicism and substrate stability have opposite requirements. Dynamic content needs evolution, versioning, and graceful schema migration. The substrate needs byte-for-byte determinism, fixed layout, and O(1) divergence localization. The compilation boundary reconciles these by making the transition between them explicit, deterministic, and auditable.

**Relationship to existing contracts**: This document extends the reusable contract shape template defined in [philosophy.md](philosophy.md) §9 and specializes those patterns for the ByteState substrate.

---

## 2. Invariants

### 2.1 Compilation Is a Pure Function

Given identical inputs, compilation produces identical ByteState bytes. The inputs are:

```
compile(payload, schema_descriptor, registry_snapshot) → ByteStateV1
```

Where:
- **payload**: The domain-specific content (solve request state, ARC grid, induced domain fields) in its transport representation.
- **schema_descriptor**: `(schema_id, schema_version, schema_hash)` identifying the ByteStateSchema to use.
- **registry_snapshot**: `(registry_epoch, registry_hash)` identifying the Code32 ↔ ConceptID mapping.

If any of these three inputs differ, the output may differ. If all three are identical, the output must be identical. This is the determinism contract.

### 2.2 Decompilation Preserves Domain Meaning

```
decompile(compile(payload, S, R), S, R) ≡ payload   (up to declared equivalence)
```

Not all domains have perfect invertibility. A payload may contain fields that are not represented in ByteState (human-readable descriptions, debug metadata, optional context). The equivalence relation is declared per schema and tested in conformance suites:

- **Fields in the identity plane**: Must round-trip exactly (concept identity is preserved).
- **Fields in the status plane**: Must round-trip exactly (governance state is preserved).
- **Fields outside ByteState scope**: Declared as non-compiled in the schema. Not expected to survive round-trip. Must not affect reasoning.

### 2.3 The Substrate Is Not Dynamically Mutated

ByteState is static per epoch. The compilation boundary enforces this by construction:

- The **active epoch** has a frozen registry and fixed schema. All compilation during this epoch uses those frozen artifacts.
- **Schema evolution** (new fields, new domains, new concept types) produces a *new versioned artifact* — a proposed schema or registry extension — that enters the promotion pipeline.
- **Epoch transitions** happen between episodes, not during them. A new epoch activates a new registry snapshot and (optionally) a new schema version. In-flight episodes are never affected by epoch transitions.

This means induction does not mutate the substrate at runtime. It produces artifacts that, once promoted, define the next epoch's compilation parameters.

### 2.4 Fail-Closed on Mismatch

If the compilation boundary detects any of:

- Schema version mismatch between request and active epoch
- Registry epoch mismatch (request references a registry version not active)
- Unknown concept in payload (no Code32 allocation exists)
- Payload field that violates schema constraints (wrong type, out-of-range, violates invariant)

...compilation fails closed. No partial ByteState is produced. The failure is a typed artifact:

```python
@dataclass(frozen=True)
class CompilationFailure:
    """Typed compilation failure — never silently degraded."""
    failure_type: str       # "schema_mismatch" | "registry_mismatch" | "unknown_concept" | "constraint_violation"
    detail: str             # Human-readable description
    request_digest: str     # Hash of the compilation request for audit
    schema_descriptor: tuple[str, str, str]  # (id, version, hash)
    registry_descriptor: tuple[str, str]     # (epoch, hash)
```

---

## 3. The Envelope

The compilation boundary adopts the standard envelope template from [philosophy.md](philosophy.md) §6 (v1 reference: [`philosophy_full.md`](../reference/v1/philosophy_full.md) §6):

```
{
    request_version,       // Schema version of this compilation request
    capability_id,         // Which primitive or solve type is being compiled for
    domain_id,             // Source domain (e.g., "pn", "wordnet", "minecraft", "arc")
    stream_id,             // Versioned artifact stream this request belongs to
    epoch,                 // Target registry epoch for compilation
    seq,                   // Sequence number within the stream (monotonic)
    tick_id,               // Logical time (not wall clock — deterministic)
    payload                // The domain-specific content to compile
}
```

**Envelope vs payload hashing** mirrors the ByteTraceV1 split (see [code32_bytestate.md](code32_bytestate.md) §4.2):

| Field | Hashed? | Why |
|-------|---------|-----|
| `request_version` | Yes | Determines compilation behavior |
| `capability_id` | Yes | Determines which schema applies |
| `domain_id` | Yes | Determines registry partition |
| `stream_id` | No | Observability / routing only |
| `epoch` | Yes | Determines which registry snapshot |
| `seq` | Yes | Ordering determinism |
| `tick_id` | No | Observability only (wall clock may vary) |
| `payload` | Yes | The actual content being compiled |

The **compilation manifest hash** covers all hashed fields:

```
compilation_manifest_hash = sha256(
    canonical_json(request_version, capability_id, domain_id, epoch, seq, payload)
)
```

---

## 4. Compilation Artifacts

### 4.1 CompilationRequestV1

A compilation request bundles the envelope with the schema and registry descriptors needed to compile it:

```python
@dataclass(frozen=True)
class CompilationRequestV1:
    """Everything needed to compile a domain payload into ByteState."""

    # Envelope fields (hashed subset)
    request_version: str
    capability_id: str
    domain_id: str
    epoch: str
    seq: int
    payload: bytes              # Canonical JSON bytes of domain-specific content

    # Compilation parameters
    schema_descriptor: tuple[str, str, str]   # (schema_id, schema_version, schema_hash)
    registry_descriptor: tuple[str, str]       # (registry_epoch, registry_hash)

    # Envelope fields (not hashed)
    stream_id: str = ""
    tick_id: str = ""

    def manifest_hash(self) -> str:
        """Deterministic hash of all compilation-relevant fields."""
        ...
```

### 4.2 CompilationResultV1

A successful compilation produces:

```python
@dataclass(frozen=True)
class CompilationResultV1:
    """The output of compiling a domain payload into ByteState."""

    # The compiled state
    identity_plane: bytes       # uint8[layer_count * slot_count * 4]
    status_plane: bytes         # uint8[layer_count * slot_count]

    # Provenance
    request_manifest_hash: str  # Hash of the CompilationRequestV1 that produced this
    schema_descriptor: tuple[str, str, str]
    registry_descriptor: tuple[str, str]

    # Compilation manifest (canonical JSON, deterministic)
    compilation_manifest: bytes  # Records every dependency hash used during compilation

    def result_hash(self) -> str:
        """Hash of identity + status planes. Must match ByteState hashing rules."""
        ...
```

The **compilation manifest** is a canonical JSON document listing every dependency consumed during compilation:

```json
{
    "request_manifest_hash": "sha256:...",
    "schema_id": "pn_v1",
    "schema_version": "1.0",
    "schema_hash": "sha256:...",
    "registry_epoch": "2026-02-15-001",
    "registry_hash": "sha256:...",
    "concept_codes_used": ["0x01020003", "0x01020004", ...],
    "slot_mapping": {"syntax.head": 0, "syntax.dep": 1, ...}
}
```

This manifest is stored alongside the ByteState artifact. It enables:
- **Audit**: Which concepts were compiled, under which registry, into which slots.
- **Replay**: Given the same manifest inputs, re-compilation must produce identical bytes.
- **Drift detection**: If a concept's Code32 allocation changes between epochs, the manifest identifies which compilations are affected.

### 4.3 DecompilationResultV1

Decompilation (ByteState back to domain terms) produces:

```python
@dataclass(frozen=True)
class DecompilationResultV1:
    """The output of decompiling ByteState back into domain terms."""

    # The decompiled payload
    payload: bytes              # Canonical JSON bytes of domain-specific content

    # Provenance
    identity_plane_hash: str    # Hash of the source identity plane
    status_plane_hash: str      # Hash of the source status plane
    schema_descriptor: tuple[str, str, str]
    registry_descriptor: tuple[str, str]

    # Equivalence witness
    round_trip_ok: bool         # True if compile(decompile(x)) == x for identity+status planes
```

---

## 5. Conformance Suites

### 5.1 Golden Fixture Tests

For each domain schema, a set of golden fixtures defines the expected compilation:

```
Given:
    payload = <canonical JSON of domain state>
    schema  = <schema_id, version, hash>
    registry = <epoch, hash>
Expected:
    identity_plane = <exact bytes>
    status_plane   = <exact bytes>
```

These fixtures are content-addressed and stored alongside the schema. Any change to the compilation logic that alters the output for a golden fixture requires a schema version bump.

### 5.2 Round-Trip Tests

For each domain schema, round-trip equivalence is tested:

```
For every golden fixture payload P:
    result = compile(P, S, R)
    P'     = decompile(result, S, R)
    assert equivalence(P, P', declared_equivalence_relation)

    result' = compile(P', S, R)
    assert result.identity_plane == result'.identity_plane
    assert result.status_plane   == result'.status_plane
```

The second assertion (compile-of-decompile matches original compilation) is the critical one: it proves the round-trip is stable, not just "close."

### 5.3 Cross-Epoch Migration Tests

When a new registry epoch introduces remapped Code32 allocations:

```
For every golden fixture payload P:
    result_old = compile(P, S, R_old)
    result_new = compile(P, S, R_new)

    # Identity plane bytes may differ (different Code32 allocations)
    # But decompiled meaning must be identical:
    P_old = decompile(result_old, S, R_old)
    P_new = decompile(result_new, S, R_new)
    assert equivalence(P_old, P_new, declared_equivalence_relation)
```

This proves that registry remapping preserves semantic meaning even though the byte representation changes.

---

## 6. The Dynamic Domains Handshake

### 6.1 How Induced Domains Enter the System

Domain induction (`DomainMiner`, `DomainSpecIR`) produces **proposed artifacts**, not runtime mutations. The handshake follows the `DomainDeclarationV1` / `DomainSessionV1` split:

| Artifact | Lifecycle | Content-Addressed? | What It Contains |
|----------|-----------|-------------------|-----------------|
| `DomainDeclarationV1` | Long-lived, changes on re-certification | Yes | Primitive claims, budgets, supported extensions |
| `DomainSessionV1` | Ephemeral, per-connection | No | KG ref, operator pack, active schema binding |
| `ByteStateSchema` (new) | Long-lived, changes on schema version bump | Yes (via `schema_hash`) | Layout, slot semantics, capacity budgets |
| `RegistryEpoch` (new) | Long-lived, changes on epoch transition | Yes (via `registry_hash`) | Code32 ↔ ConceptID bijection snapshot |

An induced domain proposes:

1. A **DomainDeclarationV1** declaring its capabilities and claiming primitives.
2. A **ByteStateSchema** declaring its layout (layer count, slot count, layer semantics, capacity budgets).
3. **Code32 allocation requests** for new concepts discovered during induction.

These proposals enter the promotion pipeline. They are not active until promoted.

### 6.2 Epoch Transitions

An epoch transition activates a new registry snapshot and (optionally) new schemas:

```
Epoch N (active):
    registry_N: frozen Code32 ↔ ConceptID mapping
    schemas_N:  frozen set of ByteStateSchemas
    ↓
    [episodes execute using registry_N + schemas_N]
    ↓
    [between episodes: induction produces proposals]
    ↓
    [promotion gates: proposals pass certification pipeline]
    ↓
Epoch N+1 (activate):
    registry_N+1: registry_N + newly allocated Code32 entries (append-only)
    schemas_N+1:  schemas_N + new/updated schemas (version-bumped)
```

**Epoch transition rules**:

- **Append-only registry**: Epoch N+1 includes all Code32 allocations from Epoch N, plus any newly promoted allocations. No existing allocations are removed or reassigned. This ensures that ByteState artifacts from Epoch N remain readable under Epoch N+1.
- **Schema version bumps**: A schema change within a domain requires a version bump. The old version remains available for decompilation of historical artifacts.
- **No mid-episode transitions**: An episode that starts under Epoch N completes under Epoch N. Epoch transitions are between-episode boundaries.
- **Replay uses epoch artifacts**: Replaying a trace from Epoch N uses registry_N and schemas_N, regardless of the current epoch. The epoch descriptor is embedded in the ByteTraceV1 header.

### 6.3 How External Domains Deliver Rules at Solve Time

The Minecraft pattern — "the bot sends rules and state to Sterling at solve time; Sterling does not need domain-specific knowledge baked in" — is the general case for the compilation boundary.

An external domain (Minecraft, ARC, any future rig) delivers:

1. **A solve request** following the `SolveRequestV1` shape.
2. **Domain-specific operators** as typed definitions (preconditions, effects, costs) — not arbitrary code.
3. **State** in the domain's transport representation.

The compilation boundary's job is to:

1. **Validate** the request against the active schema and registry epoch.
2. **Compile** the state into ByteState (identity + status planes).
3. **Compile** operator definitions into ByteState masks (precondition_mask/value, effect_mask/value, status_effect_mask/value).
4. **Execute** the inner loop over ByteState tensors.
5. **Decompile** the result back into domain terms for the response.

Steps 2 and 3 are the compilation boundary in action. Steps 1, 4, and 5 are the surrounding I/O contract and substrate execution.

**For operators that cannot compile to masks** (relational operators requiring graph traversal, constraint propagation, or other non-positional checks): the compilation boundary compiles the local/positional part into masks and leaves the relational part as an index lookup callback. This matches the scope-of-advantage analysis in [code32_bytestate.md](code32_bytestate.md) §6: ByteState accelerates local operations; relational operators still use indexes.

### 6.4 How Learned Operators Enter ByteState

Learned operators (from induction, ARC macro discovery, or online learning) follow the same path as any other operator:

1. **Proposal**: Induction produces an `OperatorSketchCoreIR` with preconditions, effects, and provenance.
2. **Synthesis**: `OperatorSynthesizer` compiles the sketch into the constrained Operator IR.
3. **Verification**: The Operator IR verifier checks that the operator only touches declared fields and satisfies invariants.
4. **Mask compilation**: If the operator's preconditions and effects are positional (local), they compile directly into ByteState masks (`uint32` precondition_mask/value, effect_mask/value). If they include relational components, the local part compiles to masks and the relational part remains as a typed callback.
5. **Promotion**: The compiled operator enters the shadow lane (soft influence only) or, after passing the certification pipeline, the production lane.

The key constraint: **learned operators do not get special treatment**. They enter ByteState through the same compilation boundary as hand-authored operators. The boundary enforces that they respect the active schema, only touch declared fields, and produce deterministic effects.

---

## 7. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [code32_bytestate.md](code32_bytestate.md) | Defines the substrate this boundary compiles into. This doc does not redefine Code32 layout, hashing, or evidence format. |
| [philosophy.md](philosophy.md) | Provides the contract shape template and the "dynamic as versioned streams" principle that this doc operationalizes for ByteState. |
| [neural_usage_contract.md](neural_usage_contract.md) | Neural models may observe ByteState but not write to it. The compilation boundary enforces this: only governed symbolic compilation writes ByteState. |
| [text_boundary_index.md](../reference/v1/canonical/text_boundary_index.md) *(v1 reference)* | The text boundary (Text → IR) is a domain-specific instance of the compilation boundary (Domain Payload → ByteState). Both share the same trust model: boundary artifacts are hash-critical; transport metadata is not. |
| [minecraft_domains.md](../reference/v1/minecraft_domains.md) *(v1 reference)* | Existence proof: Minecraft already delivers rules + state at solve time. The compilation boundary formalizes and generalizes this pattern for ByteState. |

---

## 8. Implementation Status

As of 2026-02-16, the following aspects of this boundary are implemented:

**Implemented:**
- Pure-function compilation: `DomainCompiler` protocol in `core/carrier/compiler.py` with `compile_step_log()` and `compile_events()` methods
- Four domain compilers: Rome, Mastermind, EscapeGame, WordNet (in `core/carrier/schemas/`)
- `RegistrySnapshotV1` with content-addressed epoch hashing (`core/carrier/compiler.py`)
- `OperatorCodeBookV1` with deterministic operator→Code32 mapping (`core/carrier/operator_codebook.py`)
- Fail-closed behavior: unknown concepts raise `KeyError`; schema mismatches raise `ValueError`
- ByteTraceV1 envelope/payload split with deterministic payload hashing (`core/carrier/bytetrace.py`)
- Golden conformance tests: byte-for-byte regression gates for all 4 domains (`tests/conformance/carrier/`)
- Round-trip tests: `compile_state()` → `decompile_state()` equivalence per domain
- Registry materialization CLI: `scripts/carrier/generate_registry.py` (from traces or fixtures)

**Not yet implemented:**
- `CompilationRequestV1` / `CompilationResultV1` / `DecompilationResultV1` as formal dataclasses (§4) — compilation currently uses domain compiler methods directly
- `CompilationFailure` as a typed artifact (§2.4) — failures currently raise Python exceptions
- Cross-epoch migration tests (§5.3) — only single-epoch compilation tested
- Epoch transition machinery (§6.2) — registries are per-run, not yet governed by promotion pipeline
- Dynamic domain handshake (§6.1) — domain compilers are statically registered
- Learned operator compilation (§6.4) — operator codebooks are hand-authored

---

## 9. Summary

The compilation boundary is not part of the substrate. It is the **codec** between dynamic, evolving domain content and the frozen, deterministic inner loop.

The key properties:

1. **Compilation is pure**: Same inputs produce same bytes. No hidden state, no ambient configuration.
2. **Decompilation preserves meaning**: Round-trip equivalence is tested per schema, not assumed.
3. **The substrate is never dynamically mutated**: Schema evolution happens through versioned artifacts and epoch transitions, not runtime mutation.
4. **Fail-closed on mismatch**: Unknown concepts, schema mismatches, and constraint violations produce typed failures, not partial results.
5. **Learned operators use the same boundary**: Induction does not bypass the compilation boundary. Learned operators compile to masks through the same path as hand-authored ones.
6. **Epoch transitions are the reconciliation point**: New concepts, new schemas, and new operators activate between episodes. In-flight episodes are never affected.
7. **The Minecraft pattern is the general case**: External domains deliver rules and state at solve time; Sterling compiles them into its internal representation without baking in domain knowledge.
