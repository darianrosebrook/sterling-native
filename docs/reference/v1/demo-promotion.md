Right now, demos prove possibility. Your promotion pipeline needs to prove structural value to the engine as defined by:
	•	Layer 0 reasoning substrate guarantees  ￼
	•	The capability absorption philosophy (capsules, rigs, contracts)  ￼
	•	The primitive catalog P01–P21  ￼
	•	The ByteState compilation determinism boundary  ￼

If a demo cannot survive those surfaces, it should never enter the core capability registry.

Below is a formalized Demo → Certified Capability Promotion Pipeline aligned to your architecture and governance posture.

⸻

I. The Core Shift: From “It Works” to “It Changes the Capability Surface”

A demo answers:

Can Sterling solve this task in this world?

Promotion must answer:

Does this introduce a reusable primitive or strengthen an existing one under cert-grade guarantees?

That requires evaluating along four independent axes:
	1.	Primitive alignment
	2.	Contract extractability
	3.	Deterministic replayability
	4.	Transfer potential

If any axis fails, the demo stays a demo.

⸻

II. The Four-Stage Promotion Ladder

Stage D0 — Exploratory Demo (Unstructured)

Purpose: explore a hypothesis.

Allowed:
	•	Hardcoded heuristics
	•	Domain-specific constants
	•	Inline scoring tweaks
	•	Informal evaluation

Not allowed:
	•	Registry mutation
	•	Primitive spec modification
	•	Capability claim

Deliverable:
	•	Hypothesis note
	•	Failure modes discovered
	•	Observed search traces

This stage is disposable.

⸻

Stage D1 — Structured Benchmark Harness

The demo must be wrapped in a benchmark harness with:
	•	Deterministic seeds
	•	Fixed input corpus
	•	Budget constraints
	•	Structured metrics

At this stage, you ask:

Does performance hold across N variations?

Minimum requirements:
	•	≥ 30 problem instances
	•	Clear success metric
	•	Trace capture
	•	Deterministic replay across reruns

If performance collapses under minor perturbations, this is not a capability — it is a brittle strategy.

⸻

Stage D2 — Primitive Mapping

Now the critical filter.

Every demo must map to exactly one of:
	•	An existing primitive (P01–P21)  ￼
	•	A new primitive candidate (with formal boundary)

You must write:
	•	Formal signature (state, operators, objective)
	•	Invariants
	•	Minimal substrate requirements
	•	Required evidence kinds

If you cannot express it in primitive form, it is not absorbable.

This enforces the ingestion philosophy rule:

Sterling owns semantics at the level of contracts and invariants, not domain tricks  ￼

If the demo relies on domain-specific ontology (e.g., HOSTILE_KINDS, WordNet depth assumptions), it fails D2.

⸻

Stage D3 — Capsule Extraction

You now create the Sterling-owned capsule:
	•	Contract types
	•	Invariants
	•	Conformance suite
	•	Determinism harness
	•	Versioned spec entry

No domain imports. No world constants.

This step is the absorption boundary.

If the behavior cannot be extracted without dragging domain constants inside, it remains a rig artifact.

⸻

Stage D4 — Transfer Validation

A primitive is not promoted unless it passes on a second domain.

Minimum requirement:
	•	Two fixture sets
	•	Same conformance suite
	•	No code modifications in the capsule

This is the only proof of domain-agnostic status.

Until transfer is proven, the primitive stays in Provisional registry state.

⸻

III. Required Evaluation Gates Before Registry Inclusion

Promotion into PrimitiveRegistry must pass these gates:

Gate G1 — Deterministic Compilation

All inputs compile via the ByteState boundary deterministically:
	•	Same payload + schema + registry snapshot → identical bytes  ￼
	•	Replay produces identical state hashes

If this fails, it is substrate leakage.

⸻

Gate G2 — Conformance Suite Pass

Must pass:
	•	Structural contract tests
	•	Semantic invariant tests
	•	Edge-case fixtures
	•	Adversarial perturbations

No flaky tests allowed.

⸻

Gate G3 — Performance Envelope

Demonstrate:
	•	Monotonic improvement under increasing budget
	•	Stable degradation under reduced budget
	•	No pathological compute spikes

Promotion requires cost-per-solve metrics, not just accuracy.

⸻

Gate G4 — Regression Sweep

Run entire certified primitive suite.

If any prior primitive regresses outside tolerance:
	•	Promotion is blocked.

This protects against capability drift.

⸻

Gate G5 — Artifact Closure

Produce:
	•	Capsule spec version
	•	Conformance suite hash
	•	Registry epoch
	•	Trace sample bundle
	•	Performance summary
	•	CapabilityDescriptorV1

