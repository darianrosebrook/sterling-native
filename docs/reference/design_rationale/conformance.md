---
authority: reference
status: advisory
---

# Theory Conformance

**Advisory -- not normative.** This document describes proof obligations for
theory conformance invariants. Do not cite as canonical. See
[parity audit](../../architecture/v1_v2_parity_audit.md) for capability
status.

## Purpose

Theory conformance invariants (TC-1 through TC-11) are testable contracts that
prevent drift from Sterling's core theory. They ensure the system realizes the
theory rather than merely passing benchmarks. Unlike hard merge-blocking
invariants, conformance violations trigger investigation, not automatic
rejection.

## Invariant Summary

| ID | Name | Proof Obligation |
|----|------|-----------------|
| TC-1 | No-Regression Semantic Invariance | Adding hybrid/ML components never reduces correctly solved instances vs structural-only |
| TC-2 | IR-Only Inputs | Value heads consume only structured IR, never raw text or chain-of-thought |
| TC-3 | Bounded Student-Teacher Gap | Student predictions stay within declared thresholds relative to teacher |
| TC-4 | No CoT in Decision Loop | Value functions are pure stateless scorers; no text generation |
| TC-5 | Latent is Advisory | Latent reorders search but cannot override operator/goal semantics |
| TC-6A | Provenance Tracking | All hypothesis nodes have auditable DERIVES_FROM edges |
| TC-7A | Hypothesis Influence Gate | Hypotheses cannot affect ranking without tested predictions |
| TC-8 | Invariance Checking | Hypotheses must pass cross-example invariance before influencing search |
| TC-9A | Applicability Preservation | Hypothesis influence only reorders candidates, never adds or removes |
| TC-10 | Registered Interpreters | All hypothesis interpreters must be registered with declared contracts |
| TC-11 | Prior Lineage Replay Determinism | Prior artifacts are deterministically verifiable via hash chain |

## Hybrid/ML Conformance (TC-1, TC-3, TC-5)

These invariants specifically govern the boundary between symbolic and
learned components:

**TC-1 (No Regression):** For any canonical benchmark suite, run
structural-only and run full hybrid. Every instance solved by structural-only
must also be solved by hybrid. This ensures learned components are truly
additive. Differences in path length, node count, or operator sequence are
allowed; differences in solve/no-solve verdict are not.

**TC-3 (Student-Teacher Gap):** When distilling a teacher model into a
student, three metrics are bounded: mean squared value error, operator
disagreement rate, and difficulty disagreement rate. Thresholds are declared
per domain and updated only by explicit documented decision with
justification.

**TC-5 (Latent Advisory):** Value predictions inform search ordering but do
not override operator applicability (determined by preconditions) or goal
satisfaction (determined by task predicate). The symbolic/IR layer remains
authoritative.

## Decision Loop Purity (TC-2, TC-4)

**TC-2 (IR-Only):** Value heads consume only features that are pure functions
of (IR, StateGraph, SWM/Decay). They must not read raw text prompts, call
external LLMs, or depend on stateful history outside the graph.

**TC-4 (No CoT):** Value heads are stateless scorers. They do not generate
text, maintain conversation state, or call external models. Same state must
produce same score. Explanation generation is a separate, post-hoc process.

## Hypothesis Governance (TC-6A through TC-10)

These invariants govern how hypotheses (inductive inferences) interact with
search:

- **TC-6A:** Every hypothesis has auditable provenance (DERIVES_FROM edges).
- **TC-7A:** Untested hypotheses cannot influence search ordering.
- **TC-8:** Hypotheses must generalize across examples before gaining
  influence.
- **TC-9A:** Hypothesis influence only reorders the candidate set, never
  modifying its membership. This maps to the existing advisory-only scorer
  invariant — enforcement would use a membership-equality check on the
  candidate set before and after scoring.
- **TC-10:** Hypothesis interpreters must be registered, preventing black-box
  interpretation.

## Artifact Integrity (TC-11)

Prior artifacts (value-head weights, feature configs, calibration tables) must
be deterministically verifiable via content hash and lineage chain. Any
deployed prior can be traced back to training provenance and independently
verified. Two-phase write protocol: stage artifact, verify hash, then commit.

## Proven by Existing Lock Tests

Several conformance invariants are already partially exercised by the current
codebase:

| Invariant | Evidence |
|-----------|---------|
| TC-2 (IR-Only) | All value scoring in search uses structured CandidateScoreV1, not text |
| TC-4 (No CoT) | Search loop contains zero LLM calls; all transitions via apply() |
| TC-5 (Latent Advisory) | TableScorer is advisory-only; cannot override operator legality |
| TC-9A (Applicability) | Scorer reorders candidates but cannot add/remove (advisory invariant) |
| TC-11 (Lineage) | Content-addressed artifacts with canonical_hash throughout bundle |

TC-1, TC-3, TC-6A through TC-8, and TC-10 require ML components not yet
present in the codebase and represent future proof obligations.

## Enforcement Posture

Conformance invariants trigger investigation, not rejection. This
distinguishes them from hard merge-blocking invariants (core constraints).
The rationale: you cannot require "already won" as a precondition for
experimentation. But persistent violation of a conformance invariant should
prompt architecture review and roadmap correction.
