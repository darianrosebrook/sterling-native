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
| `glossary.md` | Curated v2 vocabulary (~100 terms). For the comprehensive v1 glossary, see `docs/reference/v1/glossary_full.md` |
| `core_constraints.md` | The 11 architectural invariants (INV-CORE-01 through INV-CORE-11) |
| `global_invariants.md` | Global invariant declarations |
| `neural_usage_contract.md` | Rules for neural component usage (advisory only, API-enforced) |
| `bytestate_compilation_boundary.md` | Compilation boundary contract |
| `code32_bytestate.md` | Code32 identity atom, ByteStateV1 substrate, ByteTraceV1 evidence format |

## Relationship to v1

- **`docs/reference/v1/glossary_full.md`**: The comprehensive v1 glossary with full narrative context (archived, non-authoritative).
- **`docs/reference/v1/philosophy_full.md`**: The v1 philosophy with implementation anchors and source file indexes (archived, non-authoritative).
- **`docs/reference/v1/canonical/`**: 29 contract files from Sterling v1, quarantined with promotion criteria.

A v1 contract gets promoted to this directory only after passing the review criteria in `docs/reference/v1/canonical/README.md`:

1. Aligns with the v2 compilation boundary spine
2. Uses DEV/CERTIFIED governance taxonomy (not old run-intent modes)
3. No parallel implementations (INV-CORE-12)
4. ByteTrace is canonical trace artifact (ADR 0002)
5. Has version metadata and change policy header

## Adding or modifying canonical docs

See `docs/policy/canonical_doc_change_policy.md`. Changes require a version bump and review.
