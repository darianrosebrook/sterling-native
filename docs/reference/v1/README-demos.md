# Test Scenarios Capability Evidence Ledger

This document is not a demo showcase. It is a capability evidence ledger for the proving-ground surfaces under `test-scenarios/`.

Primary capability targets are defined in:

- `docs/reference/capability_primitives_bundle/00_INDEX.md` (P01..P21)
- `docs/roadmaps/capability_campaign_plan.md` (certification campaign surfaces)
- `test-scenarios/demo-promotion.md` (D0->D4 promotion ladder and G1->G5 gates)

Run snapshot for this ledger:

- Date: 2026-02-23
- Host: arm64, Python 3.13.4
- Environment: local `.venv`

## Boundary

What this file is for:

- Record what current demos/benchmarks empirically support
- Map that evidence to capability primitives
- Mark what remains unproven

What this file is not for:

- Claiming certification-grade capability completion
- Substituting for unified benchmark campaign artifacts

## Executed Surfaces

Executed successfully in this pass:

- `test-scenarios/diffusion-demo/run_demo.py`
- `test-scenarios/graphing-calc-demo/run_demo.py`
- `test-scenarios/lemma-grammar-demo/run_demo.py`
- `test-scenarios/phonology-g2p-demo/run_demo.py`
- `test-scenarios/gridkeys-p06-demo/run_demo.py`
- `test-scenarios/ling-ops-demo/run_demo.py`
- `test-scenarios/structured-patch-demo/run_demo.py`
- `test-scenarios/swm-io-demo/run_demo.py`
- `test-scenarios/toy-e2e-demo/run_demo.py`
- `test-scenarios/induction-synthesis-demo/run_demo.py`
- `test-scenarios/bytestate-benchmark/run_benchmark.py`

Not executed as a single quick runner:

- `test-scenarios/sterling-workbench/` (interactive frontend/backend workflow)
- `test-scenarios/perceptual-substrate-demo/` (D1 in core; demo runner raises `NotImplementedError`)
- `test-scenarios/dips-02-internalization/` (scaffold only; all 36 CPG gates unstarted; on hold)

## Evidence Summary

All 10 runnable demos exited `0` (`0/10` failures). One benchmark surface also exited `0`.

