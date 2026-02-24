# Benchmark Run Manifest Template

This manifest accompanies each benchmark run bundle.

Required top-level fields:
- run_id
- created_at
- engine_version
- epoch_id
- policy_snapshot (mode, gate set, waivers, budgets)
- schema_descriptor (id, version, hash)
- registry_snapshot (epoch, hash)
- environment (os, cpu, python/rust versions, build flags)
- input_set (fixtures digest, task count)
- artifacts (paths + digests)
- verification (replay status, gate verdicts)
- eligibility (eligible/ineligible + reasons)
