# Sterling Native Project Inventory

**Generated**: 2026-02-25 01:15 UTC
**Entries**: 68
**Version**: 1.0.0

## Summary

| Category | Count |
|----------|-------|
| Architecture | 4 |
| Architecture Decision Records | 6 |
| Benchmarks | 5 |
| CAWS Feature Specs | 1 |
| Canonical Contracts | 8 |
| Git Hooks | 2 |
| Policy | 6 |
| Schemas | 2 |
| Scripts | 1 |
| Specs | 27 |
| Templates | 2 |
| Tools | 4 |

## Architecture

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `docs/architecture/clean_sheet_architecture.md` | Sterling v2: Clean-Sheet Architecture | 482 | architecture |
| `docs/architecture/module_map.md` | Module Map (Sterling Native) | 14 | architecture |
| `docs/architecture/v1_contract_promotion_queue.md` | v1 Contract Promotion Queue | 70 | architecture |
| `docs/architecture/v2_success_rubric.md` | Sterling v2 Success Rubric | 299 | architecture |

### Descriptions

- **`docs/architecture/clean_sheet_architecture.md`**: Design Document (not a commitment — a compass)
- **`docs/architecture/v2_success_rubric.md`**: Design target

## Architecture Decision Records

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `docs/adr/0001-compilation-boundary.md` | ADR 0001: Compilation Boundary as Architectural Spine | 26 | adr |
| `docs/adr/0002-byte-trace-is-canonical.md` | ADR 0002: ByteTrace is the Canonical Trace Artifact | 22 | adr |
| `docs/adr/0003-neural-advisory-only.md` | ADR 0003: Neural Components are Advisory Only (Enforced by A | 21 | adr |
| `docs/adr/0004-operator-taxonomy-names.md` | ADR 0004: Operator Taxonomy Names (Seek/Memorize/Perceive/Kn | 46 | adr |
| `docs/adr/0005-v1-is-oracle-not-dependency.md` | ADR 0005: v1 Is a Test Oracle, Not a Dependency | 38 | adr |
| `docs/adr/README.md` | Architecture Decision Records (ADRs) | 19 | None |

### Descriptions

- **`docs/adr/0001-compilation-boundary.md`**: Accepted (Design target)
- **`docs/adr/0002-byte-trace-is-canonical.md`**: Accepted (Design target)
- **`docs/adr/0003-neural-advisory-only.md`**: Accepted (Design target)
- **`docs/adr/0004-operator-taxonomy-names.md`**: Accepted
- **`docs/adr/0005-v1-is-oracle-not-dependency.md`**: Accepted

## Benchmarks

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `benchmarks/README.md` | Benchmarks | 19 | None |
| `benchmarks/artifacts/README.md` | Benchmark artifacts (reference) | 4 | None |
| `benchmarks/artifacts/benchmark_report_20260206_122058.md` | Sterling Benchmark Report | 89 | None |
| `benchmarks/artifacts/unified_benchmark_20260220_153525.json` | unified_benchmark_20260220_153525.json | 557 | None |
| `benchmarks/artifacts/unified_benchmark_20260220_153525_policy_validation.json` | unified_benchmark_20260220_153525_policy_validation.json | 138 | None |

## CAWS Feature Specs

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `.caws/specs/SPINE-001.yaml` | V2 Spine: Kernel contract surface, ByteState/Code32, ByteTra | 127 | None |

## Canonical Contracts

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `docs/canonical/README.md` | Canonical Definitions (Sterling Native) | 40 | None |
| `docs/canonical/bytestate_compilation_boundary.md` | ByteState Compilation Boundary | 421 | canonical |
| `docs/canonical/code32_bytestate.md` | Code32 and ByteStateV1: Hardware-Native Semantic Representat | 666 | canonical |
| `docs/canonical/core_constraints.md` | Core Constraints v1 | 46 | canonical |
| `docs/canonical/global_invariants.md` | global_invariants.md | 22 | canonical |
| `docs/canonical/glossary.md` | Sterling Native Glossary | 108 | canonical |
| `docs/canonical/neural_usage_contract.md` | Neural Usage Contract | 135 | canonical |
| `docs/canonical/philosophy.md` | Sterling Native Design Philosophy | 136 | canonical |

### Descriptions

- **`docs/canonical/bytestate_compilation_boundary.md`**: Partially implemented (core compilation boundary operational; epoch transitions and dynamic domain handshake not yet wired)
- **`docs/canonical/code32_bytestate.md`**: Implemented (V1 substrate complete)
- **`docs/canonical/glossary.md`**: Definitions enforced by contracts, invariants, and ADRs.
- **`docs/canonical/philosophy.md`**: v2 canonical

## Git Hooks

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `.githooks/commit-msg` | CAWS commit-msg hook: enforce conventional commits + work ta | 47 | None |
| `.githooks/pre-commit` | CAWS pre-commit hook: doc authority linter + docs index stal | 27 | None |

### Descriptions

- **`.githooks/commit-msg`**: CAWS commit-msg hook: enforce conventional commits + work tags

Policy:
- All commits must use conventional commit format: type(scope): message
- Commits touching kernel/ or docs/canonical/ on the main branch
must include a CAWS spec reference: [SPINE-NNN] or (SPINE-NNN)

Install: git config core.hooksPath .githooks
- **`.githooks/pre-commit`**: CAWS pre-commit hook: doc authority linter + docs index staleness check
Lints staged, non-ignored docs against the index (staged content).
Install: git config core.hooksPath .githooks

## Policy

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `docs/policy/benchmarking_policy.md` | Benchmarking Policy | 84 | policy |
| `docs/policy/canonical_doc_change_policy.md` | Canonical Document Change Policy | 28 | policy |
| `docs/policy/doc_authority_policy.md` | Document Authority Policy | 39 | policy |
| `docs/policy/domain_transfer_policy.md` | Domain Transfer Scenario Policy | 87 | policy |
| `docs/policy/governance_policy.md` | Governance Policy | 36 | policy |
| `docs/policy/versioning_policy.md` | Versioning and Epoch Policy | 25 | policy |

### Descriptions

- **`docs/policy/benchmarking_policy.md`**: All benchmarks and performance claims
- **`docs/policy/canonical_doc_change_policy.md`**: “docs/canonical/* and other canonical surfaces”
- **`docs/policy/doc_authority_policy.md`**: All documents in docs/
- **`docs/policy/domain_transfer_policy.md`**: Transfer claims across worlds / truth regimes
- **`docs/policy/governance_policy.md`**: Run modes, eligibility, and artifact semantics
- **`docs/policy/versioning_policy.md`**: schemas, registries, epochs, and compatibility

## Schemas

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `schemas/SolveRequestV1.schema.json` | SolveRequestV1.schema | 112 | None |
| `schemas/SolveResponseV1.schema.json` | SolveResponseV1.schema | 61 | None |

## Scripts

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `scripts/new_feature.sh` | Create a new CAWS feature spec + optional worktree | 80 | None |

### Descriptions

- **`scripts/new_feature.sh`**: Create a new CAWS feature spec + optional worktree

Usage:
scripts/new_feature.sh SPINE-002 "ByteState/Code32 implementation"
scripts/new_feature.sh SPINE-002 "ByteState/Code32 implementation" --worktree

This script:
1. Creates a feature spec in .caws/specs/<ID>.yaml
2. Optionally creates a git worktree for isolated development

## Specs

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `docs/specs/primitives/00_INDEX.md` | Capability Primitive Implementation Specs Bundle | 373 | architecture |
| `docs/specs/primitives/00_IO_CONTRACT.md` | Sterling Rig Harness I/O Contract (Draft) | 69 | architecture |
| `docs/specs/primitives/P01_deterministic_transformation_planning_resource_product.md` | Capability Primitive 1: Deterministic transformation plannin | 94 | architecture |
| `docs/specs/primitives/P02_capability_gating_and_legality_what_actions_are_permitted.md` | Capability Primitive 2: Capability gating and legality (what | 94 | architecture |
| `docs/specs/primitives/P03_temporal_planning_with_durations_batching_and_capacity.md` | Capability Primitive 3: Temporal planning with durations, ba | 94 | architecture |
| `docs/specs/primitives/P04_multi_strategy_acquisition_alternative_methods_different_fai.md` | Capability Primitive 4: Multi-strategy acquisition (alternat | 94 | architecture |
| `docs/specs/primitives/P05_hierarchical_planning_macro_policy_over_micro_solvers.md` | Capability Primitive 5: Hierarchical planning (macro policy  | 94 | architecture |
| `docs/specs/primitives/P06_goal_conditioned_valuation_under_scarcity_keep_drop_allocate.md` | Capability Primitive 6: Goal-conditioned valuation under sca | 94 | architecture |
| `docs/specs/primitives/P07_feasibility_under_constraints_and_partial_order_structure.md` | Capability Primitive 7: Feasibility under constraints and pa | 94 | architecture |
| `docs/specs/primitives/P08_systems_synthesis_compose_components_to_satisfy_a_behavioral.md` | Capability Primitive 8: Systems synthesis (compose component | 94 | architecture |
| `docs/specs/primitives/P09_contingency_planning_with_exogenous_events.md` | Capability Primitive 9: Contingency planning with exogenous  | 94 | architecture |
| `docs/specs/primitives/P10_risk_aware_planning_tail_risk_not_just_expected_cost.md` | Capability Primitive 10: Risk-aware planning (tail risk, not | 94 | architecture |
| `docs/specs/primitives/P11_epistemic_planning_belief_state_and_active_sensing.md` | Capability Primitive 11: Epistemic planning (belief-state an | 94 | architecture |
| `docs/specs/primitives/P12_invariant_maintenance_non_terminal_goals_control_by_receding.md` | Capability Primitive 12: Invariant maintenance (non-terminal | 94 | architecture |
| `docs/specs/primitives/P13_irreversibility_and_commitment_planning.md` | Capability Primitive 13: Irreversibility and commitment plan | 94 | architecture |
| `docs/specs/primitives/P14_program_level_planning_search_over_compressed_representation.md` | Capability Primitive 14: Program-level planning (search over | 94 | architecture |
| `docs/specs/primitives/P15_fault_diagnosis_and_repair_hypotheses_tests_fix.md` | Capability Primitive 15: Fault diagnosis and repair (hypothe | 94 | architecture |
| `docs/specs/primitives/P16_representation_invariance_and_state_canonicalization.md` | Capability Primitive 16: Representation invariance and state | 97 | architecture |
| `docs/specs/primitives/P17_credit_assignment_tied_to_execution_not_plans.md` | Capability Primitive 17: Credit assignment tied to execution | 96 | architecture |
| `docs/specs/primitives/P18_multi_objective_optimization_and_preference_articulation.md` | Capability Primitive 18: Multi-objective optimization and pr | 96 | architecture |
| `docs/specs/primitives/P19_audit_grade_explanations_why_this_plan_why_not_that_plan.md` | Capability Primitive 19: Audit-grade explanations (why this  | 95 | architecture |
| `docs/specs/primitives/P20_adversarial_robustness_rule_injection_hardening.md` | Capability Primitive 20: Adversarial robustness / “rule inje | 96 | architecture |
| `docs/specs/primitives/P21_entity_belief_maintenance_and_saliency.md` | Capability Primitive 21: Entity belief maintenance and salie | 153 | architecture |
| `docs/specs/primitives/P22_perceptual_substrate_and_visual_realization.md` | P22: Perceptual Substrate + Visual Realization | 39 | architecture |
| `docs/specs/primitives/README.md` | Capability Primitive Specs Bundle | 26 | architecture |
| `docs/specs/primitives/templates/global_invariants.md` | global_invariants.md | 25 | architecture |
| `docs/specs/primitives/templates/rig_template_A_to_H.md` | rig_template_A_to_H.md | 18 | architecture |

### Descriptions

- **`docs/specs/primitives/00_INDEX.md`**: imported
- **`docs/specs/primitives/00_IO_CONTRACT.md`**: imported
- **`docs/specs/primitives/P01_deterministic_transformation_planning_resource_product.md`**: imported
- **`docs/specs/primitives/P02_capability_gating_and_legality_what_actions_are_permitted.md`**: imported
- **`docs/specs/primitives/P03_temporal_planning_with_durations_batching_and_capacity.md`**: imported
- **`docs/specs/primitives/P04_multi_strategy_acquisition_alternative_methods_different_fai.md`**: imported
- **`docs/specs/primitives/P05_hierarchical_planning_macro_policy_over_micro_solvers.md`**: imported
- **`docs/specs/primitives/P06_goal_conditioned_valuation_under_scarcity_keep_drop_allocate.md`**: imported
- **`docs/specs/primitives/P07_feasibility_under_constraints_and_partial_order_structure.md`**: imported
- **`docs/specs/primitives/P08_systems_synthesis_compose_components_to_satisfy_a_behavioral.md`**: imported
- **`docs/specs/primitives/P09_contingency_planning_with_exogenous_events.md`**: imported
- **`docs/specs/primitives/P10_risk_aware_planning_tail_risk_not_just_expected_cost.md`**: imported
- **`docs/specs/primitives/P11_epistemic_planning_belief_state_and_active_sensing.md`**: imported
- **`docs/specs/primitives/P12_invariant_maintenance_non_terminal_goals_control_by_receding.md`**: imported
- **`docs/specs/primitives/P13_irreversibility_and_commitment_planning.md`**: imported
- **`docs/specs/primitives/P14_program_level_planning_search_over_compressed_representation.md`**: imported
- **`docs/specs/primitives/P15_fault_diagnosis_and_repair_hypotheses_tests_fix.md`**: imported
- **`docs/specs/primitives/P16_representation_invariance_and_state_canonicalization.md`**: imported
- **`docs/specs/primitives/P17_credit_assignment_tied_to_execution_not_plans.md`**: imported
- **`docs/specs/primitives/P18_multi_objective_optimization_and_preference_articulation.md`**: imported
- **`docs/specs/primitives/P19_audit_grade_explanations_why_this_plan_why_not_that_plan.md`**: imported
- **`docs/specs/primitives/P20_adversarial_robustness_rule_injection_hardening.md`**: imported
- **`docs/specs/primitives/P21_entity_belief_maintenance_and_saliency.md`**: imported
- **`docs/specs/primitives/P22_perceptual_substrate_and_visual_realization.md`**: imported
- **`docs/specs/primitives/README.md`**: imported
- **`docs/specs/primitives/templates/global_invariants.md`**: imported
- **`docs/specs/primitives/templates/rig_template_A_to_H.md`**: imported

## Templates

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `docs/templates/benchmark_run_manifest_template.md` | Benchmark Run Manifest Template | 17 | None |
| `docs/templates/transfer_pack_template.md` | Transfer Pack Template | 29 | None |

## Tools

| Path | Title | Lines | Authority |
|------|-------|-------|-----------|
| `tools/docs_index.py` | CAWS Structural Docs Indexer | 304 | None |
| `tools/generate_inventory.py` | Sterling Native Project Inventory (MOC) Generator | 497 | None |
| `tools/lint_docs.py` | CAWS Document Authority Linter | 325 | None |
| `tools/llm_client.py` | Shared LLM client for MOC generation scripts | 479 | None |

### Descriptions

- **`tools/docs_index.py`**: CAWS Structural Docs Indexer

Builds a deterministic JSON index of all docs/ files with their metadata:
- authority level (from YAML front-matter)
- path, title, status
- cross-reference links

Usage:
    # Build structural index (deterministic, no LLM)
    python tools/docs_index.py --mode structural

    # Check if index is stale (for pre-commit, reads working tree)
    python tools/docs_index.py --check

    # Check if index is stale against staged bytes (for pre-commit hook)
    python tools/docs_index.py --check --staged

    # Build with LLM-augmented summaries (optional, requires API key)
    python tools/docs_index.py --mode augmented

Output:
    docs/_index/docs_index.v1.json  (tracked in git)

Exit codes:
    0 = index is up to date (or was rebuilt successfully)
    1 = index is stale (--check mode)
- **`tools/generate_inventory.py`**: Sterling Native Project Inventory (MOC) Generator

Scans docs, scripts, tools, schemas, and config files to produce a
project-wide Map of Content (MOC) inventory.

Two modes:
  --mode structural   Deterministic, commit-worthy. Extracts metadata from
                      YAML front-matter, docstrings, and header comments.
  --mode augmented    LLM-powered descriptions, cache-backed by git blob sha.
                      Only regenerates entries whose content has changed.

Output:
  docs/MOC/PROJECT_INVENTORY.json  — machine-readable
  docs/MOC/PROJECT_INVENTORY.md    — human-readable, grouped by category

Cache (augmented mode only):
  .cache/inventory/descriptions.json  — keyed by blob_sha + prompt_version

Author: @darianrosebrook
- **`tools/lint_docs.py`**: CAWS Document Authority Linter

Validates that markdown files in docs/ follow the doc authority policy:
- YAML front-matter with `authority:` field present
- authority value matches path convention (canonical/, policy/, adr/, architecture/)
- Authority-specific banned terms (e.g. canonical forbids ephemeral language)
- Link hygiene: resolves relative links and checks target existence
- No ephemeral docs staged for commit
- README/index files and templates are exempt; reference docs require `authority: reference`

Usage:
    # Lint staged files only (for pre-commit hook)
    python tools/lint_docs.py --staged [files...]

    # Lint specific files
    python tools/lint_docs.py docs/canonical/glossary.md docs/policy/foo.md

    # Lint all docs
    python tools/lint_docs.py --all

Exit codes:
    0 = all checks pass
    1 = lint errors found
- **`tools/llm_client.py`**: Shared LLM client for MOC generation scripts.

Provides a unified interface for generating descriptions of code,
documentation, and script files. Defaults to a local in-process
Transformers harness for stability; Ollama remains opt-in.

Backend selection via MOC_LLM_BACKEND env var:
  - "local" (default): in-process Transformers (MOC_LOCAL_MODEL_PATH)
  - "ollama": local Ollama HTTP/CLI fallback

Author: @darianrosebrook