| Surface | Key observed evidence | Artifact |
|---|---|---|
| `diffusion-demo` | `60/60` all gates pass; determinism `130` digest checks pass | `test-scenarios/diffusion-demo/results/diffusion_report.json` |
| `graphing-calc-demo` | claim adjudication `6/6` correct; G5 soundness `6/6`; G6 minimality `1/1`; determinism pass; CPG-0..8 all complete for `p27.ea@1.0` (96 proof tests); promoted at merge `542c60c1` | `test-scenarios/graphing-calc-demo/results/graphing_calc_report.json` |
| `lemma-grammar-demo` | all gates pass (`32/32`); `31` satisfied, `1` refused; determinism pass | `test-scenarios/lemma-grammar-demo/results/lemma_grammar_report.json` |
| `phonology-g2p-demo` | all gates pass; determinism `80` digests; homograph evidence `30/30`; stress `16/16`; OOV safety `12/12` | `results/phonology_g2p_report.json` |
| `gridkeys-p06-demo` | `54/54` instances pass; 6 key types validated; grid transfer deterministic; D4 validated | `test-scenarios/gridkeys-p06-demo/results/` |
| `ling-ops-demo` | `66/66` fixtures pass (60 sat, 6 unsat); 376 scenario + 182 proof = 558 total; CPG-0..8 all complete for `p25.lo@1.0` (182 proof tests); negative transfer control (near-miss domain), file-content-derived evidence hashes, path leakage exclusion all proven; see `docs/promotion-overlay.md` for gate-by-gate evidence | `test-scenarios/ling-ops-demo/results/` |
| `structured-patch-demo` | six gate families all `32/32`; 129 tests; deterministic and refusal/boundary checks pass; type-confusion guard lock + inventory traceability; D4 validated | console output + `test-scenarios/structured-patch-demo/tests/` |
| `swm-io-demo` | determinism pass (`70` digests); bytestate/bytetrace round-trip pass; performance correctness pass; CPG-0..8 all complete for `p06@1.0` (127 proof tests); promoted at merge `1f86834b` | `test-scenarios/swm-io-demo/results/swm_io_report.json` |
| `toy-e2e-demo` | cross-branch governance pass; SWM/diff/LG determinism all pass; `72/72` diffusion gates pass; **SAP-1a claim pass** (6/12 gated, 4-plane semantic agreement: identity + role + negation scope + governance); 15 negative controls + 23 smoke tests; 6 excluded sentences with documented carrier limitations; **bridge hardening**: langpack projection-only analysis (12-sentence cohort characterized), BridgeProfile STRICT/COERCED with BridgeAudit, unlicensed-roles + frame-underspecified + duplicate-role-edge guards, 64 snapshot tests, trace schema locks; **langpack report integration**: unified report carries langpack sidecar per sentence (COERCED + STRICT), 32 report-level lock tests; **structural commitments**: 7/12 role_bindings extracted (7/7 agree with anchor), canonical SAP role labels, fail-closed on var fillers/multi-event/empty bindings, 86 unit+invariant tests; 298 total tests; review runbook in README | `test-scenarios/toy-e2e-demo/results/toy_e2e_report.json` |
| `induction-synthesis-demo` | 542 proof tests passing; CPG-0..8 all complete for `p22.di@1.0` and `p14.ps@1.0`; alpha/beta/gamma/delta transfer domains; 11-mutation campaign; artifact closure bundle; promoted at merge `63b830d3` | `tests/proofs/` (38 CPG-7 tests, 595 total post-merge) |
| `bytestate-benchmark` | correctness pass; byte-native avg `11.38x`; packed avg `17.67x` (max `35.63x`) | `test-scenarios/bytestate-benchmark/results/benchmark_results.json` |

## Primitive Mapping (P01-P21)

Status legend:

- `EVIDENCED`: direct supporting evidence from this run
- `PARTIAL`: signal exists but insufficient to claim primitive-level proof
- `NOT YET`: no direct evidence from this run

| Primitive | Status | Evidence from this run |
|---|---|---|
| P01 Deterministic transformation planning | EVIDENCED | Deterministic traces and legality/boundedness behavior in `structured-patch-demo`, `lemma-grammar-demo`, `graphing-calc-demo`; ling-ops (deterministic BFS planning across 66 fixtures, 2 domains) |
| P02 Capability gating and legality | PARTIAL | Refusal/legality-style checks visible in demos, but no explicit capability-acquisition planning proof |
| P03 Temporal planning with durations/capacity | NOT YET | No cert-grade temporal scheduler or capacity planning surface in this pass |
| P04 Multi-strategy acquisition | PARTIAL | Some strategy variation in demos, but no strong adaptive strategy-prior campaign evidence |
| P05 Hierarchical planning | NOT YET | No explicit macro-micro planner validation in executed surfaces |
| P06 Goal-conditioned valuation under scarcity | EVIDENCED | gridkeys-p06 (goal-conditioned grid+keys allocation under scarcity) |
| P07 Feasibility and partial-order structure | EVIDENCED | ling-ops (constraint feasibility: sat/unsat classification, partial-order scope chains) |
| P08 Systems synthesis | NOT YET | No component composition/spec-satisfaction synthesis rig executed here |
| P09 Contingency planning with exogenous events | NOT YET | No exogenous event policy benchmark executed here |
| P10 Risk-aware planning | NOT YET | No distributional/tail-risk benchmark surface executed here |
| P11 Epistemic planning and active sensing | NOT YET | No belief-probe policy benchmark executed here |
| P12 Invariant maintenance | NOT YET | No receding-horizon invariant maintenance benchmark executed here |
| P13 Irreversibility and commitment planning | EVIDENCED | ling-ops (irreversibility: expected-unsat fixtures prove refusal of contradictory/cyclic constraints) + structured-patch (boundary/refusal gates) |
| P14 Program-level planning | EVIDENCED | `p14.ps@1.0` capsule promoted via induction-synthesis-demo (CPG-0..8 complete); program synthesis with budget enforcement, failure routing, typed detail types |
| P15 Fault diagnosis and repair | PARTIAL | `graphing-calc-demo` shows adjudication discipline; no full hypothesis->test->fix campaign run |
| P16 Representation invariance/canonicalization | EVIDENCED | Canonicalization and deterministic state hashing in `graphing-calc-demo`, `lemma-grammar-demo`, bytestate surfaces; ling-ops (canonical hashing, domain-prefixed sha256, report_digest) |
| P17 Credit assignment tied to execution | PARTIAL | Execution-grounded traces present; no direct prior-update gate run in this pass |
| P18 Multi-objective optimization | NOT YET | No explicit Pareto/preference sensitivity benchmark executed here |
| P19 Audit-grade explanations | EVIDENCED | Rich artifact trails with replayable digests and gate-level evidence across demos |
| P20 Adversarial robustness/rule-injection hardening | PARTIAL | Boundary/refusal/validity checks present, but no dedicated adversarial injection benchmark in this pass |
| P21 Entity belief maintenance under partial observability | NOT YET | No entity-track belief maintenance rig executed here |

