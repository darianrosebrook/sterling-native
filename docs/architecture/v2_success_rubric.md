---
status: Design target
authority: architecture
date: 2026-02-23
---
# Sterling v2 Success Rubric

---

Here’s a concrete “v2 success rubric” you can treat as a scoreboard. Each item is a measurable claim, an explicit falsifier (what would prove the claim wrong), and the v2 architectural components that are responsible for making it true.

I’m intentionally mixing two classes of claims:

* “Hard wins” where transformer-centric stacks are structurally disadvantaged (determinism, replay, audit completeness, semantic drift control).
* “Competitive wins” where you need to show you’re not paying an unacceptable capability tax (task success under budgets, transfer, learning without regressions).

If Sterling v2 hits the hard wins and is within striking distance on the competitive wins, you have a credible argument that the “reasoning authority” should move from transformers to the governed kernel, with transformers demoted to codecs/heuristics.

---

1. Deterministic replay is exact, not approximate

Claim: For any certified run, given identical inputs (policy snapshot + payload bytes + schema descriptor + registry snapshot), Sterling reproduces bit-identical `ByteState` evolution and bit-identical `ByteTrace` on repeat runs (same machine, different machine, different OS) within the same version epoch.

Measurements:

* Run the same fixture set N times per environment, across at least 3 environments.
* Compare digests: `hash(ByteTrace)` and final `hash(ByteState)` must match exactly.

Falsifier:

* Any mismatch in trace digest or final state digest for the same inputs under the same version epoch.
* Any “we can’t replay because X wasn’t recorded” exceptions in CERTIFIED mode.

Responsible components:

* `carrier/compile` (pure function contract)
* `carrier/bytestate`, `carrier/bytetrace`
* `proof/hash` and `proof/certificate`
* Unified World Harness (ensures all inputs are bound and recorded)

Why it matters vs transformer-centric stacks:

* Most agent frameworks can’t make “bit-identical run replay” a contract because the model is stochastic and the orchestration isn’t content-addressed end-to-end.

---

2. Trace completeness is 100%: no hidden routers, no unlogged decisions

Claim: Every state transition, tool interaction, and cross-domain selection is represented as an operator application in the canonical trace. There are no “side decisions” that only exist as control flow in Python glue.

Measurements:

* Define a trace completeness schema: required event types and required fields per event.
* For each certified run, validate that the trace satisfies coverage: 100% of tool calls have a corresponding operator application + transcript, 100% of domain selections are MetaPlan operators, 100% of state mutations occur only via operator write-sets.

Falsifier:

* Any tool call without a tool transcript bound into the trace.
* Any state mutation that cannot be attributed to an operator application.
* Any domain routing decision that is not a MetaPlan step recorded in the trace.

Responsible components:

* `operators/signature` (write-sets, preconditions)
* `search/engine` (single loop emitting canonical trace events)
* `worlds/metaplan` (domain composition is explicit)
* `proof/governance` (CERTIFIED mode blocks on missing trace obligations)
* Unified World Harness (standardizes step records and artifact emission)

Why it matters:

* This is the operational form of INV-CORE-08. If you can’t prove this, “no hidden routers” is aspirational.

---

3. Semantic drift is structurally prevented, not “disciplined”

Claim: Any change that alters semantic meaning must cause a digest/version boundary to change in a visible, audited way. Conversely, refactors that do not change meaning must preserve golden digests.

Measurements:

* Maintain golden digest locks for canonical outputs per world/scenario.
* Classify PRs: “semantic” vs “non-semantic” based on whether they require a schema/epoch bump.
* Track drift incidents: cases where behavior changed without an epoch/schema bump.

Falsifier:

* A PR changes certified outcomes without triggering a required version bump (or without failing golden locks).
* Two canonical implementations for the same semantic surface exist (IR/hash/loop/etc.), even if “one is deprecated.”

Responsible components:

* INV-CORE-12 enforcement checks (build-time + CI)
* `proof/hash` as the only canonical hash surface
* `carrier/schema` + registry snapshot discipline
* Unified World Harness (forces fixture binding to versions)

Why it matters:

* Transformer-centric stacks often accept drift as normal (“the model updated,” “prompt changed,” “tool behavior changed”). You’re claiming drift control as a core advantage.

---

4. Tool safety is transactional by default, with auditable rollback/verify

Claim: Tool-use is safe-by-construction: operations are (a) staged, (b) verified against declared postconditions, and (c) rollback-capable with full transcripts and evidence binding. The system can prove what it did and undo it when verification fails.

Measurements:

* In the Transactional KV Store truth-regime world, require:

  * 100% tool actions have transcripts
  * 100% actions either verify or rollback (no “best effort” partial commits in CERTIFIED mode)
  * “no side effects without commit” property tests

Falsifier:

* Any tool call that mutates persistent state without passing through a governed operator with an explicit commit step.
* Any failure mode where rollback is impossible or unverifiable.
* Any mismatch between declared postconditions and observed state that does not fail closed in CERTIFIED.

Responsible components:

* `operators/signature` (postconditions/write-sets)
* `proof/governance` (CERTIFIED fail-closed)
* `worlds/harness` (tool transcript binding)
* `worlds/metaplan` (tool selection and commit logic is explicit plan state)

Why it matters:

* “Tool calling agents” often log tool I/O, but they don’t make transactional semantics a core capability with a proof trail.

---

5. Transfer is real: the same capability holds across orthogonal truth regimes

Claim: A capability (operator family + proof portfolio) can transfer across at least 3 worlds without world-specific patches that change semantics (i.e., no hidden special casing). The same claim catalog is satisfied across worlds.

