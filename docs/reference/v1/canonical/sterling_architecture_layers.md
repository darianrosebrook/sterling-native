> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Sterling Architecture Layers

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Version**: 1.1
**Date**: 2026-02-20
**Author**: @darianrosebrook
**Status**: Active

---

## Supersedes

This document supersedes [light_vs_full.md](light_vs_full.md) as the authoritative definition of Sterling's layered architecture. The prior document framed Sterling as "Light" (symbolic reasoning) vs "Full" (Light + ViT compression + CoreML deployment). That framing was wrong in two ways and no longer reflects the active runtime path (Rust + Code32/ByteState):

1. It treated "Full" as an efficiency optimization over "Light" — same reasoning, just compressed and deployed. But the actual vision is broader: Sterling Full replaces not just the reasoning loop but also the memory system and the I/O codec, eliminating transformer dependency entirely.
2. It implied a single step from Light to Full. The actual architecture is four layers, each replacing a different component that current AI systems delegate to transformers.

The old document is retained for historical context.

---

## 1. The Core Thesis

Current AI systems use a single architecture — the autoregressive transformer — for four distinct functions:

| Function | What the transformer does | Why it's the wrong tool |
|----------|--------------------------|------------------------|
| **Reasoning** | Chain-of-thought token generation | Non-deterministic, non-auditable, scales with token count not problem structure |
| **Memory** | KV cache attention over prior tokens | Bounded by context window, no governance, no selective retention |
| **Understanding** | Embedding + attention over input tokens | Conflates parsing with reasoning; can't separate what was understood from how it was used |
| **Generation** | Left-to-right token prediction | Cannot globally constrain output; semantic fidelity is statistical, not structural |

Sterling replaces each function with a purpose-built layer. Each layer is independently valuable. Together, they eliminate the transformer from the cognitive loop entirely, confining it to an optional, non-authoritative role.

---

## 2. The Four Layers

### Layer 0: Reasoning Substrate

**What it replaces**: Transformer chain-of-thought reasoning.

**What it is**: Domain-agnostic governed search over typed state representations. Operators have preconditions and effects. Search is deterministic. Every decision is a node in a StateGraph with replay-verifiable provenance.

**Key components**:
- UtteranceState IR with S/M/P/K/C operator taxonomy
- StateGraph search with path algebra (usage, recency, novelty weights)
- Typed operator contracts with preconditions, effects, and costs
- Governance kernel: fail-closed promotion, operator certification, replay verification

**Current status**: Proven. Cert-grade domains operational (WordNet, Rome, Mastermind, EscapeGame). ~9,360 tests. Deterministic replay verified across domains. Operator induction pipeline producing candidates with stable identity.

**Key documents**:
- [Capability Campaign Plan](../../analysis/capability_campaign_plan.md) — cert-grade domain surfaces
- [Core Realignment Closeout](../../planning/core_realignment_2026_consolidated_closeout.md) — implementation evidence

### Layer 1: Memory Architecture

**What it replaces**: Transformer KV cache / context window.

**What it is**: Semantic Working Memory (SWM) with explicit lifecycle states, governed projection boundaries, and deterministic state digests. Knowledge lives in versioned, hash-chained artifacts — not in attention weights that vanish when the context window shifts.

**Key components**:
- **Node lifecycle**: Committed / Shadow / Weak status with explicit promotion gates
- **Projection purity**: PlannerView (Committed-only, fail-closed) vs RetrievalView (all)
- **MeaningStateDigest**: Deterministic hash-chained state identity with genesis sentinel and parent pointers
- **EpisodeTrace**: Hash-chained episode artifacts binding state-in, state-out, patch, witness, and operator sequence
- **Bounded working set**: Budget-controlled materialization, canonical eviction, rehydration from persistent substrate
- **Myelin sheath**: Certified fast-path corridors — proven operator sequences that can be replayed without re-search

**Current status**: Implemented. All three trackers complete. 17 named invariants, 24 acceptance tests, 3 system integration gates passing. SWM contract v0 is canonical.

