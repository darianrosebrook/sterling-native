---
status: v2 canonical
authority: canonical
date: 2026-02-27
supersedes: "v1 proof_evidence_system — see docs/reference/capabilities/governance.md for remaining proof obligations"
---
# Search Evidence Contract

This document defines the evidence model for search bundles in Sterling Native: what artifacts are produced, what each verification profile guarantees, and how the verification pipeline works.

---

## Search bundle artifacts

A search bundle produced by `run_search()` contains the following artifacts:

| Artifact | Normative | Content | Present |
|----------|-----------|---------|---------|
| `fixture.json` | Yes | Compiled world fixture (ByteState, schema, registry) | Always |
| `compilation_manifest.json` | Yes | Compilation boundary evidence (schema descriptor, registry digest, compile parameters) | Always |
| `policy_snapshot.json` | Yes | Frozen policy configuration used for the run | Always |
| `search_graph.json` | Yes | Canonical search transcript (SearchGraphV1) | Always |
| `search_tape.stap` | Yes | Binary hot-loop event log (SearchTapeV1) with chain-hash integrity | Always |
| `verification_report.json` | Yes | Binding digests cross-referencing all other artifacts | Always |
| `operator_registry.json` | Yes | Operator catalog with op IDs, signatures, effect kinds, and contract metadata | Always |
| `concept_registry.json` | Yes | Code32↔ConceptID bijective mapping (RegistryV1 canonical bytes) | Always |
| `scorer.json` | Yes | Scorer configuration and digest (TableScorer only) | Table scorer only |
| `trace.bst1` | **No** (observational) | Carrier-level ByteTrace for compile/apply replay | When carrier trace is enabled |

**Artifact count**: 8 (UniformScorer) or 9 (TableScorer).

**Normative vs observational**: Normative artifacts participate in the bundle digest (via digest basis). Observational artifacts are present in the manifest but excluded from the digest. `trace.bst1` is observational because its envelope may carry non-deterministic metadata; its payload-level commitments (`payload_hash`, `step_chain_digest`) are captured in the normative verification report.

---

## Verification profiles

| Profile | Tape required | What it checks | When to use |
|---------|---------------|----------------|-------------|
| **Base** | No (tape checks run only if tape present) | Integrity + bindings + scorer coherence | Default. Backward-compatible with pre-tape bundles. |
| **Cert** | Yes (`TapeMissing` if absent) | Base + tape→graph canonical byte equivalence | Promotion-eligible verification. |

`verify_bundle()` uses Base. `verify_bundle_with_profile(bundle, Cert)` uses Cert.

---

## Verification pipeline

The pipeline runs as a sequence of numbered steps. Each step is fail-closed: a failure stops verification and returns a typed `BundleVerifyError`. Steps are ordered so that earlier steps catch structural issues before later steps attempt semantic checks.

### Integrity checks (Steps 1–6)

1. **Content hash**: For every artifact, recompute `canonical_hash(DOMAIN_BUNDLE_ARTIFACT, content)` and compare to stored `content_hash`. Mismatch → `ContentHashMismatch`.
2. **Manifest consistency**: Recompute manifest from artifact hashes; compare to stored manifest bytes. Mismatch → `ManifestMismatch`. Non-canonical → `ManifestNotCanonical`.
3. **Digest basis consistency**: Recompute normative projection (sorted normative artifact hashes); compare to stored digest basis. Mismatch → `DigestBasisMismatch`. Non-canonical → `DigestBasisNotCanonical`.
4. **Bundle digest**: Recompute `canonical_hash(DOMAIN_BUNDLE_DIGEST, digest_basis)` and compare to stored digest. Mismatch → `DigestMismatch`.
5. **Canonical form**: Every normative JSON artifact must be in canonical JSON form. Violation → `ArtifactNotCanonical`.
6. **Trace integrity** (if `trace.bst1` present): Parse trace, recompute `payload_hash` and `step_chain_digest`, compare to report values. Mismatch → `PayloadHashMismatch` or `StepChainMismatch`.

