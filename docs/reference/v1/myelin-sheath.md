Let’s now talk about how the internal IR can grow and decay dynamically. This was one of the early things we needed to look into with Sterling before we agreed on what it is, because if we aren’t careful this technically becomes context window under a different abstraction.

Throwing out the IR that has grown per turn forces us into the same problem we were trying to solve with Sterling (the back and forth attention that requires reading all/ingesting the whole context again to predict the next token in a transformer architecture)

However holding on to everything explodes the graph, even though our structure is really efficient anyways, it will have a computational limit at some point.

I wonder if right now this is a domain to hold off on full integration until we get good signal or design a better operating space for this.

I think that’s it, we can prove that it’s possible and learn from it, but not bake the full thing as a sterling capability yet

You’re pointing at the real cliff: if the IR is “everything we’ve ever seen,” you rebuild a context window under a new name. If the IR is “only this turn,” you’ve recreated transformer-style rereading because you keep throwing away the very structure Sterling is supposed to preserve.

The way out is to treat IR growth/decay as a first-class systems problem with explicit budgets, explicit retention semantics, and explicit replay hooks. Not “store everything” vs “store nothing,” but “store the right things at the right fidelity for the current operating mode,” with deterministic policies that you can audit.

The design pattern that tends to work is a tiered, budgeted state store with reversible compaction.

1. What “dynamic IR” should mean in Sterling

Instead of one monolithic graph, treat conversation/world understanding as three coupled stores:

A. Working Set (WS)
Small, hot, fast. The nodes/edges you expect to touch in the next few reasoning steps. This is analogous to a CPU cache: high churn, strongly budgeted, aggressively pruned.

B. Persistent Substrate (PS)
Slow, audited, replayable. This is where “committed” facts, definitions, and promoted structures live. It should grow, but it grows with compaction rules and promotion gates.

C. Frontier / Index (FI)
Large-ish, cheap-to-query, not authoritative. Holds “unlit hallways” and retrieval structures (candidate edges, embedding indexes, lexical indexes), but these are not allowed to directly influence the committed substrate except through explicit promotion operators.

That gives you a clean answer to the “context window by another abstraction” risk: you never require the full historical substrate to be loaded to take the next step. You carry a small WS forward, and you rehydrate from PS/FI only when needed.

2. The important invariant: you don’t delete meaning; you change fidelity

Graph explosion happens when you keep all details at full granularity forever. The right move is not (only) deletion; it’s compaction: turning a subgraph into a summarized object that preserves the parts you care about and retains a reversible link back to the original evidence.

Think of compaction like a compiler lowering pass in reverse: you can “fold” a region into a macro-node plus a proof artifact.

Example compaction artifacts:

* EpisodeBundle summaries: “turns 14–21 established entity E7 as the same referent as mentions m23,m41,m58; evidence spans are …”
* Proposition compression: a chain of derived propositions collapses into one “DerivedClaim” with a witness chain hash.
* Frontier pruning: keep only top-k hypotheses per attachment point, with a record of what got pruned and why.

Key property: compaction is itself an operator that emits a patch + witness, and it produces a digest that can be replay-verified. That’s what prevents “memory drift” from becoming silent model behavior.

3. Growth model: promotion lanes, not accumulation

If everything becomes committed, you explode. If nothing becomes committed, you keep re-deriving.

So you need promotion lanes with budgets. Concretely:

* Lane 0: Surface anchors (cheap, lossless) persist for replay.
* Lane 1: Minimal committed semantics (entities, predicates, propositions, scope operators) persist, but only if they meet a “usefulness threshold.”
* Lane 2: Higher-order structures (discourse graphs, causal graphs, schemas, scripts) are expensive; they should be mostly frontier until a task demands them.
* Lane 3: Grounding (WordNet/Wikidata, etc.) stays frontier by default and only becomes committed when it is repeatedly useful and stable.

The system’s default behavior should be: generate rich frontier, promote sparingly, compact routinely.

4. Decay model: eviction by utility under a fixed compute/memory budget

