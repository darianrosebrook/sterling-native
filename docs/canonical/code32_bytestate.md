---
version: "1.3"
authority: canonical
date: 2026-02-24
author: "@darianrosebrook"
status: "Implemented (V1 substrate complete)"
changelog: "1.1 — Fix identity/status conflation, deterministic hashing, capacity policy, endianness, frame layout. 1.1a — Status update: V1 implementation complete across four domains (Rome, Mastermind, EscapeGame, WordNet). Core carrier modules, domain compilers, ByteTraceV1 format, content-addressed hashing, divergence localization, and golden conformance tests all operational. See §7 for updated research path status. 1.2 — Fix INITIAL_STATE_SENTINEL value, document payload hash domain prefix, add source file index. 1.3 — Fix sentinel to_uint32() values to match little-endian byte ordering (0x00010000 not 0x00000001)."
notice: "This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label."
---
# Code32 and ByteStateV1: Hardware-Native Semantic Representation

---

## Supersedes

This document supersedes the ViT-based compression framing of "Sterling Full" (January 24, 2026). The prior framing treated RGBA as a route to a ViT-shaped compressor (IR → RGBA grid → ViT latent → CoreML). That approach has the wrong inductive bias: Sterling's IR is sequential, not spatial, and forcing it into a 2D grid for a vision transformer imposes fake structure.

This document reframes the 4-byte tuple as what it actually is: a **hardware-native substrate for symbolic operations**, not an intermediate format for a neural compressor.

---

## 1. Thesis

**ByteStateV1 is a canonical, versioned, fixed-layout, 32-bit-coded carrier for semantic state and trace evidence. It is isomorphic to IR, replay-verifiable byte-for-byte, and optimized for batch operations over large frontiers using integer kernels. Neural models may consume it as an observer or compressor, but never as semantic authority.**

Three claims, each requiring empirical proof:

1. **Representation efficiency**: A full UtteranceState fits in ~512 bytes as a Code32 tensor, eliminating Python object overhead, JSON serialization, and dynamic dispatch from the inner loop.
2. **Frontier throughput**: Batch precondition checking and scoring over N candidate states becomes a vectorized integer operation, with gains scaling with frontier size.
3. **Evidence unification**: The same bytes that operators compute on are the bytes that get hashed, stored, and verified — collapsing runtime representation, provenance chain, and replay evidence into a single format.

---

## 2. Code32: The Atomic Unit

### 2.1 Definition

A **Code32** is a fixed-width 32-bit identity atom (`uint8 × 4`). All four bytes are consumed by concept identity. It is **not** content-addressed (not derived by hashing a descriptor); it is a **stable, registry-bijective compact code** assigned by allocation policy within a certification epoch.

The distinction matters:

- **ConceptID** (authoritative): The content-addressed identity of a concept, derived from `sha256(canonical_descriptor)`. Used in governance, provenance, and cross-epoch compatibility checks.
- **Code32** (hot-path carrier): A compact 32-bit code assigned by the registry, proven bijective with ConceptID for a given epoch. Used in computation, hashing of state tensors, and evidence.

The registry proves the bijection: `Code32 ↔ ConceptID` for each certification epoch. Remapping (changing which Code32 maps to which ConceptID) requires a version bump and replay verification.

Code32 can be viewed in multiple ways, but all four bytes belong to identity:

| View | When Used | Constraints |
|------|-----------|-------------|
| `uint32` | Hashing, equality checks, serialization | Bijective with ConceptID within epoch |
| `uint8[4]` | Batch tensor operations, SIMD | Little-endian canonical byte order (see §2.4) |
| `(R, G, B)` via palette | Visualization, debugging, human review tooling | Optional ΔE perceptual distinctness, separate from identity |

The bijection lives in the ID registry. Compute does not inherit perceptual constraints. Visualization uses a separate palette projection from Code32 to display colors.

### 2.2 Code Space Structure

Reserve high bits for structured allocation; use remaining bits for per-(domain, kind) local identifiers.

```
┌─────────┬──────────┬────────────────────────────┐
│ Byte 0  │  Byte 1  │     Bytes 2-3              │
│ Domain  │  Kind    │     Local ID               │
│ (8 bit) │  (8 bit) │     (16 bit)               │
└─────────┴──────────┴────────────────────────────┘
```

- **Domain byte**: Partitions the code space by world (PN, WordNet, EscapeGame, governance, etc.). Enables cheap domain membership tests via single-byte comparison or range check.
- **Kind byte**: Distinguishes concept types within a domain (operator, state-slot value, relation, metadata). Enables type-level guards without table lookup.
- **Local ID**: 65,536 unique concepts per (domain, kind) pair. 256 domains × 256 kinds × 65,536 local IDs = ~4.3 billion total capacity.

