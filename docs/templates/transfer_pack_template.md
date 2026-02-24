# Transfer Pack Template

A Transfer Pack is a standardized bundle runnable by the Unified World Harness.

## Directory layout

transfer_pack/
  claims/
    claim_catalog.v1.yaml
  budgets/
    budget_profiles.v1.yaml
  fixtures/
    <world_id>/
      *.jsonl | *.ndjson | *.json
  verifiers/
    <world_id>/
      verifier_spec.v1.md
  falsifiers/
    negative_controls.v1.md
  expected/
    goldens.v1/  (optional)

## Required fields

- claim_catalog: stable IDs, preconditions, success criteria, failure families, required evidence surfaces
- budgets: explicit limits and costs
- fixtures: sealed payloads with schema/registry references
- verifiers: deterministic checkers for correctness
- falsifiers: at least one negative control per capability family
