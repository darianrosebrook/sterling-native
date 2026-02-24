# ADR 0001: Compilation Boundary as Architectural Spine

Status: Accepted (Design target)
Date: 2026-02-23

## Decision

Sterling Native treats the compilation boundary as the core architectural contract:

compile(payload, schema_descriptor, registry_snapshot) → ByteState

ByteState is the only runtime truth used by the inner loop for compute, hash, diff, and replay.

## Rationale

- Prevent semantic drift via incidental runtime representations.
- Make determinism and replay a contract surface.
- Force domain evolution to occur via promoted artifacts (schema/registry), not runtime mutation.

## Consequences

- Substrate cannot be mutated mid-episode.
- Schema and registry changes require version/epoch transitions.
- Worlds must provide compilers/decompilers and operate through the Unified World Harness.
