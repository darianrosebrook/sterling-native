---
date: 2026-02-23
authority: architecture
author: "@darianrosebrook"
status: "Design Document (not a commitment — a compass)"
---
# Sterling v2: Clean-Sheet Architecture

---

## Premise

Sterling is 543 Python files across ~40 packages. The *ideas* are excellent — the invariants, the operator contracts, the search-not-prediction thesis. But the implementation grew organically, with each exploration leaving sediment. This document asks: **if we rebuilt from what we know now, what would we keep, what would we kill, and what would we change?**

This is not a refactoring plan. It's a target architecture that informs incremental consolidation decisions.

---

## What Sterling Got Right (Preserve These)

These are the decisions that survived contact with implementation and should be carried forward unchanged.

### The Core Thesis

Reasoning as graph search over semantic state space. LLMs as I/O codecs only. This is the right bet. The north star analogy (Wikipedia Pathfinder) is the clearest explanation of what Sterling does and should remain the mental model.

### The 11 Invariants (INV-CORE-01 through 11)

| ID | Constraint | Verdict |
|----|-----------|---------|
| INV-CORE-01 | No free-form CoT | Keep. Proven by H3.2 |
| INV-CORE-02 | Explicit state | Keep. UtteranceState + KG |
| INV-CORE-03 | Structural memory | Keep. Episode summaries + path algebra |
| INV-CORE-04 | No phrase routing | Keep. Scored search only |
| INV-CORE-05 | Computed bridges | Keep. Runtime domain transitions |
| INV-CORE-06 | Contract signatures | Keep. Typed operator contracts |
| INV-CORE-07 | Explicit bridge costs | Keep. Hysteresis-aware |
| INV-CORE-08 | No hidden routers | Keep. Auditable StateGraph |
| INV-CORE-09 | Oracle separation | Keep. No future knowledge |
| INV-CORE-10 | Value target contract | Keep. Versioned + hash-verified |
| INV-CORE-11 | Sealed external interface | Keep. Governed operators only |

All 11 are load-bearing. None are vestigial.

### Operator Taxonomy (S/M/P/K/C)

The five-category operator classification (Seek, Memorize, Perceive, Knowledge, Control) is clean and provides the right granularity for contract enforcement. Keep.

### Append-Only StateGraph

Immutable search trace is the right design for auditability and replay. Keep.

**Authority clarification**: ByteTrace is the canonical persisted trace for replay verification and certification (see ADR 0002). StateGraph is a deterministic derived view rendered from ByteTrace — useful for visualization and debugging, but not the replay spine.

### Value Function Protocol

Separating structural features from learned scoring, with a hybrid combiner. Clean abstraction boundary. Keep.

### Witness-First Validation

Requiring evidence bundles before asserting quality. This is the governance discipline that makes Sterling trustworthy. Keep.

### TD-12 Certification

Step and episode certificates with cryptographic evidence. The right primitive for proof-carrying artifacts. Keep.

---

## What We'd Change

### 1. The Compilation Boundary Is the Architecture

The most important architectural decision is not "use Rust" — it's the **compilation boundary** between dynamic authoring and frozen runtime.

```
compile(payload, schema_descriptor, registry_snapshot) → ByteState
```

This is a pure function: equal inputs produce identical bytes. The carrier (ByteState) is the **only representation the inner loop computes on, hashes, diffs, and replays**. Everything else (domain payloads, fixture specs, operator catalogs, policies) is a versioned authoring representation that lives outside the loop.

**Why this matters architecturally**:
- **No runtime mutation of the substrate.** Epoch transitions happen between episodes. Schema/registry evolution is promoted, not patched at runtime. This forces induction, new operators, and new domains to become governed artifacts — not magical runtime expansion.
- **Fail closed on mismatch.** Schema/registry/constraint violations produce typed failures, not degraded "best effort."
- **Determinism is a contract surface, not a nice-to-have.** Same inputs → same bytes → same digest. This is what makes replay verification and promotion gates meaningful.

This boundary is the single biggest sprawl killer Sterling has. It forces governance to work by making the wrong thing structurally impossible.

### 2. Language: Rust Core + Python Bindings

Rust is a *consequence* of wanting the compilation boundary to be fast and correct, not the primary motivation.

**Proposed split**:

| Layer | Language | Rationale |
|-------|----------|-----------|
| Carrier (ByteState/ByteTrace), compilation boundary, hashing, search loop, operator dispatch | **Rust** | The sealed kernel. ~15k LOC. Ownership model enforces immutability at compile time. |
| PyO3 bindings | **Rust/Python bridge** | Expose core to Python ecosystem |
| Value function training, IR parsing (transformers), surface generation | **Python** | PyTorch/HF are Python-native. ~5k LOC. |

**What stays Python**: Everything that touches PyTorch, HuggingFace, or spaCy. The ML pipeline, the IR parser, the surface realizer. Scenario harnesses, fixture generation, report formatting.

**What moves to Rust**: Carrier operations, frontier management, batch scoring inputs, deterministic replay verification, canonical hashing, the minimal StateGraph append-only trace writer.

**Why not C++ or Zig**: Rust's ownership model maps directly to Sterling's append-only, content-addressed design. The borrow checker enforces immutability invariants at compile time that we currently enforce by convention.

**The kernel's public API is small and serialization-based**:
- Inputs: `(policy_snapshot, payload_bytes, schema_descriptor, registry_snapshot)`
- Outputs: `(outcome_rows, replay_trace, metrics_bundle, tool_transcript)`

Python is never the semantic substrate. It is a convenient control plane while the authoritative computation and evidence surfaces live in Rust + the carrier.

### 3. Layer Structure: 7 Modules, Not 40 Packages

Current Sterling has ~40 top-level packages under `core/`. Most of the sprawl comes from treating every concern as its own package. Redesign into 7 clear layers with one-way dependencies:

```
sterling/
├── carrier/        # Layer 0: The SUBSTRATE (sealed, authoritative)
│   ├── bytestate    # ByteStateV1 fixed layout
│   ├── bytetrace    # ByteTraceV1 evidence format
│   ├── code32       # Code32 ↔ ConceptID bijection per epoch
│   ├── compile      # The compilation boundary (pure function)
│   └── schema       # Schema descriptors, registry snapshots
│
├── state/          # Layer 1: What things ARE
│   ├── ir           # ONE IR schema (unify the 5 current variants)
│   ├── utterance    # UtteranceState
│   ├── world        # WorldState
│   ├── graph        # StateGraph (append-only search trace)
│   ├── node         # SearchNode (utterance + world + metadata)
│   └── memory       # SWM (bounded activation, decay, landmarks)
│
├── operators/      # Layer 2: What things DO
│   ├── signature    # OperatorSignature (typed contracts)
│   ├── registry     # OperatorRegistry (S/M/P/K/C dispatch)
│   ├── builtin/     # Built-in operators (seek, memorize, perceive, etc.)
│   └── induced/     # Runtime-learned operators (with promotion pipeline)
│
├── search/         # Layer 3: How to NAVIGATE
│   ├── engine       # Core search loop (one implementation)
│   ├── value        # Value function protocol + hybrid scorer
│   ├── budget       # Step budget, domain-crossing costs
│   └── episode      # Episode recording, credit assignment
│
├── proof/          # Layer 4: How to TRUST
│   ├── certificate  # TD-12 certificates (step + episode)
│   ├── hash         # Canonical hashing (ONE implementation)
│   ├── witness      # Evidence bundles, fence witnesses
│   ├── provenance   # Chain-of-custody tracking
│   └── governance   # Policy object (DEV/CERTIFIED modes), verdict bundling
│
├── worlds/         # Domain adapters (pluggable, through harness)
│   ├── harness      # Unified World Harness (see §8)
│   ├── adapter      # WorldAdapter trait/protocol
│   ├── metaplan     # Cross-domain composition (see §9)
│   ├── discourse    # Linguistic reasoning
│   ├── wordnet      # Lexical graph
│   ├── pn           # Predicate nominal (ONE location, not 8)
│   └── ...          # Other domains as needed
│
└── ml/             # Python-side ML (advisory-only, non-authoritative)
    ├── ir_parser    # LLM → IR intake
    ├── realizer     # State → NL surface generation
    ├── value_trainer# Train value function from episodes
    └── encoder      # Optional latent compression
```

**Dependency direction**: `carrier ← state ← operators ← search ← proof ← worlds ← ml`. One-way. No world-specific imports in the sealed layers. `ml/` can only emit scores over already-legal transitions — there is literally no method a neural module can call that mutates state.

The goal is not "fewer files" — it's **single canonical ownership per semantic surface**. One IR, one hashing implementation, one search loop, one operator contract representation, one carrier. Everything else becomes views or adapters.

#### Key Consolidations