You need an explicit budget and eviction policy, otherwise you’ll build a system that “usually works” until it catastrophically doesn’t.

A practical decay policy uses a small number of measurable signals:

Retention score for a node/edge could be computed from:

* Recency: last touched step/time.
* Frequency: how often referenced by operators.
* Connectivity: how central it is to currently active tasks (graph distance to active goals).
* Stability: whether it has been promoted/verified (committed beats frontier).
* Cost: size of its neighborhood / fanout.
* Replacement value: can it be cheaply re-derived from PS or re-retrieved from FI?

Then enforce budgets with “GC operators”:

* Demote: move a committed-but-low-utility structure back into frontier (rare; requires careful semantics).
* Compact: collapse a subgraph to a summary node + witness.
* Evict from WS only: keep in PS, but not loaded/active.
* Prune frontier: keep top-k and discard tail.

The critical nuance: most “decay” should be WS eviction and frontier pruning, not destructive deletion from PS.

5. Avoiding transformer-like rereading: rehydration must be targeted and bounded

Your fear about “attention back and forth” is valid if rehydration is “load everything.” Don’t do that.

Rehydration should look like:

* Identify a small set of “needles” (entities/propositions/operators relevant to the current step).
* Pull a bounded neighborhood around those needles from PS (k-hop, capped fanout).
* Pull additional candidates from FI by retrieval (lexical match, embedding similarity, structural match).
* Reconstruct just enough to run the next operator step.

This is essentially “bounded retrieval + bounded expansion,” and it should be as deterministic as possible given the same retrieval seeds and indexes.

6. A clean operating-space split: treat dynamic growth/decay as a separate subsystem

I agree with your instinct not to bake this fully as a Sterling capability yet. There’s a strong way to do that without punting the problem:

Define an interface now, implement a simple policy now, and postpone sophisticated policies.

Interface (minimal):

* `materialize(seeds, budget) -> working_subgraph`
* `commit(patch, witness) -> persistent_store_update`
* `compact(region, policy_id) -> summary_node + witness`
* `evict(policy_id) -> patch` (WS-only eviction; optionally frontier pruning)
* `stats() -> size, fanout, hotset, compaction_ratio`

Then implement a “boring but safe” v0 policy:

* WS budget: fixed node/edge cap (e.g., N nodes, M edges).
* Always keep: anything touched in last T steps, anything directly connected to active goals, all PromotionRecords and their anchors.
* Compact: any subgraph older than T with fanout above threshold into a summary node.
* Frontier pruning: top-k per hypothesis kind per attachment point.
* Never destructively delete committed nodes in v0 (only compact and WS-evict).

That gives you proof-of-possibility without turning it into a research sink.

7. What you can prove now without full integration

You can set up a proof harness that answers the key question: “Can Sterling operate with bounded WS without rereading all history, while maintaining correctness on tasks?”

A good initial proof is:

* Run tasks that require reference to older context (coref across turns, deferred commitments, causal consistency).
* Enforce a small WS budget so that naive “keep everything” is impossible.
* Measure: operator success rate, number of rehydration calls, average rehydration size, compaction ratio, and replay determinism.
* Require: identical outputs across runs (same seeds), plus a bounded compute profile.

If that harness works, you’ve validated the operating space. Then you can iterate on the retention/compaction heuristics later.

8. The decision you’re implicitly making (and I think it’s right)

Sterling’s differentiator isn’t “infinite context.” It’s “explicit state + explicit transforms + auditable retrieval.” Dynamic IR growth/decay is infrastructure, not core reasoning capability—at least until you’ve got strong signal that your tasks demand it and your policies are stable.

So: design the contract, build the minimal WS/PS/FI split, add compaction as an operator with witnesses, prove bounded operation in a harness, and stop there for now. That gets you the learning without the architectural commitment that could lock you into a brittle memory policy too early.

If you want the next concrete artifact, I’d write a “Memory/IR Lifecycle Contract v0” that specifies: budgets, allowed decay operations, compaction witness requirements, and a minimal benchmark suite to validate “bounded WS, no full reread.”


