> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Sterling Light vs Full

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

> **Fully Superseded (2026-02-16)**: This document's "Light vs Full" framing has been superseded by the four-layer architecture definition in [Sterling Architecture Layers](sterling_architecture_layers.md). The original framing treated "Full" as "Light + compression," but the actual architecture is four independent layers (Reasoning, Memory, Carrier, Realization), each replacing a different transformer dependency. See [Code32 and ByteStateV1](code32_bytestate.md) for the carrier substrate and [Sterling Architecture Layers](sterling_architecture_layers.md) for the complete system definition. As of 2026-02-20, CoreML/ANE is not an active runtime target; performance work is centered on the Rust + Code32/ByteState path. This document is retained for historical context only.

---

## Overview

Sterling is organized in a layered architecture, with different versions building on the same core:

| Layer | Description | Focus |
|-------|-------------|-------|
| **Sterling Core** | Domain-agnostic reasoning engine (IR + search) | UtteranceState IR, S/M/P/K/C operators, StateGraph search, pluggable world interface, latent rep. |
| **Sterling Light** | Core + a linguistic micro-world + symbolic rules | Proof-of-concept in a constrained domain with minimal ML components. |
| **Sterling Full** | Light + latent compression + on-device deployment | Adds RGBA latent encoding and a ViT-based compressor, enabling Apple Silicon (GPU/ANE) execution and mobile (CoreML) deployment. |

## Sterling Core

**Sterling Core** is the theoretical foundation:
- Defines the IR structures
- Defines the operator taxonomy (S/M/P/K/C)
- Defines the search algorithm
- Defines interfaces for domain knowledge

It is **domain-agnostic** - the same core can operate across different domains via pluggable world adapters.

## Sterling Light

**Sterling Light** is the first implementation of the Core:
- Applied to a small linguistics domain (CAWS) with a symbolic knowledge graph and rules
- Aims to prove the concept with **as little machine learning as possible**
- Focus is on correctness, interpretability, and ensuring the symbolic system works end-to-end

Sterling Light demonstrates that:
- Semantics can live in IR + KG, not transformer weights
- Path algebra + edge-relative plasticity can guide search
- Compression-gated decay and episodic summaries work as structural objects
- The transformer can be an optional skin, not the skeleton

## Sterling Full

**Sterling Full** extends Light by introducing:
- A **semantic compression layer** (to condense the IR into a compact latent)
- **On-device deployment** via Apple Silicon GPUs or Neural Engines

Key technical components:
- **Vision Transformer (ViT)** encoder for IR compression
- **RGBA token grid** representation for efficient encoding
- **CoreML** export for mobile deployment

## Critical Distinction: Full Adds Compression, NOT Cognitive Authority

**Sterling Full does NOT change the reasoning logic.**

The same decisions and results should occur as in Sterling Light, as long as the compression is faithful. Sterling Full is an experiment in:
- **Information compression**: How much can we shrink intermediate representations and still maintain accuracy?
- **Deployment efficiency**: Can we do all this in real-time on consumer hardware?

The ViT encoder in Sterling Full is **non-authoritative**:
- It compresses and indexes, but does not make decisions
- It cannot emit new operators, mutate KG/UtteranceState, or introduce new facts
- The symbolic layer remains the source of truth

This is the key invariant: **Full = more efficient, not more neural.** Adding compression does not mean adding cognitive authority to neural components.

## Comparison Table

| Aspect | Sterling Light | Sterling Full |
|--------|----------------|---------------|
| **Reasoning** | Symbolic (IR + KG + operators) | Symbolic (same as Light) |
| **Neural components** | Minimal (parsing, explanation) | + ViT compression |
| **Compression** | None | RGBA latent encoding |
| **Deployment** | CPU-based | GPU/ANE optimized |
| **Cognitive authority** | Symbolic only | Symbolic only (unchanged) |
| **Neural role** | I/O codec | I/O codec + compression |

## Why This Matters

If "Sterling Full = more neural = less Sterling," we've failed. The whole point is to prove that:

1. Structured semantic search outperforms token-context scaling for reasoning tasks
2. Neural components can be confined to I/O and compression without becoming the cognitive core
3. Efficiency gains come from better structure, not bigger models

Sterling Full succeeds only if it maintains the same reasoning guarantees as Light while being faster and more deployable.
