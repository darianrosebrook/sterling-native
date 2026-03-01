# Docs Index

## Canonical (normative for Sterling Native — v2 vocabulary only)
- `canonical/README.md` — what "canonical" means, promotion criteria
- `canonical/philosophy.md` — design philosophy, boundary separations, meta-principles
- `canonical/glossary.md` — curated v2 vocabulary (~100 terms)
- `canonical/core_constraints.md` — 11 architectural invariants (INV-CORE-01..11)
- `canonical/global_invariants.md` — global invariant declarations
- `canonical/neural_usage_contract.md` — neural advisory-only contract (API-enforced)
- `canonical/bytestate_compilation_boundary.md` — compilation boundary contract
- `canonical/code32_bytestate.md` — Code32 identity atom, ByteStateV1, ByteTraceV1

## Specs (forward-looking v2 vocabulary)
- `specs/primitives/00_INDEX.md` — capability primitives P01-P21

## Architecture (durable)
- `architecture/clean_sheet_architecture.md` — v1 audit, sprawl sources, target architecture
- `architecture/module_map.md` — 7-module target (carrier, state, operators, search, proof, worlds, ml)
- `architecture/v2_success_rubric.md` — 9 measurable claims with falsifiers
- `architecture/v1_v2_parity_audit.md` — capability migration map with legacy contract disposition appendix

## Ephemeral (temporal, non-canonical — becomes stale)
- `ephemeral/roadmaps/roadmap_v0_spine.md` — milestone roadmap M0–M7 (spine first, governance second, reuse by evidence)

## Policy
- `policy/doc_authority_policy.md` — **required `Authority:` front-matter field** (canonical | policy | adr | architecture | reference | ephemeral)
- `policy/benchmarking_policy.md`
- `policy/domain_transfer_policy.md`
- `policy/governance_policy.md`
- `policy/versioning_policy.md`
- `policy/canonical_doc_change_policy.md`

## ADRs
- `adr/README.md`
- `adr/0001-compilation-boundary.md`
- `adr/0002-byte-trace-is-canonical.md`
- `adr/0003-neural-advisory-only.md`
- `adr/0004-operator-taxonomy-names.md`
- `adr/0005-v1-is-oracle-not-dependency.md`

## Templates
- `templates/transfer_pack_template.md`
- `templates/benchmark_run_manifest_template.md`

## Reference (advisory, non-authoritative)

All reference docs carry `authority: reference` front-matter. They describe proof obligations, design rationale, and historical context — never cite as canonical requirements. See `reference/README.md` for the full index.

### Capabilities (proof obligations v2 must eventually host)
- `reference/capabilities/memory.md` — SWM, landmarks, decay
- `reference/capabilities/text_boundary.md` — text IR, realization trust boundary
- `reference/capabilities/induction.md` — operator induction pipeline
- `reference/capabilities/governance.md` — certification campaigns, verdicts
- `reference/capabilities/discourse.md` — intent/speech act system
- `reference/capabilities/knowledge_graph.md` — KG contract, entity/relation model

### World Design (how to build worlds + unproven axes)
- `reference/world_design/capability_axes.md` — 10 orthogonal capability axes
- `reference/world_design/world_catalog.md` — planned worlds with status
- `reference/world_design/promotion_gates.md` — D0–D4 ladder, CPG-0 through CPG-8

### Design Rationale (enduring design philosophy)
- `reference/design_rationale/absorption_pipeline.md` — capability absorption + differentiators
- `reference/design_rationale/operator_policy.md` — operators as policies
- `reference/design_rationale/value_architecture.md` — composable value heads
- `reference/design_rationale/conformance.md` — TC-1 through TC-11 theory conformance
- `reference/design_rationale/evaluation_gates.md` — EVAL-01/02/03 research gates
- `reference/design_rationale/search_complexity.md` — Big-O analysis of search

### Historical (context only)
- `reference/historical/retrospective.md` — v1 retrospective
- `reference/historical/north_star.md` — original thesis