We can hold onto things like weak maps too, or just hold a reference to the source and keep the committed graph of what Sterling understands of the meaning especially if parsing is cheap (since it’s not tokens). This goes back to the semantic working memory we had originally been working towards. 

That Sterling operates in the semantic space and only uses language as i/o

That implies two strong invariants:
	•	The committed graph is Sterling’s authoritative “meaning-state,” not a mirror of the raw string.
	•	The raw text is retained as a provenance substrate: you can always point back to spans and show “why” a semantic commitment exists.

We can treat weak references to already ephemeral or likely to change relationships, and stable git-like references for determining when some reference is dirty, relocated, missing, with the same kind of shadow node/operators for things that are assumed but require stable evidence or verification. Then for decay it’s going to be our already designed “use it or lose it” neural sheathing reinforcement design we have been using to tentatively reinforce or deprioritize/prune the graph. 

Things can be reintroduced into the shadow nodes if they become relevant again but until then they get ornamental treatment only, and don’t affect any of the path algebra or relations once decayed. This can also be gated in a way that current episode becomes the way we promote canonical representation of key paths, nodes, operations, essentially letting episode shapes be the A/B state of the graph. Then the entry and exit can be replayed with episode parsed as the hash we salt the path with and can pull up with the given evidence packet.

This allows the multiple different paths to essentially do the “per episode” and keep growing per reasoning turn, grow and be deterministic with a growing graph when the hash is stable, and allow us to revisit what made state b of our reasoning trace true

Your framing is coherent, and it gives you a clean way to avoid “context window by another name” while still letting Sterling accumulate meaning over time.

If I restate what you’ve proposed in systems terms: you’re defining a semantic state machine with (1) an authoritative meaning-state, (2) a provenance substrate you can always replay against, and (3) a decaying periphery of non-authoritative structure that can be rehydrated and re-promoted. You then bind transitions to episode artifacts (hashes + evidence packets) so the system is both bounded and auditable.

The important move is: make “decay” and “rehydration” *semantic operations* with explicit type boundaries, not a garbage collector quietly mutating your world model.

Here’s a concrete way to turn your ideas into an internal contract.

1. Two invariants, plus the missing third

You already have the two strongest invariants:

(1) Committed graph = authoritative meaning-state (not a mirror of raw text).
(2) Raw text retained as provenance substrate (you can always point to spans / evidence).

You’ll want a third invariant to keep the system stable under decay/rehydration:

(3) Only committed edges participate in planning/search/path algebra; shadow/weak edges are observational or advisory and cannot change successor legality, only prioritization when explicitly allowed.

That third one is what prevents “ornamental structure” from silently becoming meaning.

2. Split the semantic store into 4 classes of objects (not 2)

Instead of only “committed vs frontier,” add “weak” and “shadow” as separate typed statuses with different semantics.

A. Committed (authoritative)

* Participates in reasoning, path algebra, legality checks.
* Must have stable witnesses (anchors to provenance + promotion record).
* Must be replayable deterministically.

B. Weak (soft references / speculative edges)

* May exist as convenience for retrieval, clustering, and “likely” relations.
* Cannot change legality; can only be used as a heuristic input (ranking) if the neural-usage contract allows it.
* Has TTL / decay score; can be dropped without semantic loss.

C. Shadow (assumed-but-unverified commitments)
This is your “git-like dirty reference” analog.

* Has the *shape* of a committed claim/edge, but is explicitly flagged as “requires verification” and is excluded from “hard reasoning.”
* Shadow nodes can be shown to the user as “Sterling currently assumes X because Y, but it isn’t certified.”
* Promotion from Shadow → Committed requires a verification operator (evidence packet + checks).

D. Provenance artifacts (immutable substrate)

* Raw text spans, parse observations, extraction outputs, evidence bundles.
* Content-addressed, never mutated; references can be relocated/invalidated only by explicit “source mapping updates,” which create new artifacts.