### Report binding checks (Steps 7–11)

7. **Report parse**: `verification_report.json` must be valid JSON. Failure → `ReportParseError`.
8. **Policy digest**: Recompute content hash of `policy_snapshot.json`, compare to report's `policy_digest`. Mismatch → `PolicyDigestMismatch`.
9. **Search graph digest**: Recompute content hash of `search_graph.json`, compare to report's `search_graph_digest`. Mismatch → `SearchGraphDigestMismatch`. Missing field → `SearchGraphDigestMissing`.
10. **Mode coherence**: If `search_graph.json` exists, report `mode` must be `"search"`. Missing/wrong mode → `ModeMissing` / `ModeSearchExpected`.
11. **World ID binding**: Report `world_id` must match graph metadata `world_id`. Mismatch → `MetadataBindingWorldIdMismatch`.

### Metadata binding checks (Steps 12–12d)

12. **Policy binding**: Graph metadata `policy_snapshot_digest` must match `policy_snapshot.json` content hash (as hex without `sha256:` prefix). Mismatch → `MetadataBindingPolicyMismatch`.

12b. **Compilation manifest coherence**: Cross-verify `compilation_manifest.json` fields against graph metadata and verification report: schema descriptor, payload hash, registry hash, root identity digest, root evidence digest. Missing fields → `CompilationManifestMissingField` / `CompilationManifestGraphMissingField`. Mismatches → `CompilationManifestMismatch`.

12c. **Concept registry artifact binding**: Recompute semantic digest of `concept_registry.json` (via `HashDomain::RegistrySnapshot`) and verify it matches both `compilation_manifest.json.registry_hash` and graph metadata `registry_digest`. In Cert mode, `concept_registry.json` must be present (fail-closed). Missing → `ConceptRegistryMissing`. Mismatch → `ConceptRegistryDigestMismatch`.

12d. **Compilation boundary replay** (Cert only): Parse `concept_registry.json` via `RegistryV1::from_canonical_bytes()`, extract schema descriptor from `compilation_manifest.json`, recompile the fixture via `compile()`, and verify the resulting ByteState matches the bundle's `fixture.json`. This proves the compilation boundary is reproducible from bundle-shipped inputs alone. Parse failure → `CompilationReplayRegistryParseFailed`. Schema mismatch → `CompilationReplaySchemaDescriptorMismatch`. Compile failure → `CompilationReplayCompileFailed`. State mismatch → `CompilationReplayStateMismatch`.

### Scorer coherence checks (Steps 13–15)

13. **Scorer artifact binding**: If report has `scorer_digest`, `scorer.json` must exist and its content hash must match. Mismatch → `ScorerDigestMismatch`. Artifact missing → `ScorerArtifactMissing`.
14. **Scorer digest in graph metadata**: Bidirectional — if either `scorer.json` artifact or graph metadata `scorer_digest` exists, both must be present and content hash must match. Mismatch → `MetadataBindingScorerMismatch` / `MetadataBindingScorerMissing`.
15. **Candidate score source coherence**: If `scorer.json` exists and search completed normally with expansions, at least one candidate must reference `ModelDigest` matching the bound scorer digest. Mismatch → `CandidateScoreSourceScorerMismatch`. No evidence → `ScorerEvidenceMissing`.

### Operator registry binding (Steps 16–17)

16. **Operator set digest binding**: If report has `operator_set_digest`, `operator_registry.json` must exist and its content hash must match. If `operator_registry.json` exists, report `operator_set_digest` is mandatory. Missing → `OperatorRegistryDigestMissing`. Artifact missing → `OperatorRegistryArtifactMissing`. Mismatch → `OperatorRegistryDigestMismatch`.
17. **Operator set digest in graph metadata**: Graph metadata `operator_set_digest` must match `operator_registry.json` content hash (hex). Missing → `MetadataBindingOperatorRegistryMissing`. Mismatch → `MetadataBindingOperatorRegistryMismatch`.

### Tape verification (Step 18, when tape present)