| Current State | Problem | Proposed |
|---------------|---------|----------|
| 5 IR schemas (`ir/`, `linguistics/ir_v0/`, `text/ir.py`, `pn/ir_builder.py`, `pseudocode/ir.py`) | No canonical IR | One IR schema with domain-specific tagged extensions |
| 7+ hashing/canonicalization implementations scattered across packages | Ownership unclear | `proof/hash` — one module, one truth |
| 3 reasoning loop implementations (`loop/`, `instrumented/core.py`, `search.py`) | Duplication, drift risk | One search engine; instrumentation via compile-time feature flag |
| PN code in 8 packages | Scattered domain logic | One file in `worlds/pn` |
| `memory/verifier_engine.py` (2,600 LOC doing proof work) | Wrong package | Verification → `proof/`; memory decay → `state/memory` |
| `worlds/` vs `domains/` vs `capsules/` | Unclear distinction | Just `worlds/` |

### 4. Collapse the Induction Pipeline

**Problem**: 120 files (38% of the codebase) for one pipeline. The induction subsystem is doing important work — learning new operators at runtime — but it's over-factored into per-stage, per-report, per-baseline, per-scoring-variant files.

**Proposed**: 5 modules with explicit extension points:

```
operators/induced/
├── propose      # Stage 1: Generate operator candidates from episodes
├── evaluate     # Stage 2: Pluggable evaluators (MDL, replay holdout, falsifier suites)
├── promote      # Stage 3: Produce governed promotion proposal (same artifact schema every time)
├── store        # Content-addressed persistent storage + index
└── report       # Pure view layer over promotion artifacts (optional, never authoritative)
```

The key design choice: **evaluators are the extension point**. Adding a new scoring method (MDL variant, new replay strategy, domain-specific falsifier) means registering a new evaluator, not rewiring the pipeline. The `evaluate` module defines the evaluator protocol; implementations can live in the module or be contributed by worlds.

The `report` module is explicitly non-authoritative — it renders what `promote` already decided. This prevents the current pattern where 4,682-line report files accumulate decision logic that should live in the promotion stage.

The current sub-stage granularity (separate files for proposal, refinement, scoring, decision, lifecycle, stage reports, baselines, synthesizers, orchestrators) can collapse into configuration on a smaller set of well-designed types. The 120-file split was useful for exploration; it's not useful for maintenance.

### 5. Dependency Diet

**Problem**: ~130 dependencies including audio processing (Whisper, PyAudio, espeak), ONNX runtime, Docker, image processing (Pillow), web servers (FastAPI, Gunicorn, uvicorn), and RDF (rdflib). Most are not core to Sterling's thesis.

**Proposed tiering**:

```
[core]                        # Required for Sterling to function
  petgraph (or Rust equivalent)  # Graph operations (replaces NetworkX)
  blake3                         # Fast hashing (replaces xxhash + custom)
  serde                          # Serialization

[ml]                          # Required for neural value function + IR parsing
  torch >= 2.0
  transformers >= 4.0

[nlp]                         # Only if doing linguistic domain work
  spacy >= 3.8
  nltk >= 3.9

[audio]                       # Only if doing phonology domain
  whisper, pyaudio, espeak

[serve]                       # Only if running as a service
  fastapi, uvicorn

[dev]                         # Development only
  pytest, ruff, mypy, hypothesis
```

**Drop from default install**: audio processing, ONNX, Docker, image processing, web server stack, RDF, pandas/pyarrow (use Rust serde instead), Textual/Rich TUI.

A clean Sterling install should have ~15 dependencies, not ~130.

### 6. One Flagship + Orthogonal Truth Regimes, Not 15 Partial Demos

**Problem**: 15 test-scenarios, many incomplete (some have docs but no tests). Effort spread thin.

A single flagship demo is necessary for narrative clarity but **insufficient for governance**. One scenario risks overfitting the engine's core semantics to one truth regime. The existing capability axes work already proved this — Mastermind, Slippery Gridworld, and Poisoned Curriculum each broke something different that a single demo wouldn't have caught.

**Proposed**: 1 flagship for narrative + 3-5 minimal truth-regime scenarios for certification coverage.

**Flagship: "Wikipedia Pathfinder"** — Given a start concept and a target concept in a knowledge graph, find a reasoning path using Sterling's search. This directly embodies the north star analogy.