This four-way split matches what you want: you can keep meaning stable while allowing a large periphery to churn.

3. Weak maps and “dirty references” as first-class edges

Model “weak references” the way runtimes model weak pointers: they do not keep the target alive. But you need the semantics in your graph engine:

WeakEdge

* src, dst, label
* evidence_ref (optional)
* decay_score (float)
* last_touched_step
* policy_tag (what created it, and whether it is allowed to influence ranking)
* “activation conditions” (optional): e.g., only becomes visible in WS when a query mentions related anchors

DirtyRef / RelocationRef

* points to a provenance artifact (span, doc version, external KB id)
* has status: {clean, dirty, missing, relocated}
* has a resolver: deterministic mapping strategy (e.g., span relocation by substring match with guardrails, or external entity ID update rules)
* updates are recorded as artifacts with witnesses

This is what gives you the “git-like” behavior: you can know when a link is stale without pretending it is valid.

4. Decay policy as “use it or lose it” reinforcement, but with safety rails

Your sheathing reinforcement idea becomes safer if you make it monotonic in what it can affect:

* Reinforcement can increase or decrease:

  * WS residency (what stays loaded)
  * retrieval priority (what gets rehydrated)
  * frontier candidate ordering (what gets proposed first)
* Reinforcement cannot directly:

  * create committed edges
  * change committed truth conditions
  * alter operator legality

So the decay loop is allowed to prune weak/shadow/hypothesis structures aggressively without risking silent semantic drift.

A practical scoring function for decay that won’t surprise you later:

retention_score(x) =
a * recency(x) +
b * frequency(x) +
c * goal_distance(x) +
d * “certification_level”(x) –
e * neighborhood_cost(x)

Then:

* if score < θ1: evict from WS
* if score < θ2: compress to summary witness node
* if score < θ3: prune (only if weak/frontier; shadow stays but becomes inert and excluded from planning)

5. “Ornamental treatment” needs a formal exclusion boundary

You said: “don’t affect any of the path algebra or relations once decayed.”

Make that literal in the planner interface:

PlannerView = projection(MeaningGraph, include_status={Committed}, optionally include_status={Shadow} only for explanation, never for search)

And add a second view for retrieval/rehydration:

RetrievalView = projection(MeaningGraph, include_status={Committed, Shadow, Weak, Frontier}) but with strict “non-authoritative” tags.

This prevents accidental leakage: no one “accidentally” runs A* over weak edges.

6. Episodes as “A/B state” is the right substrate — formalize it as StateDelta Bundles

What you’re describing is: each reasoning turn yields an EpisodeTrace artifact that includes:

* entry state digest (meaning-state hash)
* exit state digest
* delta patch digest
* evidence packet digest
* operator sequence with impl digests

Then you “salt the path” with the episode hash so you can replay the exact corridor that made state B true.

That maps cleanly to two things you already care about:

* determinism (replay)
* governance (promotion gating)

Mechanically, define:

StateDigest = hash(canonical serialization of Committed subgraph + stable metadata)
EpisodeDigest = hash(StateDigest_in + patch_digest + witness_digest + evidence_digest + operator_impl_digests)

Then “episode shapes” become comparable objects:

* You can A/B evaluate two different operator sequences producing the same goal state.
* You can store multiple EpisodeDeltas from the same entry state and choose which gets promoted into canonical policy.

The subtle but important constraint: you don’t want to fork the committed meaning-state per episode (that’s branching world states); you want to fork *episodes as alternative transition proofs* that can be replayed and compared, while the current meaning-state remains singular and authoritative. Episodes are history, not a multiverse.

7. Reintroduction via shadow nodes: define a “rehydrate to shadow” operator

You said “things can be reintroduced into shadow nodes if they become relevant again.”

Make it explicit:

Operator: RehydrateShadow
Inputs: seed (entity/predicate/proposition), evidence query, budget
Output: create Shadow nodes/edges for the retrieved structure, attach DirtyRefs if provenance has shifted, and emit Frontier hypotheses for verification/promotion.

