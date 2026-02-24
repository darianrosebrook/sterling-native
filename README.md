# Sterling Native

Sterling Native is a clean-slate successor architecture to Sterling v1.

Core thesis: transformers are demoted to I/O codecs and advisory scorers; the cognitive core is explicit state + typed operators + governed search with deterministic replay and evidence-carrying traces.

This repository is a documentation-first scaffold: canonical contracts, policies, and architecture targets that guide an incremental migration (boundary-first, not rewrite-first).

## Start here

1) Architecture target: `docs/architecture/clean_sheet_architecture.md`
2) Canonical constraints: `docs/canonical/core_constraints.md`
3) Neural usage contract: `docs/canonical/neural_usage_contract.md`
4) Compilation boundary: `docs/canonical/bytestate_compilation_boundary.md`
5) Benchmarking and transfer policies:
   - `docs/policy/benchmarking_policy.md`
   - `docs/policy/domain_transfer_policy.md`

## Key ideas

- Deterministic replay is a contract surface, not a best-effort property.
- ByteState/ByteTrace are the canonical carriers for compute and evidence.
- Worlds plug in via a Unified World Harness; cross-domain work composes via MetaPlan (no hidden routers).
- Memory exists as architecture, but integration is gated on governed evidence (WS/PS/FI tiering).

## Repo structure

- `docs/canonical/` — single-source-of-truth contracts and invariants (versioned)
- `docs/specs/` — forward-looking capability primitives (P01-P21)
- `docs/architecture/` — target architecture, module map, and success rubric
- `docs/policy/` — benchmarking, transfer, governance, versioning policies
- `docs/adr/` — architecture decision records
- `docs/reference/v1/` — carried-over reference docs from v1 (non-authoritative)
- `docs/templates/` — templates for transfer packs, benchmark manifests, claim catalogs
- `docs/ephemeral` — temporal documents where the work is quickly outdated or stale (gitignored)
- `benchmarks/` — benchmark harness conventions and run bundle layout

## Contributing

See `CONTRIBUTING.md`. Canonical surfaces require version bumps and disciplined review (`docs/policy/canonical_doc_change_policy.md`).
