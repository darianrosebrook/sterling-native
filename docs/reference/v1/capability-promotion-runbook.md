# Capability Promotion Run Book

Deterministic procedure from demo to admitted capability. Each step maps
to one or more CPG gates. A step is complete only when its acceptance
criteria are met and evidenced.

## Prerequisites

- Working capsule extraction (D0-D4 complete)
- `core/` capsule code with no domain imports
- Two distinct fixture domains
- All content-addressed types available: `CapsuleSpecV1`, `CPGResultsV1`,
  `CPGVerdictBundle`, `PromotionProposalV1`

---

## Step 0: Claim Catalog (Required Before All Gates)

Before any CPG gate can be evaluated, the capability must declare what
it claims. Promotion is "claim-complete," not "suite-complete."

**Required artifact:** `docs/claim-catalog.md` in the scenario directory.

Each claim must include:
- **Claim ID** (stable): e.g., `DI-CLAIM-CANON-001`
- **Statement**: one sentence describing the property
- **Scope**: what inputs/conditions it applies to
- **Counter-claim**: how it could be false (forces falsifiability)
- **Evidence obligations**: which test files + test names provide proof
- **Status**: Proven / Partial / Not yet addressed

The catalog must also include a **Non-Claims** section listing properties
that are explicitly not asserted for this promotion level. This prevents
reviewers from assuming coverage that does not exist.

**Reviewer gate:** A reviewer must be able to answer "what does this
capability claim, and where is the evidence?" in under 2 minutes using
only the claim catalog.

**Acceptance criteria:**
- Every claimed property has at least one test file + test name
- Every non-claim is explicitly listed with rationale
- No claim references tests that do not exist

---

## Proof Portfolio Minimums

Passing tests is necessary but not sufficient. Promotion requires a
structured proof portfolio demonstrating the capability's claims are
falsifiable and have been demonstrated under perturbation.

### A. Determinism Envelope

| Requirement | Minimum |
|-------------|---------|
| In-process determinism | N≥3 reruns, all hashed surfaces identical |
| Cross-process determinism | Subprocess with PYTHONHASHSEED={0,1,random} |
| Working directory independence | Run from ≥2 different directories |
| Locale independence | LC_ALL=C vs en_US.UTF-8 |
| Policy sensitivity | Changing policy changes all downstream digests |
| Canonicalization integrity | Canonical-equal ⇒ structurally-equal for all set-like fields |

### B. Falsifiability / Negative Controls

| Requirement | Minimum |
|-------------|---------|
| Negative transfer control | ≥1 near-miss domain (same structure, disjoint vocab) that fails transfer |
| Self-check | Same domain vs itself expected to fail meaningful checks |
| Positive control | Original domain pair still passes after adding negative controls |

### C. Boundary Enforcement / Mutation Campaign

| Requirement | Minimum |
|-------------|---------|
| Mutation fixtures | ≥10 distinct mutations covering identifier hygiene, missing keys, ordering, seq integrity |
| Refusal typing | Each mutation maps to expected outcome (success/refusal) |
| Digest stability | Refusal digests stable across N≥3 reruns per mutation |
| Ordering invariance | Item reordering does not change any hashed surface |

### D. Failure-Family Reachability

| Requirement | Minimum |
|-------------|---------|
| Reachable families | Each claimed failure family has ≥1 end-to-end trigger path |
| Unreachable families | Explicitly listed as non-claims |
| Refusal surface | Refusal claim surface contains only failure digest, no partial success |
| Failure digest stability | No raw exception text in hashed surfaces |

---

## Falsification Budget

Each promoted capability must define what perturbation classes it survives.
This prevents overfitting to a golden path.

**Required perturbation classes:**

1. **Ordering permutations** — item reordering in evidence does not affect digests
2. **Policy variation** — changing policy changes artifacts; same policy is idempotent
3. **Fixture mutations** — ≥10 adversarial mutations with classified outcomes
4. **Cross-domain near-misses** — ≥1 structurally identical but vocab-disjoint domain fails transfer
5. **Runtime environment** — PYTHONHASHSEED, working directory, locale

**What constitutes failure:**
- Digest instability (same input, different digest)
- Untyped exception reaching the claim surface
- Partial-success leakage in refusal path
- Nondeterministic refusal (different failure_digest for same bad input)

---

## Reviewer Checklist: Claim-Complete vs Test-Complete

When reviewing a promotion proposal, verify:

- [ ] Claim catalog exists and lists all claimed properties
- [ ] Every claim has traced evidence (file + test name)
- [ ] Non-claims section exists and is honest about gaps
- [ ] At least one negative control proves a transfer check can fail
- [ ] At least one self-check (A vs A) expected to fail
- [ ] Mutation campaign covers ≥10 mutations with classified outcomes
- [ ] Failure digests exclude raw exception text
- [ ] Cross-process determinism tested (not just in-process)
- [ ] Policy sensitivity demonstrated (not just "same policy works")
- [ ] Canonicalization integrity enforced (set-like fields normalized on storage)

---

## Step 1: Declare scope and tiering (CPG-0)

Build a `CapsuleSpecV1` from the normative fields in the formal spec module.

