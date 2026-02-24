You’ve validated something *very rare* already: **the engine is becoming a machine**—deterministic, auditable, constraint-respecting—inside controlled worlds. That’s the “physics engine” layer of reasoning.

A deployable, domain-agnostic, continually learning reasoning engine still needs proof along several **orthogonal capability axes** that your current toy suite only partially touches. The easiest way to see the gaps is: *what failure modes exist in the wild that your current worlds do not force Sterling to confront?*

Below is the “unproven surface area” I’d want closed before calling Sterling a complete reasoning engine.

---

## 1) Domain induction as a first-class, audited artifact

**What you’ve proven:** reasoning *within* a domain once it exists.

**Not yet proven:** the system can **construct** a domain model (state schema, invariants, affordances) from raw interactions and then operate over it reliably.

**Why it matters:** Domain-agnostic isn’t “many domains.” It’s “domain creation is a capability.”

**Prove it with:**

* **DomainSpecIR** induced from data (not hand-authored), content-addressed, replayable.
* Cross-check: two independent induction runs produce the same DomainSpecIR given the same evidence.
* Failure mode: domain spec must degrade gracefully (explicit unknowns), not hallucinate.

**Acceptance:**

* `DomainSpecIR.digest` stable across reruns
* DomainSpecIR → Operator affordances → Search is end-to-end deterministic
* A new task family produces a new domain spec without new code

---

## 2) Partial observability and belief-state reasoning

**What you’ve proven:** mostly fully observed, static or near-static state.

**Not yet proven:** reasoning when the state is **incomplete**, information is costly, and you must plan *what to observe next*.

**Why it matters:** Deployment is almost always partially observable (APIs fail, logs missing, user intent implicit, environments changing).

**Prove it with:**

* POMDP-like tasks: hidden variables; observations are actions.
* “Active sensing” operators: query, probe, inspect, test.

**Acceptance:**

* Explicit belief representation (even if coarse): hypotheses + probabilities/weights + evidence ledger
* Decisions are auditable: “we chose to probe X because it maximized expected info gain under budget”
* Robustness to missing/contradictory observations

---

## 3) Stochasticity and non-deterministic environments (while keeping deterministic *certification*)

**What you’ve proven:** determinism of artifacts and traces.

**Not yet proven:** operating in environments where transitions/outcomes are stochastic *without losing auditability*.

**Why it matters:** Real systems have randomness, concurrency, network timing, partial failures. You need determinism in *the reasoning record*, not determinism in the world.

**Prove it with:**

* Worlds where actions have probabilistic outcomes or delayed effects.
* Deterministic replay via recording randomness seeds/outcome witnesses.

**Acceptance:**

* Trace replay reproduces the *observed* episode (not necessarily the underlying stochastic dynamics)
* Policy learning doesn’t overfit to a single sampled trajectory
* Certificates commit to evidence distributions / summary statistics, not single paths

---

## 4) Adversarial robustness and “hostile inputs”

**What you’ve proven:** cooperative tasks with clean semantics.

**Not yet proven:** resisting manipulation—inputs designed to cause brittle heuristics, policy exploitation, or unsafe operator induction.

**Why it matters:** If Sterling learns online, it will be attacked (poisoning, prompt-injection analogs, adversarial patterns).

**Prove it with:**

* Adversarial fixtures: misleading examples, decoy correlations, poisoned episodes.
* “Red team operator induction”: can the system be tricked into learning a bad operator?

**Acceptance:**

* Poisoning detection gates (evidence anomaly / inconsistency)
* Quarantine lane for suspected episodes
* Operator revocation works and propagates (provenance chain invalidation)

---

## 5) Uncertainty calibration and “I don’t know” as a governed outcome

**What you’ve proven:** pass/fail in fixed domains.

**Not yet proven:** calibrated abstention, probabilistic confidence, and decision-making under risk.

**Why it matters:** Deployment requires knowing when to stop, ask, defer, or gather more evidence.

**Prove it with:**

* Tasks where the correct move is to abstain or request clarification.
* Scoring that rewards calibrated confidence (Brier score / ECE style), not only accuracy.

