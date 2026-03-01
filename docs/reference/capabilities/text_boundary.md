---
authority: reference
status: advisory
date: 2026-03-01
capability: text_boundary
parity_audit_sections: "H1"
---

# Text Boundary and Realization

**Advisory — not normative.** This document describes proof obligations for future v2 work. Do not cite as canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability status.

## Overview

Sterling processes text at a trust boundary: surface-level text (natural language input/output) is non-authoritative, while internal representations (IR) carry authority for reasoning. This separation is a foundational design principle (ADR 0003 establishes that neural components are advisory, not authoritative). This reference captures the proof obligations for implementing that boundary in the native substrate.

The sterling Python repo defined three overlapping text contracts: a trust-boundary contract with Realizer specifications, a three-layer TextIR with hole-filling mechanics, and a four-partition Linguistic IR with operator witnesses. This document does not prescribe which design carries forward.

## Key Concepts

### Trust Boundary

The central invariant: **surface text is advisory; IR is authoritative.** No reasoning path may treat raw surface text as ground truth. Surface text enters the system through a parse boundary that produces IR; IR exits through a render boundary that produces surface text. Neither boundary is authoritative on its own — authority lives in the IR and the operators that transform it.

This maps to the existing native substrate pattern: worlds produce ByteState (authoritative) from payloads (advisory input), and operators transform ByteState under witnessed contracts. Text processing would follow the same shape: parse produces IR-as-ByteState, operators transform it, render produces advisory output.

### Intermediate Representation Layers

The sterling Python repo explored multi-layer IR designs:

- **Surface layer**: Tokenized text, spans, raw linguistic features. Non-authoritative.
- **Syntax/Structure layer**: Parse trees, dependency structures, constituent analysis. Structurally authoritative but semantically incomplete.
- **Semantic/Committed layer**: Resolved references, bound entities, committed meaning. Authoritative for reasoning.
- **Frontier layer**: Speculative interpretations, implicature candidates, figurative readings. Advisory only.

The key design question is granularity: how many IR layers does the native substrate need to enforce the trust boundary? The minimal answer is two (advisory surface vs authoritative IR). The maximal answer is four (matching the sterling Python repo's linguistic IR contract).

### Hole Mechanism

Text IR may contain holes — unresolved positions that require further processing. A hole is a typed placeholder with constraints on what can fill it. Holes make partial processing explicit: an IR with holes is structurally valid but semantically incomplete, and the system can reason about what remains unresolved.

In the native substrate, holes are analogous to ByteState slots in the `Hole` status — a position that exists in the schema but has no committed value. The same slot lifecycle (Hole → Provisional → Committed) could model text IR completion.

### Operator Witnesses for IR Transforms

Every IR transformation (filling a hole, resolving a reference, committing an interpretation) should be a witnessed operator. The sterling Python repo's `OperatorWitness` and `LinguisticDeltaPatch` types capture this: each transform records what changed, what operator caused it, and the resulting state digest.

This aligns with the existing `apply()` contract where operators declare effects via `EffectKind` and post-apply validation confirms the declaration.

## Design Decisions (Open)

| Decision | Options | Constraint |
|----------|---------|------------|
| IR layer count | 2 (surface/authority) or 4 (surface/structure/committed/frontier) | Must enforce trust boundary; more layers adds expressiveness but complexity |
| IR as ByteState or new type | Encode text IR into ByteState slots or define a parallel TextStateV1 | ByteState reuse gets existing verification for free; new type allows text-specific optimizations |
| Realizer location | Rust (native render) or Python (external render with artifact handoff) | Python realizer is pragmatic short-term; Rust realizer is the long-term target per ADR 0006 |
| Hole representation | Reuse slot Hole status or define TextHoleV1 | Slot Hole is proven but may be too coarse for linguistic holes |
| Budget profiles | Reuse SearchPolicyV1 budget model or define TextPolicyV1 | Text processing has different resource profiles than search |

## Proof Obligations for v2

1. **Trust boundary enforcement.** No code path allows surface text to be treated as authoritative input to reasoning operators. This is enforced by type system (distinct Surface vs IR types), not by convention.

2. **Parse boundary produces content-addressed IR.** The output of parsing is a content-addressed IR artifact. Given the same input and parse configuration, the output is deterministic and verifiable.

3. **Render boundary is advisory.** The output of rendering is explicitly marked non-authoritative. No downstream consumer may treat rendered text as a certified claim.

4. **IR transforms are witnessed operators.** Every IR mutation is performed by a registered operator with a declared effect kind. The transform produces a delta artifact recording the change.

5. **Holes are explicit.** Unresolved positions in IR are represented as typed holes, not as absent data. The system can enumerate all holes in a given IR state.

6. **IR artifacts are bundle-compatible.** Text IR artifacts can be included in an ArtifactBundleV1 with content hashes, normative/observational classification, and digest-basis participation.

7. **Minimal demonstration.** Before building the full IR pipeline, demonstrate the trust boundary with a minimal example: parse advisory input into authoritative IR, transform IR with a witnessed operator, render advisory output. The demonstration must produce a verifiable evidence bundle.

## Parity Audit Reference

This document covers capability **H1** (Text IO boundary, IR partitions) from the [parity audit](../../architecture/v1_v2_parity_audit.md).

Current status: **Not started.** The parity audit notes the open decision of whether to implement the sterling Python repo's four-partition IR or design a new realization pipeline.

Import obligations from the parity audit (Import Group F):
- A minimal text boundary demo: parse/render components as advisory, never authority
- A verifiable realization artifact surface (even if the realizer remains Python)

See also capability **H2** (Discourse / speech act contracts) in the parity audit, covered by the companion [discourse reference](discourse.md).
