---
status: Accepted
authority: adr
date: 2026-02-23
---
# ADR 0001: Compilation Boundary as Architectural Spine

## Decision

Sterling Native treats the compilation boundary as the core architectural contract:

```
compile(payload_bytes, schema_descriptor, registry) → CompilationResultV1
```

`CompilationResultV1` contains the compiled `ByteStateV1` plus provenance metadata (schema descriptor, registry descriptor, compilation manifest). ByteState is the only runtime truth used by the inner loop for compute, hash, diff, and replay. Policy lives in the harness layer, not the carrier layer — `compile()` is purely a substrate operation.

## Rationale

- Prevent semantic drift via incidental runtime representations.
- Make determinism and replay a contract surface.
- Force domain evolution to occur via promoted artifacts (schema/registry), not runtime mutation.

## Consequences

- Substrate cannot be mutated mid-episode.
- Schema and registry changes require version/epoch transitions.
- Worlds must provide compilers/decompilers and operate through the Unified World Harness.