Only then can it enter the capability claim registry  ￼

⸻

IV. Scoring Whether a Demo Is “Worth Adding”

You need a quantitative intake filter. Otherwise every clever idea gets promoted.

Define a Promotion Scorecard (0–5 each):

Axis	Question
Structural Reuse	Does this generalize beyond the original domain?
Primitive Fit	Does it strengthen or introduce a formal primitive?
Determinism	Is replay fully stable?
Transfer	Has second-domain validation occurred?
Cost Efficiency	Does it improve cost/solve vs baseline?
Failure Transparency	Does it fail-closed with typed artifacts?

Require ≥ 22/30 to enter provisional registry.

Below that, it remains demo-only.

⸻

V. Common Failure Modes to Block

These are patterns that look promising but should not promote:
	1.	Heuristic Leakage
Domain constants creeping into capsule.
	2.	Benchmark Overfitting
Performance collapses on minor perturbation.
	3.	Trace Non-Minimality
Excess operator churn; accidental success.
	4.	Implicit Ontology
Hardcoded feature vocabularies embedded in primitive.
	5.	Non-Transferable Scoring
Learned priors tied to specific domain distributions.
	6.	Compilation Drift
Behavior depends on dynamic schema mutation (violates boundary).

⸻

VI. What This Changes Operationally

You need a formal directory structure:

/experiments/        → D0 demos
/benchmarks/         → D1 harnesses
/primitives_proposed/→ D2 specs (unregistered)
/capsules/           → D3 extracted contracts
/testkits/           → conformance suites
/data/primitive_specs/index.json → registered

Promotion requires moving through directories, not just passing tests.

⸻

VII. Alignment with Architecture Layers

Promotion must respect layer boundaries  ￼:
	•	Layer 0: Reasoning logic change? → Must not break determinism.
	•	Layer 1: Memory impact? → Must not violate SWM invariants.
	•	Layer 2: Carrier impact? → Must compile under frozen schema  ￼.
	•	Layer 3: Realization only? → Cannot introduce semantic authority.

A demo that changes multiple layers simultaneously is too entangled to promote safely.

---

## VIII. Capability Promotion Gates (CPG)

The D-levels (D0-D4) above are **progress labels** for tracking demo
maturity. They are not admission criteria. Admission into the capability
registry requires passing all 9 Capability Promotion Gates:

| Gate | Name | What it proves |
|------|------|----------------|
| CPG-0 | Scope Declaration | Capsule declares boundary, tiering, and public surface via `CapsuleSpecV1` |
| CPG-1 | Hash Surface Lock | All content-addressed artifacts have golden digest lock tests |
| CPG-2 | Contract Separation | Evidence contract vs registry contract naming is unambiguous |
| CPG-3 | Domain Leakage Audit | Promoted capsule has no semantic dependency on the demo domain |
| CPG-4 | Conformance Suite | Domain-independent test suite proves capsule works in a toy domain |
| CPG-5 | Determinism Harness | Identical inputs produce identical outputs across N runs |
| CPG-6 | Transfer Validation | Capsule transfers to a second distinct domain with zero code changes |
| CPG-7 | Artifact Closure | `PromotionProposalV1` binds spec, suites, results, and verdicts |
| CPG-8 | Regression Sweep | Full project test suite and global invariants pass at merge time |

Gates are hard, fail-closed. No automated runner — gates are evaluated by
tests, CI, and audits per the promotion run book.

### Promotion artifacts

- **`CapsuleSpecV1`** — structured, content-addressed capsule specification
- **`CPGResultsV1`** — deterministic test results (hashes structured data, not raw output)
- **`CPGVerdictBundle`** — content-addressed bundle of all 9 gate verdicts
- **`PromotionProposalV1`** — the promotion envelope wrapping all evidence

The proposal's `to_capability_descriptor(domain_id)` bridge produces the
`CapabilityDescriptorV1` that gets registered. The caller must supply
`domain_id` explicitly — a capsule is not a domain.

### Run book

See `capability-promotion-runbook.md` for the deterministic 10-step
procedure from demo to admitted capability.

### Scorecards

The Promotion Scorecard (Section IV) remains a useful **prioritization tool**
for deciding which demos are worth investing in. But the scorecard alone
does not grant admission. Admission = all CPG gates pass + closure artifact
exists + `PromotionProposalV1` wrapping a valid `CPGVerdictBundle`.

---

## IX. The Deeper Strategic View

The reasoning engine's authority comes from:
- Determinism
- Replayability
- Typed invariants
- Transferable primitives

The promotion pipeline (D-levels for progress, CPG gates for admission)
protects those properties.

Without this separation, Sterling degenerates into a collection of clever
domain-specific heuristics that happen to share an engine.
