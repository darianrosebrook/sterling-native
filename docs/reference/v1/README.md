# Sterling – Semantic Tree Embedding & Reasoning for Linguistic Neuro-Symbolic Graphs

**Version:** 1.3.1  
**Last Updated:** February 14, 2026
**Status:** Research Prototype (Proof-of-Concept)

## Sterling is NOT a Language Model

<!-- BEGIN CANONICAL: north_star -->
Sterling is a **path-finding system over semantic state space**, where:

| Concept | Definition |
|---------|------------|
| **Nodes** | Meaningful states (UtteranceState, WorldState, summaries/landmarks) |
| **Edges** | Typed moves (operators) |
| **Learning** | Path-level credit assignment (what edges help from what regions) |
| **Memory** | Compression-gated landmarks + durable provenance |
| **Language** | I/O, not cognition (IR intake + explanation only) |

Sterling explicitly tries to **replace LLMs as the reasoning substrate** (the cognitive core / long-horizon state machine). Sterling does NOT try to replace LLMs at **language generation** (surface form production). Transformers handle the translation between symbols and sentences, but the semantic navigation - the actual reasoning - happens in the graph.

See [docs/canonical/north_star.md](docs/canonical/north_star.md) for the full operational definition.
<!-- END CANONICAL: north_star -->

## Introduction

**Sterling** is a neurosymbolic reasoning engine that explicitly models language understanding as a structured graph search problem. Instead of relying on a massive opaque LLM to implicitly "know" everything, Sterling breaks down language interpretation into **symbolic structures and operations** guided by small neural components. The goal is to achieve useful, explainable reasoning in a narrow domain while avoiding hallucination and maintaining an audit trail of how conclusions are reached.

**Sterling’s Goal:** Build a domain-agnostic reasoning engine that:

* **Structures semantics in an IR:** Represents linguistic content as a structured **UtteranceState** (tree/graph with layered syntax, semantics, pragmatics, etc.).
* **Explores interpretations via graph search:** Traverses a **state graph** of possible interpretations using typed operators (S/M/P/K/C for different reasoning moves).
* **Compresses meaning into latents:** Optionally compresses semantic content into compact **latent codes** for efficient retrieval and value estimation (enabling on-device and learned components).
* **Ensures auditable inference:** Keeps all reasoning steps as **symbolic, inspectable inferences** (no hidden chain-of-thought, preventing hallucination in the reasoning process).
* **Adapts to multiple domains:** Allows pluggable **“world” interfaces** (domain knowledge graphs, constraints, reward signals) so the same core engine can operate across different domains.

Sterling emphasizes the following key ideas:

* **Explicit Structure:** All intermediate reasoning is done over a structured **Intermediate Representation (IR)** and a knowledge graph, rather than free-form text.
* **Small Neural Models:** Uses relatively tiny neural networks (tens of millions of parameters, not billions) as encoders/decoders to handle language surfaces, rather than as the central reasoning engine.
* **On-Device Operation:** Targets **Apple Silicon** GPUs/Neural Engine via MPS/ML Compute (and CoreML for mobile) to run entirely on-device, demonstrating efficient local reasoning without cloud services.
* **Semantic Compression:** Introduces **discrete semantic codes** (RGBA token grids) to compress complex representations into fixed-size vectors, instead of handling long text sequences.
* **Learned Heuristics:** Trains lightweight neural **value functions** on past reasoning episodes to guide search more efficiently, while still following symbolic rules for correctness.

## Sterling Core vs. Light vs. Full

Sterling is organized in a layered architecture, with different versions building on the same core:

| **Layer**          | **Description**                                         | **Focus**                                                                                                                              |
| ------------------ | ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| **Sterling Core**  | Domain-agnostic reasoning engine (IR + search)          | _UtteranceState IR, S/M/P/K/C operators, StateGraph search, pluggable world interface, latent rep._                                    |
| **Sterling Light** | Core + a linguistic micro-world (CAWS) + symbolic rules | _Proof-of-concept in a constrained domain (linguistic reasoning) with minimal ML components._                                          |
| **Sterling Full**  | Light + latent compression + on-device deployment       | _Adds **RGBA** latent encoding and a ViT-based compressor, enabling Apple Silicon (GPU/ANE) execution and mobile (CoreML) deployment._ |

* **Sterling Core** is the theoretical foundation: it defines the IR structures, operator taxonomy, search algorithm, and interfaces for domain knowledge. It is domain-agnostic.
* **Sterling Light** is the first implementation of the Core applied to a small **linguistics domain** (called CAWS) with a symbolic knowledge graph and rules. It aims to prove the concept with as little machine learning as possible. The focus is on correctness, interpretability, and ensuring the symbolic system works end-to-end.
* **Sterling Full** will extend the Light version by introducing a **semantic compression layer** (to condense the IR into a compact latent) and deploying the models on-device. This version will use a **Vision Transformer (ViT)** to encode IR tokens (converted to RGBA pixels) into a vector, and run the encoder/decoder on Apple GPUs or Neural Engines. The Full version is about efficiency and portability, while keeping the same core logic and guarantees as Light.

_(For formal definitions of concepts like UtteranceState, operator types, value functions, see the theory reference in `docs/theory/reasoningframework.md`.)_

## Key Design Principles

Sterling’s design is fundamentally different from the standard "LLM + retrieval" approach. It commits to a **graph-based, transparent reasoning process** with strict guarantees. Some contrasting points:

| **Conventional LLM Agents**                 | **Sterling’s Approach**                            |
| ------------------------------------------- | -------------------------------------------------- |
| Transformer is the core "intelligence"      | Transformer is just a **codec** (parser/generator) |
| Knowledge in model weights + vector DB      | Knowledge in **explicit KG nodes/edges**           |
| Retrieval = fetch text chunks, feed to LLM  | Retrieval = **graph queries** and linked facts     |
| Free-form chain-of-thought (text) reasoning | **No free-form CoT** – uses IR + graph state       |
| Memory = long chat history + summaries      | Memory = **bounded working memory + decay**        |
| Routing by prompt engineering or heuristics | Routing by **scored operator choices** (value fn)  |
| Semantics buried in neural embeddings       | Semantics captured in **IR structures**            |

