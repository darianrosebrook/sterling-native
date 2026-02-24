# Contributing to Sterling Native

## Ground rules

1) Preserve invariants. If a change weakens an invariant, it must be explicit and versioned.
2) Single Source of Truth (INV-CORE-12). Do not introduce parallel implementations for canonical surfaces.
3) Evidence first. If you claim a capability or performance change, include the artifact bundle requirements in your proposal.

## Canonical docs

Edits to `docs/canonical/*` require:
- a version bump inside the doc
- rationale and consequences
- alignment updates (any other docs that reference the contract)

See `docs/policy/canonical_doc_change_policy.md`.

## Adding a world / scenario

- Implement through the Unified World Harness contract.
- Provide fixtures + deterministic verifiers.
- Provide a claim catalog and at least one negative control.

## Adding a benchmark

Benchmarks are governed by `docs/policy/benchmarking_policy.md`.
No performance claim is eligible without the required artifact bundle and eligibility checks.