## Primitive -> Certification Owner Surface (Proposed)

This table ties each primitive to the primary benchmark/certification surface that should own promotion evidence.

| Primitive | Primary certification owner surface | Secondary/transfer surface |
|---|---|---|
| P01 | `wordnet_tiered` | `rome_extended` |
| P02 | `wordnet_tiered` (capability constraints in task contracts) | `transactional_kv_store` (Phase C) |
| P03 | `transactional_kv_store` (Phase C) | `escapegame_extended` |
| P04 | `rome_extended` | `wordnet_tiered` |
| P05 | `rome_extended` | `mastermind` (Phase C) |
| P06 | `poisoned_curriculum` (Phase C) | `transactional_kv_store` (Phase C) |
| P07 | `wordnet_tiered` | `mastermind` (Phase C) |
| P08 | `code_refactoring` | `transactional_kv_store` (Phase C) |
| P09 | `slippery_gridworld` (Phase C) | `escapegame_extended` |
| P10 | `slippery_gridworld` (Phase C) | `poisoned_curriculum` (Phase C) |
| P11 | `mastermind` (Phase C) | P21 entity-belief transfer surface |
| P12 | `transactional_kv_store` (Phase C) | `slippery_gridworld` (Phase C) |
| P13 | `mastermind` (Phase C) | `transactional_kv_store` (Phase C) |
| P14 | `code_refactoring` | `rome_extended` |
| P15 | `transactional_kv_store` (Phase C) | `code_refactoring` |
| P16 | `wordnet_tiered` | `rome_extended` |
| P17 | `wordnet_tiered` | `rome_extended` |
| P18 | `poisoned_curriculum` (Phase C) | `slippery_gridworld` (Phase C) |
| P19 | `wordnet_tiered` + `rome_extended` | any Phase C world with replay artifacts |
| P20 | `poisoned_curriculum` (Phase C) | `transactional_kv_store` (Phase C) |
| P21 | dedicated P21 rig (from primitives bundle) | `mastermind` (Phase C) |

## Promotion Implications (Demo -> Capability)

Fully promoted (CPG-8 complete):

- `p01_plan_trace@1.0` promoted (pre-demo infrastructure; no demo surface — lives in `core/capsules/p01_spec.py`)
- induction-synthesis-demo: `p22.di@1.0` + `p14.ps@1.0` promoted, merged at `63b830d3`
- graphing-calc-demo: `p27.ea@1.0` promoted (96 proof tests), merged at `542c60c1`
- swm-io-demo: `p06@1.0` promoted (127 proof tests, 154 total), merged at `1f86834b`
- ling-ops-demo: `p25.lo@1.0` promoted (182 proof tests, 558 total), merged at `0e5bda52`

