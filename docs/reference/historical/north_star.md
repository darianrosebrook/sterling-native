---
authority: reference
status: advisory (historical context)
---
# Sterling's North Star

**Advisory — historical context.** This thesis document remains valid. The path-finding thesis is realized in v2 as best-first graph search over compiled ByteState with deterministic tape evidence. See `search/src/search.rs`, [`docs/canonical/search_evidence_contract.md`](../../canonical/search_evidence_contract.md).

---

## Sterling is NOT a language model.

Sterling is a **path-finding system over semantic state space**, where:

| Concept | Definition |
|---------|------------|
| **Nodes** | Meaningful states (ByteState snapshots, search nodes) |
| **Edges** | Typed moves (registered operators with contracts) |
| **Learning** | Path-level credit assignment (what edges help from what regions) |
| **Memory** | Compression-gated landmarks + durable provenance |
| **Language** | I/O, not cognition (IR intake + explanation only) |

## What This Means

Sterling's core cognition is:
- **Search**: best-first graph search over compiled state space
- **Path algebra**: edge-relative plasticity, credit assignment along trajectories
- **Evidence**: proof-carrying artifacts (tape, graph, bundle) with replay verification
- **Operators**: typed state transitions with registered contracts and fail-closed dispatch
- **Value heads**: learned heuristics for move ordering (advisory only)

Sterling's interfaces are:
- **IR parser**: LLM for text → structured IR (advisory, never authoritative)
- **NL surface generator**: LLM for rendering explanations (advisory)
- **Encoder**: optional compression for latent indexing (advisory)

The transformer is a **replaceable codec** — just one module that plugs into a more general semantic engine. This is the opposite of "the transformer is the mind, everything else is tools."

## The Wikipedia Game Analogy

Sterling should be able to play "All Links Lead to Rome" on its own state space:

- **Nodes** = ByteState snapshots (compiled semantic states)
- **Edges** = operator-applied transitions (registered, typed, fail-closed)
- **Goal** = reach target state within budget
- **Episodes** = one search session through the state space, recorded as tape + graph
- **Evidence** = the route you took, cryptographically bound into a verifiable bundle

Over time, Sterling:
- Discovers **landmarks** ("if we can get here, the goal is easy")
- Learns **good paths** ("from Biology → Europe → Rome")
- Identifies **dead zones** (regions that rarely lead to goal in budget)

You don't memorize word sequences, you memorize **routes**. You eventually compress whole swaths of experience into **"get to X, and you're basically there."**

## What Sterling Replaces

Sterling explicitly tries to **replace LLMs as the reasoning substrate** (the cognitive core / long-horizon state machine).

Sterling does NOT try to replace LLMs at **language generation** (surface form production).

This is the key distinction: transformers handle the translation between symbols and sentences, but the semantic navigation — the actual reasoning — happens in the governed search substrate.
