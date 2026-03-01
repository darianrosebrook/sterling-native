---
status: Accepted
authority: adr
date: 2026-02-24
context: "Sterling Native is a clean-slate successor to Sterling v1. The question of how much v1 code to reuse must be answered with a policy, not a philosophical debate."
---
# ADR 0005: v1 Is a Test Oracle, Not a Dependency

---

## Decision

v1 code is not imported into the v2 kernel. v1 is a test oracle, not a dependency.

### Allowed (safe reuse)

- Specs and docs as reference material (`docs/reference/`)
- Goldens, fixtures, and evidence bundles as test vectors
- Minimal reference implementations used only to generate/validate vectors (kept quarantined)

### Allowed but quarantined (strangler-fig style)

- A v1 reference implementation may exist under `reference/v1_impl/` (or a separate repo/submodule) solely to:
  - Generate canonical test vectors (ByteState, ByteTrace, hashes)
  - Act as a differential oracle during early development
- Never linked into the v2 runtime. Never imported by v2 kernel packages.

### Not allowed

- Copying v1 core modules into v2 "because it works"
- Any v1 import path reachable from the v2 kernel build graph
- "Temporary" code that bypasses `compile(...) → ByteState` or writes trace in ad hoc formats

## Consequences

- v2 kernel surfaces will be written fresh, guided by v2 specs and tested against v1 oracle output where applicable.
- Milestone M7 is a checkpoint to confirm this posture or grant exceptions (each requiring its own ADR).
- The kernel build graph must have zero v1 dependencies. CI should enforce this as an import boundary check.

## Path Update (2026-03-01)

The `docs/reference/v1/` directory has been reorganized into version-agnostic advisory docs under `docs/reference/` (capabilities/, world_design/, design_rationale/, historical/). All reference docs carry `authority: reference` front-matter and are explicitly non-normative.

For v1 source material, the Sterling (Python) repo at commit `9b9c0daa` is the pinned baseline. In-repo reference docs are version-agnostic rewrites preserving proof obligations, not copies of v1 contracts.