D4 (transfer validation) achieved:

- gridkeys-p06-demo: 54 instances, 6 key types, grid transfer
- structured-patch-demo: 32 cases, 129 tests, 6 gate families

D3 (capsule extraction) achieved:

- diffusion-demo, toy-e2e-demo (SAP-1a adds falsifiable semantic agreement proof with anchor independence, predicate-typed role normalization, gated/diagnostic scope split, and structured disagreement reporting; see `test-scenarios/toy-e2e-demo/README.md` review runbook)

D1 (structured harness, no capsule extracted):

- phonology-g2p-demo (146 tests, 80 fixtures, 6 gates; no capsule spec, no CPG gates started; promotion intent undecided — see `test-scenarios/phonology-g2p-demo/PROMOTION_INTENT.md`)

Most ready for D4 promotion evaluation:

- P01, P06, P07, P13, P14, P16, P19

Likely next campaign candidates (need dedicated harnesses):

- P02, P04, P05, P17, P20

Still lacking direct benchmark evidence:

- P03, P08, P09, P10, P11, P12, P15, P18, P21

## What This Run Does Not Prove

- It does not replace certification artifacts from unified benchmark campaign surfaces.
- Four demos have reached full CPG-8 promotion (graphing-calc, swm-io, induction-synthesis, ling-ops). Two demos remain at D4 (gridkeys-p06, structured-patch). Remaining demos have not yet proved D4.
- Note on D3 designations: `diffusion-demo` and `toy-e2e-demo` are listed as D3 in this ledger. Under the strict definition in `demo-promotion.md` (capsule extraction to `core/` + conformance suite in `tests/proofs/`), neither has the full hallmarks of D3. The D3 label was applied under a looser interpretation (core carrier integration or typed-witness presence). `toy-e2e-demo` now has SAP-1a (four-plane semantic agreement proof with negative controls, anchor independence, and structured disagreements), which strengthens the D3 case but does not constitute D4 transfer validation. `phonology-g2p-demo` was previously listed as D3 but has been reclassified to D1 (structured harness, no capsule extracted, no CPG gates; `StepWitnessV1` renamed to `PhonologyStepWitnessV1` to avoid type confusion). The strict rubric should be used for future designations.
- SAP-1a claim set is bounded to 6/12 sentences. Expanding requires carrier upgrades: POS/syntax-aware diffusion role assignment (S3/S7/S11), multiword entity handling (S4), modality/quantification scope encoding (S5/S8). The langpack projection-only analysis (bridge hardening) characterizes all 12 sentences through SELECT_FRAMES+LINEARIZE even when full realization is not possible. See `test-scenarios/toy-e2e-demo/README.md` for the full exclusion policy.
- It does not establish capability completion outside the primitives with direct evidence above.

## Addendum: Unified Benchmark Snapshot (2026-02-18)

> **Eligibility note**: All numbers below are v1 development measurements using temporary `tmp/` artifacts. They are not backed by v2 benchmarking policy artifact bundles (sealed inputs, canonical traces, verification bundles) and are not eligible for published claims. LLM comparison asymmetries are not explicitly documented (Sterling produces verified traces; LLM outputs are unverified final answers). See [`docs/policy/benchmarking_policy.md`](../../policy/benchmarking_policy.md).

Commands executed:

```bash
python scripts/eval/run_unified_benchmark.py --domains wordnet_tiered --providers sterling --tasks 20 --seed 42 --deterministic-priority --native-mode off --native-verify off -o tmp/benchmark_wordnet_tiered_python.json
python scripts/eval/run_unified_benchmark.py --domains wordnet_tiered --providers sterling --tasks 20 --seed 42 --deterministic-priority --native-mode native --native-verify warn -o tmp/benchmark_wordnet_tiered_native.json
python scripts/eval/run_unified_benchmark.py --domains wordnet --providers sterling openai moonshot mistral --tasks 5 --seed 42 -o tmp/benchmark_wordnet_llm_compare.json
```

