# Test-Scenarios as Capability Proving Grounds

What, why, and how we use `test-scenarios/` to converge on Sterling-native capabilities.

## What this directory is

`test-scenarios/` is not a demo gallery. It is Sterling's capability proving portfolio: a set of intentionally small domains and harnesses designed to force clarity about (a) what the engine can claim, (b) what evidence must exist for the claim to be admissible, and (c) where the boundary sits between engine, capsule, adapter, and domain data.

A scenario exists to prove one of two things:

1. **A core primitive claim**: "Sterling can reliably do X under defined contracts," where X is a reusable capability that should survive domain changes (representation independence).
2. **A promotion pipeline claim**: "Sterling can produce proof-carrying artifacts for X that are deterministic, falsifiable, and transferable," such that a reviewer can replay, audit, and regression-test the capability without trusting the developer's intent.

Each scenario is therefore an evidence surface, not a success story. "Green tests" are necessary but not sufficient; admissibility is defined by falsifiers, transfer, determinism, and artifact closure.

## Why we run toy domains at all

Toy domains are not about being easy. They are about being small enough that you cannot hide the semantics.

The portfolio is designed to create productive tension. Each domain pressures a different failure mode and forces the substrate/governance to become explicit:

- **Adjudication truth**: avoid false equivalences, surface counterexamples, prove soundness/minimality on an evidence chain (verifier posture).
- **Safety truth**: adversarial refusal with measurable deltas against an unconstrained baseline (boundary posture).
- **Model-creation truth**: induce or synthesize domain objects under governance (metamodel posture).
- **Planning truth**: budget pressure, monotonic degradation curves, typed failure modes (scarcity posture).
- **Operator-calculus truth**: typed operators with preconditions/effects, refusal as a first-class outcome (algebra posture).
- **Substrate truth**: single-material execution and audit representation (carrier posture).

The demos are not competing with each other; they are complementary "truth regimes" that collectively constrain what Sterling is allowed to claim.

The second reason is economic. A governed proof system is expensive if it requires bespoke work every time. Toy domains give you repeatable templates for: deterministic replay, negative controls, mutation campaigns, failure-family reachability, and transfer validation. Over time, that work migrates into shared proof infrastructure and promotion gates. The portfolio is valuable because it's steadily reducing marginal cost of proving the next capability.

## How a scenario proves "Sterling-native capability"

A scenario is considered to prove a Sterling-native capability only when it can answer five questions in a reviewer-resistant way:

1. **Claim surface**: what exactly is being claimed, and what is explicitly not being claimed?
2. **Falsifiers**: what evidence would disprove the claim, and are those failure modes reachable in the harness?
3. **Stress axis**: what knob can we turn (budget, branching, noise, adversarial pressure, distribution shift) and does behavior degrade predictably rather than chaotically?
4. **Transfer**: can the same capsule code run on at least two distinct domains (or encodings) with zero capsule modifications?
5. **Deterministic artifact closure**: can a third party rerun and reproduce the same digests and verdicts (in-process and cross-process), and can promotion assemble a closed evidence bundle that survives regression sweeps?

If any of those are missing, you may have a working system, but you do not yet have an admissible capability claim.

## Multi-domain transfer as the definition of "capability"

Transfer is not a bonus gate; it is the definition of "engine-owned" semantics. A scenario is only proving something Sterling-native when domain changes relocate complexity into adapters/fixtures -- not into the capsule. That is how you distinguish:

- **Engine/capsule owns**: operator semantics, refusal semantics, proof artifacts, determinism, replay.
- **Domain adapter owns**: encoding/decoding, evidence collection format, fixture wiring, domain-specific measurement.
- **Fixtures own**: world instances, edge cases, negative controls, adversarial inputs.

If a capability "works" only because the domain is friendly, transfer will kill it. That's intended.

## Promotion thresholds