| Layer | Exercise |
|-------|----------|
| State | KG nodes as WorldState, query as UtteranceState |
| Operators | Navigate (follow edge), Expand (load neighbors), Memorize (mark landmark), Bridge (cross domain) |
| Search | A* with learned value function over graph features |
| Proof | Every path step certified, episode provenance tracked |
| Learning | After N episodes, value function improves, landmarks emerge |
| Memory | SWM bounds activation; dead-end signatures prune explored regions |

**Truth-regime scenarios** (each exists to break a different failure mode):

| Scenario | Pressure Axis | What It Catches |
|----------|---------------|-----------------|
| WordNet/Rome corridor | Deterministic symbolic navigation | Audit surface stability, multi-domain cert progression |
| Mastermind | Partial observability | Belief discipline (belief set size must decrease monotonically after probes) |
| Slippery Gridworld | Stochasticity | Certification binds to recorded evidence, not environment; distributional evidence over seed sets |
| Transactional KV Store | Safe tool execution | Plan/apply/verify/rollback; tool transcripts as evidence artifacts |
| Poisoned Curriculum | Adversarial robustness | Quarantine/revocation of the learning loop itself |

Everything else becomes either a sub-testkit inside one of these worlds, or a D0/D1 experiment that stays explicitly non-promotable until it proves a distinct axis. 15 demos with 5 tests each → 6 scenarios with 20-50 solid tests each.

### 7. Simplify Governance: Centralize Ownership, Two Modes

**Problem**: 3+ governance modes, operator promotion lanes, gate verdicts, witness generation, certification contracts, run intents, enforcement modes, 10 theory conformance checks — scattered across `governance/`, `contracts/`, `certification/`, `safeguards/`.

The problem is not that modes exist — it's that mode semantics are scattered across many packages. Removing dev mode entirely would just cause people to work around governance, which is worse than a lighter version of it.

**Proposed**: Keep the *outcomes*, centralize the *ownership*, reduce to **two modes**.

| Keep | Why |
|------|-----|
| Operator contracts (signatures, preconditions, effects) | Core to INV-CORE-06 |
| TD-12 certificates | Core to proof-carrying artifacts |
| Append-only provenance | Core to auditability |
| Witness-first validation | Core to evidence discipline |

| Change | How |
|--------|-----|
| 4 RunIntent modes → 2 (DEV, CERTIFIED) | DEV records the same artifacts but doesn't block on failures. CERTIFIED is fail-closed. No PROMOTION/REPLAY as separate modes — those are operations within CERTIFIED. |
| Governance scattered across 4+ packages → `proof/governance` | One location owns policy. One policy object, passed explicitly, recorded into every artifact. |
| RunIntent + GovernanceContext + GateVerdict → one PolicyContext | The concepts are right but the type hierarchy is deep. Flatten to: a policy mode, a set of gate results, and a witness trail. |

### 8. Unified World Harness

**Problem**: Each new domain reinvents its own proof plumbing — fixture loading, evidence emission, negative controls, determinism checks. This is the primary source of per-domain sprawl.

**Proposed**: A **Unified World Harness** that standardizes the contract every world must satisfy. All worlds become harness configurations, not standalone runners.

The harness standardizes:
- **WorldStep records**: observation → action → transition witness (every step, every world)
- **EvidenceEmitter interface**: one way to emit bound inputs, outcome rows, replay traces, metrics bundles, tool transcripts
- **Shared artifact schemas**: the same governed types regardless of domain
- **Cert-mode behavior**: fail-closed on missing fixtures, swallowed errors forbidden, negative controls mandatory

Worlds provide: payload schema, compiler/decompiler (to/from ByteState), operator catalog (as data), and measurement hooks. They do **not** get to invent their own hashing, canonicalization, trace semantics, or evidence formats.

This is the "sprawl killer" engineering move. It turns "add a new domain" from "build a bespoke pipeline" into "configure the harness and define your operators."

### 9. MetaPlan as First-Class World (Early, Not Late)

**Problem**: Our doc describes worlds as pluggable adapters but doesn't address how domains *compose*. Without an explicit composition mechanism, cross-domain work becomes a hidden router — violating INV-CORE-08.

**Proposed**: Build MetaPlan early as the standard mechanism for cross-domain agentic work.

MetaPlan treats plan-as-state: every "select domain," "probe," "commit step," "replan," and "stop" is an operator application recorded in the StateGraph. This is how INV-CORE-08 (no hidden routers) works across domains.

| MetaPlan Concept | Implementation |
|------------------|----------------|
| Intake | Discourse (or any world) turns input into GoalSpec + constraints |
| Planning | MetaPlanWorldAdapter exposes plan steps, capability snapshots, probes, stop conditions as domain objects |
| Decision | Domain/tool selection is scored search over declared capabilities, not phrase matching |
| Success | A proof (certified path through the plan graph) |
| Failure | A minimal typed counter-proof with evidence (not "it gave up") |