18a. **Tape presence** (Cert only): If profile is Cert and tape absent → `TapeMissing`. If profile is Base and tape absent → pass (skip remaining tape checks).

18b. **Tape digest binding**: Report `tape_digest` must be present and must match `search_tape.stap` content hash. Missing → `ReportFieldMissing`. Mismatch → `TapeDigestMismatch`.

18c. **Tape parse + chain hash**: Parse tape via `read_tape()`. This internally verifies the chain hash across all records. Parse failure or chain hash mismatch → `TapeParseFailed`.

18d. **Header binding against authoritative artifacts** (fail-closed — all fields required when tape present):
- `world_id` ↔ graph metadata `world_id`
- `registry_digest` ↔ graph metadata `registry_digest`
- `search_policy_digest` ↔ graph metadata `search_policy_digest`
- `root_state_fingerprint` ↔ graph metadata `root_state_fingerprint`
- `policy_snapshot_digest` ↔ `policy_snapshot.json` content hash (hex)
- `scorer_digest` ↔ `scorer.json` content hash (if scorer present; mode coherence enforced)
- `operator_set_digest` ↔ `operator_registry.json` content hash (hex)
- Mismatch on any field → `TapeHeaderBindingMismatch`

18e. **Schema version**: Tape header `schema_version` must be `"search_tape.v1"`. Missing or wrong → `TapeParseFailed`.

18f. **Tape→graph equivalence** (Cert only): Render tape to `SearchGraphV1` via `render_graph()`, serialize to canonical JSON bytes, compare byte-for-byte to `search_graph.json` content. Mismatch → `TapeGraphEquivalenceMismatch`.

---

## What Base guarantees

When `verify_bundle()` passes (Base profile):

- Every artifact's content is intact (content hash verified).
- The bundle's structural integrity is sound (manifest, digest basis, bundle digest all consistent).
- All cross-artifact binding digests match (report ↔ graph ↔ policy ↔ scorer ↔ operator registry ↔ concept registry ↔ compilation manifest).
- Compilation manifest fields are coherent with graph metadata and the verification report (schema, payload, registry, root state digests).
- If a tape is present, its chain hash is intact, its header binds correctly to authoritative artifacts, and its digest matches the report.
- The bundle was produced by a coherent pipeline — no artifact was substituted or tampered.

Base does **not** guarantee tape presence or concept registry presence.

## What Cert adds

When `verify_bundle_with_profile(bundle, Cert)` passes:

- Everything Base guarantees, plus:
- Tape **must** be present (fail-closed on absence).
- `concept_registry.json` **must** be present (fail-closed on absence).
- Compilation boundary replay succeeds: recompiling from bundle-shipped inputs (registry bytes, schema descriptor, fixture payload) produces byte-identical ByteState.
- Tape and graph describe identical search behavior (tape→graph canonical byte equivalence).
- The tape is the evidence spine; the graph is a verified derived view.

Cert is the promotion-eligible profile. A bundle that passes Cert verification provides a complete, tamper-evident evidence chain from compilation inputs through search execution to search outputs.

---

## Design rationale

**Why two evidence formats (tape + graph)?** The tape is optimized for the hot loop: small records, binary format, minimal allocation. The graph is optimized for analysis: structured JSON, named fields, human-readable. Cert mode proves they're equivalent, so consumers can use whichever format suits their needs with confidence that both tell the same story.

**Why fail-closed on missing fields?** When tape is present, it asserts a verification contract. Missing binding fields would create silent gaps where tampering is undetectable. Every field is required, and absence is an error.

**Why bind tape headers to authoritative artifacts, not to the report?** The report is a derived summary. If a tampered report self-declares matching values, binding to the report proves nothing. Binding tape headers to the authoritative artifacts (graph metadata, policy content hash, scorer content hash) ensures independent verification.

**Why is `trace.bst1` observational?** The trace envelope may carry non-deterministic metadata (timestamps, process IDs) in production use. Making it normative would break deterministic bundle digests. Instead, its payload-level commitments are captured in the normative verification report.
