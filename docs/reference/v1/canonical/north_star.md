> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**
>
> **v2 realization**: The thesis here (path-finding over semantic state space) is implemented in v2 as best-first graph search over compiled ByteState with deterministic tape evidence. See `search/src/search.rs`, `docs/canonical/search_evidence_contract.md`. The Wikipedia Pathfinder analogy maps to: ByteState nodes = "pages," operator-applied transitions = "links," tape = "the route you took."

# Sterling's North Star

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

---

## Sterling is NOT a language model.

Sterling is a **path-finding system over semantic state space**, where:

| Concept | Definition |
|---------|------------|
| **Nodes** | Meaningful states (UtteranceState, WorldState, summaries/landmarks) |
| **Edges** | Typed moves (operators) |
| **Learning** | Path-level credit assignment (what edges help from what regions) |
| **Memory** | Compression-gated landmarks + durable provenance |
| **Language** | I/O, not cognition (IR intake + explanation only) |

## What This Means

Sterling's core cognition is:
- **KG** (knowledge graph with typed nodes/edges)
- **Path algebra** (edge-relative plasticity, credit assignment along trajectories)
- **SWM** (Semantic Working Memory with bounded activation)
- **Value head** (learned heuristics for move ordering)
- **Operators** (typed state transitions with contracts)

Sterling's interfaces are:
- **IR parser** (LLM for text -> structured IR)
- **NL surface generator** (LLM for rendering explanations)
- **Encoder** (optional compression for latent indexing)

The transformer is a **replaceable codec** - just one module that plugs into a more general semantic engine. This is the opposite of "the transformer is the mind, everything else is tools."

## The Wikipedia Game Analogy

Sterling should be able to play "All Links Lead to Rome" on its own KG:

- **Nodes** = concepts (Wikipedia pages / KG entities)
- **Edges** = relations (hyperlinks / typed edges)
- **Goal** = reach target state in N steps
- **Episodes** = one reasoning session through the KG

Over time, Sterling:
- Discovers **landmarks** ("if we can get here, the goal is easy")
- Learns **good paths** ("from Biology -> Europe -> Rome")
- Identifies **dead zones** (regions that rarely lead to goal in budget)

You don't memorize word sequences, you memorize **routes**. You don't need to re-read every page every time; you remember **"go via this concept class"**. You eventually compress whole swaths of experience into **"get to X, and you're basically there."**

## What Sterling Replaces

Sterling explicitly tries to **replace LLMs as the reasoning substrate** (the cognitive core / long-horizon state machine).

Sterling does NOT try to replace LLMs at **language generation** (surface form production).

This is the key distinction: transformers handle the translation between symbols and sentences, but the semantic navigation - the actual reasoning - happens in the graph.