To preserve transparency and correctness, Sterling enforces **11 architectural invariants** (Core Constraints v1):

<!-- BEGIN CANONICAL: core_constraints_v1 -->
| ID | Constraint | Description |
|----|------------|-------------|
| INV-CORE-01 | No Free-Form CoT | No generative LLM chain-of-thought in the decision loop |
| INV-CORE-02 | Explicit State | All task state in UtteranceState + KG, not transformer KV cache |
| INV-CORE-03 | Structural Memory | Episode summaries + path algebra for long-horizon, not transcript prompts |
| INV-CORE-04 | No Phrase Routing | No phrase dictionary or regex-based routing; all routing via scored search |
| INV-CORE-05 | Computed Bridges | Cross-domain bridges computed at runtime, not static lookup tables |
| INV-CORE-06 | Contract Signatures | Landmark/operator signatures are typed contracts, not learned embeddings |
| INV-CORE-07 | Explicit Bridge Costs | Domain transitions carry explicit costs with hysteresis |
| INV-CORE-08 | No Hidden Routers | All routing decisions auditable via StateGraph; no secret bypasses |
| INV-CORE-09 | Oracle Separation | No future/oracle knowledge in inference inputs; only in training signals |
| INV-CORE-10 | Value Target Contract | Canonical value targets versioned and hash-verified |
| INV-CORE-11 | Sealed External Interface | External tools cannot mutate internal state except via governed operators with declared write-sets |

See [docs/canonical/core_constraints_v1.md](docs/canonical/core_constraints_v1.md) for full details.
<!-- END CANONICAL: core_constraints_v1 -->

### Neural Usage Contract

**Core Principle: Neural is advisory; Symbolic is authoritative.**

Neural components may rank or prioritize **already-legal** symbolic moves (hybrid value function), but they cannot:
- Create new operators or bypass operator preconditions
- Mutate KG / UtteranceState directly or introduce new facts
- Override symbolic logic (only advise on move ordering)

See [docs/canonical/neural_usage_contract.md](docs/canonical/neural_usage_contract.md) for the full contract.

### Evaluation Gates

Sterling's success is measured by three evaluation gates (research targets, not invariants):

| ID | Gate | Success Criterion |
|----|------|-------------------|
| EVAL-01 | Compute Parity | Match/beat transformer baselines in compute per task |
| EVAL-02 | Long-Horizon State | All long-horizon state in KG + summaries, not prompts |
| EVAL-03 | Reasoning Substrate | Sterling replaces LLM reasoning, not wraps it |

**Current Status (January 2026)**: All gates passing. See [docs/canonical/evaluation_gates_v1.md](docs/canonical/evaluation_gates_v1.md) for details.

By adhering to these principles, Sterling maintains a clear separation between the **symbolic reasoning core** and any learned or external components. The result is a system that is graph-based and transparent “all the way down,” rather than a black-box LLM with some knowledge graph garnish.

_(For a deeper discussion on these design choices and how Sterling compares to related approaches like Graph-RAG or ReAct, see the internal note in `docs/internal/notes/not_graph_rag.md`.)_

**New to Sterling?** See [docs/guides/newcomers.md](docs/guides/newcomers.md) for an onboarding guide with learning paths.

## Architecture Overview

Sterling’s architecture consists of a pipeline that transforms an input utterance into a structured interpretation and then into an explanation, with optional neural compression in the loop. The high-level flow is illustrated below:

mermaid

Copy code

`flowchart TD
    A([Text Query])
    B([IR Extraction])
    C([UtteranceState IR])
    D([Symbolic Reasoner])
    D1([Rule Engine + Knowledge Graph])
    D2([Planner<br/>(Explanation Plan)])
    E([Compression Path (Sterling Full)])
    E1([IR Tokens → IDs])
    E2([RGBA Token Grid])
    E3([ViT Encoder])
    E4([Compact Latent])
    F([Decoder<br/>(Template or micro-LLM)])
    G([Explanation Text<br/>+ Optional Reasoning Trace])

    A --> B --> C
    C --> D
    D --> D1 & D2
    C --> E
    E --> E1 --> E2 --> E3 --> E4
    D1 --> D2
    D2 --> F
    E4 --> F
    F --> G
`

**Sterling Light (Symbolic Core):** For the current Sterling Light system, follow the left-hand path in the diagram. The steps are:

* **IR Extraction (A → B → C):** The input text query or sentence is parsed into a deterministic **Intermediate Representation (IR)** format, producing an **UtteranceState** object (for example, a syntax/semantic tree or frame structure). This IR is a machine-interpretable encoding of the sentence’s meaning and context, with a stable serialization format. _(Sterling defines versioned IR schemas to ensure consistency and round-trip accuracy in parsing.)_
* **Symbolic Reasoner (C → D):** The core **SterlingEngine** takes the UtteranceState IR and begins a reasoning loop. It creates a **StateGraph** data structure to explore different interpretations or reasoning steps. Each edge in this graph represents applying an **operator** to the state (classified as S, M, P, K, or C – e.g. Search, Memory, Perception, Knowledge, or Control operations). The engine uses a combination of deterministic rules and a learned heuristic (value function) to traverse this graph efficiently in search of a goal state or solution.
* **Knowledge Graph & Rule Engine (D1):** Sterling Light uses a built-in **Knowledge Graph (KG)** of linguistic facts (and potentially domain facts) and a set of symbolic **rules** (like Datalog-style or hand-coded rules). These are used to infer additional information, classify structures, or verify conditions. For example, rules might identify that a sentence has a _predicate nominal_ construction if certain patterns are present, or check logical consistency of a statement against known facts. The rule engine queries and updates the KG as the reasoning progresses. The KG is relatively small and specific to the domain (hundreds of nodes rather than millions) and is fully transparent.
* **Planner / Explanation Assembly (D2):** Once the reasoning loop reaches a conclusion or goal (e.g., it has determined the classification of the input or verified a claim), the system invokes a **Planner** to assemble a structured **explanation plan**. This plan is an object that cites the supporting facts, rules applied, and the outcome of the reasoning in a structured form (not just text). For instance, it may list the construction identified, which conditions were met, which rules fired, and what evidence from the input or KG supports the conclusion.
* **Decoder to Text (F):** Finally, a **Decoder** component turns the explanation plan into a human-readable **explanation text (G)**. In Sterling Light, this is done primarily with **template-based generation** (filling in explanation templates with the specific details) to ensure faithfulness. Optionally, a small language model can be used as a **stylist** to paraphrase or polish the text for fluency, but **critically, this model is not allowed to introduce or remove information** – it must stick to the plan. (If a neural decoder is used, the output can be validated against the plan to guarantee no hallucination.)