This sacrifices some of the flat address space but buys:
- **O(1) domain membership**: `code[0] == DOMAIN_PN`
- **O(1) type guards**: `code[1] == KIND_OPERATOR`
- **Debuggability**: A code visually encodes what kind of thing it is
- **Mask-based preconditions**: Operators can express "any PN concept" as a byte mask

**Capacity policy**: The 65,536-per-(domain, kind) ceiling is the real limiter, not the global 4.3B space. Domains with large concept spaces (e.g., WordNet synsets, lemmas, relations can exceed 65K under coarse kind partitioning) must use fine-grained kind partitioning so each bucket stays under 65K. The required policies:

- Each domain declares a **capacity budget** per kind in its `ByteStateSchema`, specifying expected and maximum concept counts.
- If a (domain, kind) bucket approaches 90% capacity, the registry emits a warning.
- If a (domain, kind) bucket would exceed 65,536, allocation fails closed. The resolution is to split the kind into finer subtypes (e.g., `KIND_SYNSET_NOUN`, `KIND_SYNSET_VERB` instead of `KIND_SYNSET`).
- An explicit **overflow policy** is: no overflow. Exceeding capacity is a schema-level error requiring a kind split and registry version bump. This is intentionally strict to prevent unplanned canonical changes at runtime.

Future consideration: if kind splitting proves insufficient, a `Code48` or `Code64` variant can be introduced as `ByteStateV2` with a wider local ID field. This is explicitly out of scope for V1.

**Governance requirement**: The mapping is versioned and frozen for a given certification regime. Code allocations within a certification epoch are append-only. Remapping requires a version bump and replay verification against the prior epoch.

**System sentinels**: Domain 0 is reserved for system-level sentinel codes. These are defined in `core/carrier/code32.py`:

| Constant | Value | `to_uint32()` | Purpose |
|----------|-------|---------------|---------|
| `PADDING_SENTINEL` | `Code32(0, 0, 0)` | `0x00000000` | Identity-plane filler for empty/unused slots |
| `INITIAL_STATE_SENTINEL` | `Code32(0, 0, 1)` | `0x00010000` | Operator code for frame 0 (no operator applied) |
| `TERMINAL_SENTINEL` | `Code32(0, 0, 2)` | `0x00020000` | Marks terminal/goal states |

### 2.3 Relationship to Existing IDRegistry

The existing `IDRegistry` (`core/id_registry.py`) provides the bijective mapping infrastructure. Code32 extends it with:

- Structured allocation (domain/kind/local) instead of flat assignment
- Separate allocation policies for compute vs visualization
- Version-stamped registry snapshots for certification

The perceptual distinctness constraint (ΔE distance enforcement) becomes **optional and visualization-only**. The compute registry enforces bijectivity and structured allocation; a separate palette projection can provide perceptual distinctness for any visualization tooling that needs it.

### 2.4 Canonical Byte Order

All Code32 values are stored and hashed in **little-endian** byte order. This is the native byte order for x86-64 and ARM64, which are Sterling's target platforms.

When Code32 is viewed as `uint32`, the byte layout is:

```
Memory address:  [+0]     [+1]     [+2]     [+3]
Byte content:    Domain   Kind     LocalLo  LocalHi
uint32 value:    (LocalHi << 24) | (LocalLo << 16) | (Kind << 8) | Domain
```

This means the domain byte is at the lowest memory address, which is convenient for byte-level domain checks (`ptr[0] == DOMAIN_PN`) regardless of how the value is interpreted as an integer.

**Enforcement**: Any code that converts between `uint32` and `uint8[4]` views must use the canonical byte order. Any accelerator interop must explicitly handle byte order if the backend uses a different convention. Hashing must always operate on the canonical byte representation, never on a platform-dependent integer representation.

---

## 3. ByteStateV1: Packed Semantic State

### 3.1 Definition

ByteStateV1 is a **canonical packed encoding** of the same semantic state that IR represents. It is not a new IR — it is the canonical tensor form of the existing IR, used in the hot path and evidence streams.

```
IR ↔ ByteStateV1    (bijective, version-gated, round-trippable)
```

IR remains the meaning. ByteStateV1 is the carrier.

### 3.2 Layout

An UtteranceState has layers (syntax, semantics, pragmatics, world). Each layer has slots. Each slot holds a Code32.

```
ByteStateV1 = uint8[layer_count × slot_count × 4]
```

For a typical state (4 layers, 32 slots):

```
4 layers × 32 slots × 4 bytes = 512 bytes
```

The schema is explicit and fixed per domain:

