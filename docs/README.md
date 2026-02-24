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
- `architecture/v1_contract_promotion_queue.md` — disposition table for 29 quarantined v1 contracts

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

## Reference (v1 carry-over, non-authoritative)

### v1 full docs (archived from canonical)
- `reference/v1/glossary_full.md` — comprehensive v1 glossary (narrative, all sections)
- `reference/v1/philosophy_full.md` — v1 philosophy with implementation anchors and source file indexes

### v1 canonical contracts (quarantine — awaiting promotion review)
- `reference/v1/canonical/README.md` — promotion criteria and inventory
- 29 contract files (see `architecture/v1_contract_promotion_queue.md` for disposition)

### v1 docs
- `reference/v1/newcomers.md` — comprehensive v1 introduction
- `reference/v1/README.md` — v1 project overview
- `reference/v1/core_features.md` — path algebra, decay, SWM, value function
- `reference/v1/capability_campaign_plan.md` — cert-grade domain surfaces
- `reference/v1/primitive-inventory.md` — primitive inventory summary
- `reference/v1/not_graph_rag.md` — differentiation from Graph-RAG
- `reference/v1/myelin-sheath.md` — certified fast-path corridors
- `reference/v1/toy-domains.md` — capability axes and proving grounds
- `reference/v1/minecraft_domains.md` — Minecraft domain formulations
- `reference/v1/toy_model_plan.md` — original micro-domain plan (historical, pre-Code32)
- `reference/v1/future_state_vision_2027.md` — future state gap analysis
- `reference/v1/industry_convergence.md` — field convergence toward Sterling's ideas
- `reference/v1/retrospective.md` — v1 retrospective and surprising realizations
- `reference/v1/PROVING-GROUNDS.md` — proving grounds reference
- `reference/v1/capability-promotion-runbook.md` — promotion runbook
- `reference/v1/demo-promotion.md` — demo promotion guide
- `reference/v1/README-demos.md` — demos reference

### v1 navigational data (non-normative)
- `reference/v1/moc/` — maps of content (JSON), useful for tooling but not contract surfaces