Then:
Operator: VerifyAndPromote
Consumes: Shadow nodes + evidence packet + checks
Produces: Committed nodes/edges + promotion record

This gives you a disciplined pipeline: relevance brings it back as shadow, not as truth.

8. What to implement now vs later (matches your earlier instinct)

You can get the benefits of your proposal without building the full dynamic system:

Implement now (high leverage, low risk):

* Status types: Committed / Weak / Shadow / Frontier
* PlannerView projection that includes only Committed
* EpisodeTrace artifact: (state_in, state_out, patch, witness, evidence)
* WS eviction (not deletion): keep only a bounded materialized subgraph in memory; everything else is in the store
* “RehydrateShadow” operator as the only reintroduction path

Defer (researchy / easy to overfit):

* Sophisticated decay scoring (start with recency + frequency + cost)
* Automatic relocation of spans across evolving documents (span “git rebase”)
* Neural sheathing as a learned retention policy (start rule-based)

9. The core risk to watch

The main failure mode isn’t graph size; it’s *semantic leakage*:

* weak/shadow edges accidentally influencing search/legality
* rehydration pulling in too much, turning into “load history”
* compaction summaries becoming authoritative without witnesses

Your design already anticipates this, but it’s worth making “projection boundaries” and “promotion-only semantics” the non-negotiable constraints in the code.

If you want a next concrete artifact, I’d write a “Semantic Working Memory Contract v0” that specifies:

* the 4 statuses and their allowed uses
* the two projections (PlannerView vs RetrievalView)
* the EpisodeTrace schema (state digests, patch digests, evidence packet digests)
* the three lifecycle operators: EvictWS, RehydrateShadow, VerifyAndPromote
* a small benchmark harness: “bounded WS, no full reread, identical replay, and weak/shadow never change legality”

Yes let’s create that contract for those specifications. A fully committed myelin sheath should be the canonical node/edge path through the episode. They should serve as fully promoted and invariant, cannot be operated against without changing the meaning or goal essentially treating that as our new State A. We will need a cryptographic like chain that requires us to hold proof of where State A came from as part of this design.

Below is a draft “Semantic Working Memory Contract v0” that you can drop into your docs/canonical set. I’ve written it as a normative contract: definitions, invariants, allowed operations, and acceptance tests. I’ve also integrated your added requirement: a fully committed myelin sheath is the canonical node/edge path through an episode, and the system must maintain a cryptographic chain of provenance such that State A is never “just assumed,” it is derivably bound to the episode chain and evidence packets.

Proposed file: `docs/canonical/semantic_working_memory_contract_v0.md`
Schema IDs introduced (v0):

* `sterling.semantic_working_memory_contract.v0` (doc only)
* `sterling.meaning_state_digest.v0`
* `sterling.episode_trace.v0`
* `sterling.myelin_sheath.v0`

---

SEMANTIC WORKING MEMORY CONTRACT v0

1. Purpose

Sterling operates in semantic space. Language is I/O only. This contract defines how Sterling’s internal meaning-state grows and decays without degenerating into a context window abstraction, while preserving auditability and deterministic replay.

This contract specifies:
(a) A four-status model for graph content (Committed / Shadow / Weak / Frontier).
(b) Projection boundaries (PlannerView vs RetrievalView).
(c) Lifecycle operators for eviction, rehydration to shadow, verification/promotion.
(d) EpisodeTrace artifacts that cryptographically chain State transitions.
(e) “Myelin sheath” promotion: canonical, invariant paths bound to episodes, defining new State A.

2. Core invariants

I1. Meaning-state authority
The Committed graph is Sterling’s authoritative meaning-state. It is not required to mirror the raw string.

I2. Provenance substrate
Raw text and derived observations are retained as immutable provenance artifacts (content-addressed), enabling “show why” through stable references to evidence and spans.

I3. Planning boundary
Only Committed edges/nodes participate in legality and path algebra. Shadow/Weak/Frontier must not change successor legality. They may only affect ranking/prioritization where explicitly permitted by policy.

