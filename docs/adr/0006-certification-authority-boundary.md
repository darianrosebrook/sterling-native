---
status: Accepted
authority: adr
date: 2026-02-28
context: "Sterling has two v2 codebases: sterling-native (Rust, deterministic execution corridor) and sterling (Python, ML/governance/orchestration). Without an explicit authority boundary, two quasi-authorities emerge that can disagree on what constitutes 'certified.'"
---
# ADR 0006: Certification Authority Boundary (Python Control Plane, Rust Evidence)

## Decision

**Python is the certification control plane. Rust is the evidence generator and verifier.**

Concretely:

1. **Rust produces and verifies execution evidence**: ArtifactBundleV1, ByteTrace, SearchTapeV1, SearchGraphV1, VerificationProfile (Base/Cert). Rust code is the sole authority for execution integrity, search determinism, and bundle verification.

2. **Python issues governance claims**: Campaign definitions, promotion verdicts, gate results, certification attestations (H2/TD-12). Python is the sole authority for "this operator was promoted," "this campaign passed," "this run is certified for deployment."

3. **The interface contract**: Every Python certification claim that depends on execution/search integrity **must reference a Rust-verified artifact set by digest**. A Python cert is valid only if the referenced Rust bundle passes `verify_bundle_with_profile()` at the claimed verification profile. If the Rust bundle fails verification, the Python cert is invalid regardless of what the Python-side governance claims.

4. **No overlapping claims**: Rust does not issue governance verdicts. Python does not independently verify execution integrity (it delegates to Rust verification). Neither codebase re-encodes claims the other already covers.

5. **Migration path**: If governance surfaces later move into Rust (Option B from the parity audit), that is a deliberate phase change requiring a new ADR, not emergent creep.

## Rationale

- The retrospective's "governance is the system" lesson means the boundary between evidence and governance must be explicit, not implicit.
- Without this, Python certs could assert "certified" while the referenced Rust bundle fails verification — a split-brain failure mode.
- The compilation boundary (ADR 0001) already established that the Rust kernel is the sole runtime truth. This ADR extends that principle to the cross-codebase boundary.

## Consequences

- Python governance tooling must include Rust bundle digests in certification artifacts.
- Python must be able to invoke Rust verification (via CLI, FFI, or shared fixture) to validate referenced bundles.
- The cross-codebase equivalence harness (parity audit §Cross-Codebase Equivalence Harness) becomes a hard requirement, not aspirational.
- New Rust evidence surfaces (e.g., operator registry artifact) automatically fall under Rust's authority; Python references them by digest.