Measurements:

* Pick 2–3 core capability families (e.g., navigation + landmarking, probe-driven belief reduction, transactional tool application).
* Define identical claim IDs and falsifiers across worlds.
* Measure pass rate on held-out fixtures per world.

Falsifier:

* Capability only works after introducing world-specific routing hacks, prompt heuristics, or bespoke evaluator logic that effectively changes the capability definition.
* Capability passes in World A but fails its analogous claims in Worlds B/C under the same policy and budget assumptions.

Responsible components:

* Unified World Harness (common evidence + test format)
* `operators/registry` (capability definitions as data, not code sprawl)
* `proof/certificate` + claim catalogs (portable obligations)
* `search/value` (scoring is advisory; legality and semantics remain stable)

Why it matters:

* This is where “reasoning system” distinguishes itself from “a solver that learned one environment.”

---

6. Partial observability is handled with explicit belief discipline

Claim: In Mastermind-like domains, belief state evolution obeys formal constraints (monotonic reduction after informative probes; no “belief inflation” unless explicitly justified by new evidence types). The policy makes belief discipline measurable.

Measurements:

* Encode belief size and belief consistency as trace-visible state.
* Enforce monotonicity claims after probe operators.
* Track violations over a large test sweep (including randomized fixtures).

Falsifier:

* Any trace where a probe operator increases belief set size without a permitted evidence-type transition.
* Any success achieved by “oracle leakage” (e.g., peeking hidden state) detected by negative controls.

Responsible components:

* `worlds/*` domain definitions compiled into ByteState (belief is explicit state)
* `operators/signature` (probe semantics)
* `proof/governance` + falsifier suites (negative controls)

Why it matters:

* Transformer-centric agents often appear to reason under partial observability but can’t prove belief discipline. Sterling can, if you force belief into the substrate.

---

7. Stochastic environments certify against evidence, not “the environment”

Claim: In stochastic worlds (Slippery Gridworld), certification binds to recorded evidence (seeds, transition witnesses, observation envelopes) and supports distributional evaluation over seed sets. Replay reproduces the recorded trajectory exactly; generalization is measured statistically, not narratively.

Measurements:

* Two tiers:

  1. Exact replay on recorded episodes (must be exact).
  2. Distributional performance over a defined seed set with confidence intervals and locked evaluation protocol.

Falsifier:

* Cannot exactly replay a recorded trajectory due to missing witness data.
* Certified success depends on rerunning the environment rather than replaying recorded evidence.
* Performance claims lack an agreed seed set / statistical protocol (i.e., “worked for me”).

Responsible components:

* Unified World Harness (seed/witness binding)
* `proof/certificate` (what is being certified)
* `carrier/bytetrace` (canonical evidence record)
* `proof/governance` (CERTIFIED mode requirements)

Why it matters:

* This is the line between “we tested it” and “we can reproduce exactly what we tested.”

---

8. Learning (induction) produces promotable operators with no silent regressions

Claim: Induction can propose and promote new operators such that:

* they improve a target metric on held-out fixtures,
* they do not break previously certified claims (regression-free),
* and the promotion is justified by a fixed artifact schema (same evidence surfaces every time).

Measurements:

* For each promoted operator:

  * Before/after evaluation on held-out set (locked)
  * Regression sweep across truth-regime suite
  * Mutation testing / negative controls specifically targeting the new operator’s claims

Falsifier:

* Promoted operator increases success rate but breaks determinism/replay/stability obligations.
* Promotion logic depends on reports or ad hoc logic not captured in the promotion artifact schema.
* Evaluators are modified per-case in ways that invalidate comparability.

Responsible components:

* `operators/induced/{propose,evaluate,promote,store}` (with evaluators as extension point)
* `proof/governance` (eligibility + certification semantics)
* Unified World Harness (uniform artifact emission)
* Golden locks + regression gates

Why it matters:

* This is the “scientific method loop” turned into a governed capability pipeline instead of a research playground.

---

9. Transformer demotion is real: ML is helpful, but never authoritative

Claim: Removing or swapping the ML components changes performance (maybe) but does not change legality/trace correctness/certification semantics. The kernel remains the semantic authority.

Measurements:

* Run the same certified fixture set with:

  * ML scoring on
  * ML scoring off (or replaced with a baseline heuristic)
* Compare:

  * determinism/replay (must remain exact)
  * trace obligations (must remain complete)
  * success rate and efficiency (can vary; that’s the ML contribution)

Falsifier:

* Without ML, the system cannot produce valid traces or violates operator legality (meaning ML was implicitly acting as a router or authority).
* ML path can mutate state or bypass preconditions (API boundary violation).

Responsible components:

* `ml/` API boundary (advisory only)
* `search/engine` + `operators/registry` (legality independent of scoring)
* INV-CORE-13 enforcement
* `proof/governance` (CERTIFIED mode blocks boundary violations)

Why it matters:

* This is the decisive evidence that Sterling is not “an LLM agent with extra steps.”

---

How to use this rubric as a “credible win” story

A transformer-centric stack can often match raw task success on a single benchmark. The place you can win decisively is: reproducibility, auditability, controlled drift, transactional tool semantics, and transfer under certification.

A credible “dethroning” narrative (practically) looks like this:

* Hard wins (must be near-perfect): #1, #2, #3, #4, #9
* Competitive wins (must be strong enough, not necessarily dominant): #5, #6, #7, #8

If you want, next I can convert this into a one-page scorecard template (columns: Claim ID, World(s), Metrics, Falsifiers, Required artifacts, Baseline comparison, Current status) so you can track it the same way you track CPG gates and promotion overlays.