**Acceptance:**

* Explicit confidence models tied to evidence (not vibes)
* Policies can be certified for “safe abstention”
* Clear separation: “can’t solve under budget” vs “insufficient evidence” vs “ambiguous goal”

---

## 6) Continual learning without catastrophic forgetting

**What you’ve proven:** learning in a single phase; determinism and certification pipelines.

**Not yet proven:** online learning that improves performance over time **without regressing** prior competence.

**Why it matters:** If Sterling learns in production, it will face drift and long-tail tasks. Forgetting is the default failure mode.

**Prove it with:**

* Curriculum streams: task families introduced over time.
* Regression gates across a rolling benchmark suite.
* Memory budget pressure (limited operator library, limited priors).

**Acceptance:**

* “No silent regressions” gate in the promotion loop
* Operator library pruning / consolidation that is also audited and reversible
* Measured stability: earlier tasks remain within tolerance while new tasks improve

---

## 7) Transfer and compositional generalization across domains

**What you’ve proven:** competence per-domain.

**Not yet proven:** learned operators / priors transfer *meaningfully* to new domains, and compose into new capabilities.

**Why it matters:** Domain-agnostic is ultimately “shared abstractions.”

**Prove it with:**

* Train in one domain; evaluate in another where the underlying structure is isomorphic.
* Require compositional reuse: solve new tasks by chaining learned operators from prior domains.

**Acceptance:**

* Transfer > 0 with measurable improvement and explainable reuse
* “Bridge evidence”: trace shows which priors/operators were reused and why they applied
* Invariance checks confirm the reuse wasn’t accidental

---

## 8) Real-world I/O: grounding language into IR and back, under audit

**What you’ve proven:** worlds with already-structured representations.

**Not yet proven:** robust linguistic I/O where:

* parsing is uncertain
* discourse accumulates
* meanings shift with context
* and the entire mapping remains auditable

**Why it matters:** Deployment usually begins with messy text, not neat IR.

**Prove it with:**

* A controlled “language-to-IR” pipeline with uncertainty + provenance:

  * multiple parse hypotheses
  * evidence-backed commits
  * user clarification turns as observation actions

**Acceptance:**

* IR extraction produces an **artifact** with hashes, alternatives, and confidence
* Downstream reasoning never depends on uncommitted interpretations
* Explanation traces reference IR artifacts, not raw text

---

## 9) Tool and system integration: acting on external systems safely

**What you’ve proven:** internal simulated action loops.

**Not yet proven:** safe execution over tools (APIs, files, DBs, services) with reversible actions, sandboxes, and reliable failure handling.

**Why it matters:** Deployment means tool calls. Tool calls are where systems break—irreversibly.

**Prove it with:**

* Tool worlds with nondeterministic latency, partial failures, and permission boundaries.
* “Two-phase commit” style action gating: plan → simulate → execute → verify.

**Acceptance:**

* Every external action has a precondition, postcondition, and rollback (or explicit non-reversible classification)
* Observability: logs and tool outputs are evidence artifacts
* Fail-closed on missing verification

---

## 10) Compute governance: bounded reasoning under cost/latency constraints

**What you’ve proven:** budgets and determinism controls in certification.

**Not yet proven:** predictable, stable performance under real-time constraints, and rational tradeoffs between search depth, inference, and memory access.

**Why it matters:** Deployment is economics: cost, latency, throughput.

**Prove it with:**

* Hard time budgets + resource throttling.
* “Anytime” behavior: returns best-known answer with trace at any cutoff.

**Acceptance:**

* Monotonic improvement curve (more budget never makes it worse)
* Stable degradation: under tight budgets it does something sensible and explainable
* Clear metrics: cost per solved task, not just solve rate (ARC-AGI-2 is good here)

---

# A practical “coverage map” of what your current domains *don’t* force

If I had to boil it down: your suite is strong on **controlled-state reasoning** and increasingly strong on **auditability and governance**. The missing orthogonal axes are:

1. **Induced domains (not coded domains)**
2. **Partial observability + active information gathering**
3. **Stochastic environments with deterministic replay artifacts**
4. **Adversarial robustness + poisoning resistance**
5. **Uncertainty calibration + abstention**
6. **Continual learning without forgetting**
7. **Cross-domain transfer and compositional reuse**
8. **Language grounding as audited IR**
9. **Safe tool execution + rollback/verification**
10. **Real-time economics: latency/cost/anytime reasoning**

---

## The clean next move if you want “domain induction + operator learning”

ARC is a good candidate because it pressures (1), (5), (7), (10), but it still won’t cover (2), (3), (4), (9) unless you explicitly design for them.

If you want the smallest next proof that changes everything, it’s this:

> **Induce DomainSpecIR + learn macro-operators + show transfer across task families, with regression gates and uncertainty-aware abstention.**

That single program forces Sterling to stop being “a reasoning engine in worlds” and start being “a reasoning engine that can create worlds.”

If you want, I can turn the 10 axes above into a **verification-first roadmap addendum**: each axis gets (a) a minimal test world, (b) acceptance criteria, (c) artifacts/certificates, and (d) where it plugs into Stage K/L/M and promotion.


Below are “toy” (but signal-rich) domains that specifically pressure **#2 partial observability**, **#3 stochasticity**, **#4 adversarial robustness**, and **#9 tool/system integration**—with **clean, auditable success criteria** that let you say “Sterling solved the axis” without baking in domain-specific hacks.

I’m treating “toy domain” as: small enough to build + iterate quickly, but structured enough to create **hard-to-fake evidence** in the trace/cert artifacts.

---

## 2) Partial observability + belief-state reasoning

You want worlds where **observations are actions**, and acting without information is punished. The key is forcing Sterling to maintain **explicit competing hypotheses** and choose queries by **value-of-information** under budget.

### A. Mastermind / Bulls & Cows (active inference, compact hypothesis space)

**Why it’s ideal:**
It’s “pure belief update.” Every guess is both an *action* and an *observation probe*. No spatial confounds, no environment dynamics.

**What it forces Sterling to prove**

* Belief state representation isn’t cosmetic: it must shrink the consistent hypothesis set.
* The engine chooses probes that maximize expected elimination (or minimize expected remaining entropy).
* It can explain *why* a probe was chosen in terms of discriminative power.

**Clear “solved” signal**

* Median guesses near known optimal baselines for small code lengths.
* Trace shows: hypothesis set size ↓ monotonically after each observation.
* Decision points cite: `expected_information_gain`, `hypothesis_partition_score`, `probe_cost`.

**Implementation simplifier:** start with 3–4 pegs, 4–6 colors; make the “oracle” deterministic.

---

### B. Battleship (spatial belief maps + probe planning)

**Why it’s ideal:**
Same belief-update mechanic, but adds **structured uncertainty** (spatial correlations) and forces “search vs exploit.”

**What it forces Sterling to prove**

* It can maintain a probability heatmap / belief grid and update it based on hits/misses.
* It chooses probes that trade off discovering ships vs finishing known ones.
* It supports hierarchical strategies without hardcoding them (“hunt/target” emerges as learned operator priors).

**Clear “solved” signal**

* Outperforms random and matches simple heuristic baselines.
* Trace shows: belief mass migrates; probes correlate with peaks in posterior.
* “Active sensing” operators become reusable across other domains (transfer evidence).

---

### C. Wumpus World / “Logic grid with fog-of-war” (symbolic constraints + uncertainty)

**Why it’s ideal:**
Forces Sterling to blend **constraint propagation** with **uncertainty**, and to choose between safe moves and probing.

**Clear “solved” signal**

* It can separate:

  * *entailed safe* vs *risky but informative* moves,
  * and *abstain/clarify* when belief is too diffuse.
* Trace produces explicit constraint-derived eliminations plus probabilistic weighting where needed.

---

## 3) Stochastic / non-deterministic environments with deterministic certification

You don’t need a chaotic world; you need a world where **the same action can yield different outcomes**, and Sterling must keep determinism **in the record**, not in the world. That means: **seed + outcome witness** and certs that commit to **distributions**, not a single run.