I4. Determinism under identical inputs
Given identical inputs (same provenance artifacts, same operator implementations, same policy configuration hashes), EpisodeTrace generation and MeaningState digests must be identical.

I5. No silent semantic drift
Changes to Committed meaning-state can occur only through operators that emit (a) a canonical delta patch, and (b) a witness bundle that binds the change to provenance artifacts and precondition checks.

I6. Bounded working set
Materialized in-memory subgraphs are bounded by explicit budgets. Exceeding budgets triggers eviction and/or compaction operations that cannot alter Committed semantics (except via explicit, witnessed transformations).

3. Data model: four statuses

3.1 Committed (authoritative)
Definition: Nodes/edges that participate in reasoning, legality checks, and path algebra.

Requirements:

* Must have a PromotionRecord (or genesis record) that links to evidence packets and witnesses.
* Must be included in MeaningStateDigest computation.
* Must be replayable deterministically.

3.2 Shadow (assumed but unverified)
Definition: Structures that have semantic shape but are explicitly non-authoritative until verified.

Requirements:

* Must be excluded from PlannerView.
* Must carry VerificationRequirements: what checks/evidence are needed for promotion.
* May be presented in explanations as “assumed” with explicit uncertainty.

3.3 Weak (ephemeral / convenience)
Definition: Soft relationships and caches intended to improve retrieval and heuristics, not semantics.

Requirements:

* Must be excluded from PlannerView.
* Must be safe to drop without semantic loss.
* Must carry decay metadata (scores, TTLs, last_touched).

3.4 Frontier (hypotheses / unlit hallways)
Definition: Candidate interpretations and suggested links (coref, discourse, grounding, implicit args, etc.)

Requirements:

* Cannot be referenced by Committed semantics except via PromotionRecords.
* Must include generator provenance and stable candidate ordering.
* May be pruned by policy (top-k per attachment) without changing Committed semantics.

4. Projection boundaries

4.1 PlannerView (authoritative reasoning view)
PlannerView is a projection of the internal IR containing:

* All Committed nodes/edges required for legality and successor generation.
* Optionally minimal provenance handles (IDs only) for explanation linking, never for reasoning.

PlannerView must exclude:

* Shadow nodes/edges
* Weak edges/nodes
* Frontier hypotheses/candidates

Invariant PV-1: Any search/planning algorithm must accept PlannerView only. Attempts to feed non-Committed structures into legality must fail closed.

4.2 RetrievalView (rehydration and explanation view)
RetrievalView may include:

* Committed + Shadow + Weak + Frontier
* Provenance artifacts and indices
* Embedding/lexical retrieval structures

Invariant RV-1: RetrievalView may propose candidates and materialize context, but can only affect Committed semantics through explicit operators producing patches and witnesses.

5. State, episode, and cryptographic chain of custody

5.1 MeaningStateDigest v0
MeaningStateDigest is computed over a canonical serialization of the Committed meaning-state plus critical metadata.

Schema: `sterling.meaning_state_digest.v0`
Fields:

* schema_version
* state_id (hash)
* committed_graph_digest (hash of canonical Committed subgraph)
* policy_digest (retention/projection policy config hash)
* operator_registry_digest (hash of operator_id → operator_impl_digest mappings)
* provenance_root_digest (hash root of the provenance artifact set reachable by PromotionRecords)
* parent_state_id (optional; when chained through episodes)

State digest requirement S-1: state_id = H(committed_graph_digest || policy_digest || operator_registry_digest || provenance_root_digest || parent_state_id)

5.2 EpisodeTrace v0
An EpisodeTrace represents one transition step from State A to State B.

Schema: `sterling.episode_trace.v0`
Fields:

