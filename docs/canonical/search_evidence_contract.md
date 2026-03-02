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
| `tool_transcript.json` | Yes | Tool interaction transcript with stage/commit/rollback audit trail | Tool worlds only (gated by `tool_transcript_v1` obligation) |
| `epistemic_transcript.json` | Yes | Epistemic replay transcript with belief evolution and invariant evidence | Epistemic worlds only (gated by `epistemic_transcript_v1` obligation) |
| `trace.bst1` | **No** (observational) | Carrier-level ByteTrace for compile/apply replay | When carrier trace is enabled |

**Artifact count**: 8 (UniformScorer) or 9 (TableScorer) for worlds without evidence obligations. Tool worlds add `tool_transcript.json` (+1). Epistemic worlds add `epistemic_transcript.json` (+1). The presence of conditional artifacts is gated by the world's declared `evidence_obligations` in `fixture.json`.

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

### Tool transcript verification (Step 19, conditional)

Step 19 is gated by `evidence_obligations` in `fixture.json`, not by operator registry contents.

19a. **Obligation gating** (Cert only): If obligations include `"tool_transcript_v1"`, `tool_transcript.json` must be present. Missing → `ToolTranscriptMissing`.

19b. **Digest binding** (both profiles, required-if-present): Report `tool_transcript_digest` must match `tool_transcript.json` content hash. Mismatch → `ToolTranscriptDigestMismatch`.

19c. **Structural integrity** (both profiles): `entry_count` must match entries array length; `step_index` must be monotonic and in-bounds. Violation → `ToolTranscriptEntryCountMismatch`.

19d. **Equivalence render** (Cert only): Independently render transcript from tape + operator registry, serialize to canonical JSON bytes, compare byte-for-byte. Mismatch → `ToolTranscriptEquivalenceMismatch`.

19e. **Trace-order audit** (Cert only): Verify no layer-0 writes before COMMIT and no writes after ROLLBACK. Violation → `ToolTranscriptTraceOrderViolation`.

**Belt-and-suspenders** (Cert only): If tape contains tool operator frames but `evidence_obligations` does not include `"tool_transcript_v1"`, fails with `ObligationMismatch`.

### Epistemic transcript verification (Step 20, conditional)

Step 20 is gated by `evidence_obligations` including `"epistemic_transcript_v1"`.

20a. **Obligation gating** (Cert only): If obligations include `"epistemic_transcript_v1"`, `epistemic_transcript.json` must be present. Missing → `EpistemicTranscriptMissing`.

20b. **Digest binding** (both profiles, required-if-present): Report `epistemic_transcript_digest` must match `epistemic_transcript.json` content hash. Mismatch → `EpistemicTranscriptDigestMismatch`. If transcript is present but report field is absent → `EpistemicTranscriptDigestMissing`.

20c. **Structural integrity** (both profiles): `entry_count` must match entries array length. Violation → `EpistemicTranscriptEntryCountMismatch`.

20d. **Equivalence render** (Cert only): Independently render transcript via winning-path replay (using the compiled root state from Step 12d), serialize to canonical JSON bytes, compare byte-for-byte. Mismatch → `EpistemicTranscriptEquivalenceMismatch`. This is stronger than Step 19d because it re-executes the operator sequence rather than scanning tape records, simultaneously verifying both transcript content and replay correctness.

### Winning-path replay witness (Step 21, Cert only, conditional)

Step 21 is gated by `evidence_obligations` including `"winning_path_replay_v1"`. Cert only.

21a. **Path extraction**: Reconstruct goal path from tape `NodeCreation` parent chain, walking backwards from goal to root.

21b. **Edge uniqueness**: For each parent→child pair on the winning path, exactly one `Applied` candidate must exist in the parent's expansion. Zero matches → `ReplayEdgeMissing`. Multiple matches → `ReplayEdgeAmbiguous`.