### A. Slippery Gridworld (MDP with action noise)

**Why it’s ideal:**
Minimal stochasticity (e.g., 20% slip) is enough to require policy reasoning over outcome distributions.

**What it forces Sterling to prove**

* It can plan with transition probabilities.
* It doesn’t “fail determinism” because the world is stochastic: the trace commits to the sampled outcome + seed.
* Learning doesn’t overfit a single trajectory.

**Clear “solved” signal**

* Evaluation uses many seeds; improvements hold in expectation.
* Replay determinism: given (seed, action sequence) you reproduce the same observed episode.
* Certificate ties to: `{seed_set_hash, outcome_witnesses_digest, aggregate_metrics}`.

---

### B. Multi-armed bandit with nonstationarity (drift)

**Why it’s ideal:**
Pressures *continual adaptation* and *stochastic reward* without needing complex state.

**Clear “solved” signal**

* Regret curves improve vs baselines across seed ensembles.
* Trace explains exploration choices as expected value under uncertainty.
* Promotion gates require performance stability across time windows, not just overall average.

---

### C. Delayed-effect stochastic world (“action schedules”)

**Why it’s ideal:**
Real tool use has delayed effects. Simulate that: action A schedules an effect that may succeed/fail later.

**Clear “solved” signal**

* Sterling maintains pending commitments and revises beliefs when effects resolve.
* It can choose follow-up probes to disambiguate failure modes.

---

## 4) Adversarial robustness + hostile inputs + poisoning resistance

Here the goal isn’t “security theater.” It’s: can Sterling **refuse to learn** bad operators, quarantine evidence, and show provenance-driven revocation. You need worlds where the *learning loop* itself is the attack surface.

### A. Poisoned Curriculum World (controlled spurious correlations)

**Design:**
Generate tasks where a tempting heuristic works on 80–90% of training episodes but fails catastrophically on a hidden slice.

Example: “shortcut feature” correlates with success except on adversarially constructed counterexamples.

**What it forces Sterling to prove**

* Evidence anomaly detection: distribution shift / inconsistency triggers quarantine.
* Learned operators must pass *robustness gates*, not just SR gains.
* Promotion includes “poison sensitivity” metrics.

**Clear “solved” signal**

* The system either:

  * declines to promote the heuristic, or
  * promotes only with scoped validity + explicit constraints that block the adversarial slice.
* Trace shows: suspicious clusters flagged; episodes quarantined; promotion fails closed.

---

### B. Adversarial Operator Induction Arena (“red-team induction”)

**Design:**
A second generator produces episodes that specifically try to induce an operator that:

* breaks invariants,
* causes silent regressions,
* or exploits scoring weaknesses.

**Clear “solved” signal**

* Induction produces candidate sketches, but promotion gate blocks them with explicit reason codes.
* Revocation works: if a bad operator slips in, you can trace → revoke certificate → dependent artifacts become invalid.

---

### C. “Prompt-injection analog” for IR extraction (but kept toy)

**Design:**
Inputs contain misleading instructions that would derail a naive parser. Your IR extraction must yield:

* multiple hypotheses,
* low confidence,
* and a request-for-clarification operator.

**Clear “solved” signal**

* Downstream reasoning never commits to a single poisoned interpretation without evidence.
* The engine routes to clarification rather than “helpfully” hallucinating structure.

---

## 9) Tool + system integration (safe acting on external systems)

This axis is about **two-phase commit**, **rollback classification**, **postcondition verification**, and **fail-closed** execution. The toy domain must include: permissions, partial failures, latency, and irreversible operations.

### A. Transactional Key-Value Store World (two-phase commit + rollback)

**Design:**
Sterling interacts with a simulated KV store supporting:

* `plan()`, `apply()`, `verify()`, `rollback()`
* conflict errors, timeouts, permission denials
* read-your-writes consistency rules

**What it forces Sterling to prove**

* Plans are explicit artifacts.
* Every write has preconditions + postconditions.
* On uncertainty, it probes state rather than assuming success.

**Clear “solved” signal**