### A) Sterling Python vs Native (same tiered task batch)

| Mode | Domain | Success | Success Rate | Wall Time | Avg Path Len | Avg Expected Len | Artifact |
|---|---|---:|---:|---:|---:|---:|---|
| Python (`native-mode off`) | `wordnet_tiered` | `17/18` | `94.4%` | `2.12s` | `5.12` | `5.12` | `tmp/benchmark_wordnet_tiered_python.json` |
| Native (`native-mode native`) | `wordnet_tiered` | `16/18` | `88.9%` | `3.06s` | `5.62` | `4.88` | `tmp/benchmark_wordnet_tiered_native.json` |

Observed note: native run emitted authoritative divergence warnings (counter/path/success differences on some tasks), consistent with current parity workstream status.

### B) LLM Comparison (`wordnet`, 5 tasks)

| Provider / Model | Success | Rate | Avg Latency | Total Tokens | Wall Time | Artifact |
|---|---:|---:|---:|---:|---:|---|
| `sterling/structural12` | `5/5` | `100%` | `62.6ms` | `0` | `0.73s` | `tmp/benchmark_wordnet_llm_compare.json` |
| `openai/gpt-4o` | `4/5` | `80%` | `2338.4ms` | `957` | `11.72s` | `tmp/benchmark_wordnet_llm_compare.json` |
| `openai/gpt-4o-mini` | `5/5` | `100%` | `1940.3ms` | `1036` | `9.70s` | `tmp/benchmark_wordnet_llm_compare.json` |
| `moonshot/kimi-k2-0905-preview` | `5/5` | `100%` | `3452.8ms` | `1089` | `17.26s` | `tmp/benchmark_wordnet_llm_compare.json` |
| `moonshot/kimi-k2-thinking` | `0/5` | `0%` | `3133.2ms` | `880` | `15.67s` | `tmp/benchmark_wordnet_llm_compare.json` |
| `mistral/mistral-small-latest` | `5/5` | `100%` | `1409.0ms` | `1471` | `7.05s` | `tmp/benchmark_wordnet_llm_compare.json` |
| `mistral/devstral-small-latest` | `5/5` | `100%` | `1354.7ms` | `1471` | `6.77s` | `tmp/benchmark_wordnet_llm_compare.json` |

Benchmark-reported efficiency highlights vs Sterling for this run:

- vs `openai/gpt-4o`: `37.3x` faster, `957` tokens saved per task
- vs `openai/gpt-4o-mini`: `31.0x` faster, `1036` tokens saved per task
- vs `moonshot/kimi-k2-0905-preview`: `55.1x` faster, `1089` tokens saved per task
- vs `mistral/mistral-small-latest`: `22.5x` faster, `1471` tokens saved per task

## Re-run Commands

From repo root:

```bash
source .venv/bin/activate

python test-scenarios/diffusion-demo/run_demo.py
python test-scenarios/graphing-calc-demo/run_demo.py
python test-scenarios/lemma-grammar-demo/run_demo.py
python test-scenarios/phonology-g2p-demo/run_demo.py
python test-scenarios/structured-patch-demo/run_demo.py
cd test-scenarios/gridkeys-p06-demo && PYTHONPATH=. python run_demo.py && cd ../..
cd test-scenarios/ling-ops-demo && PYTHONPATH=. python run_demo.py && cd ../..
python test-scenarios/swm-io-demo/run_demo.py
python test-scenarios/toy-e2e-demo/run_demo.py
python test-scenarios/induction-synthesis-demo/run_demo.py
python test-scenarios/bytestate-benchmark/run_benchmark.py
```
