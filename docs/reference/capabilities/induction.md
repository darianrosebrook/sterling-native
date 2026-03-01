---
authority: reference
status: advisory
date: 2026-03-01
capability: induction
parity_capabilities: [F1]
---

# Operator Induction Pipeline

**Advisory — not normative.** This document describes proof obligations for future v2 work. Do not cite as canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability status.

## Overview

Induction is the process by which Sterling discovers, evaluates, and promotes new operators (or operator configurations like policy tables and scorer weights). The sterling Python repo defined a full induction pipeline with sketch types, hypothesis evaluation, invariance witnesses, and a three-tier promotion lifecycle. This reference captures the proof obligations that pipeline imposes on the native substrate.

The strategic target (per `clean_sheet_architecture.md` section 4) is 5 modules with evaluators as the extension point and uniform promotion packaging. This is a deliberate compression of the sterling Python repo's larger pipeline.

## Key Concepts

### Propose-Evaluate-Promote Lifecycle

Induction follows a three-phase cycle:

1. **Propose**: Generate a candidate (operator sketch, policy table, scorer weights) from observation of search/execution evidence.
2. **Evaluate**: Run the candidate through a standardized evaluation packet — a set of worlds, policies, and acceptance criteria — and collect evidence.
3. **Promote**: If the candidate passes all gates, promote it to a higher trust tier with regression locks.

Each phase produces artifacts. The proposal is a content-addressed sketch. The evaluation produces an evidence bundle. The promotion produces a registry update with the new operator (or configuration) and its certification evidence.

### Three-Tier Promotion

Candidates move through trust tiers:

- **Shadow**: Proposed but unevaluated. May be used in advisory/heuristic contexts (e.g., candidate scoring) but never in authoritative execution.
- **Provisional**: Evaluated and passing gates, but not yet regression-locked. May be used in DEV mode but not in CERTIFYING runs.
- **Production**: Fully certified with lock tests and regression gates. Eligible for CERTIFYING and PROMOTION runs.

This mirrors the memory status model (Committed/Shadow/Weak/Frontier) and the existing Base/Cert verification profiles. Shadow maps to advisory-only (like scorer reordering). Production maps to fail-closed verification (like Cert profile bundle checks).

### Standard Evaluation Packet

An evaluation packet binds:
- A candidate (content-addressed sketch or configuration)
- A set of worlds to evaluate against
- A policy snapshot defining budgets and constraints
- Acceptance criteria (thresholds, invariants, regression baselines)

The packet format must be uniform across candidate types. Whether inducing a scorer table or an operator definition, the evaluation surface is the same: run, collect evidence, check criteria. Evaluators are the extension point — new candidate types add new evaluators, not new pipeline stages.

### Promotion Gates

A promotion gate is a predicate over evaluation evidence. Gates include:

- **Regression check**: The candidate does not degrade performance on previously certified worlds.
- **Lock test generation**: The promotion produces at least one lock test that would detect regression if the candidate were later modified.
- **Evidence completeness**: The evaluation bundle is complete (all artifacts present, content hashes valid, normative projection verifiable).
- **Policy compliance**: The candidate operates within the policy budget that governed its evaluation.

Gates are composable. A promotion from Shadow to Provisional requires a subset of gates; Provisional to Production requires all gates.

## Design Decisions (Open)

| Decision | Options | Constraint |
|----------|---------|------------|
| First induction target | Policy/scorer tables or operator definitions | Tables are simpler (no new dispatch); operators require registry integration |
| Sketch representation | ByteState-encoded or dedicated SketchV1 type | ByteState reuse gets existing verification; dedicated type allows richer structure |
| Evaluator extension model | Trait object (`dyn Evaluator`) or registry-dispatched | Trait object is idiomatic Rust; registry-dispatched matches operator model |
| Module count | 5 (per clean_sheet) or fewer for MVP | Start with 3 (propose, evaluate, promote); add packaging and registry integration later |
| Regression baseline format | Golden fixture snapshots or computed thresholds | Snapshots are simpler but brittle; thresholds require statistical infrastructure |

## Proof Obligations for v2

1. **Content-addressed candidates.** Every proposed candidate (sketch, table, configuration) has a ContentHash. No candidate exists without an address.

2. **Evaluation evidence is a bundle.** Evaluation produces an ArtifactBundleV1 (or equivalent) with content-addressed artifacts, a verification report, and a normative projection. The bundle is verifiable by the existing `verify_bundle()` pipeline.

3. **No silent regressions.** Promotion from Provisional to Production requires evidence that all previously certified claims still hold. The regression check is mechanical (re-run certified worlds, compare evidence), not manual.

4. **Lock tests at promotion.** Every Production-tier candidate has at least one lock test that would fail if the candidate's behavior changed. This is the mechanical backstop against regression.

5. **Uniform evaluation packet.** The evaluation packet format is the same regardless of candidate type. New candidate types add evaluators, not new packet formats.

6. **Promotion is a governed operation.** Promotion produces an operator registry update (or policy update) that is itself content-addressed and included in the evidence chain. The registry before and after promotion are both recoverable.

7. **Shadow is advisory-only.** Shadow-tier candidates may influence heuristic decisions (scorer reordering, candidate prioritization) but must never affect authoritative execution paths. This is enforced by type system or API shape.

8. **Evaluators are the extension point.** Adding a new kind of inducible artifact (e.g., a new operator category) requires implementing a new evaluator, not modifying the propose/promote infrastructure.

## Parity Audit Reference

This document covers capability **F1** (Operator induction pipeline) from the [parity audit](../../architecture/v1_v2_parity_audit.md).

Current status: **Not started.** The parity audit identifies this as an intentional redesign — the sterling Python repo's pipeline will be compressed to 5 modules with evaluators as the extension point.

### What exists today (verifiable)

- Operator registry with typed signatures — `kernel/src/operators/operator_registry.rs` `OperatorRegistryV1`
- Three-phase apply() with effect validation — `kernel/src/operators/apply.rs`
- Content-addressed operator artifacts in evidence bundles — `harness/src/bundle.rs` `operator_registry.json`

### What is proposed (not implemented)

- A propose→evaluate→promote lifecycle for new operators (Shadow → Provisional → Production)
- An identifiability gate requiring learned operators to outperform UniformScorer on held-out instances
- Promotion gate types (a PromotionGate evaluator) with campaign evidence requirements
- Evaluator extension points for custom promotion criteria