```python
@dataclass(frozen=True)
class ByteStateSchema:
    """Defines the layout for a domain's ByteStateV1 tensors."""
    schema_version: str          # e.g. "1.0"
    domain_id: str               # e.g. "rome", "wordnet", "escapegame"
    layer_count: int             # number of semantic layers
    slot_count: int              # slots per layer (fixed, padded)
    layer_semantics: tuple[str, ...]  # names for each layer
    padding_code: Code32         # Code32 for empty/unused slots (must use reserved domain=0)
    ordering_rule: str           # canonical slot ordering (e.g. "positional", "sorted_by_id")
    byte_order: str              # "little" (canonical, required for V1)
```

**Deferred fields** (not in V1 implementation, reserved for future capacity management):
- `padding_status: int` — SlotStatus default for empty slots (currently hardcoded to 0)
- `capacity_budget: dict[str, int]` — expected max concepts per kind (enforced by registry tooling, not schema)

**Critical invariants**:

- **Fixed dimensions**: No ragged tensors. Variable-arity states use canonical padding with a designated padding Code32.
- **Canonical ordering**: Slot order is deterministic and defined by the schema's `ordering_rule`. Without explicit ordering, permutation-equivalent states produce different hashes, leaking nondeterminism into replay.
- **Schema-versioned**: The schema version is embedded in every ByteStateV1 artifact. Mismatched schemas fail closed.

### 3.3 Two-Plane Model: Identity and Status

**Blocker resolved**: An earlier draft overloaded the fourth byte of Code32 as governance metadata ("alpha channel"). This breaks the Code32↔ConceptID bijection (status changes would create a different "identity"), breaks equality/hashing semantics (the same concept at different certification stages would compare unequal), and makes Code32 identity unstable across governance transitions.

**Fix**: Identity and status are separate planes. Code32 is the identity atom (all 32 bits). A parallel `SlotStatus` plane (`uint8`) carries governance state per slot.

```
ByteStateV1 layout (two planes):

  Identity plane:  uint8[layer_count × slot_count × 4]   (Code32 per slot)
  Status plane:    uint8[layer_count × slot_count]        (governance state per slot)
```

For the typical state (4 layers, 32 slots):

```
Identity: 4 × 32 × 4 = 512 bytes
Status:   4 × 32 × 1 = 128 bytes
Total:    640 bytes
```

#### SlotStatus Values

| Value | Meaning |
|-------|---------|
| `255` | Certified, fully grounded |
| `192` | Promoted, awaiting full certification |
| `128` | Provisional, under evaluation |
| `64`  | Shadow, exploratory only |
| `0`   | Hole — unresolved semantics |

#### Governance Checks as Arithmetic

The status plane preserves all the vectorized governance check properties:

- "Does this state contain any holes?" → `min(status_plane) == 0`
- "Is the entire state certified?" → `min(status_plane) == 255`
- "Which slots are provisional?" → `status_plane == 128`

All single-pass reductions. No separate metadata lookup. The difference from the earlier draft is that these reductions operate on the status plane, not on the identity bytes.

#### RGBA Rendering

When rendering for visualization:

```
R, G, B = palette_projection(code32)    # Identity → color via visualization palette
A       = status_plane[layer, slot]     # Governance state → transparency
```

This keeps the visual intuition (holes are transparent, certified concepts are solid) without corrupting identity.

#### Equality and Hashing

- **State equality**: Compares identity plane only. Two states with the same concepts in the same slots are equal regardless of governance status.
- **State hashing** (for deduplication, cycle detection): Hashes identity plane only. Uses domain prefix `b"STERLING::BYTESTATE_IDENTITY::V1\0"`.
- **Evidence hashing** (for replay verification): Hashes both planes (identity + status concatenated). Uses domain prefix `b"STERLING::BYTESTATE_EVIDENCE::V1\0"`. A governance status change at any slot produces a different evidence hash, which is correct — the evidence should reflect the full state including provenance metadata.

Both hash functions return `"sha256:<hex>"` (prefixed format). Schema bundle hashing uses prefix `b"STERLING::BYTESTATE_SCHEMA_BUNDLE::V1\0"`.

**Constraint**: The status plane semantics are defined by the governance regime, not by the domain. A domain cannot repurpose status values for domain-specific meaning. This ensures governance checks are domain-agnostic.

### 3.4 Operators as Masks

Sterling's operators have typed preconditions and effects. In ByteState space, masks operate on the identity plane. Status plane updates (if any) are separate governed writes.

**Precondition** = "slots at positions P must hold specific Code32 values"

Masks are defined at `uint32` granularity (one mask entry per slot), not `uint8`. This avoids accidentally checking partial Code32 values and enables full-width SIMD comparison:

```
precondition_mask:  uint32[layer_count × slot_count]  (0x00000000 = don't care, 0xFFFFFFFF = must match)
precondition_value: uint32[layer_count × slot_count]  (expected Code32 where mask is nonzero)
```