Throughout the Light pipeline, all intermediate data (the IR, any state updates, rule inferences, etc.) are kept for inspection. The **StateGraph** provides an append-only log of every operation taken, so one can trace exactly how the system arrived at its conclusion. The search process is typically bounded by a **Semantic Working Memory policy** that limits how much of the state space is explored or kept in memory, applying a form of forgetting or pruning with well-defined rules.

**Sterling Full (Semantic Compression & On-Device):** The Full architecture augments the above pipeline with the right-hand path (E1 → E4 in the diagram), which introduces a learned compression of the IR and moves heavy computation to specialized hardware:

* **IR-to-RGBA Compression (E1 → E2):** The structured IR (which may be a tree or graph) is converted into a sequence of discrete IDs (E1), which are then mapped into a set of **RGBA pixels** (E2). Essentially, each token or element in the IR gets represented as a 4-byte code (R,G,B,A values from 0–255) and arranged into a fixed-size 2D grid. This grid is not an actual image but a visual encoding of the IR’s symbolic content.
* **ViT Encoder (E3 → E4):** A small **Vision Transformer** (or similar CNN/Transformer hybrid) processes the RGBA token grid to produce a compact **latent vector** (E4) summarizing the IR. This acts as a continuous embedding of the entire state. The ViT encoder is designed to run efficiently on Apple GPU/ANE (Metal Performance Shaders or Core ML). In Sterling Full, this latent could be used in two ways:  
   1. As input to the Decoder (F) to help it generate the final text in a more flexible, learned manner (instead of purely template-based). The decoder can attend to this latent representation to add nuance to the explanation, while the symbolic plan still provides structure.  
   2. Potentially, as part of the reasoning loop’s heuristic (value function) or for IR reconstruction tasks. In practice, Sterling uses this primarily to test if the IR can be faithfully reconstructed from the latent (to ensure the compression hasn’t lost critical information).
* **On-Device Deployment:** By using the RGBA+ViT pipeline, **Sterling Full can run the computationally intensive parts (parsing, encoding, decoding)** on dedicated hardware. The **Sterling Core** (graph search, KG, rules, planning) remains lightweight and runs on the CPU, while the **encoder and any neural decoders run on GPU/Neural Engine**. This enables the whole system to run on-device (e.g., on a MacBook or iPhone) without server support. For mobile, the trained PyTorch models are planned to be exported to **CoreML** (with fixed input sizes and supported ops) so they can execute on iOS devices.

Importantly, Sterling Full **does not change the reasoning logic** – it compresses and accelerates it. The same decisions and results should occur as in Sterling Light, as long as the compression is faithful. Sterling Full is essentially an experiment in _information compression and deployment_: how much can we shrink the intermediate representations and still maintain accuracy, and can we do all this in real-time on consumer hardware?

_(For technical details on the RGBA token approach and the on-device deployment strategy, see `docs/architecture/quickreference.md#semantic-compression` and the CoreML compatibility notes in `scripts/coremlcompatibilitychecklist.md`.)_

**Proof and Verification Systems:** In addition to the core reasoning pipeline, Sterling includes a _proof logging and verification subsystem_ to ensure the integrity of its outputs:

* Sterling’s reasoning trace can be certified using a scheme called **TD-12** (a step/episode certificate standard) and a **Memory Substrate (MS)** for long-term ledgering. Every step and state can be hashed and recorded, allowing third parties (or a test harness) to replay the reasoning and verify it arrives at the same result with the same intermediate states. This prevents tampering and ensures reproducibility of the reasoning process.
* The integration of TD-12 and MS means an explanation or answer from Sterling can come with cryptographic evidence that “I followed the rules correctly and here’s the proof”. Different levels of verification are defined (from basic artifact integrity up to full replay by an independent verifier).
* These proofs are **compositional** rather than a single chain: a TD-12 certificate can include references to a memory substrate summary, and a verifier can drill in as needed. The system distinguishes between verifying the content of reasoning (replaying it) vs. just trusting stored summaries, with policies for what is acceptable in different contexts.

In short, not only does Sterling aim to reason correctly, it also can _prove_ to you that it reasoned correctly by showing its work and making it checkable. (See `docs/architecture/verification_truth_table.md` for a formal definition of the verification levels, and `docs/versions/MS/` for the Memory Substrate design.)

## Current Status and Results (Sterling Light v1.3.1)

As of v1.3.1, **Sterling Light** is a working proof-of-concept of the core ideas. It has a fully implemented reasoning engine and demonstrates significant capabilities in a controlled domain. Key features and achievements include:

* **StateGraph Reasoning Engine:** A first-class representation of the reasoning process as a graph of states and operator applications. The StateGraph logs every inference step with an **append-only audit trail**, enabling complete transparency into the reasoning path.
* **Typed Operator Taxonomy:** All reasoning operators are categorized into five types – **Seek (S)**, **Memorize (M)**, **Perceive (P)**, **Knowledge (K)**, **Control (C)** – following an ontology of reasoning. This taxonomy is enforced at runtime (unknown or uncategorized operators raise warnings) and helps structure the search.
* **Semantic Working Memory (SWM) Policies:** The system uses configurable SWM policies to manage which parts of the state and knowledge remain “active” during reasoning. These policies can prioritize certain types of information or enforce limits (e.g., depth limits, blacklisting certain operators in some contexts). This is **data-driven** (config files) so different tasks can tweak memory handling without changing code.
* **Memory Decay Engine:** Integrated a usage-based **memory decay** mechanism [Sterling](./README.md#L62-L70). As the reasoning proceeds, it marks knowledge and intermediate results with usage tags like _SUCCESS\_PATH_, _EXPLORED_, or _NEGATIVE\_EVIDENCE_. Less useful parts of the search space can be pruned or compressed over time. This helps focus the reasoning and is crucial for long-horizon searches.
* **Self-Correction and Backtracking:** A self-correction subsystem monitors the reasoning confidence and patterns. If the engine detects contradictions or dead-ends, it can backtrack and try alternative paths, similar to a logical backtracking or DFS with learning. The pattern-based learning component updates the decay engine or search preferences to avoid repeating mistakes (see the Self-Correction design for details).
* **Robust Knowledge Graph Interface:** The **KG interface** has been hardened to support multiple backends (an in-memory graph for testing and a “FullKG” for larger scale). Connectivity issues and update anomalies have been fixed so that the KG remains consistent across deep copy operations and state transitions. (This was a major refactor labeled K6.2, introducing a centralized KG Registry for performance and consistency.)
* **Strict Goal Semantics:** Task goals are defined with precise logical conditions – e.g., all sub-criteria must be met (AND semantics) unless specified – and the system normalizes fields like entity names so that goal checking is reliable. This prevents false positives in reaching a solution state.
* **Learned Value Function (“TransitionScorer”):** Sterling Light includes a trainable **TransitionScorer** model that evaluates the potential of each state transition. It uses **structural feature vectors** extracted from the state (with various modes of detail, e.g., an 8-dim or 12-dim feature representation of the state and operator). In v1.3.1, a model has been trained on successful reasoning traces to bias the search towards fruitful paths [Sterling](./README.md#L66-L74). This learned heuristic can drastically improve efficiency without sacrificing correctness. (See the Feature Modes spec for the different feature sets and how they incorporate goal-distance signals.)
* **Cross-Domain Generalization:** The same value function and reasoning framework have been tested on multiple domains:  
   * **WordNet Navigation:** Using Sterling to navigate a WordNet-like lexical graph (finding relations between words), it achieved results **29× to 600× faster** than frontier LLMs (GPT-5.1, o3, kimi-k2) while maintaining _zero errors_ on a benchmark of 10 canonical tasks [Sterling](./README.md#L66-L74). This shows the system can outperform large LMs in graph search tasks by leveraging structured knowledge and efficient search. (See the WordNet navigation benchmark for experiment details.)  
   * **IR Constraint Solving:** The value function generalized to a non-linguistic task of solving an **IR navigation puzzle** (a constraint-satisfaction problem represented as a graph). Sterling Light solved 100% of test cases and did so with a **38% reduction in search steps** compared to an unguided search [Sterling](./README.md#L66-L74).  
   * **Rush Hour Puzzle Domain:** As a further test, a simplified version of the Rush Hour sliding block puzzle was implemented as another world. The structural features and learned scorer were re-used, successfully guiding the solver. This demonstrates that the learned search policy wasn’t overfitted to language – it captured general search heuristics.
* **Cross-Domain Bridge Architecture:** Implemented the ability to **bridge between worlds/domains**. For example, Sterling can start in a discourse (language) world, identify a sub-problem that is better handled by a different world (e.g. a knowledge lookup or a puzzle solver), transition into that via a **LandmarkWorld** adapter, and then return. This uses special BRIDGE\_ENTRY and BRIDGE\_EXIT operators with associated costs. The architecture ensures there’s no uncontrolled switching; all bridges are deliberate parts of the search with cost accounting and mild hysteresis to prevent bouncing around.
* **Governance & Transparency:** Several governance features are in place to ensure the system behaves as designed:
   * **No Hidden Routers invariant** is actively enforced (unit tests cover that all routing decisions are logged and explainable via state transitions).
   * **Oracle separation** checks ensure the value model is not accessing forbidden data at runtime.
   * **Value Target Contract** enforcement means every value prediction can be interpreted against a known target definition (preventing unintended optimization goals).
   * A **TraceAuditor** is included to verify these conditions during runs.
   * **Governance Modes (January 2026):** Three orthogonal governance flags control system strictness: (1) **Invariant Strictness** for state layer integrity (I1/I2/I4/I5 violations), (2) **RunIntent** enum (`DEV`/`CERTIFYING`/`PROMOTION`/`REPLAY`) for semantic validation rigor, and (3) **Promotion Strictness** for certification gates. In strict modes (`is_strict=True`), Sterling enforces **fail-closed** behavior: missing dependencies raise errors, never silently skip. See `core/governance/run_intent.py` and `docs/guides/study_materials/10_governance_and_lifecycle_deep_dive.md`.
   * **Witness-First Governance (January 2026):** All gates must emit structured **FenceWitness** objects (status, reason, details), not just "no exception". Tests assert witness presence AND content. The `PromotionLane` class emits witnesses for regression fences, sandbox validation, and certification gates. See `core/induction/promotion_lane.py`.
   * **Parallel Expansion Governance (January 2026):** Fail-closed, deterministic parallel candidate expansion with injective attribution. `CandidateIdentityV1` canonicalizes ordering (operator_id, args_canonical_hash, neighbor_id); certifying mode requires index-based attribution. `EnvelopeTelemetryV1` normalizes telemetry for commitment-usable projections (stable ordering, quantization, redaction). Deterministic reducer; byte-identical episode logs. See `docs/specifications/optimization/parallelization_policy_v1.md`.
   * **Aging Policy Integration (January 2026):** Implemented continual induction aging policy with deterministic eviction ordering (sorted by `created_episode_seq, hash_id` with reason precedence). Revoked hypotheses/priors are excluded from selection and persisted with content-addressed evidence in `PriorStore`. The policy translator checks revocation status before translation, ensuring revoked artifacts cannot influence search. Eviction events use a two-hash model: `semantic_hash` (excludes timestamps for determinism) and `record_id` (includes timestamps for human-readable logging).
   * **Evidence Bundle CI Integration (January 2026):** Evidence bundle emission and verification gate PRs. PR runs use stratified CI-small regimes (50 episodes, hard/normal split, step budget 5); verification result check blocks merge on failure. Nightly runs execute full regimes with strict verification and no error swallowing. Episode fixtures for promotion-lane tests are documented in `data/datasets.csv` and `data/episodes/README.md`.
* **Deterministic Episode Logging:** Sterling logs each reasoning **episode** (one query's processing) with deterministic artifacts. Every state, operator decision, and outcome is recorded with content hashes. This allows for replaying an episode exactly or detecting if any component's behavior has drifted (e.g., if a model update changes decisions, the content hash comparison will flag it). A Phase 4.5 milestone was completed to integrate this loop: any prior outputs can be reloaded and verified for consistency, aiding regression detection and trust. Episode schema v3 (January 2026) adds governance artifacts: `registry_hash` for operator drift detection and `prior_influence` with deterministic hashing for replay verification.
* **Theory Conformance (TC-1 through TC-10):** Sterling enforces 10 theory conformance invariants that are testable contracts preventing drift from the core theory. TC-1 through TC-5 cover semantic invariance, IR-only inputs, student-teacher gap, no CoT in decision loop, and latent advisory. TC-6A through TC-10 (January 2026) add: provenance tracking, hypothesis influence gate, invariance checking, applicability preservation (TC-9A), and registered interpreters. See `docs/reference/canonical/conformance.md`.
* **H1 Promotion Lane Closed (January 2026):** The capability test (`tests/integration/test_promotion_loop_closure.py`) passes with real PN engine episodes. Stage K certifies on frozen fixtures; promotion lane runs end-to-end (synthesis, provenance, load, replay). Promotability and evidence-portability readiness gates are met per `docs/roadmaps/sterling_realignment_roadmap_2026.md`.
* **Determinism and Governance:** TD-12 and memory substrate certificate determinism; version-gated canonicalization, normalized hashing, real-time induction flush controls, proposal cadence policy. Production-readiness still requires further work (see below).
* **Lightweight Neural Components:** In summary, Sterling Light provides the full pipeline of a **symbolic IR, a knowledge-backed reasoning loop, a rule-based inference system, a planner for explanations, and a tiny neural encoder/decoder** to handle input/output language. All neural parts are kept “on a leash” – they do not make decisions about truth or strategy, only assist with translation to/from human language or compressing representations.
* **Planned (Sterling Full additions):** _The following are in progress or planned:_ integration of **RGBA latent tokens**, a ViT encoder for those tokens, and deployment of the learned models via **MPS (Mac GPU)** and **CoreML (iOS)**. These will allow running Sterling with minimal performance overhead on local devices and scaling to richer input data without altering the core reasoning logic.

Overall, **Sterling Light v1.3.1 has validated the core thesis**: it can reliably perform complex reasoning in a narrow domain with full transparency and significantly improved efficiency over naive approaches. It has passed all core unit tests and semantic evaluations for its domain, and even tackled two additional domains with no changes to the core algorithm. This suggests the approach is generalizable.

**Performance & Quality:** As of February 2026, the test suite contains **9,360 tests** with a **~97% pass rate** (9,081 passed, 173 failed, 96 skipped). All **core reasoning tests and invariants** pass (100% success on unit tests for the IR, rules, StateGraph behavior, and governance checks). The remaining failures are primarily integration test infrastructure issues (missing `pytest-asyncio` for async tests, data dependency issues in LOFO partition tests). The system's reasoning accuracy on designed tasks is 100% (it produces correct explanations and logical judgments on all curated test cases). However, **Sterling is still a proof-of-concept and not production-ready** – it lacks comprehensive error handling, persistence (the current KG is in-memory only), rigorous security hardening, and has not been optimized for speed or resource usage in a production setting.

**Conclusion:** _Sterling Light demonstrates that a neurosymbolic approach can achieve the target outcomes (transparent reasoning, multi-step inference, and knowledge integration) with a fraction of the model size of an LLM. The next steps will focus on polishing the core and then attempting the compression and deployment aspects to see if the approach holds up end-to-end on device._

## Getting Started

### Requirements

* Python 3.9+ (developed primarily on Python 3.13).
* Operating System: Sterling runs on macOS and Linux. For Sterling Light, no special hardware is required. (For Sterling Full’s future GPU/ANE features, an Apple Silicon device would be needed, but that part is optional for now.)
* Recommended environment: use a Python virtual environment.
* Key dependencies: `numpy`, `networkx` (for graph operations), `torch` (for the small neural models), plus standard utilities. Jupyter is optional (for exploring provided notebooks).

### Installation

Clone the repository and install the requirements:

bash

Copy code

`git clone <repo-url> sterling
cd sterling
python -m venv .venv
source .venv/bin/activate   # On Windows: .venv\Scripts\activate
pip install -r requirements.txt
`

This will install the necessary Python packages. (If you plan to run tests or experiments, you might also install dev requirements as needed.)

### Running the Demo and Tests

Sterling Light comes with a few demo scripts and an extensive test suite.

* **Quick Demo:** Run the end-to-end demo on a sample sentence:  
bash  
Copy code  
`python scripts/run_light_demo.py  
`  
This will output the input sentence, its serialized IR, the reasoning steps (rules applied, etc.), the structured explanation object, and the final generated explanation text. It’s a good way to see the system in action on a simple example.
* **Task A (IR Reconstruction) Evaluation:**  
bash  
Copy code  
`python scripts/run_task_a_eval.py  
`  
This will take a set of example sentences, encode and decode them with the tiny neural encoder, and report the reconstruction accuracy of the IR. It essentially tests that the **IR → Encoder → Decoder → IR** round-trip works. In Sterling Light, this uses a baseline encoder model (`models/light/encoder_baseline.pt`) which should perfectly reconstruct the IR (since it’s mostly an identity transformation in the baseline).
* **Task B (Explanation Generation) Evaluation:**  
bash  
Copy code  
`python scripts/run_task_b_eval.py  
`  
Runs a series of reasoning cases and compares the generated explanation objects/text to the expected gold explanations (`data/gold/task_b_explanations.json`). This tests the rule engine and planner – it verifies that Sterling can produce correct structured explanations for known linguistic phenomena.
* **Task C (Logical Consistency) Evaluation:**  
bash  
Copy code  
`python scripts/run_task_c_eval.py  
`  
This evaluates Sterling’s ability to act as a truth-checker. It feeds the system a set of claims and checks whether it outputs “Accept”, “Reject”, or “Cannot Verify” correctly by using its knowledge graph and rules. This tests the verifier logic in `core/reasoning/verifier.py`.
* **Full Test Suite:**  
You can run the entire suite of unit and integration tests with:  
bash  
Copy code  
`pytest  
`  
This will execute all tests in `tests/`. The suite covers everything from IR serialization and KG operations to multi-step reasoning and learning integration. (A full run is quite exhaustive, comprising over 9,000 tests, and is useful to ensure no regressions.)

If you encounter any issues during installation or running the demos (for example, missing packages or GPU configuration problems), please check the documentation in `docs/` or open an issue.

## Repository Structure

The repository is organized by component, reflecting the architecture layers and separation of concerns:

* **`core/`** – The core reasoning engine and internal representations. This includes:  
   * `core/engine.py`: The main **SterlingEngine** class that orchestrates processing of an utterance through the pipeline (parsing to IR, reasoning loop, etc.).  
   * `core/ir_serialization.py`: Definition of the IR format and (de)serialization functions. Ensures deterministic conversion between text, IR objects, and tokens.  
   * `core/reasoning/`: The heart of the reasoning loop. Contains:  
         * `state_graph.py`: The **StateGraph** class and related data structures (SearchNode, OperatorEdge, EdgeType) that represent the reasoning search space.  
         * `loop.py`: The implementation of the reasoning loop that expands the StateGraph by applying operators. Integrates the value function to choose expansions.  
         * `rules.py` and `verifier.py`: The symbolic rule definitions and the logic for verifying claims or applying inference rules in context.  
         * `planner.py`: Assembles the structured explanation object from the results of the reasoning (which rules fired, what facts were used, etc.).  
         * `staged_search.py`: Coordinator for **staged search** and cross-domain bridging (handles multiple worlds and bridge transitions).  
         * `episode_logger.py` and `episode_profile.py`: Tools for logging and analyzing reasoning episodes (used in training and debugging).  
         * `models/transition_scorer.py`: The definition of the learned TransitionScorer neural network (PyTorch) that scores state transitions.  
   * `core/worlds/`: The **World Adapters** which define domain-specific behavior. For instance:  
         * `discourse.py`: The main **DiscourseWorldAdapter** for handling general linguistic tasks (this is the entry world in Sterling Light).  
         * `pn.py`: A specific **PredicateNominalWorld** (for the predicate nominal construction example).  
         * `wordnet.py`: The **WordNet navigation world** (for the lexical graph search domain).  
         * `code.py`: A stub or early implementation of a **code refactoring world** (to demonstrate Sterling on code-editing tasks, planned for future).  
         * `landmarks.py`: The **LandmarkWorldAdapter** that manages bridging between worlds via landmark operators (cross-domain mappings).  
   * `core/value/`: The value function and related components:  
         * `protocol.py`: The interface for any ValueFunction (what methods it must implement).  
         * `structural.py`, `memory.py`, `task_heads.py`: Different heads that compute parts of the value function (structural features, memory/decay-based features, task-specific rewards).  
         * `hybrid.py`: Combines multiple heads into a single **HybridValueFunction** with weighted sum. This is used to integrate structural and memory-based heuristics.  
         * `landmark_embeddings.py`: Manages the learned embedding table for landmark (operator) representations, used in bridge decisions.  
         * `latent/`: (For Sterling Full) Components for latent compression:  
                  * `sterling_encoder.py`: A prototype transformer encoder that is aware of Sterling’s layered IR (used in advanced compression experiments).  
                  * `ir_latent_v1.py`: Schema for how to serialize an IR into a latent-friendly format (with hints of world context).  
                  * `latent_value_model_v2.py`: An experimental combined model that tries to predict value and difficulty from a latent state (used in ongoing research for Stage F).  
   * `core/kg/` – The knowledge graph and memory management layer:  
         * `registry.py`: A central **KG Registry** that tracks all nodes/edges in a content-addressable way. It provides KGRefs (immutable references with hashes) to ensure consistency when the world state is copied or branched.  
         * `ontology.py`: Defines the mapping from KG edge types to operator categories (S/M/P/K/C). Essentially a lookup for what type of reasoning a given relation represents.  
         * `swm_policy.py`: Definitions of **Semantic Working Memory policies** for different tasks. Policies include limits on how many nodes of certain types to keep, which edges to prioritize or prune, etc.  
         * `decay.py`: The **Decay Engine** implementation that handles forgetting/unloading parts of the KG based on usage patterns (with the UsageKind classifications).  
         * `path_algebra.py`: Utilities for analyzing paths in the knowledge graph, including an algebra for combining path properties (used in advanced reasoning to judge the value of exploring certain connections).  
         * `full_kg.py`: Implementation of a **full-scale KG backend** (could be backed by a database or larger graph library) to use in place of the default simple in-memory graph, for scaling up.  
         * `kg_nodes.json`, `kg_edges.json`: Example or default node/edge data for the initial knowledge graph (in `data/` directory).  
         * `schema_v1.md`, `registry_v1.json`: Documentation and data for the KG schema and initial registry state.  
   * `core/tasks.py`: Definitions of the tasks (A, B, C) and convenience wrappers to run them through the engine.
* **`data/`** – Data and corpora:  
   * `corpus/`: Contains example sentences and annotations (e.g., `sentences_v1.json`, `annotations_v1.json`) used to build or test the system.  
   * `gold/`: Gold-standard outputs for tasks (for evaluation). For example, `task_a_ir_pairs.json` contains pairs of input and expected IR for testing the encoder; `task_b_explanations.json` contains expected explanation outputs for certain inputs; `task_c_claims.json` contains truth-value judgments, etc.  
   * `benchmarks/`: Benchmark result records and definitions, such as the WordNet navigation results, IR navigation results, etc., often with their own README or notes.
* **`models/`** – Pre-trained or saved model files:  
   * `models/light/encoder_baseline.pt`: A tiny baseline encoder model for Sterling Light (likely a trivial identity or small transformer that encodes IR tokens to itself).  
   * `models/full/vit_encoder.mlmodel` and `decoder_micro_llm.mlmodel`: Placeholders or examples for the CoreML models for Sterling Full (when those are implemented). These would be the exported versions of the ViT encoder and the micro decoder LLM for mobile use.
* **`scripts/`** – Python scripts for running demos, evaluations, and utilities:  
   * `run_light_demo.py`, `run_task_a_eval.py`, `run_task_b_eval.py`, `run_task_c_eval.py`: Command-line interfaces for the main demo and tasks (as described in _Getting Started_ above).  
   * `run_pn_verification.py`: A script demonstrating the predicate nominal verification (Task C in a specific scenario).  
   * `demo_end_to_end_training.py`: An example pipeline that shows how one might train components end-to-end (from generating data to training the model to evaluating).  
   * `test_feature_vector.py`: A utility to print out or validate the feature vector computed for a given state transition, to ensure the feature extraction matches the spec.  
   * `coreml_compatibility_checklist.md`: Documentation of requirements and tips for exporting models to CoreML (for Sterling Full deployment).
* **`tests/`** – The test suite, covering:  
   * Unit tests for core components (`test_state_graph.py`, `test_ontology.py`, `test_decay_integration.py`, `test_kg_interface.py`, etc.).  
   * Integration tests for end-to-end scenarios and multi-component interaction.  
   * Specific tests for new features or regressions (e.g., KG registry tests, value head integration tests, etc.).  
   There are over 9,000 tests ensuring that the system meets its specifications and that new changes don't break existing functionality.
* **`experiments/`** – Experimental scripts and analysis:  
   * `train_transition_scorer.py`: Script to train the TransitionScorer model on logged episodes (for learning the value function).  
   * `value_head_eval.py`: Evaluate performance of the learned value head versus baselines.  
   * `README_transition_scorer_training.md`: Documentation for the training process of the value function (how data is collected, training regimen, etc.).  
   * `metrics.py`: Utilities for computing evaluation metrics on reasoning episodes.  
   * `latent/` (within experiments): might contain specific experiments for the latent compression (Stage F) development, such as ablation studies or monotonicity tests.
* **`docs/`** – Documentation and design notes:  
   * `GLOSSARY.md`: A glossary of Sterling terminology (mapping theory terms to implementation). Useful for understanding internal code names vs. concept names.  
   * `theory/` and `architecture/`: Detailed design documents on various aspects of Sterling. For example, `docs/architecture/core/self_correction.md` describes the self-correction system, `docs/architecture/core/kg_registry.md` covers the KG registry refactor, etc.  
   * `specifications/`: Formal specs for certain components (like `feature_modes.md` describes the feature vectors used by the value function).  
   * `roadmaps/`: Planned development roadmaps. For instance, `core_roadmap.md` outlines the stages (A through H) to realize the full Sterling theory, and `scenario_competence_roadmap.md` discusses plans for building complex evaluation scenarios.  
   * `scenarios/`: If present, definitions of evaluation scenarios/use-cases for Sterling (e.g., multi-utterance discourse, code refactoring tasks) for testing the system in realistic settings.  
   * `archive/` and `internal/`: Older designs, analyses, risk assessments (e.g., “risks\_full\_model.md”, “not-another-graph-rag.md”). These provide historical context or discarded ideas.  
   * `external/README.md`: Possibly a stub for an external-facing overview (if a public version of the README is maintained separately).

_(For more details on any component, the docs directory is the best place to start. The internal design docs provide rationales and mathematical formalisms for many parts of the system.)_

## Roadmap and Future Work

Sterling is under active development, guided by a multi-stage roadmap that bridges theoretical milestones and practical implementation:

* **Core Theoretical Stages (A–H):** The project has defined stages _A_ through _H_ in the core roadmap (`docs/roadmaps/core_roadmap.md`). Stages **A–D** (formalizing the state hierarchy, operator calculus, basic transformation tasks, and a unified I/O API) are already completed, solidifying the foundation of Sterling Core. Stage **E** (introducing a multi-factor value function combining structural and memory-based signals) is also completed with the HybridValueFunction integration. Stage **F** (the latent compression layer) is in progress – prototypes of the Sterling-specific transformer encoder and latent value model are being tested for fidelity and monotonicity. Stages **G** (applying Sterling Core to a second domain to test domain-transfer) and **H** (extending to multi-utterance discourse with a DiscourseState) are on the horizon. These stages focus on validating that Sterling’s design scales to new scenarios and longer dialogues. Many of the theoretical goals (like strict invariants and separation of concerns) have already been achieved in the implementation.
* **Sterling Full (Compression & Deployment):** The next major milestone is the transition to **Sterling Full** (labeled Phase 2b through Phase 4 in planning). This involves:  
   * Completing the **RGBA compression pipeline** and ensuring the ViT encoder can compress and decompress the IR without loss of critical information. Success here will be measured by IR reconstruction accuracy from latents and the impact on reasoning performance.  
   * **On-device performance optimization:** Using Metal Performance Shaders (MPS) in PyTorch to run models on GPU/ANE for Mac, and exporting to CoreML for iOS. This will require simplifying model architectures to be CoreML-compatible and possibly quantizing or distilling models to meet mobile constraints.  
   * Testing Sterling on-device with non-trivial workloads to ensure it meets reasonable latency and memory usage targets. The target is to prove that even a relatively complex symbolic system with a neural front-end can run interactively on consumer hardware.
* **Scenario-based Evaluation:** As Sterling matures, a key focus is demonstrating its abilities on more **realistic scenarios**:  
   * **Track A – Grounded Multi-hop Reasoning:** Scenarios where Sterling must reason over a knowledge graph with multiple hops (like answering questions by piecing together information from different sources, ensuring each step is grounded and explained). This tests long-horizon search and the system’s ability to avoid hallucination by verifying each link.  
   * **Track B – Code Refactoring Assistant:** Using Sterling’s approach to perform a sequence of code transformations (with unambiguous goals, e.g., apply a series of code refactoring rules or verify a piece of code against a spec). This will stress the cross-domain abilities (language to code world bridging) and the system’s capacity to handle a larger, more complex state (like code ASTs in the IR).  
   * **Multi-agent dialogues and planning:** Future scenarios may include Sterling coordinating multiple sub-agents or handling dialogues that involve planning (for example, a conversation where the system needs to plan steps to achieve a user’s goal, involving both language understanding and planning actions).  
These scenarios will serve as **end-to-end tests** of Sterling in conditions closer to real applications. The expectation is that on such tasks requiring extensive reasoning with no tolerance for hallucination, Sterling can achieve comparable or better success rates than large LLM-based agents, while using far fewer resources (in terms of model calls or tokens) by leveraging its structured approach.
* **Production Readiness Work:** Before Sterling could be considered for production use, several engineering tasks remain:  
   * Implementing a persistent storage for the knowledge graph and state (currently, everything is in-memory only). Likely integration with a fast graph database or an on-disk store for larger knowledge bases.  
   * Rigorous **linting, type-checking, and security auditing** of the codebase. (Static analysis tools, security scanners, etc., have yet to be fully applied.)  
   * Performance profiling and optimization of the Python code – possibly moving some critical loops to C++/Rust or using vectorized operations to handle larger graphs.  
   * Improved error handling, fallback strategies for the neural components (in case of model uncertainty), and more robust user input parsing.  
   * A user-friendly interface or API on top of `SterlingEngine` so that it can be integrated into applications easily (the current CLI and Python API are primarily for development/testing).

The **guiding vision** is that once these pieces are in place, Sterling can serve as a foundation for applications that need **trustworthy reasoning**. It could power an assistant that not only answers questions but shows exactly which facts and rules led to the answer, or a tool that checks the consistency of documents, or an educational tutor that reasons through problems step-by-step with the student.

_For more details on future plans and design discussions, refer to the roadmap and scenario documents in the `docs/` folder._ We maintain those files to track progress and ensure our implementation aligns with the theoretical blueprint.

## License and Attribution

This project is open-source (see the `LICENSE` file for details). We ask that if you use Sterling’s code, ideas, or data in your research or applications, you **cite the project or link back to this repository** to give credit.

Sterling builds upon ideas and components from many fields – from classic AI knowledge representation and graph search, to modern deep learning and neurosymbolic research, to specific tools like WordNet. We have tried to acknowledge these influences throughout the documentation (see the references in `docs/architecture/quick_reference.md`). If you feel something is missing attribution, please let us know.

Contributions are very welcome! In particular, the project would benefit from:

* **IR and Parser Improvements:** Suggestions for better intermediate representation designs or more robust parsing techniques for richer language phenomena.
* **New Linguistic Phenomena:** Expanding Sterling Light’s rule set and KG to cover additional constructions or languages.
* **Knowledge Graph Expansion:** Incorporating larger or more dynamic knowledge sources while maintaining performance (e.g., hooking up a real WordNet or Wikidata subset in place of the toy KG).
* **Compression & Model Optimization:** Ideas for improving the RGBA encoding, reducing the size of the ViT or stylist LLM, or alternative compression schemes that preserve semantics. Also, techniques for optimizing on-device inference (Quantization, distillation, using Apple’s Neural Engine, etc.).
* **Testing & Verification:** More test cases, especially complex scenarios, and improvements to the proof logging/verification system. Third-party audits of the “no hidden state” and “no hidden router” guarantees would be valuable.
* **Documentation & Examples:** Feedback on the clarity of documentation, and contributions of tutorials or notebooks demonstrating how to extend Sterling to new domains or how it can be used in practice.

Throughout development, our north star has been to **preserve Sterling Light’s symbolic guarantees** even as we add learning and compression. We welcome contributions that align with this philosophy. Sterling Full’s neural components should be seen as a _compression and style layer on top of a robust symbolic core_, not a replacement of the reasoning process. By keeping this principle, we hope to deliver AI systems that are both powerful and trustworthy.

**Contact:** For major questions or to discuss collaborations, you can reach out via the issue tracker or the contact info provided in the repository. We’re excited to see what others might do with or learn from Sterling, and we’re happy to help orient new contributors who share an interest in neurosymbolic reasoning.

Let’s push the boundaries of interpretable AI together, one reasoning step at a time 
 