21c. **Sequential apply**: Starting from compiled root ByteState (obtained from Step 12d's compilation replay), re-execute each operator on the winning path via `apply()`.

21d. **Fingerprint verification**: After each apply, compute state fingerprint and compare to tape's `NodeCreation` fingerprint for the child node. Mismatch → `ReplayFingerprintMismatch`.

21e. **World-specific invariants**: At each step, invoke the world's `ReplayInvariantChecker`. For partial observability worlds, this checks: no truth-layer writes (INV-POBS-01), feedback correctness (INV-POBS-03), belief monotonicity (INV-POBS-02), and declare correctness (INV-POBS-04). Violation → `WinningPathReplayFailed`.

21f. **Strict belief decrease**: At least one feedback step must strictly decrease belief cardinality. No decrease → `WinningPathReplayFailed`.

Note: Steps 20d and 21 share a single replay pass. The epistemic transcript is rendered as a replay visitor output, so one replay simultaneously proves transcript equivalence and winning-path correctness.

### Binding direction conventions

Not all artifacts participate in corridor binding the same way:

**Upstream bindings** (values known at search start or during search): fixture digest, policy digest, registry digests, scorer digest, operator set digest. These use 3-point binding (graph metadata + tape header + report) because the values are available before or during search execution.

**Downstream derived artifacts** (rendered from the authoritative trace after search completes): `tool_transcript.json`, `epistemic_transcript.json`, and future derived artifacts. These must NOT be placed in upstream surfaces — doing so creates a dependency cycle (the tape bytes would need to contain a digest of content derived from those same tape bytes). Downstream artifacts bind via:

1. **Normative artifact commitment**: included in the digest basis (Steps 1–6), providing primary integrity binding.
2. **Report convenience field**: digest pointer in `verification_report.json` (e.g., `tool_transcript_digest`, `epistemic_transcript_digest`). Required-if-present in Base, mandatory in Cert when obligation declared.
3. **Cert equivalence render**: verifier independently renders the artifact from authoritative sources and asserts byte-identical match. For tool transcripts, this renders from tape + registry. For epistemic transcripts, this renders via winning-path replay from compiled root state + tape + registry.

This convention applies to all future derived-from-tape artifacts: declare an evidence obligation, commit via digest basis, bind a convenience digest in the report, and prove correspondence via Cert equivalence render.

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
- **Obligation-gated checks** (when declared by the world):
  - Tool transcript equivalence: independently rendered transcript matches shipped artifact byte-for-byte (Step 19d). Trace-order audit verifies stage/commit/rollback protocol compliance (Step 19e).
  - Epistemic transcript equivalence: independently rendered transcript via replay matches shipped artifact byte-for-byte (Step 20d).
  - Winning-path replay: re-executes operators from compiled root state, verifies fingerprints at each step, and checks world-specific semantic invariants (Step 21).

Cert is the promotion-eligible profile. A bundle that passes Cert verification provides a complete, tamper-evident evidence chain from compilation inputs through search execution to search outputs, with correspondence proofs for any declared evidence obligations.

---

## Design rationale

**Why two evidence formats (tape + graph)?** The tape is optimized for the hot loop: small records, binary format, minimal allocation. The graph is optimized for analysis: structured JSON, named fields, human-readable. Cert mode proves they're equivalent, so consumers can use whichever format suits their needs with confidence that both tell the same story.

**Why fail-closed on missing fields?** When tape is present, it asserts a verification contract. Missing binding fields would create silent gaps where tampering is undetectable. Every field is required, and absence is an error.

**Why bind tape headers to authoritative artifacts, not to the report?** The report is a derived summary. If a tampered report self-declares matching values, binding to the report proves nothing. Binding tape headers to the authoritative artifacts (graph metadata, policy content hash, scorer content hash) ensures independent verification.

**Why is `trace.bst1` observational?** The trace envelope may carry non-deterministic metadata (timestamps, process IDs) in production use. Making it normative would break deterministic bundle digests. Instead, its payload-level commitments are captured in the normative verification report.