MetaPlan is not "another world adapter." It's the mechanism that prevents Sterling from needing a router. Build it in Phase 2, not Phase 5.

### 10. Neural Components: Advisory by API Shape, Not Convention

**Problem**: "LLM as codec" is stated as philosophy but not enforced at the API level.

**Proposed**: The `ml/` module's API is designed so that **the wrong thing is unrepresentable**. Neural components can:
- Propose IR parses (text → structured representation)
- Emit scores over already-legal transitions
- Generate surface text from state
- Produce embeddings for indexing

Neural components **cannot**:
- Call any method that mutates state
- Create operators
- Bypass preconditions
- Directly write to the StateGraph

This is not a linting rule or a convention — the sealed kernel simply does not expose mutation methods to the ML layer. The API boundary enforces INV-CORE-01 (no free-form CoT) and INV-CORE-11 (sealed external interface) structurally.

### 11. Two New Invariants

**INV-CORE-12: Single Source of Truth**

Every concept (IR, hashing, operator dispatch, state representation) has exactly one canonical implementation. No shadow implementations, no "instrumented variant," no "v0/v1" coexistence. No `*-enhanced.*`, `*-new.*`, `*-v2.*` files.

This is the constraint that prevents the sprawl from recurring.

**INV-CORE-13: Neural Advisory Only**

Neural components can rank, parse, compress, and generate — but can never create operators, bypass preconditions, or mutate committed state. Enforced by API shape: the kernel exposes no mutation methods to the ML layer.

---

## Summary: What Changes

| Aspect | Current | Proposed |
|--------|---------|----------|
| Central architectural decision | Implicit (Python objects are runtime truth) | Compilation boundary: `compile() → ByteState` is the only runtime truth |
| Language | Pure Python (543 files) | Rust sealed kernel + Python ML/orchestration layer |
| Packages | ~40 top-level under `core/` | 7 modules (carrier, state, operators, search, proof, worlds, ml) |
| IR schemas | 5+ variants | 1 canonical IR with tagged extensions |
| Search implementations | 3 (loop, instrumented, search) | 1 (instrumentation via feature flag) |
| Induction files | 120 | 5 modules with evaluators as extension point |
| Hash/canon implementations | 7+ scattered | 1 in `proof/hash` |
| Dependencies | ~130 | ~15 core + optional domain extras |
| Demos | 15 (many incomplete) | 1 flagship + 5 truth-regime scenarios |
| Per-world proof plumbing | Bespoke per domain | Unified World Harness (one contract, all worlds) |
| Cross-domain composition | Ad hoc | MetaPlan as first-class world (early build target) |
| PN locations | 8 packages | 1 world adapter |
| Governance modes | 4 (DEV/CERTIFYING/PROMOTION/REPLAY) | 2 (DEV/CERTIFIED), one policy object, one location |
| Neural boundary | Convention ("LLMs are codecs") | API-enforced (kernel exposes no mutation to ML layer) |
| Invariants | 11 | 13 (add Single Source of Truth + Neural Advisory Only) |
| LOC estimate | ~80k Python | ~15k Rust + ~5k Python |

## What This Preserves

- All 11 original invariants (INV-CORE-01 through 11)
- Operator taxonomy (S/M/P/K/C)
- Typed signatures with contracts
- Append-only StateGraph
- Value function protocol (structural + learned hybrid)
- Witness-first validation
- Episode-based credit assignment
- SWM with bounded activation
- Domain adapter pattern (WorldAdapter)
- LLM-as-codec philosophy (now enforced by API shape)
- TD-12 certification
- Promotion lane concept (certify → promote → freeze → replay)
- Dev/research iteration mode (lighter, but still auditable)
- Orthogonal capability-axis validation (proven by truth-regime suite)

## What This Kills

- The notion that Sterling needs 543 files to express itself
- Multiple IR schemas coexisting
- Induction sprawl (120 files for one pipeline)
- Scattered hashing/canonicalization implementations
- 15 half-finished demos with bespoke proof plumbing each
- 130 Python dependencies in one flat requirements.txt
- The "instrumented variant" pattern (instrument the real code)
- `worlds/` vs `domains/` vs `capsules/` distinctions nobody can explain
- 4 governance modes scattered across 4+ packages
- v0/v1 coexistence of any abstraction
- Neural components that could theoretically mutate state (convention → API impossibility)
- Ad hoc cross-domain routing (MetaPlan replaces hidden routers)

