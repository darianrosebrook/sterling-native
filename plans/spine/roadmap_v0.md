# SPINE-001 Roadmap: Kernel Spine M0–M3

> Contract deliverable for `.caws/specs/SPINE-001.yaml`.
> This file is the authoritative milestone plan for the kernel spine.

## Milestones

### M0 — Kernel contract surface and repo layout

**Status**: Complete

**Deliverables**:
- Rust workspace (`kernel/`, `tests/lock/`) with pinned toolchain (1.90.0)
- `compile()`, `apply()`, `replay_verify()` API signatures (stubs, logic deferred)
- Carrier types: `Code32`, `ByteStateV1`, `SlotStatus`, `ByteTraceV1`, `SchemaDescriptor`, `RegistrySnapshot`
- Operator types: `OperatorSignature`, `IdentityMaskV1`, `StatusMaskV1`, `OperatorCategory`
- Proof types: `ContentHash`, domain prefix constants, `ReplayVerdict`
- JSON schemas: `bytetrace_descriptor`, `trace_bundle`, `claim`, `schema_descriptor`, `registry_snapshot`, `policy_snapshot`
- S1-M0 lock test: build-graph isolation (no v1 deps in kernel)
- CI workflow: check, clippy, fmt, test, docs lint

**Acceptance**: S1-M0 — no v1 dependency reachable from kernel build graph.

### M1 — ByteState/Code32 + canonical hashing

**Status**: Next

**Deliverables**:
- `sha2` dependency for canonical hashing (SHA-256, V1-compatible domain prefixes)
- `canonical_hash()` implementation producing `"sha256:<hex>"` `ContentHash`
- `ByteStateV1` encode/decode (identity_bytes, evidence_bytes)
- `compile()` implementation for Rome domain (payload = canonical JSON of initial planes)
- Golden byte fixtures generated from v1 offline, committed as test expectations
- Lock tests: hash stability, golden bytes, cross-machine determinism

**Acceptance**: S1-M1 — compile() on two machines produces identical bytes and digest. S1-M1-GOLDEN — output matches golden fixture bit-for-bit.

**Fixture strategy**: v1 is the oracle for wire format bytes and sha256 hashes. Generate fixtures from v1 Python, commit the bytes, validate Native against committed fixtures. Never import v1 into Native build graph.

**compile() strategy**: Rome payload is a canonical JSON representation of initial identity/status planes plus shape metadata. `compile()` parses, validates against registry, produces `ByteStateV1` deterministically. No invented semantics — keep it boring so golden fixtures are derivable from v1.

### M2 — ByteTrace writer + replay verifier

**Status**: Planned

**Deliverables**:
- ByteTrace binary writer (append-only, schema-first header)
- ByteTrace reader
- `replay_verify()` implementation (hashing, monotonicity, snapshot consistency)
- Minimal `apply()` — single operator application primitive
- Single-step replay lock test

**Acceptance**: S1-M2 — replay_verify() verdict matches original execution. S1-M2-DIV — injected divergence localizes to first differing step.

### M3 — Unified World Harness hello world

**Status**: Planned

**Deliverables**:
- World harness contract: `encode_fixture`, `decode_state`, `operator_catalog_data`, `domain_verifier`
- One minimal world (Rome or equivalent toy graph)
- Closed artifact bundle output (inputs/trace/verification/metrics)
- Determinism lock test

**Acceptance**: S1-M3 — harness produces self-contained artifact bundle. S1-M3-DETERMINISM — consecutive runs with identical inputs produce identical ByteTrace digests.

## Non-goals

- Neural model integration (observer/compressor role is post-M3)
- Multi-world orchestration (M3 proves one world only)
- Performance optimization beyond correctness (SIMD, parallelism are post-M3)
- V2 artifact format (blake3, wider Code types — explicitly out of scope for V1)
- Policy enforcement in the kernel (policy lives at the induction/harness layer)

## Design decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Artifact version | V1-compatible | v1 is byte-for-byte oracle for carrier/trace/hashing |
| Hash algorithm | SHA-256 | Matches v1; blake3 reserved for V2 artifact bump |
| `compile()` policy arg | Not present | Policy is induction-layer per v1 |
| `ByteStateV1` Eq/Hash | Not derived | Explicit `identity_eq` prevents conflation bug |
| Operator masks | Full-width packed vectors | SIMD-ready, no API break at M2 |
| Sentinel canonical form | Bytes | u32 view is display-only |
| `compile()` semantics | payload → initial `ByteState` | v1 "compiler" = trace encoder, different concept |
| Fixture oracle | v1 for bytes/hashes only | `compile()` boundary is new — no v1 oracle for that |