* No “phantom success”: all applied changes are verified or rolled back.
* Trace has a consistent action lifecycle: `propose → simulate → execute → verify → commit`.
* Certificates commit to tool transcripts as evidence artifacts.

---

### B. Git / Patch Application World (reversible diffs + invariants)

**Design:**
The “world” is a repo state. Actions are patch applications. Failures include conflicts and test regressions.

**Clear “solved” signal**

* Sterling can:

  * generate a patch plan,
  * apply it,
  * run verifications,
  * revert cleanly on failure.
* Great audit surface: diffs are naturally content-addressable.

---

### C. “HTTP microservice sandbox” world (permissions + nondeterministic latency)

**Design:**
Calls return:

* variable latency,
* intermittent failures,
* rate limits,
* auth errors,
* and inconsistent reads.

**Clear “solved” signal**

* Sterling treats tool outputs as evidence artifacts.
* Retries are governed (bounded, reason-coded).
* Permission boundaries are enforced by policy; it doesn’t “work around” them.

---

## A practical pairing: build *one* harness that hosts all four axes

You’ll move faster if you implement a **Unified World Harness** with toggles:

* observability: full vs partial,
* transition: deterministic vs stochastic,
* data: clean vs poisoned,
* tools: internal vs external.

Then each “toy domain” becomes a configuration of the same harness, which reinforces your “no hand-rolled domains” instinct.

---

## If you want the sharpest “next proven domain” after your current suite

ARC(-like) is good for: generalization pressure and cost curves, but it **does not inherently** test #2, #3, #4, #9 unless you wrap it.

A better *proving-ground sequence* (fast feedback, strong signal) is:

