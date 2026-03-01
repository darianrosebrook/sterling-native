---
status: Accepted
authority: adr
date: 2026-02-23
---
# ADR 0002: Binary Traces are the Canonical Evidence Artifacts

## Decision

Binary trace formats are the canonical persisted evidence for replay verification and certification. Structured JSON transcripts are deterministic derived views rendered from binary traces.

Sterling Native has two evidence layers, each following this pattern:

| Layer | Binary trace (canonical) | Derived view | Equivalence check |
|-------|--------------------------|--------------|-------------------|
| **Carrier** | `ByteTraceV1` (`.bst1`) | (no separate derived view — replay operates directly on trace frames) | `replay_verify()` against compiled state |
| **Search** | `SearchTapeV1` (`.stap`) | `SearchGraphV1` (`search_graph.json`) | Cert-mode tape→graph byte equivalence |

## Rationale

- Single-source-of-truth trace format prevents “graph vs trace” drift.
- Enables bit-identical replay checks.
- Makes proofs and artifacts composable across worlds.
- Chain-hash integrity in binary traces provides tamper detection at parse time.

## Consequences

- All certified runs must emit binary traces (ByteTrace for carrier, SearchTape for search).
- Any visualization or analysis layer (e.g., SearchGraphV1) must be derived, not authoritative.
- Cert verification requires tape→graph equivalence, proving the derived view is faithful.