**Key documents**:
- [SWM Implementation Trackers](../../planning/semantic_working_memory_implementation.md) — full tracker status
- `docs/canonical/semantic_working_memory_contract_v0.md` — canonical contract
- `docs/reference/canonical/linguistic_ir_contract_v0.md` — IR contract

### Layer 2: Hardware-Native Carrier

**What it replaces**: Python object overhead, JSON serialization, dynamic dispatch in the inner loop.

**What it is**: Code32/ByteStateV1 — a 32-bit identity atom packed into dense byte tensors for sub-millisecond vectorized operations. The same bytes that operators compute on are the bytes that get hashed, stored, and verified. Collapses runtime representation, provenance chain, and replay evidence into a single format.

**Key components**:
- **Code32**: 32-bit registry-bijective identity atom (domain/kind/local_id)
- **ByteStateV1**: Two-plane packed encoding (identity + status), ~640 bytes per state
- **ByteTraceV1**: Fixed-frame evidence format with envelope/payload split for byte-for-byte replay verification
- **Compilation boundary**: Governed codec between dynamic domain payloads and the frozen inner-loop tensor
- **Operators as uint32 masks**: Preconditions and effects as vectorized integer operations

**Current status**: Implemented (carrier substrate v1). Code32/ByteStateV1 and ByteTraceV1 are operational, with Rust as the active hot-path implementation direction for throughput and deterministic replay.

**Key documents**:
- [Code32 and ByteStateV1](code32_bytestate.md) — substrate spec
- [ByteState Compilation Boundary](bytestate_compilation_boundary.md) — codec spec

### Layer 3: Semantic Realization

**What it replaces**: Autoregressive left-to-right token generation.

**What it is**: Diffusion-based text generation conditioned on Sterling's semantic IR. The insight: if the full semantic representation is already computed (propositions, discourse roles, pragmatic intent, reconstruction templates from Colen/Hawkins/Frazier), then surface realization is *rendering*, not *generating*. A diffusion model denoises toward fluent text while being constrained by the IR — fixed slots stay fixed, generative slots have controlled variation, semantic fidelity is verified post-realization.

**Key components**:
- **Generative inversion**: Frazier's comprehension model inverted — semantics first, syntax retrofitted during realization
- **MaskIntent schema**: Slot-level control (MUST_INCLUDE / SHOULD_INCLUDE / MAY_INCLUDE) derived from IR structure
- **Shared semantic latent**: StateLatent bridges IR and realization; operator groups drive conditioning
- **Post-realization verification**: Realized text is checked against source IR for semantic fidelity
- **Reconstruction templates**: Colen's PN subtypes, Hawkins' EIC principles, and Frazier's heuristics guide how compressed semantics are unpacked into linguistic structure for conditioning

**Current status**: Experimental (pre-implementation). Architecture proposal documented. Depends on Layers 0-2 being proven — the diffusion model needs a reliable, deterministic semantic substrate to condition on.

**Key documents**:
- [Generative Inversion](../../working/experiments/diffusion/generative-inversion.md) — architecture proposal
- [SEDD / Diffusion LLM Research](../../working/experiments/diffusion/diffusion-llm.md) — background research
- [SWM I/O Integration](../../working/experiments/diffusion/SWM-IO.md) — intake pipeline spec
- [Theory Document](../../theory/linguistic/sterling_full.md) — reconstruction templates from Colen/Hawkins/Frazier

---

## 3. How the Layers Compose