**Effect** = "write Code32 values to positions E"
```
effect_mask:  uint32[layer_count × slot_count]  (0x00000000 = unchanged, 0xFFFFFFFF = overwrite)
effect_value: uint32[layer_count × slot_count]  (new Code32 where mask is nonzero)
```

**Status effect** (optional) = "update governance status at positions S"
```
status_effect_mask:  uint8[layer_count × slot_count]  (0 = unchanged, 0xFF = overwrite)
status_effect_value: uint8[layer_count × slot_count]  (new SlotStatus where mask is nonzero)
```

Operator application:
```python
# View identity plane as uint32 for full-width comparison
identity_u32 = state_identity.view(numpy.uint32)  # [layer_count × slot_count]

# Precondition check (vectorized, uint32 lanes)
satisfied = ((identity_u32 & precondition_mask) == (precondition_value & precondition_mask)).all()

# Identity effect application (vectorized)
new_identity = numpy.where(effect_mask, effect_value, identity_u32)

# Status effect application (vectorized, separate plane)
new_status = numpy.where(status_effect_mask, status_effect_value, state_status)
```

This works for **local operators** — those whose preconditions and effects are positional slot checks and writes. For relational operators (graph traversal, constraint propagation), the mask expresses the local part; the relational check remains an index lookup. ByteState makes the surrounding work cheap, but does not eliminate relational lookups.

**Implementation note**: When the actual storage is `uint8[..., 4]` (for byte-level access), the `uint32` view must respect the canonical little-endian byte order (§2.4). Implementers should use `numpy.ndarray.view()` or equivalent, never manual byte-shifting that could introduce platform-dependent behavior.

---

## 4. ByteTraceV1: Evidence as Computed Bytes

### 4.1 The Tape Recording

A reasoning trace is a sequence of states and operator applications. ByteTraceV1 records this as raw bytes — the same bytes that were computed on during reasoning. There is no serialization step. The runtime representation is the evidence artifact.

The structure separates **envelope** (non-deterministic observability metadata, excluded from hashing) from **payload** (deterministic, hashed byte-for-byte):

```
┌─────────────────────────────────────────────────────────────┐
│ ENVELOPE (not hashed):                                      │
│   timestamp, trace_id, runner_version, wall_time_ms,        │
│   human-readable notes                                      │
├─────────────────────────────────────────────────────────────┤
│ PAYLOAD (hashed byte-for-byte):                             │
│   Header: schema_version, domain_id, registry_version_hash, │
│           fixture_hash, step_count, bytes_per_step          │
│                                                             │
│   Frame 0: [input state identity] [input state status]      │
│   Frame 1: [operator_id (Code32)] [operator_args (padded)]  │
│            [result state identity] [result state status]     │
│   Frame 2: [operator_id (Code32)] [operator_args (padded)]  │
│            [result state identity] [result state status]     │
│   ...                                                       │
│   Frame K: [final state identity] [final state status]      │
│                                                             │
│   Footer: outcome_hash, suite_identity                      │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Envelope vs Payload

**The envelope** contains non-deterministic fields (timestamps, trace IDs, wall-clock timings, human notes). It is stored alongside the payload for observability but is **explicitly excluded from the replay verification hash**. This resolves the blocker: replaying a trace at a different time does not produce a hash mismatch.

**The payload** contains only deterministic fields — values derived entirely from bound inputs, registry version, schema, and the reasoning engine's behavior. The payload hash is:

```
payload_hash = sha256(domain_prefix + magic + canonical_header_bytes + body_bytes + canonical_footer_bytes)
```

where `domain_prefix = b"STERLING::BYTETRACE::V1\0"`. The domain prefix ensures ByteTrace payload hashes cannot collide with hashes from other Sterling subsystems.

If any field in the payload would vary between runs with identical inputs (timestamps, UUIDs, process IDs), it belongs in the envelope, not the payload. This is enforced by construction: the payload header schema has no optional or non-deterministic fields.

### 4.3 Fixed-Width Step Frames

**All step frames have identical byte width.** This enables O(1) divergence localization from a byte offset.

A step frame contains:

```
[operator_code:  4 bytes (Code32 of the operator applied)]
[operator_args:  arg_slot_count × 4 bytes (Code32, padded to fixed width)]
[result_identity: layer_count × slot_count × 4 bytes (identity plane after operator)]
[result_status:   layer_count × slot_count bytes (status plane after operator)]
```

The frame width is constant and declared in the payload header:

```
bytes_per_step = 4 + (arg_slot_count × 4) + (layer_count × slot_count × 4) + (layer_count × slot_count)
```

Frame 0 (the initial state) is a special case: no operator, just the input state:

```
[operator_code:  4 bytes, set to INITIAL_STATE_SENTINEL (0x00010000)]
[operator_args:  arg_slot_count × 4 bytes, all zero-padded]
[result_identity: initial state identity plane]
[result_status:   initial state status plane]
```

This means Frame 0 has the same width as all other frames, which keeps the stride constant.

### 4.4 Format

ByteTraceV1 is stored as **canonical raw bytes** — not PNG, not JPEG, not any image format with encoding variability.

```
[envelope_len: uint16, little-endian]
[envelope: envelope_len bytes, canonical JSON, NOT hashed]
[magic: 4 bytes "BST1"]
[header_len: uint16, little-endian]
[header: header_len bytes, canonical JSON (sorted keys, no whitespace), schema-versioned]
[body: raw uint8 bytes, fixed-stride frames as defined above]
[footer_len: uint16, little-endian]
[footer: footer_len bytes, canonical JSON (sorted keys, no whitespace) with hashes]
```

**Payload hash covers**: `magic + header_len + header + body + footer_len + footer` (everything after the envelope).

**Canonical JSON requirements**: Keys sorted lexicographically, no optional whitespace, no trailing commas, UTF-8 encoding. This ensures two implementations producing the same logical header produce identical bytes.

**Why not PNG/JPEG**: Image encoders introduce variability — metadata, compression settings, color profiles, library version differences. Any of these would break byte-for-byte replay verification. The raw format is the simplest thing that works: bytes in, bytes out, hash matches or it doesn't.

### 4.5 Verification Protocol

Replay verification becomes:

1. **Record**: Run the trace. Emit ByteTraceV1. Compute `sha256(payload_bytes)`.
2. **Replay**: Re-run with the same inputs and registry version. Emit a new ByteTraceV1.
3. **Verify**: Compare payload bytes. If identical, replay passes. If not, the first differing byte position localizes the divergence to a specific step, layer, and slot.

The diff itself is the diagnostic. Because frames are fixed-width:

```python
payload_a = extract_payload(trace_a)
payload_b = extract_payload(trace_b)
divergence_byte = numpy.argmax(payload_a != payload_b)

