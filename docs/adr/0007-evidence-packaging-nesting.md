---
status: Accepted
authority: adr
date: 2026-02-28
context: "Rust produces ArtifactBundleV1 (content-addressed, fail-closed verification). Python produces H2 evidence bundles and TD-12 certificates (governance attestation). Without a defined relationship, both evolve as peers, creating a permanent 'which evidence is canonical?' ambiguity."
---
# ADR 0007: Evidence Packaging Relationship (Nesting, Not Parallel)

## Decision

**TD-12 certificates and H2 evidence bundles must import (and bind to) the Rust ArtifactBundleV1 digest basis as a required substrate artifact.**

Governance sits on top of a Rust bundle, never parallel to it.

Concretely:

1. **Nesting**: A Python governance artifact (H2 bundle, TD-12 cert, campaign record) that makes claims about execution integrity **must include** the referenced `ArtifactBundleV1` bundle digest (or digest basis) as a required field. The governance artifact is a wrapper; the Rust bundle is the substrate.

2. **No independent re-encoding**: Python governance must not independently compute content hashes, manifest projections, or digest bases for artifacts that Rust already packages. Python references the Rust-computed digests. This eliminates the "translation tax" where both systems hash the same bytes differently.

3. **Disjoint claim scopes**: The Rust bundle covers execution integrity (content hashes, manifest, digest basis, verification pipeline). The Python wrapper covers governance semantics (campaign identity, promotion verdicts, evaluator settings, acceptance criteria). No claim appears in both.

4. **Verification is composable**: To verify a governance claim, first verify the referenced Rust bundle (via `verify_bundle_with_profile()`), then verify the governance wrapper's own integrity. If the Rust bundle fails, the governance claim is invalid.

## Rationale

- The alternative (parallel, disjoint scope) requires a "no overlapping claims" rule that is fragile and hard to enforce mechanically. Nesting makes overlap structurally impossible.
- Rust bundles are already the most mature verification primitive. Building governance on top of them (rather than beside them) leverages existing infrastructure.
- This matches ADR 0006: Python is the governance control plane; Rust is the evidence substrate. Nesting is the packaging expression of that authority split.

## Consequences

- Python H2/TD-12 schemas must be extended to include a `substrate_bundle_digest` (or equivalent) required field.
- Python governance tooling cannot produce valid certs without a Rust bundle to reference.
- Future Rust evidence surfaces (e.g., operator registry artifact) are automatically covered by the nesting — they're part of the Rust bundle, and the Python wrapper references the bundle digest.
- This makes the cross-codebase equivalence harness essential: if Rust bundles are the substrate for Python governance, Python must be able to consume and verify Rust bundle digests.