* episode_id (hash)
* state_in: MeaningStateDigestRef
* state_out: MeaningStateDigestRef
* patch_digest (hash of canonical delta patch)
* witness_digest (hash of witness bundle)
* evidence_packet_digest (hash root of referenced evidence artifacts)
* operator_sequence: list of {operator_id, operator_impl_digest, invocation_digest}
* myelin_delta: optional MyelinSheathRef (if a sheath is created/promoted in this episode)
* timestamp/logical_step (optional; do not include in canonical digest unless required)

Episode requirement E-1 (chain): episode_id = H(state_in.state_id || patch_digest || witness_digest || evidence_packet_digest || operator_sequence_digest)

Episode requirement E-2 (anti-forgery): state_out.parent_state_id MUST equal state_in.state_id unless this episode is explicitly a “state reset” episode type (rare, gated, witnessed).

5.3 Proof requirement for “where State A came from”
To treat a State A as authoritative, Sterling must be able to provide:

* The state’s committed_graph_digest
* The chain of episode_ids back to a genesis state (or a certified checkpoint)
* For each episode in the chain: patch + witness + evidence root
  This is the cryptographic chain you requested: State A is not an assertion, it is the head of a verifiable chain.

6. Myelin sheath: canonical promoted paths

6.1 Definition
A Myelin Sheath is a canonicalized, fully promoted node/edge path (and operator corridor) that Sterling treats as invariant for the purposes it was certified for.

The sheath is not “just a cache.” It is a certified corridor: applying it produces the same semantic transition under the same preconditions, and its use is auditable and replayable.

6.2 Semantics

* A fully committed myelin sheath defines a new canonical interpretation of the episode corridor and can be treated as the new “State A baseline” for subsequent reasoning in that domain context.
* Sheath content is immutable once promoted (append-only via new sheaths). Any change to the corridor is a new sheath, not a mutation.

6.3 Constraints
M-1. Sheath immutability: Once promoted, the sheath cannot be “operated against” (edited) without creating a new sheath artifact and therefore a new meaning-state transition.
M-2. Sheath authority boundary: Sheath membership implies Committed status for its nodes/edges and forbids reliance on Shadow/Weak/Frontier elements as part of the corridor definition.
M-3. Sheath provenance: Every sheath must include an evidence root and witness chain demonstrating why each corridor step is legal.

6.4 MyelinSheath schema v0
Schema: `sterling.myelin_sheath.v0`
Fields:

* sheath_id (hash)
* derived_from_episode_id
* entry_state_id
* exit_state_id
* canonical_corridor:

  * node_ids: ordered list of committed node IDs
  * edge_ids: ordered list of committed edge IDs (or stable edge descriptors)
  * operator_steps: ordered list {operator_id, operator_impl_digest, patch_digest, witness_digest}
* preconditions_digest: hash of the sheath’s precondition set
* evidence_root_digest
* scope: (domain tag / world tag / applicability conditions)
* invariants: list of invariant IDs the sheath asserts (e.g., “NEG attaches to prop exactly once”)

Sheath requirement MS-1: sheath_id = H(derived_from_episode_id || entry_state_id || canonical_corridor_digest || preconditions_digest || evidence_root_digest)

6.5 Operational use

* The planner may treat sheath corridors as “certified macros” for search acceleration, but only if:
  (a) The current state satisfies sheath preconditions.
  (b) The operator registry digest matches (no impl drift).
  (c) The evidence root and witnesses are resolvable.
  Otherwise, fail closed: do not apply sheath.

7. Lifecycle operators (v0)

All lifecycle changes must be expressed as operators emitting patches + witnesses and recorded in EpisodeTraces.

7.1 EvictWS (working set eviction)
Purpose: Reduce materialized in-memory WS size without changing Committed semantics.

Inputs:

* budget (node/edge caps)
* retention policy digest
  Outputs:
* Patch that evicts only materialization markers (or WS residency flags), may prune Weak/Frontier caches.
* Witness: budget exceeded, eviction decisions and scoring inputs.

Invariant: EvictWS must not change MeaningStateDigest (Committed-only digest), only WS view.

7.2 RehydrateShadow
Purpose: Reintroduce previously non-materialized context as Shadow structures for potential verification and promotion.