```
External input (text, game state, ARC grid, Minecraft rules)
    │
    ▼
┌─────────────────────────────────────────────────┐
│ Layer 2: Compilation Boundary (governed intake)  │
│   Domain payload → ByteState under frozen epoch  │
└──────────────────────┬──────────────────────────┘
                       │
    ▼                  ▼
┌──────────────────────────────────────────────────┐
│ Layer 0: Reasoning Substrate                      │
│   Search over ByteState tensors                   │
│   Governed operators as uint32 masks              │
│   Sub-millisecond episodes, deterministic replay  │
└──────────────────────┬───────────────────────────┘
                       │
    ▼                  ▼
┌──────────────────────────────────────────────────┐
│ Layer 1: Semantic Working Memory                  │
│   Committed/Shadow/Weak lifecycle                 │
│   MeaningStateDigest chains                       │
│   Myelin sheath certified corridors               │
│   Bounded WS with canonical eviction              │
└──────────────────────┬───────────────────────────┘
                       │
    ▼                  ▼
┌──────────────────────────────────────────────────┐
│ Layer 2: Compilation Boundary (governed output)   │
│   ByteState → IR → domain terms                   │
└──────────────────────┬───────────────────────────┘
                       │
    ▼                  ▼
┌──────────────────────────────────────────────────┐
│ Layer 3: Semantic Realization                     │
│   IR-conditioned diffusion → fluent text          │
│   MaskIntent constraints from theory templates    │
│   Post-realization semantic verification          │
└──────────────────────────────────────────────────┘
    │
    ▼
Surface output (text, action plan, explanation, A2A artifact)
```

**No transformer in the reasoning loop** (Layer 0). **No transformer in the memory system** (Layer 1). **No transformer in the evidence chain** (Layer 2). **No autoregressive transformer in generation** (Layer 3).

Transformers are permitted at the I/O boundaries as optional, non-authoritative components: intake parsing (text → IR hypotheses), advisory scoring (value function hints), and compression/indexing (non-authoritative latent embeddings). This is consistent with the [Neural Usage Contract](neural_usage_contract.md).

---

## 4. Naming

The old "Sterling Light / Sterling Full" naming reflected a single axis: with or without compression. The actual architecture has four layers, each independently valuable.

| Old Name | What It Actually Meant | New Framing |
|----------|----------------------|-------------|
| **Sterling Core** | Domain-agnostic reasoning definitions | Layer 0 specification (IR, operators, search algorithm) |
| **Sterling Light** | Core + PN micro-world proof-of-concept | Layer 0 implementation with cert-grade domains |
| **Sterling Full** | Light + ViT compression + CoreML (historical framing) | Superseded. The vision is Layers 0-3, not "Light + compression" |

The new naming is simply **the layer number** or, when referring to the complete system, **Sterling** without qualification. "Sterling Light" and "Sterling Full" are historical terms that described an earlier, incomplete understanding of the architecture.

When precision is needed:
- **"Sterling L0"** = the reasoning substrate (search, operators, governance)
- **"Sterling L0+L1"** = reasoning + memory (current operational state)
- **"Sterling L0+L1+L2"** = reasoning + memory + hardware-native carrier (next milestone)
- **"Sterling L0-L3"** = the complete system with semantic realization (target architecture)

---

## 5. Sequencing and Dependencies

The layers have strict dependencies:

```
Layer 0 (Reasoning) ─── proven, operational
    │
    ▼
Layer 1 (Memory) ─── implemented, passing all gates
    │
    ▼
Layer 2 (Carrier) ─── implemented, active performance path
    │
    ▼
Layer 3 (Realization) ─── experimental, depends on L0-L2
```

**Layer 3 depends on Layers 0-2 being proven.** The generative inversion needs:
- A deterministic semantic substrate to condition on (Layer 0)
- A governed memory system that ensures the IR is complete and committed (Layer 1)
- A hardware-native carrier that makes the inner loop fast enough for real-time realization feedback (Layer 2)

Without these, diffusion conditioning on IR is conditioning on an unreliable signal. The internals must land first.

**Layer 2 depends on Layer 0+1 for benchmarking.** ByteState is only provably valuable if measured against the cert-grade domains from Layer 0 (compression-on vs compression-off). SWM's MeaningStateDigest chains (Layer 1) define what must round-trip through ByteState compilation.

---

## 6. External Integration

Sterling already operates as a reasoning backend for external systems:

| Integration | Protocol | What Sterling Provides | What the External System Provides |
|-------------|----------|----------------------|----------------------------------|
| **Agent Agency V4** | A2A (JSON-RPC over HTTP) | Symbolic reasoning, domain solving | Orchestration, task delegation, language interface |
| **Minecraft bot** | WebSocket | Path-finding, operator learning | World state, rules, episode reporting |
| **Future rigs** | A2A / WebSocket | Governed search, certified operators | Domain-specific I/O, rules, state |

In all cases, the external system handles language understanding and generation (currently via transformer). Sterling handles reasoning and memory. Layer 3 (semantic realization) is the step where Sterling also handles generation, making the external transformer optional for output.

**Key document**: [Agent Agency V4 Tasks](../../../../agent-agency/iterations/v4/docs/TASKS.md) — Task 6.1.1 (A2A wrapper, complete)

---

## 7. What Each Layer Eliminates

| Layer | Transformer Dependency Eliminated | What Takes Its Place |
|-------|----------------------------------|---------------------|
| **L0: Reasoning** | Chain-of-thought token generation for planning/deciding | Governed search over typed state with deterministic replay |
| **L1: Memory** | KV cache / context window for maintaining state | SWM with Committed/Shadow/Weak lifecycle and hash-chained digests |
| **L2: Carrier** | Python object overhead + JSON serialization in hot path | Code32 byte tensors with vectorized integer operations |
| **L3: Realization** | Autoregressive left-to-right token prediction for output | IR-conditioned diffusion with MaskIntent constraints |

After Layer 3, the transformer's remaining role is:
- **Optional intake parsing**: Text → IR hypotheses (can be replaced by rule-based parsers for constrained domains)
- **Optional advisory scoring**: Value function hints for search ordering (non-authoritative per Neural Usage Contract)

---

## 8. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [light_vs_full.md](light_vs_full.md) | Superseded. Historical context for the original Light/Full framing. |
| [code32_bytestate.md](code32_bytestate.md) | Defines Layer 2's substrate. |
| [bytestate_compilation_boundary.md](bytestate_compilation_boundary.md) | Defines Layer 2's codec boundary. |
| [neural_usage_contract.md](neural_usage_contract.md) | Constrains neural component usage across all layers. |
| [Philosophy](../capability_primitives_bundle/philosophy.md) | Provides the contract shape template used by Layer 2's compilation boundary and Layer 3's realization contracts. |
| [Capability Campaign Plan](../../analysis/capability_campaign_plan.md) | Defines the cert-grade domain surfaces that benchmark Layer 0 and Layer 2. |
| [SWM Contract](../../canonical/semantic_working_memory_contract_v0.md) | Canonical contract for Layer 1. |
| [Generative Inversion](../../working/experiments/diffusion/generative-inversion.md) | Architecture proposal for Layer 3. |
| [Theory Document](../../theory/linguistic/sterling_full.md) | Reconstruction templates (Colen/Hawkins/Frazier) that inform Layer 3 conditioning. |
| [Toy Domains](../../working/experiments/worlds/toy-domains.md) | Capability axes that Layer 0 must prove across. |
| [Induced Domains](../../working/experiments/worlds/induced-domain.md) | Dynamic domain induction that tests Layer 2's compilation boundary. |

---

## 9. Summary

Sterling is not "a symbolic reasoning engine that optionally compresses its output." It is a four-layer architecture where each layer replaces a different function that current AI systems delegate to autoregressive transformers:

1. **Reasoning** — governed search, not chain-of-thought
2. **Memory** — semantic working memory, not KV cache
3. **Carrier** — byte tensors, not Python objects
4. **Realization** — IR-conditioned diffusion, not left-to-right prediction

The layers are independently valuable and strictly ordered by dependency. Layers 0 and 1 are operational. Layer 2 is specified. Layer 3 is experimental. The internals must land before the I/O can be replaced.

The end state is a complete cognitive architecture — understanding, reasoning, remembering, and generating — with no transformer in the loop. The transformer becomes what it should have been all along: an optional, non-authoritative codec at the edges, not the skeleton of thought.