1. **Mastermind** (locks #2 cleanly)
2. **Slippery Gridworld** (locks #3 cleanly)
3. **Poisoned Curriculum World** (locks #4 cleanly)
4. **Transactional KV / Git Patch World** (locks #9 cleanly)
   Then: wrap ARC with partial observability + poisoning + tool-verified execution if you want it to become an integration capstone.

---

If you want, I can turn this into a small “Domain Coverage Matrix” artifact: each axis gets 2–3 candidate toy worlds, plus the minimal operator set and the exact acceptance gates you’d add to Stage K/L/M so the proof is certificate-shaped rather than anecdotal.


Below is a **capability matrix** (not a destination map) that treats every “toy domain” as a **measurement instrument**. Each row is a capability class Sterling must prove for “deployable, domain-agnostic, continually learning, auditable reasoning.” Each column makes the purpose explicit: *what we’re solving for*, *how we know*, *what we’ve already proven*, and *what minimal worlds generate clean signal next*.

---

## Sterling Capability Matrix

**Legend**

* **Proven** = you already have repeatable evidence in current suites + governance pipeline.
* **Partially proven** = touched, but not forced hard enough to claim solved.
* **Unproven** = no domain strongly pressures this axis yet.
* **Signal world** = a “toy domain” designed to generate decisive evidence (and fail in informative ways).

---

### A. Core capabilities you’ve already proven (engine becomes a machine)

| Capability                          | What we’re solving for (operational)                                                | How we get clean signal (pass/fail evidence)                                                                    | Status                | What you used to prove it                        | What artifacts/certs demonstrate it                                                             |
| ----------------------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | --------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| **Deterministic, auditable search** | Same inputs → same episode artifacts; decision points explainable and replayable    | Reruns match on stable/semantic digests; decision witness diffs stable                                          | **Proven**            | WordNet / PN Stage K infrastructure + audit work | `stable_digest_v1`, `semantic_commit_digest_v1`, deterministic priority keys, witness artifacts |
| **Governed promotion boundary**     | No silent promotion; fail-closed when missing evidence; promotion reasons are typed | Certification passes/fails with explicit blockers; promotion lane has “hard_fail/not_implemented/informational” | **Proven**            | Stage K/K1 work + promotion lane blockers        | Stage K report blockers, certificate kind gating, typed guardrails                              |
| **Scoped evaluation contracts**     | You can ask: “does this help globally vs within scope?” without ambiguity           | Scoped K1 emits typed interpretation (policy defect vs evidence/scope insufficiency)                            | **Proven**            | PN scoped runs (canonicalize / minimal_flip)     | `k1_validity_interpretation` + scope hashes and allowlist stats                                 |
| **Operator induction viability**    | Induction produces candidates from evidence with stable identity and lineage        | Candidate sets stable across ordering; sketch IDs deterministic; lineage hashes present                         | **Proven (recently)** | OperatorInductionStrategy determinism fix        | Determinism tests + stable sketch IDs/content hashes                                            |
| **Evidence quality guardrails**     | Dataset defects do not masquerade as “induction failure”                            | Guardrail triggers (typed) when ops completeness collapses; reports capture why                                 | **Proven**            | PN scope allowlist stats / ops completeness work | `EVIDENCE_OPS_EMPTY_FOR_SCOPE_TASK_TYPE`, payload shape counts                                  |
| **Prior persistence (post-cert)**   | Certified policies can be persisted forward into the next run                       | Stage M creates and persists prior with correct scope + payload                                                 | **Proven**            | Stage M fix                                      | Stage M prior creation logs + prior artifact                                                    |

**Interpretation:** you’ve proven the *certified reasoning machine* layer: deterministic traces, typed contracts, fail-closed governance, and stable induced identities.

---

### B. The “unproven surface area” matrix (what’s missing for deployability)

This table is the real capability matrix. Each row lists:

1. what must be true in the wild,
2. how to measure it (auditably),
3. what your current domains cover (if at all),
4. recommended signal worlds (minimal, high-signal), and
5. acceptance criteria shaped as **cert-grade gates**.

---

## 1) Domain induction as a first-class audited artifact

| Item                     | What we’re solving for                                                                                                              |                          Current coverage | Signal worlds that force it                                                                                                                                                                   | Acceptance (cert-grade)                                                                                                                                                                                                             |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------: | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **DomainSpec induction** | From raw interactions/logs → induce **DomainSpecIR** (schema, invariants, affordances) that drives search without hand-coded domain | **Unproven** (domains are still authored) | **“Unknown Game” harness**: black-box transition system where Sterling must infer state fields + legal moves; **Rule-learning micro-domains** (tiny cellular automata, finite state machines) | `DomainSpecIR.digest` stable across reruns; spec includes explicit unknowns; end-to-end determinism: DomainSpecIR → operator affordances → search episodes replay; new task family yields new DomainSpecIR **without code changes** |

---

## 2) Partial observability + belief-state reasoning (ACTIVE SENSING)

| Item                      | What we’re solving for                                                   | Current coverage | Signal worlds                                                                                                                                 | Acceptance (cert-grade)                                                                                                                                                                                                                                    |
| ------------------------- | ------------------------------------------------------------------------ | ---------------: | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Belief representation** | Maintain competing hypotheses over hidden state; update via observations |     **Unproven** | **Mastermind / Bulls & Cows** (compact hypothesis pruning); **Battleship** (spatial belief map); **Wumpus World** (constraints + uncertainty) | Belief ledger exists as artifact: hypotheses + weights + evidence links; hypothesis set size/entropy decreases after probes; decisions cite `expected_info_gain` or partition score under budget; trace can answer “why probe X?” with measurable criteria |

---

## 3) Stochastic environments with deterministic certification (deterministic record, not deterministic world)

| Item                                | What we’re solving for                                                                              |                                                            Current coverage | Signal worlds                                                                                            | Acceptance (cert-grade)                                                                                                                                                                                                                                           |
| ----------------------------------- | --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------: | -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Outcome witnesses / seed replay** | Actions may have probabilistic outcomes; replay reproduces observed episode via recorded randomness | **Partially** (your determinism is strong; stochastic worlds not pressured) | **Slippery Gridworld** (action noise); **Bandits with drift**; **Delayed-effect world** (async outcomes) | Episode includes seed + outcome witness; replay reproduces observed transitions; certs commit to distributional evidence (seed set hash + aggregate stats), not a single lucky trajectory; policies improve across seed ensembles; no overfit to one sampled path |

---

## 4) Adversarial robustness + poisoning resistance (hostile online learning)

| Item                              | What we’re solving for                                                          | Current coverage | Signal worlds                                                                                                                                                                                                  | Acceptance (cert-grade)                                                                                                                                                                                                                                                        |
| --------------------------------- | ------------------------------------------------------------------------------- | ---------------: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Poison detection & quarantine** | System identifies inconsistent/poisoned evidence; blocks bad operator promotion |     **Unproven** | **Poisoned Curriculum World** (spurious heuristic works on train but fails on hidden slice); **Red-team induction arena** (episodes crafted to induce unsafe ops); **IR injection analog** (misleading inputs) | Typed anomaly gates: inconsistency / drift / spurious correlation flags; quarantine lane for suspect episodes; promotion fails closed with typed reasons; revocation works and propagates through provenance chain; robust eval shows no catastrophic hidden-slice regressions |

---

## 5) Uncertainty calibration + abstention as governed outcome

| Item                           | What we’re solving for                                                                |                                                                                      Current coverage | Signal worlds                                                                                                                                                  | Acceptance (cert-grade)                                                                                                                                                                                                               |
| ------------------------------ | ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------: | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **“I don’t know” correctness** | Correct action is sometimes to abstain / ask for clarification / gather more evidence | **Partially** (you have typed insufficiency vs defect patterns; not a calibrated abstention loop yet) | **Ambiguous-goal tasks** (multiple valid goals); **noisy observation POMDP tasks** (probe or abstain); **tool permission walls** (must stop rather than guess) | Confidence is explicit and evidence-tied; abstention is rewarded when uncertainty is high; policy can be certified for safe abstention; trace distinguishes “insufficient evidence” vs “can’t solve under budget” vs “ambiguous goal” |

---

## 6) Continual learning without catastrophic forgetting

| Item                                | What we’re solving for                                          | Current coverage | Signal worlds                                                                                                                       | Acceptance (cert-grade)                                                                                                                                                                                               |
| ----------------------------------- | --------------------------------------------------------------- | ---------------: | ----------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Regression-safe online learning** | Improve on new tasks without silently regressing old competence |     **Unproven** | **Curriculum stream harness**: tasks introduced over time with rolling benchmark; memory pressure (limited operator library/priors) | Promotion requires rolling regression gates; operator pruning/consolidation audited and reversible; explicit “no silent regressions” contract; measured stability: old tasks within tolerance while new tasks improve |

---

## 7) Transfer + compositional generalization across domains

| Item                               | What we’re solving for                                                                |                                                      Current coverage | Signal worlds                                                                                                             | Acceptance (cert-grade)                                                                                                                                                                                                  |
| ---------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------: | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Reused operators/priors matter** | Learned operators/priors from one domain measurably help in another isomorphic domain | **Partially** (you’re set up architecturally; not proven empirically) | **Isomorphic pairs**: Sokoban-like vs grid logistics; graph navigation variants; constraint puzzles with shared structure | Transfer > 0; traces show reused priors/operators with explicit applicability evidence; invariance checks confirm reuse isn’t accidental; compositional chains solve new tasks by combining previously learned operators |

---

## 8) Language grounding as audited IR (linguistic I/O under uncertainty)

| Item                          | What we’re solving for                                                                                                        |                                        Current coverage | Signal worlds                                                                                                                       | Acceptance (cert-grade)                                                                                                                                                                                                             |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------: | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **IR extraction is governed** | Text → multiple IR hypotheses with confidence + provenance; downstream reasoning never depends on uncommitted interpretations | **Partially** (PN is a start; full pipeline not forced) | **Controlled instruction+context world**: discourse accumulation + ambiguous references; “clarification turn” as observation action | IR extraction artifact contains alternatives + hashes + confidences; downstream steps reference IR artifacts (not raw text); clarification actions reduce uncertainty; audit trail shows which interpretation was committed and why |

---

## 9) Safe tool execution + rollback + verification (external systems)

| Item                           | What we’re solving for                                                                              | Current coverage | Signal worlds                                                                                                                          | Acceptance (cert-grade)                                                                                                                                                                                   |
| ------------------------------ | --------------------------------------------------------------------------------------------------- | ---------------: | -------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Two-phase commit tool loop** | Plan → simulate → execute → verify; rollback or fail-closed; tool outputs become evidence artifacts |     **Unproven** | **Transactional KV Store World** (rollback/verify); **Git patch world** (revert, tests); **HTTP sandbox** (latency, auth, rate limits) | Every external action has pre/postconditions and rollback classification; tool transcripts are hashed artifacts; missing verification fails closed; retries are governed; permission boundaries respected |

---

## 10) Compute governance (latency/cost/anytime reasoning)

| Item                           | What we’re solving for                                                                             |                                                         Current coverage | Signal worlds                                                                                      | Acceptance (cert-grade)                                                                                                                                    |
| ------------------------------ | -------------------------------------------------------------------------------------------------- | -----------------------------------------------------------------------: | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Economics + anytime curves** | Under hard budgets, behavior degrades gracefully and explainably; more budget never makes it worse | **Partially** (budget gates exist; real-time economics not fully forced) | **Budgeted POMDP + tool world**; **ARC-like cost curves** as a metric wrapper (not the core world) | Monotonic improvement curve vs budget; stable degradation under tight budgets; explicit cost-per-solve metrics; certificates report cost/latency envelopes |

---

# Coverage map: what your current toy suite *already* instruments well

| Current toy domain                   | What it strongly proves today                                                                         | What it does **not** force                                                                                              |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **WordNet navigation**               | Graph search behavior, pathfinding under constraints, determinism/audit primitives in a structured KG | Partial observability, adversarial poisoning, external tool rollback, stochastic dynamics                               |
| **Rome / Wikipedia game (proposed)** | Route learning and landmark formation narrative (if instrumented), long-horizon graph reuse           | Partial observability unless you add fog; adversarial/poisoning; tool safety unless links are treated as external calls |
| **Escape game**                      | Operator synthesis + shadow influence patterns (internal), controlled counterfactuals                 | Tool execution semantics, adversarial online learning pressure, stochastic outcomes                                     |
| **Rubik’s cube**                     | Deterministic planning with strict invariants, multi-step operator composition                        | Partial observability, tool integration, stochasticity, poisoning resistance                                            |
| **PN world (Stage K focus)**         | Linguistic micro-operators + scoped evaluation contracts + governance hardening                       | Full language-to-IR uncertainty, discourse accumulation, active sensing, external actions                               |

---

# What to build next (to cover #2, #3, #4, #9 with clean signal)

Here’s the “minimum set” that covers the requested axes without exploding scope:

| Axis                          | Recommended signal world                | Why it’s minimal but decisive                                               |
| ----------------------------- | --------------------------------------- | --------------------------------------------------------------------------- |
| **#2 Partial observability**  | **Mastermind** (first), then Battleship | Forces explicit belief + active sensing; no excuses, tiny state             |
| **#3 Stochasticity**          | **Slippery Gridworld**                  | Forces outcome witnesses and distributional certification                   |
| **#4 Adversarial robustness** | **Poisoned Curriculum World**           | Directly attacks the induction/promotion loop; forces quarantine/revocation |
| **#9 Tool integration**       | **Transactional KV Store World**        | Clean two-phase commit + rollback + evidence artifacts                      |

---

# How to keep this from becoming “the destination”

Add two governance rules (they keep the matrix honest):

1. **Every “toy domain” must map to a capability row and have an exit condition.**
   If it can’t produce a cert-grade pass/fail signal for a row, it’s not a valid proving ground.

2. **Every pass must be demonstrated in at least two structurally different worlds.**
   Otherwise you’ve proven “a trick,” not a capability. (Example: belief reasoning in Mastermind *and* Battleship.)

---

If you want, I’ll turn this into a repo-ready markdown artifact with:

* a one-page “Capability Definition” for each axis (contract language),
* a “Signal Worlds Backlog” with P0/P1 ordering,
* and the exact **Stage K/L/M hooks** each axis requires (new witness fields, new gates, new cert semantics).