Inputs:

* seeds (node IDs, mention spans, query handles)
* budget (k-hop / cap)
* retrieval policy digest
  Outputs:
* Patch that materializes selected structures as Shadow and Weak, plus Frontier hypotheses as needed.
* Witness that records retrieval seeds, budgets, index digests, and returned set digests.

Invariant: RehydrateShadow must not add Committed edges.

7.3 VerifyAndPromote
Purpose: Promote Shadow/Frontier items into Committed, producing new meaning-state.

Inputs:

* selected hypothesis/candidate IDs
* required evidence packet(s)
* verification checks required by Shadow nodes
  Outputs:
* Patch that adds/updates Committed nodes/edges, retires Shadow/Frontier items, emits PromotionRecord(s), optionally emits MyelinSheath if corridor qualifies.
* Witness includes: precondition checks, evidence roots, operator_impl_digests.

Invariant: VerifyAndPromote is the only allowed path from Shadow/Frontier to Committed.

7.4 CompactToSummary (optional in v0; recommended soon)
Purpose: Reduce graph size by collapsing an old subgraph into a summary node while retaining replay links.

Constraint: Must preserve a reversible pointer to the original artifacts via evidence roots and episode chain references.

8. “Use it or lose it” reinforcement (permitted scope)

Reinforcement scoring may influence only:

* WS residency (what stays loaded)
* Retrieval ordering and candidate ranking in Frontier
* Weak edge retention/pruning
  It must not:
* Create Committed semantics directly
* Change legality/successor generation
* Alter MeaningStateDigest except via VerifyAndPromote

9. Acceptance tests (must-pass)

A. Projection safety

* PlannerView contains only Committed structures.
* Attempts to include Shadow/Weak/Frontier in legality checks fail closed.

B. Chain integrity

* For any EpisodeTrace, state_out.parent_state_id == state_in.state_id.
* episode_id changes if any of: patch, witness, evidence root, operator impl digests change.

C. Deterministic replay

* Replaying EpisodeTrace from state_in yields identical state_out digest.
* Replaying a chain of episodes yields identical head state digest.

D. Myelin sheath immutability and authority

* A promoted sheath’s sheath_id is stable across runs.
* Applying a sheath under satisfied preconditions yields the same patch/state_out.
* Any attempt to “edit” sheath corridor generates a new sheath artifact (new sheath_id) and thus a new meaning-state transition.

E. Bounded WS without reread

* With strict WS budgets, tasks that require older context succeed via RehydrateShadow + VerifyAndPromote without loading entire history.
* Rehydration is bounded by declared budgets (k-hop, cap).

10. Implementation notes (v0 decisions)

* Default committed core remains small. Shadow/Frontier can be large.
* Provenance artifacts are content-addressed and immutable; “relocation” creates a new artifact that references the old one plus a mapping witness.
* Weak edges are explicitly non-authoritative and excluded from PlannerView.
* EpisodeTrace is append-only and forms the audit spine. Myelin sheaths are append-only certified corridors.

---

Addendum: Interpreting “new State A” with myelin

When you say “a fully committed myelin sheath should be the canonical node/edge path through the episode… treating that as our new State A,” the safest interpretation is:

* State A’ is the state whose digest is the head after applying the episode that promoted the sheath, plus a certified reference to the sheath artifact.
* The sheath does not replace the state. It is a certified corridor that becomes part of the state’s provenance root and can be reused as an invariant macro going forward.
* The chain requirement ensures you can always show how State A’ was reached: (genesis/checkpoint) → episode chain → sheath promotion episode → head state.

That satisfies the “cryptographic-like chain” requirement without turning the system into branching worlds by default.

If you want, next I can convert this into (1) a schema_registry entry set (with schema IDs and canonical docs), and (2) concrete JSON schema definitions for `episode_trace.v0`, `meaning_state_digest.v0`, and `myelin_sheath.v0` that enforce the invariants mechanically (required fields, stable ordering constraints, digest field rules).