The labels (D-levels and CPG gates) exist to prevent the common failure mode: a demo accretes complexity, starts passing its own tests, and slowly becomes impossible to audit.

### D0-D4 (scenario maturity)

- **D0**: runnable harness exists; produces a report; determinism assumptions declared.
- **D1**: contracts are explicit (schemas/types), refusal semantics exist, and failure modes are typed.
- **D2**: proof portfolio begins: negative controls, mutation campaign, basic stress axis, and deterministic replay harness.
- **D3**: capsule extraction achieved: the capability is no longer scenario-local, and the evidence surface is reproducible.
- **D4**: transfer validated: at least two domains, zero capsule code changes, conformance suite passes on both, and transfer verdict is explicit.

### CPG-0 through CPG-8 (admission maturity)

- **CPG-0 through CPG-6** establish that the capability has enforceable contracts, deterministic behavior, falsifiers, and replayable evidence under pressure.
- **CPG-7** proves artifact closure: promotion proposal assembly is deterministic and traceable to actual runs.
- **CPG-8** proves it survives contact with the rest of the repository: full regression sweep, golden locks, and no backwards capability drift.

The difference between "all green" and "proved capability" is exactly the difference between D-level success and CPG admissibility. D-level says the scenario is meaningful; CPG says the repo can safely depend on it.

## Portfolio observations

### Verification gravity

The strongest scenarios are the ones that can produce minimal counterexamples, typed refusals, and stable digests under perturbation. `graphing-calc-demo` is effectively an adjudicator with "counterexample as first-class artifact," while `structured-patch-demo` treats "refusal correctness" as a measurable property (escape-rate delta between constrained vs unconstrained). Those evaluation moves align tightly with promotion requirements around falsifiability and failure-family reachability.

### The control-axis pattern

The portfolio has converged on a reusable "control-axis" pattern: budget pressure leads to monotonic degradation curves leads to typed failure modes. `gridkeys-p06-demo` shows this in the cleanest form (scarcity curve + multiple map topologies + transfer), and `swm-io-demo` echoes it. This matters because it's the backbone for any MetaPlan-style system: once budget is a governed input and failure families are reachable and typed, "escalate / probe / stop" become auditable decisions rather than ad-hoc branching.

### Ling-ops as semantic operator algebra

`ling-ops-demo` is more strategically important than it appears if read as a "semantic operator algebra" rather than a linguistics toy. The 2-domain transfer demonstrates that you can define operators over a constrained semantic stage (roles, scope, modality, definiteness) and have those operators behave consistently under different world encodings. That is the same pattern needed for cross-domain plan mutation: stable operator semantics over varying substrates.

### Reasoning substrate vs realization substrate

The suite is starting to separate "reasoning substrate" from "realization substrate." The `diffusion-demo` + `toy-e2e-demo` pairing treats realization as a governed pipeline with determinism and leakage/coverage checks, then uses cross-branch consistency as a sanity constraint. The architectural point: realization can be swapped (rule engine to neural) without renegotiating the governance contracts.

### ByteState as hidden unifier

ByteState shows up across multiple scenarios, but only `bytestate-benchmark` treats it as the primary object of proof. Performance evidence is being generated "off-ladder" while multiple D3/D4 scenarios implicitly depend on the same carrier invariants. If ByteState is a foundational substrate, more of its invariants should eventually be locked by the same kind of promotion-grade falsification budget applied to capsules.

## Semantic redundancy as a governance primitive

The toy end-to-end cross-check pattern (independent branches converging on shared semantic markers) is a cheap integrity check that doesn't require perfect models. Generalized, it becomes a governance tool:

- Choose a small set of invariant markers (polarity, entity types, scope relations, etc.).
- Require agreement across independent codecs/pipelines.
- Treat disagreement as a first-class artifact: either a refusal (insufficient evidence) or a typed failure (semantic divergence).
- Track divergence rates under stress axes and across domains.

This is "N-version semantics" -- a path to scaling trust without pretending any single realization method is perfect.