---

## Migration Path (If We Ever Do This)

This is not a rewrite-from-scratch proposal. The critical insight: **define boundaries first, port modules second.** A "v2 that never ships" is worse than incremental consolidation.

### Phase 1: Define the compilation boundary in Python

Define the minimal kernel interface (`compile() → ByteState`) as a Python abstraction, even if the kernel is still Python internally. Make the current search loop call through this interface. This is the architectural move — everything after is implementation.

### Phase 2: Carrier + determinism locks

Implement `carrier/` as a self-contained module with the compilation boundary, canonical hashing, and deterministic replay verification. This is the foundation everything else depends on ("same input → same bytes → same digest").

### Phase 3: Unified World Harness + migrate 2-3 worlds

Stand up the harness. Migrate WordNet/Rome + Mastermind through it. This immediately proves: deterministic replay, multi-domain evidence, and partial observability in one coherent pipeline. Every subsequent world inherits the proof plumbing for free.

### Phase 4: Consolidate packages within Python

Merge the 40 packages into the 7-module structure. This is pure refactoring with no language change. Most of the value (reduced sprawl, clear ownership) comes from this step alone.

### Phase 5: MetaPlan as governed cross-domain composition

Build MetaPlan against the harness so cross-domain "agency" is governed search, not ad hoc orchestration.

### Phase 6: Extract sealed kernel to Rust

Port `carrier/` first (smallest, most self-contained, biggest perf impact per LOC). Then `proof/hash`, then the search loop. Use PyO3 to keep the Python API stable. At each step, the Python version remains the fallback.

### Phase 7: Tier dependencies + consolidate demos

Move optional deps to extras. Promote the flagship + 5 truth regimes; retire everything else to `archive/` (not deleted).

Each phase is independently valuable and independently shippable. The ordering matters: boundaries before ports, harness before worlds, carrier before search.

---

## Anti-Patterns to Avoid

These emerged from cross-referencing multiple redesign proposals:

1. **Don't rewrite first, boundary first.** Define the compilation boundary and kernel interface in current Python, then port modules. "Start over in Rust" is how v2 never ships.

2. **Don't build memory before evidence surfaces exist.** Carrier → engine → harness → memory. If you build memory first, you learn in ways you can't audit.

3. **Don't treat "fewer files" as the goal.** The goal is single canonical ownership per semantic surface. You could have 7 modules and still have sprawl if two of them hash differently.

4. **Don't let neural components call mutating methods, even in dev mode.** The API shape should make the wrong thing unrepresentable. Convention-based enforcement erodes.

5. **Don't remove dev mode entirely.** People will work around governance rather than through it. A lighter mode that still records artifacts is better than no mode at all.

---

## Open Questions

1. **Is Rust the right choice, or is the performance ceiling achievable with Cython/mypyc on the existing Python?** Rust gives compile-time invariant enforcement which aligns with Sterling's philosophy, but the migration cost is real. The compilation boundary design works regardless of implementation language.

2. **How much of the induction pipeline's complexity is *essential* complexity (the domain is hard) vs *accidental* complexity (it grew file-by-file)?** The proposal to collapse 120 files to 5 modules assumes mostly accidental. That assumption needs validation by examining which evaluator variants are genuinely distinct.

3. **Should the flagship demo be Wikipedia Pathfinder (pure graph search) or a linguistic task (PN reasoning)?** The former is more universal; the latter has more existing infrastructure. The truth-regime suite partially decouples this choice from certification coverage.

4. **What's the right boundary between `proof/` and `operators/induced/`?** Promotion certification currently spans both concerns. The harness may resolve this: `operators/induced/promote` produces a proposal; `proof/governance` evaluates it.

5. **When does `state/memory` graduate to its own module?** If path algebra and compression-gated landmarks become central to Sterling's learning story (as the north star suggests), memory earns top-level status. For now, keep it as a submodule with the expectation it may promote.

---

## Relationship to Other Documents

- **[North Star](../reference/historical/north_star.md)** — The thesis this architecture serves *(advisory)*

This document does not replace the 2026 roadmap. The roadmap describes what to build *next* within the current architecture. This document describes what the architecture *should be* if we were starting clean. The two inform each other: the roadmap's friction points validate this document's proposed changes; this document's target structure guides the roadmap's consolidation decisions.
