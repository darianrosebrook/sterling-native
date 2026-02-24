---
status: "Accepted (Design target)"
authority: adr
date: 2026-02-23
---
# ADR 0002: ByteTrace is the Canonical Trace Artifact

## Decision

ByteTrace is the canonical persisted trace format for replay verification and certification.
StateGraph is a deterministic derived view rendered from ByteTrace.

## Rationale

- Single-source-of-truth trace format prevents “graph vs trace” drift.
- Enables bit-identical replay checks.
- Makes proofs and artifacts composable across worlds.

## Consequences

- All certified runs must emit ByteTrace.
- Any visualization layer must be derived, not authoritative.