## Process rule: demo-first vs core-first

The portfolio encodes a process standard:

- **Demo-first is preferred** because it stabilizes the evidence surface early and keeps the claim honest (easy to rerun, easy to regress, hard to overfit silently).
- **Core-first is allowed only as a salvage or quarantine maneuver**, and it is not admissible evidence until the scenario-level proving surface exists again (wired runner, fixtures, replay, transfer).

`perceptual-substrate-demo` is a documented exception, not a model to emulate. The remediation path: reconstitute a runnable proving surface that forces partial observability seams (Sense vs Commit), observation-envelope hashing, and deterministic replay of evidence, before any claim re-enters the promotion lane.

## Where proof machinery bites hardest vs where it needs new artifacts

The suite is strongest where the world can be made adversarial in a *typed* way (counterexamples, mutations, refusal families, transfer near-misses), and weakest where the world is open-ended and quality is semantic (diffusion/realization, broader G2P coverage, perception). That's a map of where current proof machinery works well, and where new kinds of proof artifacts are needed (distributional gates, belief-state witnesses, stochastic replay witnesses) to maintain the same governance posture as Sterling moves toward perception and more open realization.

## What pressure testing will force next

The primitives that will "force your hand" are the ones that inherently involve uncertainty, time, and open-world variation -- where green tests stop being persuasive unless you build new substrate hooks.

### 1. Partial observability and epistemic action

When a primitive requires actions whose value is "information gain" rather than immediate progress, current toy domains stop being adequate. This forces:

- A first-class belief or uncertainty witness, and governance rules about when uncertainty is permitted to influence decisions.
- Deterministic replay of an observation *envelope* vs non-deterministic detector outputs.

### 2. Receding-horizon invariant maintenance

When planning moves from "reach goal" to "stay safe while reaching goal under changing conditions," this forces:

- Explicit invariant objects with their own hashing and failure families.
- A governance rule that makes invariant violations non-claimable outcomes (fail closed with evidence), not just "suboptimal plans."

### 3. Closed-loop diagnosis and repair

When Sterling must propose a fix, run a test, observe failure, revise, and stop when evidence is insufficient, this forces:

- A hypothesis artifact type, not just a patch artifact.
- Governance that forbids claiming success without a robust verification witness.
- A mutation campaign that proves failure families are reachable (wrong fix, overfit fix, partial fix, unsafe fix).

### 4. Credit assignment tied to execution

When multiple steps contribute to success under scarcity, you need to say "this operator choice caused the improvement" in a replayable way. This forces:

- Comparable run bundles: same seed, same budget, different policy/heuristic, with outcome deltas and typed reasons.
- Governance around "allowed influences" so credit assignment isn't contaminated by hidden randomness or untracked heuristics.

### 5. Generalization beyond curated fixtures

Primitives that only survive hand-authored fixtures need fuzzing, metamorphic testing, and distribution shift. This forces:

- Stronger failure taxonomy (you can't debug fuzz failures without typed failure families).
- Clear boundaries on what the capability is *not* allowed to infer.

## The bar

We do not accept "works on my fixtures" as evidence. A scenario is only considered convergent when it (a) defines falsifiers, (b) exposes stress axes, (c) transfers across domains with zero capsule changes, and (d) produces deterministic, replayable artifacts that can be bundled and regression-tested through promotion gates. The purpose of `test-scenarios/` is to make those requirements unavoidable.

## Related documents

- [`README-demos.md`](README-demos.md) -- capability evidence ledger with current primitive mapping
- [`demo-promotion.md`](demo-promotion.md) -- D0-D4 promotion ladder and gate definitions
- [`capability-promotion-runbook.md`](capability-promotion-runbook.md) -- operational runbook for promotion
- [`docs/reference/capability_primitives_bundle/00_INDEX.md`](../docs/reference/capability_primitives_bundle/00_INDEX.md) -- P01-P21 primitive definitions