# O(1) localization — no frame parsing needed
offset_in_body = divergence_byte - header_total_bytes
step  = offset_in_body // bytes_per_step
offset_in_frame = offset_in_body % bytes_per_step

if offset_in_frame < 4:
    location = f"step {step}: operator code"
elif offset_in_frame < 4 + (arg_slot_count * 4):
    arg_idx = (offset_in_frame - 4) // 4
    location = f"step {step}: operator arg {arg_idx}"
else:
    offset_in_state = offset_in_frame - 4 - (arg_slot_count * 4)
    identity_bytes = layer_count * slot_count * 4
    if offset_in_state < identity_bytes:
        flat_slot = offset_in_state // 4
        layer = flat_slot // slot_count
        slot  = flat_slot % slot_count
        location = f"step {step}: identity layer {layer}, slot {slot}"
    else:
        status_offset = offset_in_state - identity_bytes
        layer = status_offset // slot_count
        slot  = status_offset % slot_count
        location = f"step {step}: status layer {layer}, slot {slot}"
```

One integer tells you where reasoning diverged — no JSON diffing, no structural comparison.

### 4.6 Relationship to Existing Provenance Chain

The existing benchmark provenance (`suite_identity` → `result_hash` → `fixture_hashes`) works identically with ByteTraceV1. The underlying format changes from JSON to raw bytes; the hash-chaining structure does not.

| Current Proof Surface | ByteTraceV1 Equivalent |
|---|---|
| `bound_inputs_bundle_hash` | `sha256(header.fixture_hash + frame_0_bytes)` |
| `replay_trace` | The ByteTraceV1 payload (envelope excluded) |
| `replay_verification_hash` | `sha256(payload_bytes)` |

A single artifact satisfies all three proof surfaces that are currently missing from certification eligibility. The envelope (timestamps, wall-clock timings) is preserved for observability but does not participate in verification.

---

## 5. What This Is Not

### 5.1 Not a New IR

ByteStateV1 does not replace UtteranceState or IR_V1. It is a packed encoding of the same semantic content. The IR remains the source of truth for what things mean. ByteState is how that meaning is stored and operated on in the hot path.

Analogy: IR is the programming language; ByteState is the compiled bytecode. You write and reason about the program in the source language. The runtime executes the bytecode. Both represent the same program.

### 5.2 Not a Neural Substrate

ByteStateV1 respects the [Neural Usage Contract](neural_usage_contract.md). Neural models may read ByteState tensors (for scoring, value estimation, compression) but cannot write to them. All state mutations go through governed symbolic operators.

The fact that ByteState looks like a tensor does not make it a neural representation. It is a deterministic, symbolic, registry-bijective data structure that happens to be in a format that hardware can process efficiently.

### 5.3 Not a ViT Input

The prior "Sterling Full" framing envisioned reshaping RGBA codes into a 2D grid for a Vision Transformer. ByteStateV1 explicitly does not do this. The data is 1D (layers × slots × 4 bytes). If a neural observer wants to consume it, it reads the flat tensor. No reshape into image dimensions. No fake spatial structure.

### 5.4 Not "DeepSeek OCR for Sterling"

DeepSeek's OCR-based token ingestion (October 2025) demonstrated that pixel-block iteration can outperform token-sequence iteration for long-context ingestion because GPUs are optimized for dense, regular, cache-friendly access patterns. The transferable lesson is not "images are magic" but: **hardware is heavily optimized for dense, regular tensors with locality**.

Sterling's use of this insight is different from DeepSeek's:

| | DeepSeek OCR | Sterling ByteState |
|---|---|---|
| **Encode/decode tax** | Text → rendered pixels → model → text | None: Code32 is the native representation |
| **What's being optimized** | Long-context ingestion (thousands of tokens) | Frontier-level search (thousands of candidate states) |
| **Hardware target** | GPU vision kernels for pixel blocks | SIMD/GPU integer kernels for byte tensors |
| **Representation authority** | Neural model interprets pixels | Symbolic operators interpret Code32 |

The shared insight is: if your representation is a dense, regular byte tensor, you inherit decades of hardware optimization for free. The application is fundamentally different.

---

## 6. Scope of Advantage

ByteState does not uniformly accelerate all operations. The advantage is concentrated:

### Where ByteState Helps

| Operation | Current Path | ByteState Path | Expected Gain |
|---|---|---|---|
| State comparison | Python object equality, field-by-field | `memcmp` / vectorized `==` | Large (eliminate Python overhead) |
| Precondition checking (local) | Python attribute access, conditional logic | Masked byte comparison | Large (vectorized, branchless) |
| State hashing | JSON serialization → `sha256` | `sha256(raw_bytes)` directly | Moderate (skip serialization) |
| Frontier scoring (batch) | Python loop over N states, per-state scoring | Batch tensor operation over N×512 bytes | Scales with N |
| Governance checks | Separate metadata lookup | Status-plane reduction | Moderate (data locality) |
| Evidence serialization | Build JSON, stringify, write | Write raw bytes | Large (no serialization) |

### Where ByteState Does Not Help

| Operation | Why |
|---|---|
| Relational operator checks (graph traversal) | Requires index lookup regardless of state format |
| Constraint propagation (EscapeGame) | Structural computation, not representational |
| KG edge traversal (WordNet) | Graph adjacency is relational, not positional |

For relational operators, ByteState makes the surrounding work cheap (packing candidates, caching intermediate states, post-traversal scoring) but does not eliminate the relational lookup itself. The correct framing: ByteState accelerates local operations and batch frontier work; relational operators still rely on indexes.

---

## 7. Research Path

### Step 0: Profile Hard Problems

Instrument the current search path on the hardest benchmarks. Identify whether time is dominated by:

- **(a)** State comparison/scoring → ByteState helps directly
- **(b)** Operator precondition checking → ByteState helps if checks are local
- **(c)** Graph traversal for relational operators → ByteState helps less
- **(d)** Python overhead / object creation / GC pressure → ByteState helps enormously

If (d) dominates, the practical win comes from eliminating Python object allocation in the inner loop. The Code32 semantic identity story is the principled reason; "stopped allocating dataclasses in a hot loop" may be the actual speedup.

### Step 1: PN ByteStateV1 Prototype + Byte-Level Replay — COMPLETE

**Status**: Implemented (Slice 1). Rome domain uses a 4-layer × 32-slot schema with full round-trip compilation and byte-level replay verification. `RomeCompiler` in `core/carrier/schemas/rome_schema.py`.

- `compile_step_log()` and `compile_events()` both produce identical ByteTraceV1 artifacts
- `ByteReplayVerifier` in `core/carrier/replay.py` verifies payload hash identity
- Determinism proven via integration tests (`tests/integration/carrier/test_trace_determinism.py`)

### Step 2: Frontier Benchmark — COMPLETE

Measured precondition check, goal distance scoring, dedup, sort, and best-first
selection over frontiers of size 100–10,000. Five tiers compared:

| Tier | Geo mean speedup | Max speedup |
|------|-----------------|-------------|
| Byte-native (per-state slicing) | 11x | 18x |
| Packed buffer (struct.unpack_from) | 18x | 36x |
| numpy (vectorized uint32) | 176x | 4,093x |
| Rust / PyO3 (zero-copy &[u8]) | 339x | 1,879x |

Thesis from §7.0 validated: **(d) Python overhead dominates**. The exact same
comparison logic runs 170-340x faster when bypassing CPython's object protocol.
The gap widens with N — goal distance at N=5000 goes from 396ms (object path)
to 0.11ms (numpy) or 0.24ms (Rust).

Full benchmark, rationale, and next steps are in `test-scenarios/bytestate-benchmark/`
(v1 test artifact — to be recreated in v2 benchmarks).

> **Eligibility note**: These numbers are v1 measurements produced before the v2 benchmarking policy ([`docs/policy/benchmarking_policy.md`](../policy/benchmarking_policy.md)) existed. They are not backed by policy-governed artifact bundles and are not eligible for published claims until reproduced under CERTIFIED mode with sealed inputs, canonical traces, and verification bundles.

### Step 3: One Relational Operator Slice — NOT YET STARTED

Pick a representative WordNet operator (neighbor expansion + rank, or hypernym chain check). Measure whether the bottleneck is table lookup. If yes, ByteState still helps by making candidate state packing and post-lookup scoring cheap, but it won't make graph traversal "byte arithmetic." Document the boundary honestly.

### Step 4: Evidence Pipeline Integration — PARTIALLY COMPLETE

**Status**: Core evidence format implemented; certification pipeline integration pending.

Implemented:
- ByteTraceV1 format with envelope/payload split and content-addressed hashing
- Replay verification produces identical ByteTraceV1 artifacts (proven for all 4 domains)
- Divergence localization works: first differing byte → step/layer/slot (tested in `tests/integration/carrier/test_divergence_localization.py`)
- Golden conformance tests: byte-for-byte regression gates for all 4 domains (`tests/conformance/carrier/test_golden_compilation.py`)

Remaining:
- Wire ByteTraceV1 into the certification pipeline's `bound_inputs_bundle_hash`, `replay_trace`, and `replay_verification_hash` proof surfaces
- Provenance chain integration with `suite_identity` → `result_hash` → `fixture_hashes`

### Step 5: Multi-Domain Generalization — COMPLETE

**Status**: Implemented (Slice 2). Four domains operational:

| Domain | Schema | Domain Byte | Layers × Slots | Compiler |
|--------|--------|------------|-----------------|----------|
| Rome | 4 × 32 | 2 | current, goal, visited, path | `core/carrier/schemas/rome_schema.py` |
| Mastermind | 2 × 16 | 3 | belief, feedback | `core/carrier/schemas/mastermind_schema.py` |
| EscapeGame | 2 × 36 | 4 | occupancy, goal_zone | `core/carrier/schemas/escapegame_schema.py` |
| WordNet | 4 × 64 | 5 | current, goal, visited, path | `core/carrier/schemas/wordnet_schema.py` |

All four domains have `compile_step_log()` and `compile_events()` implementations, collector equivalence tests, trace determinism tests, divergence localization tests, and golden conformance fixtures.

---

## 8. Structural Cautions

### 8.1 Do Not Couple Perceptual Distinctness to the Compute Registry

The ΔE perceptual distance constraint in the current `IDRegistry` is useful for visualization but artificially restricts the compute code space and creates unnecessary allocation complexity. Split the concerns:

- **Compute registry**: Packed, structured (domain/kind/local), stable, perceptual constraints not enforced
- **Visualization palette**: Optional projection from Code32 → perceptually distinct colors for human-facing tooling

### 8.2 Do Not Overpromise Byte Arithmetic for Relational Semantics

Equality checks, masked writes, and batch comparisons are trivially expressible as byte operations. Relational facts (hypernym chains, constraint graphs, adjacency) are not. The correct claim is: ByteState makes state storage and batchable guards cheap; relational operators still rely on indexes. Those indexes can also be packed and structured for cache locality, but that is a separate optimization from ByteState itself.

### 8.3 Do Not Introduce Encoding Variability into Evidence

If ByteTraceV1 is the evidence format, encoding must be strictly deterministic:

- No floating-point in the byte layout (use fixed-point or integer only)
- No image codec (PNG, JPEG) in the storage path
- No library-dependent serialization (struct packing must be explicit and tested)
- Canonical byte order is little-endian (§2.4), enforced in all serialization paths
- Canonical JSON in headers/footers uses sorted keys and no optional whitespace
- `uint16` length fields in the format are little-endian
- Non-deterministic fields (timestamps, trace IDs, wall-clock timings) belong in the envelope, never in the payload

One byte of encoding variability breaks the entire replay verification contract.

### 8.4 Do Not Conflate Identity with Status

Code32 is identity. SlotStatus is governance state. They must remain separate planes. If a future optimization attempts to pack status into unused Code32 bits (e.g., "we only need 24 bits of local ID, so steal 8 bits for status"), it reintroduces the original conflation bug: the same concept at different governance stages would hash differently in the identity plane, breaking deduplication, cycle detection, and state equality. The 128-byte overhead of a separate status plane is the correct cost for this invariant.

---

## 9. Relationship to Other Canonical Documents

| Document | Relationship |
|---|---|
| *(light_vs_full — deleted)* | Superseded for "Sterling Full" definition. Full = ByteState substrate + evidence unification, not ViT compression. |
| [neural_usage_contract.md](neural_usage_contract.md) | ByteState respects the contract fully. Neural may read tensors; only governed operators may write. |
| [Capability Primitives P14](../specs/primitives/00_INDEX.md) | P14 (Search over Compressed Representations) aligns directly. ByteState is the compressed representation; search over it is the primitive. |
| [Capability Primitives P16](../specs/primitives/00_INDEX.md) | P16 (Representation Invariance and State Canonicalization) defines the hashing and canonicalization requirements ByteState must satisfy. |
| [Capability Primitives P19](../specs/primitives/00_INDEX.md) | P19 (Audit-Grade Explanations) is supported by ByteTraceV1 as the replayable evidence format. |
| [bytestate_compilation_boundary.md](bytestate_compilation_boundary.md) | Defines how external domain payloads compile into ByteState and how epoch transitions work. See §11. |

---

## 10. Source File Index

| File | Purpose |
|------|---------|
| `core/carrier/__init__.py` | Package exports (Code32, SlotStatus, sentinels) |
| `core/carrier/code32.py` | Code32 dataclass, SlotStatus enum, system sentinels |
| `core/carrier/bytestate.py` | ByteStateSchema, ByteStateV1, identity/evidence hashing |
| `core/carrier/bytetrace.py` | ByteTraceV1, ByteTraceBuilder, .bst1 format, payload hashing |
| `core/carrier/operator_codebook.py` | OperatorCodeBookV1, operator-to-Code32 mapping |
| `core/carrier/packed_frontier.py` | PackedFrontierV1 binary format (.spf1), header/validation |
| `core/carrier/compiler.py` | DomainCompiler protocol, RegistrySnapshotV1, StepLogValidator |
| `core/carrier/collector.py` | ByteTraceCollector, fail-closed event sequencing |
| `core/carrier/step_event.py` | InitialStateEvent, TransitionEvent, TerminalEvent |
| `core/carrier/partitioner.py` | Domain-specific Code32 allocation and kind partitioning |
| `core/carrier/artifact_writer.py` | Atomic .bst1 artifact persistence with verify-on-write |
| `core/carrier/telemetry.py` | CompilationTelemetry, TelemetrySink |
| `core/carrier/replay.py` | ByteReplayVerifier |
| `core/carrier/schemas/rome_schema.py` | Rome domain: 4×32 schema, RomeCompiler |
| `core/carrier/schemas/mastermind_schema.py` | Mastermind domain: 2×16 schema |
| `core/carrier/schemas/escapegame_schema.py` | EscapeGame domain: 2×36 schema |
| `core/carrier/schemas/wordnet_schema.py` | WordNet domain: 4×64 schema |

---

## 11. Summary

The 4-byte tuple is not a color. It is not a pixel. It is not an intermediate format for a neural compressor.

It is a **32-bit semantic identity atom** chosen because:

1. It uniquely identifies any concept in Sterling's semantic space (via registry-proven bijection with ConceptID)
2. It packs into dense, regular tensors that hardware is optimized to process
3. Paired with a separate status plane, it carries governance metadata without corrupting identity
4. It is deterministic, hashable, and replay-verifiable by construction
5. It collapses the runtime representation, evidence format, and provenance artifact into a single thing
6. The envelope/payload split ensures non-deterministic observability data never contaminates replay verification

Sterling Full succeeds if reasoning on ByteState tensors produces identical results to reasoning on IR objects — faster, with less overhead, and with evidence artifacts that are a side-effect of computation rather than a post-hoc serialization step.

---

## 12. Compilation Boundary (Pointer)

ByteState is the inner-loop substrate: static per epoch, byte-for-byte deterministic, layout-frozen. External domain payloads — solve requests, ARC grids, Minecraft rule sets, induced DomainSpecIR — must be **compiled** into ByteState before the inner loop operates on them, and **decompiled** back into domain terms afterward.

This compilation boundary is **not part of the substrate**. It is governed by a separate canonical document: **[ByteState Compilation Boundary](bytestate_compilation_boundary.md)**.

**Non-goal for this document**: ByteState is not dynamically mutated by induced schemas, learned operators, or runtime domain discovery. Schema evolution and new Code32 allocations happen through versioned artifact streams and epoch transitions between episodes — never during them. The compilation boundary document defines how those transitions work, what artifacts they produce, and what conformance suites verify them.
