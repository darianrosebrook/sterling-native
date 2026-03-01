---
authority: reference
status: advisory
date: 2026-03-01
capability: governance
parity_capabilities: [C1, C3]
parity_guardrails: [GR-1, GR-7]
---

# Governance and Certification

**Advisory — not normative.** This document describes proof obligations for future v2 work. Do not cite as canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability status.

## Overview

Governance in Sterling is the system by which execution claims are certified, verdicts are recorded, and promotion decisions are enforced. The sterling Python repo defined a rich governance model with typed run intents, gate verdicts, failure witnesses, execution policies, claim schemas, and campaign-level certification. This reference captures the proof obligations for extending the native substrate's existing governance surface.

The native substrate already implements several governance primitives: Base/Cert verification profiles, PolicySnapshotV1, ArtifactBundleV1 with fail-closed verification, and content-addressed evidence chains. This document focuses on what remains to be built.

## Key Concepts

### Typed Run Intents

Execution runs carry intent metadata: **DEV** (exploratory, no certification claims), **CERTIFYING** (producing evidence for certification), **PROMOTION** (evidence sufficient for tier advancement), **REPLAY** (re-executing from recorded evidence for verification). The intent determines which verification profile applies and what artifacts are required.

The native substrate's Base/Cert verification profiles map to DEV/CERTIFYING respectively. PROMOTION and REPLAY are not yet implemented but follow naturally: PROMOTION adds promotion gate checks on top of CERTIFYING evidence, and REPLAY re-executes from a bundle's recorded trace.

### Typed Verdicts and Refusals

A verdict is a first-class artifact, not a log message. Verdicts are typed: **PASS** (all gates satisfied), **FAIL** (at least one gate failed, with a FailureWitness recording which gate and why), **SKIPPED** (gate not applicable under current policy). Refusals — cases where the system declines to execute rather than risk producing unverifiable claims — are also typed artifacts.

This extends the existing BundleVerifyError pattern: `verify_bundle()` already produces typed, enumerated failure variants. Governance verdicts generalize this to the campaign level.

### Campaign Binding

A certification campaign binds:
- A policy snapshot (what rules governed the run)
- An operator set (what operators were available, via `operator_set_digest`)
- One or more evidence bundles (what was observed)
- Acceptance criteria (what "pass" means)

The native substrate already has three of these four: PolicySnapshotV1, operator_set_digest in OperatorRegistryV1, and ArtifactBundleV1. The missing piece is the campaign-level binding that ties them together with acceptance criteria.

### Fail-Closed Enforcement

Governance enforcement is fail-closed at every level:
- **Type level**: Unknown operators fail at registry lookup (UnknownOperator). Missing artifacts fail at bundle verification (BundleVerifyError).
- **Verification level**: Base profile checks structural integrity; Cert profile additionally checks tape-graph equivalence and header bindings.
- **Governance level**: A campaign that cannot produce complete evidence refuses to issue a verdict rather than issuing a partial one.

### Tool Transcripts

Every tool interaction (external side effect) follows a stage/commit/rollback protocol with an auditable transcript. Stage records intent; commit records execution; rollback records abandonment. The transcript is a content-addressed artifact bound into the evidence bundle.

The native substrate's TransactionalKvStore world already demonstrates stage/commit/rollback semantics at the world level. Governance tool transcripts extend this to external interactions.

### Claim Reducibility

Every governance claim must be reducible to Rust-verified artifacts (ADR 0006). If Python issues a certification verdict, that verdict must reference specific ArtifactBundleV1 digests, specific verification reports, and specific policy snapshots — all of which are independently verifiable by the Rust substrate. No governance claim may rest solely on Python-side assertions.

## Design Decisions (Open)

| Decision | Options | Constraint |
|----------|---------|------------|
| Campaign schema | Minimal (policy + bundles + criteria) or rich (v1's full GovernanceContext) | Start minimal; the existing artifacts already carry most of the binding |
| Verdict artifact format | Extend VerificationReportV1 or define VerdictV1 | Extending the report keeps one artifact type; separate type is cleaner for campaign-level verdicts |
| REPLAY implementation | Re-run from trace or from bundle directory | Trace replay is cheaper; bundle replay is more complete |
| Tool transcript scope | All external interactions or only side-effecting ones | "All" is safer; "side-effecting only" reduces noise |
| Python-Rust governance boundary | Python issues verdicts over Rust evidence (ADR 0006) or Rust issues verdicts directly | ADR 0006 establishes the boundary; the question is when to move more governance into Rust |

## Proof Obligations for v2

1. **Verdicts are artifacts.** Every governance verdict (PASS, FAIL, SKIPPED) is a content-addressed artifact with a ContentHash. Verdicts are never log messages or return codes.

2. **Failure witnesses are specific.** A FAIL verdict includes a FailureWitness identifying which gate failed, what evidence was examined, and why the gate was not satisfied. "It failed" is not a valid witness.

3. **Campaign binding is content-addressed.** A campaign artifact binds policy snapshot digest, operator set digest, evidence bundle digest(s), and acceptance criteria into a single content-addressed record. Tampering with any component invalidates the campaign digest.

4. **Fail-closed at governance level.** If a campaign cannot produce complete evidence (missing artifacts, verification failures, incomplete transcripts), it refuses to issue a verdict. Partial verdicts are not possible.

5. **Tool transcripts are complete.** Every tool interaction has a transcript artifact recording stage, commit or rollback, and the resulting state change. The transcript is content-addressed and bound into the evidence bundle. No external interaction occurs without a transcript.

6. **Claim reducibility to Rust artifacts.** Every governance claim issued by any component (including Python) must reference specific Rust-verified artifacts. A claim that cannot be reduced to content-addressed, verifiable evidence is not a valid governance claim (ADR 0006).

7. **Run intent determines verification profile.** The run intent (DEV, CERTIFYING, PROMOTION, REPLAY) mechanically selects which verification checks are applied. This is enforced by dispatch, not by caller discipline.

8. **Promotion requires campaign evidence.** Advancing a candidate from one trust tier to another requires a complete campaign: policy snapshot, operator set, evaluation evidence, acceptance criteria, and a PASS verdict. No promotion without campaign.

## Parity Audit Reference

This document covers capabilities **C1** (Proof-carrying artifacts + verification) and **C3** (Policy snapshots), plus governance aspects of **GR-1** (Two codebases without authority boundary) and **GR-7** (Operator registry phase boundary) from the [parity audit](../../architecture/v1_v2_parity_audit.md).

**What exists today (verifiable):**
- Content-addressed bundles with fail-closed verification — `harness/src/bundle.rs` `verify_bundle()` (C1: **Implemented**)
- Policy snapshots as normative artifacts — `harness/src/policy.rs` `PolicySnapshotV1` (C3: **Implemented**)
- Operator registry with digest binding — `kernel/src/operators/operator_registry.rs` `OperatorRegistryV1` (A3: **Partial**, Phase 0 complete)
- Authority boundary pinned by ADR 0006 (GR-1: **Resolved**)
- Base/Cert verification profiles in `harness/src/bundle.rs`

**What is proposed (not implemented):**
- Campaign-level binding (a CertificationCampaign type tying policy + bundles + acceptance criteria) — Import Group C
- Typed verdicts as first-class artifacts (a Verdict type with PASS/FAIL/SKIPPED variants and failure witnesses)
- Tool transcript integration (a ToolTranscript artifact with stage/commit/rollback protocol)
- PROMOTION and REPLAY run intents extending the existing Base/Cert profiles

See Import Group C (Governance / certification campaigns) in the parity audit for the strategic context.
