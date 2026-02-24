# Benchmarks

This directory defines conventions for benchmark runs and artifacts.

Benchmarks are governed by:
- `docs/policy/benchmarking_policy.md`
- `docs/policy/domain_transfer_policy.md` (for transfer benchmarks)

## Run bundles

Each benchmark run should be stored as an immutable bundle under `benchmarks/runs/<run_id>/` and include:
- inputs (fixtures digest, policy snapshot, schema + registry references)
- canonical trace artifacts (ByteTrace, outcome rows, metrics bundle, tool transcripts)
- verification artifacts (replay verification, gate verdict bundle)
- eligibility report (eligible/ineligible + reasons)

## Publishing

Do not publish numbers without the eligibility requirements satisfied.
