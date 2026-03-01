# Canonical Definitions (Sterling Native)

This directory contains **normative** contracts, invariants, and definitions for Sterling Native. Everything here uses v2 vocabulary only and is enforced by ADRs, invariants, and policies.

## What "canonical" means here

- **Normative**: these docs define contracts and invariants that Sterling Native enforces.
- **v2 vocabulary only**: no v1 stage names, no legacy operator labels, no RunIntent modes, no Sterling Light/Full.
- **Versioned**: edits require explicit version bumps (see `docs/policy/canonical_doc_change_policy.md`).
- **Gate-checked**: CI can fail if changes are inconsistent with these surfaces.

## Current canonical surfaces

| File | What it defines |
|------|----------------|
| `philosophy.md` | Core design philosophy — boundary separations, meta-principles, references to ADRs |
| `glossary.md` | Curated v2 vocabulary (~100 terms) |
| `core_constraints.md` | The 11 architectural invariants (INV-CORE-01 through INV-CORE-11) |
| `global_invariants.md` | Global invariant declarations |
| `neural_usage_contract.md` | Rules for neural component usage (advisory only, API-enforced) |
| `bytestate_compilation_boundary.md` | Compilation boundary contract |
| `code32_bytestate.md` | Code32 identity atom, ByteStateV1 substrate, ByteTraceV1 evidence format |
| `search_evidence_contract.md` | Search bundle artifacts, verification profiles (Base/Cert), verification pipeline, tape→graph equivalence |

## Relationship to reference docs

`docs/reference/` contains version-agnostic advisory material: proof obligations, design rationale, and historical context. These docs carry `authority: reference` and are explicitly non-normative. They inform future v2 work but must never be cited as canonical requirements. See [`docs/reference/README.md`](../reference/README.md) for the full index.

## Adding or modifying canonical docs

See `docs/policy/canonical_doc_change_policy.md`. Changes require a version bump and review.
