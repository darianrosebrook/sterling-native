Here’s the clean philosophy that will keep you from accidentally turning Sterling into “Minecraft-with-extra-steps,” while still letting rigs steadily ratchet Sterling’s real reasoning capacity upward.

Sterling’s boundary separation has one core rule: Sterling owns semantics at the level of contracts and invariants, not at the level of domain objects or domain algorithms. Domains own sensors, object models, raw feeds, and often the implementation tricks that make a primitive performant. Sterling owns the definition of “what it means” for that capability to exist, how it is tested, how it is claimed, and how it composes with other capabilities.

From that, you can derive a stable architecture that scales to dynamic, long-standing integrations (streams, KGs, operator catalogs, schema registries) without hard-baking domain specifics.

1. The boundary: three separations that must remain explicit

A. Data plane vs control plane
The data plane is the shape of information that crosses the boundary (envelopes, deltas, snapshots, KG fragments, operator signatures). It must remain boring and stable.
The control plane is how Sterling and a domain negotiate what they can do (capability claims, schema versions, feature flags, budgets, epochs/streams). It must be explicit and auditable.

If you blur these, you get footguns like “we changed a schema field name and silently changed behavior,” or “the domain started sending extra fields and Sterling started depending on them.”

B. Structural contract vs semantic contract
Structural contract: types, required fields, enums, versioning, monotonic sequencing, determinism requirements at the serialization layer.
Semantic contract: invariants, ordering constraints, monotonicity rules, boundedness, “fail closed” rules, and how uncertainty behaves.

Most teams stop at structural. That’s how you end up domain-coupled: the only thing shared is a JSON shape, but not the meaning. Your rigs are explicitly building semantic contracts (conformance suites), which is the correct move.

C. Domain ontology vs Sterling core ontology
Domains will always have their own class labels, taxonomy depth, and feature vocabularies. Sterling should not internalize those as core concepts. Sterling should instead require that any domain-specific vocabulary be namespaced, versioned, and either (1) treated as opaque tokens, or (2) explicitly aligned via a mapping artifact that has its own governance.

This is where most “domain agnostic” systems quietly die: they start encoding domain classes as first-class concepts in the engine rather than as external vocabularies with alignment layers.

2. Rigs: what they are, and what they are not

A rig is not a dependency. A rig is a certification surface.

A rig exists to do three jobs:

* Produce representative evidence streams (including adversarial edge cases) in a repeatable format.
* Provide adapters that connect a domain implementation to a Sterling-owned contract (the capsule).
* Provide proof artifacts: passing conformance suites, determinism harnesses, drift detectors, and resource-bound guarantees.

The rig does not define the primitive. The capsule does.

That single stance prevents the “leave learnings behind” problem: the learning is not in the Minecraft code; it’s in the capsule contract + tests + fixtures + invariants + proof hashes, which are owned by Sterling. Minecraft merely supplies an implementation and a proving surface.

3. The capability absorption pipeline: how a rig becomes Sterling capability

Think of absorption as a pipeline of artifacts, not a pipeline of code reuse.

Step 0: Identify the primitive boundary
You select a capability you want Sterling to be able to claim: “entity belief maintenance,” “schema-governed KG ingestion,” “operator library ingestion,” “episodic replay,” etc. You write down what Sterling needs from the other side, and what Sterling guarantees if it receives it.

Step 1: Define the capsule (Sterling-owned)
A capsule contains:

* Contract types (domain-agnostic, stable naming)
* Version identifiers and compatibility rules
* Invariants (semantic properties)
* Conformance suite(s) + determinism harnesses
* Optional extensions, each with its own sub-claim

No domain imports. No domain constants. No domain taxonomies. If you find yourself writing `HOSTILE_KINDS` or “Minecraft distance,” you’re already sliding.

Step 2: Define negotiation and capability claims (control plane)
A domain doesn’t just “send data.” It declares what it supports. For example:

* P21-A vs P21-B
* Conservative vs predictive uncertainty semantics
* Presence-risk decomposition supported vs not supported
* Budget parameters (track cap, delta cap, emission hz)
* Schema versions supported

This becomes a `CapabilityDescriptor` + `SchemaDescriptor` object that can be hashed, stored, and used for runtime routing decisions.

Step 3: Build fixtures and prove portability
You already did the right thing: minimum two fixture sets. The deeper point is: fixtures are not examples, they are executable meaning. When a second domain passes the same conformance suite, you have evidence the meaning is domain-agnostic.

Step 4: Domain implements adapter + passes certification in CI
The adapter is the only domain-specific glue Sterling should ever need.
The cert run produces an artifact (or at least a record) that the domain can cite when claiming the primitive.

Step 5: Register the capability claim with evidence
At this point, you want Sterling to be able to answer: “Does this domain implement P21-A?” without reading code.

That implies a registry entry like:

* capability_id: `p21.a`
* contract_version: `p21@1.0`
* conformance_suite_hash: content hash of the testkit suite
* fixtures_hashes: which fixture streams were used
* results_hash: proof artifact or deterministic summary
* budget_declaration: the caps used

This is exactly how you prevent “we think we support it” from becoming tribal knowledge.

Step 6: Runtime: handshake + enforcement + fail-closed
When the system runs:

* Domain announces `(capabilities, schema versions, budgets, epoch)` on connect.
* Sterling enforces the schema version and invariants that are enforceable online (sequence monotonicity, boundedness, fail-closed on malformed events).
* Anything not enforceable online remains enforceable via CI certification plus post-hoc audit sampling.

This turns “long-standing contract” from trust-based to governance-based.

4. Avoiding domain hard-baking: concrete rules that keep you honest

Here are the rules that matter in practice (and the failure modes they prevent):

Rule 1: Sterling contracts never mention domain object models
No “Minecraft entity,” no “camera frame,” no “Kubernetes pod.” Only “evidence item,” “track summary,” “operator signature,” “KG claim.”

Prevents: “the engine only works if the world looks like the first world it learned.”

Rule 2: Domain semantics enter only through declared, injectable components
If “threat classification” differs by domain, it is an injected classifier (or declared extension), not a hardcoded mapping in the primitive.

Prevents: “P21 means ‘hostile mobs’ forever.”

Rule 3: Feature vocabularies must be namespaced and treated as opaque by default
If evidence items carry `features`, Sterling should treat them as opaque payload unless an explicit extension says otherwise. Any feature used for semantics must be declared by schema + invariant tests.

Prevents: “one team starts relying on `fuse_state` and now the primitive is secretly Minecraft-shaped again.”

Rule 4: Any semantic strengthening must be introduced as an extension capability
Base primitive stays minimal; richer semantics become optional extensions with their own invariants and sub-claims.

Prevents: breaking contracts when you discover a new domain needs different semantics (your current Pivot 1 situation is exactly this).

Rule 5: Contract drift must be caught structurally and semantically
You already implemented structural drift detection (YAML ↔ capsule). Keep it.
Also add semantic drift detection where it matters (fixtures + determinism + invariants).

Prevents: silent meaning shifts masked by compatible JSON.

Rule 6: Online enforcement is fail-closed; offline enforcement is cert-based
If a required invariant can be enforced at runtime (sequence order, required track on `new_threat`, schema required fields), enforce it and drop data when violated.
If it cannot be fully enforced online (determinism across runs), certify it and store proof.

Prevents: “garbage in” poisoning cognition.

5. Dynamic, long-standing contracts: how to do KGs, operators, schemas without becoming brittle

This is the part most systems under-design. The trick is to treat “dynamic” as “versioned streams of artifacts,” not as ad-hoc runtime coupling.

A. Schema ingestion (long-standing compatibility)
You want a schema registry model:

* Every message references a `schema_id` (or contract version) and optionally a `schema_hash`.
* Schema evolution rules are explicit: additive fields are okay; semantic changes require major version bump; removal requires deprecation window.
* Domains declare supported versions; Sterling chooses one or refuses (fail-closed).

If you also want codegen later, fine—but governance first.

B. Operator ingestion (capability expansion without hard-baking)
An operator catalog should be treated like a plugin interface with verification:

* Operators are declared by signature: input types, output types, preconditions, effects, cost model, and determinism class.
* Operators are not trusted because they exist; they are trusted because they pass an operator conformance harness (replayable scenarios, invariants, bounded resource use).
* Sterling composes operators by signature and declared semantics, not by domain identity.

This gives you “Sterling can reason with operators from any domain” while still being able to say “these operators were certified.”

C. KG ingestion (knowledge without ontology lock-in)
For KG ingestion, the important distinction is:

* KG transport format (Sterling-owned): nodes/edges/claims with provenance, confidence, timestamps, and namespaced identifiers.
* Ontology alignment artifacts (domain-owned, Sterling-governed): mapping between domain predicates/types and Sterling’s core predicate set (if any), versioned and hash-addressed.

If you skip alignment governance, you’ll either (1) hard-bake domain predicates into Sterling, or (2) have a pile of opaque facts that can’t be composed.

A workable pattern:

* Sterling core uses a small set of universal meta-predicates (e.g., `is-a`, `part-of`, `located-in`, `causes`, `observed-at`, `has-attribute`) plus arbitrary namespaced predicates.
* Reasoning that depends on namespaced predicates is always mediated by an explicit alignment artifact or a domain-specific reasoning module, which itself can be certified.

6. A practical “contract shape” template you can reuse across primitives

If you want a repeatable way to design these boundaries, use the same template every time:

* Contract types: stable, minimal, domain-agnostic
* Message envelope: `{request_version, capability_id, bot_id/domain_id, stream_id, epoch, seq, tick_id/time, payload}`
* Capability descriptor: `{capability_id, contract_version, supported_extensions, budgets, determinism_class}`
* Schema descriptor: `{schema_id, schema_version, schema_hash}`
* Conformance suites: base + extension suites
* Proof artifacts: result hashes, fixture hashes, suite hash

This turns “dynamic long-standing contract” into “negotiated versioned streams plus auditable proofs,” which is the only scalable approach.

7. The key meta-principle

When you feel the urge to “absorb” code from a rig into Sterling, translate that urge into: “what is the invariant we learned, and how do we encode it as a contract + test + proof?” Code can remain in the domain as long as the meaning is standardized and certifiable.

That’s how you keep Sterling domain-agnostic while still accumulating real capability.

If you want the next step, I’d formalize a single “Capability Descriptor + Proof” object in Sterling that P21 can be the first consumer of, then reuse that pattern for KG/operator/schema ingestion. P21 becomes the reference implementation of the absorption pipeline itself, not just a one-off primitive.
