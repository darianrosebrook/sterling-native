---
authority: reference
status: advisory
---

# Promotion Gates

**Advisory -- not normative.** This document describes proof obligations for
capability promotion from demo to admitted capability. Do not cite as
canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for
capability status.

## Core Principle

Green tests are necessary but not sufficient. A scenario is only considered
convergent when it defines falsifiers, exposes stress axes, transfers across
domains with zero capsule changes, and produces deterministic, replayable
artifacts that can be bundled and regression-tested through promotion gates.

## Proven by Existing Code

The following elements of the promotion framework are already exercised by the
codebase:

- **Content-addressed artifact bundles** with fail-closed verification —
  `harness/src/bundle.rs` `verify_bundle()`
- **Determinism harness** (CPG-5 equivalent): cross-process lock tests in
  `tests/lock/` verify identical digests across N runs and varied hash seeds
- **Hash surface lock tests** (CPG-1 equivalent): golden digest lock tests
  throughout `tests/lock/tests/`
- **Ordering invariance**: `from_canonical_bytes()` rejects unsorted allocations
  (`kernel/src/proof/canon.rs`)
- **Policy sensitivity**: changing policy changes all downstream digests
  (`harness/src/policy.rs` `PolicySnapshotV1`)

## Future Proof Obligations

The D0–D4 ladder, CPG gates, falsification budget, and promotion artifacts
described below are target criteria for a future capability promotion system.
None are currently tracked or enforced by code. The promotion artifact types
(`CapsuleSpec`, `CPGResults`, `CPGVerdictBundle`, `PromotionProposal`) are
proposed — they do not exist in the codebase.

## D0-D4 Promotion Ladder

The D-levels are progress labels for tracking demo maturity. They are not
admission criteria.

| Level | Name | Purpose | Deliverable |
|-------|------|---------|-------------|
| D0 | Exploratory Demo | Explore a hypothesis | Hypothesis note, failure modes, traces |
| D1 | Structured Benchmark | Wrap demo in deterministic harness | Deterministic seeds, fixed corpus, budget constraints, structured metrics |
| D2 | Primitive Mapping | Map to formal primitive with invariants | Formal signature, invariants, substrate requirements |
| D3 | Capsule Extraction | Extract Sterling-owned contract | Contract types, conformance suite, determinism harness, no domain imports |
| D4 | Transfer Validation | Prove domain-agnostic status | Two fixture sets, same conformance suite, zero code modifications |

If any axis fails, the demo stays a demo. D-level says the scenario is
meaningful; CPG admission says the repo can safely depend on it.

## Capability Promotion Gates (CPG-0 through CPG-8)

Admission into the capability registry requires passing all nine gates. Gates
are hard, fail-closed. No automated runner -- gates are evaluated by tests,
CI, and audits.

| Gate | Name | What It Proves |
|------|------|---------------|
| CPG-0 | Scope Declaration | Capsule declares boundary, tiering, and public surface |
| CPG-1 | Hash Surface Lock | All content-addressed artifacts have golden digest lock tests |
| CPG-2 | Contract Separation | Evidence contract vs registry contract naming is unambiguous |
| CPG-3 | Domain Leakage Audit | Promoted capsule has no semantic dependency on the demo domain |
| CPG-4 | Conformance Suite | Domain-independent test suite proves capsule works in a toy domain |
| CPG-5 | Determinism Harness | Identical inputs produce identical outputs across N runs |
| CPG-6 | Transfer Validation | Capsule transfers to a second distinct domain with zero code changes |
| CPG-7 | Artifact Closure | Promotion proposal binds spec, suites, results, and verdicts |
| CPG-8 | Regression Sweep | Full project test suite and global invariants pass at merge time |

## Falsification Budget

Each promoted capability must define what perturbation classes it survives.
This prevents overfitting to a golden path.

**Required perturbation classes:**

1. **Ordering permutations** -- item reordering in evidence does not affect
   digests
2. **Policy variation** -- changing policy changes artifacts; same policy is
   idempotent
3. **Fixture mutations** -- >=10 adversarial mutations with classified
   outcomes
4. **Cross-domain near-misses** -- >=1 structurally identical but
   vocab-disjoint domain fails transfer
5. **Runtime environment** -- hash seed, working directory, locale

**What constitutes failure:**

- Digest instability (same input, different digest)
- Untyped exception reaching the claim surface
- Partial-success leakage in refusal path
- Nondeterministic refusal (different failure digest for same bad input)

## Proof Portfolio Minimums

### A. Determinism Envelope

| Requirement | Minimum |
|-------------|---------|
| In-process determinism | N>=3 reruns, all hashed surfaces identical |
| Cross-process determinism | Subprocess with varied hash seeds |
| Working directory independence | Run from >=2 different directories |
| Locale independence | LC_ALL=C vs en_US.UTF-8 |
| Policy sensitivity | Changing policy changes all downstream digests |
| Canonicalization integrity | Canonical-equal implies structurally-equal for all set-like fields |

### B. Falsifiability / Negative Controls

| Requirement | Minimum |
|-------------|---------|
| Negative transfer control | >=1 near-miss domain that fails transfer |
| Self-check | Same domain vs itself expected to fail meaningful checks |
| Positive control | Original domain pair still passes after adding negative controls |

### C. Boundary Enforcement / Mutation Campaign

| Requirement | Minimum |
|-------------|---------|
| Mutation fixtures | >=10 distinct mutations covering identifier hygiene, missing keys, ordering, sequence integrity |
| Refusal typing | Each mutation maps to expected outcome (success/refusal) |
| Digest stability | Refusal digests stable across N>=3 reruns per mutation |
| Ordering invariance | Item reordering does not change any hashed surface |

## What Makes a Scenario Admissible

A scenario proves a Sterling-native capability only when it answers five
questions in a reviewer-resistant way:

1. **Claim surface**: What exactly is being claimed, and what is explicitly
   not being claimed?
2. **Falsifiers**: What evidence would disprove the claim, and are those
   failure modes reachable in the harness?
3. **Stress axis**: What knob can we turn (budget, branching, noise,
   adversarial pressure) and does behavior degrade predictably?
4. **Transfer**: Can the same capsule code run on at least two distinct
   domains with zero modifications?
5. **Deterministic artifact closure**: Can a third party rerun and reproduce
   the same digests and verdicts?

If any of these are missing, you may have a working system, but you do not
yet have an admissible capability claim.

## Promotion Artifacts

- **CapsuleSpec** -- structured, content-addressed capsule specification
- **CPGResults** -- deterministic test results (hashes structured data, not
  raw output)
- **CPGVerdictBundle** -- content-addressed bundle of all 9 gate verdicts
- **PromotionProposal** -- the promotion envelope wrapping all evidence

The proposal's bridge to a capability descriptor requires explicit domain_id
-- a capsule is not a domain.