**Inputs:**
- Capsule ID, spec version, formal signature
- Invariant declarations (ID, description, verification method)
- Substrate tier declarations (tier ID, required protocol methods)
- Semantic assumptions
- Extension IDs, verification gate IDs

**Acceptance criteria:**
- `CapsuleSpecV1.build()` succeeds with all identity fields
- `spec_id` is deterministic (rerun produces same hash)
- Golden digest lock test passes

**Evidence:** `capsule_spec.json` containing `spec_id`

---

## Step 2: Lock hash surfaces (CPG-1)

Every content-addressed artifact type used by the capsule has a golden
digest lock test.

**Acceptance criteria:**
- Each frozen dataclass (`EquivalenceWitnessV1`, `EvalWitnessV1`,
  `StepWitnessV1`, `SamplingPolicy`, etc.) has a `test_golden_digest_lock`
  that asserts the exact hash for canonical inputs
- Rerunning tests produces identical hashes
- No test uses `startswith("sha256:")` as a substitute for exact matching

**Evidence:** Test file paths and passing test output

---

## Step 3: Verify contract separation (CPG-2)

Evidence contracts (witness types, step types) are named and located
separately from registry contracts (claim types, descriptor types).

**Acceptance criteria:**
- Evidence types live in `core/proofs/`
- Registry types live in `core/domains/`
- No type in `core/proofs/` imports from `core/domains/` (one-way dependency)
- Import audit passes (grep for violations)

**Evidence:** Import graph or grep output showing no reverse dependencies

---

## Step 4: Domain leakage audit (CPG-3)

The promoted capsule has no semantic dependency on any demo domain.

**Acceptance criteria:**
- `core/proofs/equivalence_adjudicator.py` does not import from
  `test-scenarios/` or any demo-specific module
- `core/proofs/equivalence_gates.py` same
- `core/proofs/equivalence_witness.py` same
- The conformance suite (`tests/proofs/test_equivalence_capsule_conformance.py`)
  uses only `TableState` / `EvaluableTableState` — never `ExpressionStateV1`
- `test_no_expression_graph_import` passes

**Evidence:** Conformance test output showing domain isolation test passes

---

## Step 5: Run conformance suite (CPG-4)

Domain-independent test suite proves the capsule works with toy state objects.

**Acceptance criteria:**
- `tests/proofs/test_equivalence_capsule_conformance.py` passes
- Tests cover: Tier 1 (artifact replay), Tier 2 (inline eval),
  G5 counterexample soundness, G6 minimality, edge cases
- No expression-graph dependency in test file

**Evidence:** `CPGResultsV1` built from conformance test output

---

## Step 6: Run determinism harness (CPG-5)

Identical inputs produce identical outputs across N runs.

**Acceptance criteria:**
- `tests/proofs/test_equivalence_determinism_harness.py` passes
- Adjudicator, G5, G6 all produce identical digests on repeated runs
- N >= 3 runs verified

**Evidence:** Determinism harness test output

---

## Step 7: Transfer validation (CPG-6)

Capsule transfers to a second distinct domain with zero code changes.

**Acceptance criteria:**
- `tests/proofs/test_equivalence_transfer_domain.py` passes
- Transfer domain uses different state objects (not expression graphs)
- Same capsule code, same gates, different fixture data
- `transfer_domain_ids` in proposal contains >= 2 entries

**Evidence:** Transfer test output, fixture domain IDs

---

## Step 8: Build promotion proposal (CPG-7)

Assemble `PromotionProposalV1` from evidence artifacts using the builder CLI.

```bash
python scripts/build_promotion_proposal.py \
    --capsule-spec-path evidence/capsule_spec.json \
    --cpg-results-path evidence/cpg_results.json \
    --cpg-verdicts-path evidence/cpg_verdicts.json \
    --primitive-id p01 \
    --contract-version "p01@1.0" \
    --transfer-domains graphing-calc physics-sim \
    --output evidence/proposal.json
```

**Acceptance criteria:**
- `PromotionProposalV1.build()` succeeds
- `proposal_id` is deterministic
- `to_capability_descriptor(domain_id)` produces valid `CapabilityDescriptorV1`
  for each transfer domain
- All evidence hashes trace back to actual test runs

**Evidence:** `proposal.json` containing `proposal_id`

---

## Step 9: Gate verdict bundle

Evaluate all CPG gates (CPG-0 through CPG-8) and build the verdict bundle.

**Acceptance criteria:**
- `CPGVerdictBundle.build(verdicts=...)` succeeds with all 9 gate results
- `all_passed` is True
- `verdicts_hash` is deterministic

**Evidence:** `cpg_verdicts.json` containing `verdicts_hash`

---

## Step 10: Regression sweep and merge (CPG-8)

Full project test suite and global invariants pass at merge time.

**Acceptance criteria:**
- `pytest tests/` passes (full suite)
- No pre-existing failures introduced
- Golden digest locks for all new types pass
- Bridge test (`test_bridge_core_demo.py`) passes — core and demo
  adjudicators produce identical witnesses

**Evidence:** Full test output, merge commit SHA

---

## After promotion

1. Register the `CapabilityDescriptorV1` via `to_capability_descriptor(domain_id)`
   for each target domain
2. Archive the `PromotionProposalV1` JSON as the auditable envelope
3. The `proposal_id` is the audit trail anchor; the `claim_id` is the registry entry
