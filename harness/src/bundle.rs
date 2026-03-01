//! In-memory artifact bundle: the output of a harness run.
//!
//! No file I/O in this module. The bundle is a deterministic in-memory
//! representation that can be inspected programmatically.
//!
//! # Normative vs observational artifacts
//!
//! Each artifact is tagged `normative` (participates in bundle digest)
//! or observational (present in the manifest but excluded from digest).
//!
//! `trace.bst1` is observational because it contains the envelope, which
//! carries non-deterministic metadata in production runs. Its payload-level
//! commitments (payload hash, step chain digest) are captured in the
//! normative verification report.
//!
//! The bundle digest is computed over the **digest basis**: a canonical
//! JSON projection of normative artifact hashes only.

use std::collections::BTreeMap;

use sterling_kernel::carrier::bytestate::SchemaDescriptor;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::carrier::trace_reader::bytes_to_trace;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, ContentHash, HashDomain};
use sterling_kernel::proof::trace_hash::{payload_hash, step_chain};
use sterling_search::tape_reader::read_tape;
use sterling_search::tape_render::render_graph;

/// Domain prefix for bundle artifact content hashing (harness-originated).
pub const DOMAIN_BUNDLE_ARTIFACT: HashDomain = HashDomain::BundleArtifact;

/// Domain prefix for bundle digest computation (harness-originated).
pub const DOMAIN_BUNDLE_DIGEST: HashDomain = HashDomain::BundleDigest;

/// Domain prefix for harness fixture hashing (harness-originated).
pub const DOMAIN_HARNESS_FIXTURE: HashDomain = HashDomain::HarnessFixture;

/// Domain prefix for codebook hash computation (harness-originated).
pub const DOMAIN_CODEBOOK_HASH: HashDomain = HashDomain::CodebookHash;

/// A single artifact in the bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleArtifact {
    /// Logical filename (e.g., `"fixture.json"`, `"trace.bst1"`).
    pub name: String,
    /// Raw bytes of the artifact.
    pub content: Vec<u8>,
    /// Content hash: `canonical_hash(DOMAIN_BUNDLE_ARTIFACT, content)`.
    pub content_hash: ContentHash,
    /// Whether this artifact participates in the bundle digest.
    pub normative: bool,
}

/// The complete artifact bundle from a harness run.
///
/// In-memory only (M3 non-goal: no disk persistence).
/// All JSON artifacts use kernel's `canonical_json_bytes`.
#[derive(Debug, Clone)]
pub struct ArtifactBundleV1 {
    /// Artifacts indexed by logical name, in sorted order (`BTreeMap`).
    pub artifacts: BTreeMap<String, BundleArtifact>,
    /// Full manifest: canonical JSON listing all artifacts with normative flags.
    pub manifest: Vec<u8>,
    /// Digest basis: canonical JSON listing normative artifact hashes only.
    pub digest_basis: Vec<u8>,
    /// Bundle digest: `canonical_hash(DOMAIN_BUNDLE_DIGEST, digest_basis)`.
    pub digest: ContentHash,
}

/// Error building a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleBuildError {
    /// Canonical JSON serialization failed.
    CanonError { detail: String },
    /// Caller-provided `precomputed_hash` does not match recomputed hash.
    PrecomputedHashMismatch {
        name: String,
        expected: String,
        computed: String,
    },
}

/// Input for bundle assembly.
///
/// Callers provide artifact bytes and metadata; bundle assembly computes
/// or reuses content hashes. If `precomputed_hash` is provided, it must
/// be `canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &content)` — the same domain
/// separator and algorithm that `build_bundle` would compute.
pub struct ArtifactInput {
    /// Logical filename (e.g., `"fixture.json"`, `"search_graph.json"`).
    pub name: String,
    /// Raw bytes of the artifact.
    pub content: Vec<u8>,
    /// Whether this artifact participates in the bundle digest.
    pub normative: bool,
    /// If provided, `build_bundle` reuses this hash instead of recomputing.
    pub precomputed_hash: Option<ContentHash>,
}

impl From<(String, Vec<u8>, bool)> for ArtifactInput {
    fn from((name, content, normative): (String, Vec<u8>, bool)) -> Self {
        Self {
            name,
            content,
            normative,
            precomputed_hash: None,
        }
    }
}

/// Build an `ArtifactBundleV1` from a list of artifact inputs.
///
/// Computes content hashes (or reuses precomputed ones), builds the sorted
/// manifest and digest basis, and derives the bundle digest. All JSON via
/// kernel's `canonical_json_bytes`.
///
/// Accepts `Vec<ArtifactInput>` or `Vec<(String, Vec<u8>, bool)>` (via `From`).
///
/// # Errors
///
/// Returns [`BundleBuildError`] if canonical JSON serialization fails.
pub fn build_bundle(
    artifacts: Vec<impl Into<ArtifactInput>>,
) -> Result<ArtifactBundleV1, BundleBuildError> {
    let mut artifact_map = BTreeMap::new();

    for input in artifacts {
        let input = input.into();
        let content_hash = match input.precomputed_hash {
            Some(h) => {
                let recomputed = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &input.content);
                if h != recomputed {
                    return Err(BundleBuildError::PrecomputedHashMismatch {
                        name: input.name,
                        expected: h.as_str().to_string(),
                        computed: recomputed.as_str().to_string(),
                    });
                }
                h
            }
            None => canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &input.content),
        };
        artifact_map.insert(
            input.name.clone(),
            BundleArtifact {
                name: input.name,
                content: input.content,
                content_hash,
                normative: input.normative,
            },
        );
    }

    let manifest = compute_manifest_bytes(&artifact_map)
        .map_err(|detail| BundleBuildError::CanonError { detail })?;

    let digest_basis = compute_digest_basis_bytes(&artifact_map)
        .map_err(|detail| BundleBuildError::CanonError { detail })?;

    let digest = canonical_hash(DOMAIN_BUNDLE_DIGEST, &digest_basis);

    Ok(ArtifactBundleV1 {
        artifacts: artifact_map,
        manifest,
        digest_basis,
        digest,
    })
}

/// Error from bundle integrity verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleVerifyError {
    /// An artifact's stored `content_hash` does not match recomputed hash.
    ContentHashMismatch {
        artifact: String,
        expected: String,
        actual: String,
    },
    /// Stored `manifest` bytes do not match recomputed manifest from artifacts.
    ManifestMismatch,
    /// Stored `manifest` bytes are not in canonical JSON form.
    ManifestNotCanonical,
    /// Stored `digest_basis` bytes do not match recomputed normative projection.
    DigestBasisMismatch,
    /// Stored `digest_basis` bytes are not in canonical JSON form.
    DigestBasisNotCanonical,
    /// Stored `digest` does not match recomputed hash of `digest_basis`.
    DigestMismatch { expected: String, actual: String },
    /// A normative JSON artifact is not in canonical JSON form.
    ArtifactNotCanonical { artifact: String },
    /// `trace.bst1` failed to parse.
    TraceParseError { detail: String },
    /// Trace hashing failed.
    TraceHashError { detail: String },
    /// `verification_report.json` is not valid JSON.
    ReportParseError { detail: String },
    /// Recomputed `payload_hash` from `trace.bst1` does not match report.
    PayloadHashMismatch {
        declared: String,
        recomputed: String,
    },
    /// Recomputed `step_chain_digest` from `trace.bst1` does not match report.
    StepChainMismatch {
        declared: String,
        recomputed: String,
    },
    /// Report is missing a required field.
    ReportFieldMissing { field: String },
    /// Recomputed `policy_digest` from `policy_snapshot.json` does not match report.
    PolicyDigestMismatch {
        declared: String,
        recomputed: String,
    },
    /// Recomputed `search_graph_digest` from `search_graph.json` does not match report.
    SearchGraphDigestMismatch {
        declared: String,
        recomputed: String,
    },
    /// `search_graph.json` and `verification_report.json` both exist but the
    /// report does not declare a `search_graph_digest` field.
    SearchGraphDigestMissing,
    /// Report declares `mode = "search"` but `search_graph.json` is absent.
    SearchGraphArtifactMissing,
    /// `search_graph.json` exists but report `mode` is not `"search"`.
    ModeSearchExpected { actual_mode: String },
    /// `search_graph.json` exists but report has no `mode` field.
    ModeMissing,
    /// `policy_snapshot_digest` in `search_graph.json` metadata does not match
    /// `policy_snapshot.json`'s `content_hash`.
    MetadataBindingPolicyMismatch { in_graph: String, in_policy: String },
    /// `search_graph.json` metadata is missing the mandatory `policy_snapshot_digest` field.
    MetadataBindingPolicyMissing,
    /// `world_id` in `search_graph.json` metadata does not match
    /// `verification_report.json`'s `world_id`.
    MetadataBindingWorldIdMismatch { in_graph: String, in_report: String },
    /// `search_graph.json` metadata is missing the mandatory `world_id` field.
    MetadataBindingWorldIdMissing,
    /// `verification_report.json` is missing the mandatory `world_id` field
    /// when `search_graph.json` is present.
    ReportWorldIdMissing,
    /// Report declares `scorer_digest` but `scorer.json` artifact is missing.
    ScorerArtifactMissing,
    /// `scorer.json` recomputed content hash does not match report `scorer_digest`.
    ScorerDigestMismatch {
        declared: String,
        recomputed: String,
    },
    /// `scorer.json` artifact exists but report is missing `scorer_digest`.
    ScorerDigestMissing,
    /// `scorer_digest` in `search_graph.json` metadata does not match
    /// `scorer.json`'s `content_hash`.
    MetadataBindingScorerMismatch { in_graph: String, in_scorer: String },
    /// `search_graph.json` metadata has `scorer_digest` but `scorer.json` is absent.
    MetadataBindingScorerMissing,
    /// A candidate score source `ModelDigest` does not match the bound scorer digest.
    CandidateScoreSourceScorerMismatch {
        candidate_digest: String,
        bound_digest: String,
    },
    /// `scorer.json` artifact exists, the search completed normally with expansions,
    /// but no candidate record references `ModelDigest`. Allowed when
    /// `total_expansions == 0` (root-is-goal) or termination is scorer-failure.
    ScorerEvidenceMissing {
        total_expansions: u64,
        termination_reason: String,
    },
    /// Report declares `operator_set_digest` but `operator_registry.json` artifact is missing.
    OperatorRegistryArtifactMissing,
    /// `operator_registry.json` recomputed content hash does not match report
    /// `operator_set_digest`.
    OperatorRegistryDigestMismatch { declared: String, recomputed: String },
    /// `operator_registry.json` artifact exists but report is missing
    /// `operator_set_digest`.
    OperatorRegistryDigestMissing,
    /// `operator_set_digest` in `search_graph.json` metadata does not match
    /// `operator_registry.json`'s `content_hash`.
    MetadataBindingOperatorRegistryMismatch { in_graph: String, in_artifact: String },
    /// `search_graph.json` metadata has `operator_set_digest` but
    /// `operator_registry.json` is absent.
    MetadataBindingOperatorRegistryMissing,
    /// `fixture_digest` in `search_graph.json` metadata does not match
    /// `fixture.json`'s `content_hash`.
    MetadataBindingFixtureMismatch { in_graph: String, in_fixture: String },
    /// `search_graph.json` metadata has `fixture_digest` but
    /// `fixture.json` is absent, or `fixture_digest` is missing from metadata.
    MetadataBindingFixtureMissing,
    /// `compilation_manifest.json` is missing when `search_graph.json` is present.
    CompilationManifestMissing,
    /// `compilation_manifest.json` failed JSON parsing.
    CompilationManifestNotJson { detail: String },
    /// A required field is missing from `compilation_manifest.json`.
    CompilationManifestMissingField { field: &'static str },
    /// `schema_descriptor` in graph metadata does not match compilation manifest's
    /// `schema_id:schema_version:schema_hash`.
    CompilationManifestSchemaMismatch { in_graph: String, in_manifest: String },
    /// `fixture.json` is missing when compilation manifest coherence check fires.
    CompilationManifestFixtureMissing,
    /// A required field is missing from `fixture.json` during compilation manifest
    /// coherence checking.
    CompilationManifestFixtureMissingField { field: &'static str },
    /// `initial_payload_hex` in `fixture.json` failed hex decoding.
    CompilationManifestPayloadDecodeFailed { detail: String },
    /// `initial_payload_hex` in `fixture.json` is not valid JSON after decoding.
    CompilationManifestPayloadNotJson { detail: String },
    /// `payload_hash` in compilation manifest does not match recomputed hash
    /// from `fixture.json.initial_payload_hex`.
    CompilationManifestPayloadMismatch { in_manifest: String, recomputed: String },
    /// `registry_hash` in compilation manifest (stripped to raw hex) does not match
    /// `registry_digest` in `search_graph.json` metadata.
    CompilationManifestRegistryMismatch { in_manifest_hex: String, in_graph_hex: String },
    /// `identity_digest` in compilation manifest (stripped to raw hex) does not match
    /// `root_identity_digest` in `search_graph.json` metadata.
    CompilationManifestIdentityMismatch { in_manifest_hex: String, in_graph_hex: String },
    /// `evidence_digest` in compilation manifest (stripped to raw hex) does not match
    /// `root_evidence_digest` in `search_graph.json` metadata.
    CompilationManifestEvidenceMismatch { in_manifest_hex: String, in_graph_hex: String },
    /// A required field is missing from `search_graph.json` metadata during
    /// compilation manifest coherence checking.
    CompilationManifestGraphMissingField { field: &'static str },
    /// `concept_registry.json` is missing when required (Cert profile).
    ConceptRegistryMissing,
    /// `concept_registry.json` semantic digest does not match
    /// `compilation_manifest.json.registry_hash`.
    ConceptRegistryDigestMismatch { in_artifact: String, in_manifest: String },
    /// Cert: `evidence_obligations` includes `tool_transcript_v1` but
    /// `tool_transcript.json` is absent from the bundle.
    ToolTranscriptMissing,
    /// `tool_transcript.json` content hash does not match report
    /// `tool_transcript_digest`.
    ToolTranscriptDigestMismatch { declared: String, recomputed: String },
    /// Cert: independently rendered transcript does not byte-match the
    /// shipped `tool_transcript.json`.
    ToolTranscriptEquivalenceMismatch {
        expected_hash: String,
        rendered_hash: String,
    },
    /// `tool_transcript.json` `entry_count` does not match entries array length.
    ToolTranscriptEntryCountMismatch { declared: usize, actual: usize },
    /// `tool_transcript.json` `step_index` values are invalid.
    ToolTranscriptStepIndexInvalid { detail: String },
    /// `tool_transcript_digest` expected in report but missing.
    ToolTranscriptDigestMissing,
    /// Cert: tape contains tool operator frames but `evidence_obligations`
    /// does not include `tool_transcript_v1`.
    ObligationMismatch { detail: String },
    /// Cert: a write to committed layer occurred before `OP_COMMIT`,
    /// or a write occurred after `OP_ROLLBACK`.
    CommittedWriteOrderViolation { step_index: usize, detail: String },
    /// Canonical JSON error during verification.
    CanonError { detail: String },
    /// Cert profile requires `search_tape.stap` but it is absent.
    TapeMissing,
    /// `search_tape.stap` failed to parse (wraps underlying `TapeParseError`).
    TapeParseFailed { source: String },
    /// Tape header field does not match the authoritative artifact/graph metadata value.
    TapeHeaderBindingMismatch {
        field: &'static str,
        in_tape: String,
        in_artifact: String,
    },
    /// Tape-rendered `SearchGraphV1` canonical JSON bytes differ from `search_graph.json`
    /// (Cert profile only).
    TapeGraphEquivalenceMismatch,
    /// `tape_digest` in `verification_report.json` does not match
    /// `search_tape.stap`'s `content_hash`.
    TapeDigestMismatch { declared: String, recomputed: String },
    /// `search_tape.stap` tape render to `SearchGraphV1` failed.
    TapeRenderFailed { source: String },
    /// `concept_registry.json` bytes failed `RegistryV1::from_canonical_bytes()`.
    CompilationReplayRegistryParseFailed { detail: String },
    /// `compile()` returned `CompilationFailure` when replaying from bundle inputs.
    CompilationReplayCompileFailed { detail: String },
    /// Replayed `compilation_manifest` bytes differ from stored
    /// `compilation_manifest.json`. Reports content hashes of both blobs.
    CompilationReplayManifestMismatch {
        expected_hash: String,
        recomputed_hash: String,
    },
}

/// Verification profile controlling tape evidence requirements.
///
/// `Base` (default): everyday verification; if tape present, verify parse +
/// chain hash + header bindings. Skips tape→graph equivalence.
///
/// `Cert`: certification-grade verification; requires tape presence, verifies
/// everything in Base plus tape→graph canonical equivalence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerificationProfile {
    /// Tape verification is optional. If tape present, verify parse + bindings.
    #[default]
    Base,
    /// Tape required. Full verification including tape→graph equivalence.
    Cert,
}

/// Verify the internal consistency of a bundle using the default `Base` profile.
///
/// Equivalent to `verify_bundle_with_profile(bundle, VerificationProfile::Base)`.
///
/// # Errors
///
/// Returns the first [`BundleVerifyError`] encountered.
pub fn verify_bundle(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    verify_bundle_with_profile(bundle, VerificationProfile::Base)
}

/// Verify the internal consistency of a bundle with an explicit verification profile.
///
/// This is a pure integrity check — it does NOT run `replay_verify()`.
/// It proves:
///
/// 1. Each artifact's `content_hash` matches `canonical_hash(DOMAIN_BUNDLE_ARTIFACT, content)`.
/// 2. `manifest` bytes match the canonical JSON projection recomputed from all artifacts.
/// 3. `digest_basis` bytes match the canonical JSON projection recomputed from normative
///    artifacts only.
/// 4. `digest` matches `canonical_hash(DOMAIN_BUNDLE_DIGEST, digest_basis)`.
/// 5. `manifest` and `digest_basis` are in canonical JSON form.
/// 6. Normative JSON artifacts (`.json` extension + `normative: true`) are in canonical form.
/// 7. If both `trace.bst1` and `verification_report.json` exist: `payload_hash` and
///    `step_chain_digest` recomputed from `trace.bst1` match the values declared in the report.
/// 8. If both `policy_snapshot.json` and `verification_report.json` exist: `policy_digest`
///    in the report matches `policy_snapshot.json`'s `content_hash`.
/// 9. `codebook_hash` in the verification report is NOT verified (diagnostic field).
/// 10. Tape verification (if `search_tape.stap` present or Cert profile):
///     - `tape_digest` binding: report field matches artifact `content_hash`
///     - Parse tape: chain hash integrity verified by reader
///     - Header binding: fields match authoritative artifacts/graph metadata
///     - Mode coherence: `scorer_digest` presence matches scorer mode
///     - (Cert only) Tape→graph equivalence: rendered graph bytes == `search_graph.json`
///
/// # Errors
///
/// Returns the first [`BundleVerifyError`] encountered.
pub fn verify_bundle_with_profile(
    bundle: &ArtifactBundleV1,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    // Step 1: Verify each artifact's content_hash.
    for artifact in bundle.artifacts.values() {
        let recomputed = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &artifact.content);
        if recomputed.as_str() != artifact.content_hash.as_str() {
            return Err(BundleVerifyError::ContentHashMismatch {
                artifact: artifact.name.clone(),
                expected: artifact.content_hash.as_str().to_string(),
                actual: recomputed.as_str().to_string(),
            });
        }
    }

    // Step 2: Recompute manifest from artifacts and compare byte-for-byte.
    let expected_manifest = compute_manifest_bytes(&bundle.artifacts)
        .map_err(|detail| BundleVerifyError::CanonError { detail })?;
    if expected_manifest != bundle.manifest {
        return Err(BundleVerifyError::ManifestMismatch);
    }

    // Step 3: Verify manifest is canonical JSON.
    verify_canonical_json(&bundle.manifest)
        .map_err(|()| BundleVerifyError::ManifestNotCanonical)?;

    // Step 4: Recompute digest_basis from normative artifacts and compare.
    let expected_basis = compute_digest_basis_bytes(&bundle.artifacts)
        .map_err(|detail| BundleVerifyError::CanonError { detail })?;
    if expected_basis != bundle.digest_basis {
        return Err(BundleVerifyError::DigestBasisMismatch);
    }

    // Step 5: Verify digest_basis is canonical JSON.
    verify_canonical_json(&bundle.digest_basis)
        .map_err(|()| BundleVerifyError::DigestBasisNotCanonical)?;

    // Step 6: Verify bundle digest.
    let recomputed_digest = canonical_hash(DOMAIN_BUNDLE_DIGEST, &bundle.digest_basis);
    if recomputed_digest.as_str() != bundle.digest.as_str() {
        return Err(BundleVerifyError::DigestMismatch {
            expected: bundle.digest.as_str().to_string(),
            actual: recomputed_digest.as_str().to_string(),
        });
    }

    // Step 7: Verify normative JSON artifacts are canonical.
    for artifact in bundle.artifacts.values() {
        let is_json = std::path::Path::new(&artifact.name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
        if artifact.normative && is_json {
            verify_canonical_json(&artifact.content).map_err(|()| {
                BundleVerifyError::ArtifactNotCanonical {
                    artifact: artifact.name.clone(),
                }
            })?;
        }
    }

    // Step 8: If trace.bst1 and verification_report.json both exist,
    // recompute payload commitments from trace and compare to report.
    verify_trace_report_binding(bundle)?;

    // Step 9: If policy_snapshot.json and verification_report.json both exist,
    // verify policy_digest in report matches policy artifact's content_hash.
    verify_policy_digest_binding(bundle)?;

    // Step 10: If search_graph.json and verification_report.json both exist,
    // verify search_graph_digest in report matches search_graph.json's content_hash.
    // (search_graph_digest is mandatory when both artifacts are present)
    verify_search_graph_digest_binding(bundle)?;

    // Step 11: Mode↔artifact coherence for search bundles.
    verify_mode_artifact_coherence(bundle)?;

    // Step 12: Cross-verify metadata bindings in search_graph.json.
    verify_metadata_bindings(bundle, profile)?;

    // Step 12b: Compilation manifest coherence (schema + payload + registry + root digests).
    verify_compilation_manifest_coherence(bundle, profile)?;

    // Step 12c: Concept registry artifact digest binding.
    verify_concept_registry_artifact(bundle, profile)?;

    // Step 12d: Compilation boundary replay (Cert only).
    verify_compilation_replay(bundle, profile)?;

    // Step 13: Scorer digest binding (report ↔ scorer.json).
    verify_scorer_digest_binding(bundle)?;

    // Step 14: Scorer digest in graph metadata ↔ scorer.json.
    verify_metadata_scorer_binding(bundle)?;

    // Step 15: Candidate score source consistency with scorer artifact.
    verify_candidate_scorer_consistency(bundle)?;

    // Step 16: Operator set digest binding (report ↔ operator_registry.json).
    verify_operator_set_digest_binding(bundle)?;

    // Step 17: Operator set digest in graph metadata ↔ operator_registry.json.
    verify_metadata_operator_set_binding(bundle)?;

    // Step 18: Tape verification (profile-dependent).
    verify_tape(bundle, profile)?;

    // Step 19: Tool transcript verification (profile-dependent).
    verify_tool_transcript(bundle, profile)?;

    Ok(())
}

/// Recompute manifest bytes from the artifact map.
fn compute_manifest_bytes(artifacts: &BTreeMap<String, BundleArtifact>) -> Result<Vec<u8>, String> {
    let manifest_artifacts: Vec<serde_json::Value> = artifacts
        .values()
        .map(|a| {
            serde_json::json!({
                "content_hash": a.content_hash.as_str(),
                "name": a.name,
                "normative": a.normative,
            })
        })
        .collect();

    let manifest_value = serde_json::json!({
        "artifacts": manifest_artifacts,
        "schema_version": "bundle.v1",
    });

    canonical_json_bytes(&manifest_value).map_err(|e| format!("{e:?}"))
}

/// Recompute digest basis bytes from normative artifacts only.
fn compute_digest_basis_bytes(
    artifacts: &BTreeMap<String, BundleArtifact>,
) -> Result<Vec<u8>, String> {
    let normative_artifacts: Vec<serde_json::Value> = artifacts
        .values()
        .filter(|a| a.normative)
        .map(|a| {
            serde_json::json!({
                "content_hash": a.content_hash.as_str(),
                "name": a.name,
            })
        })
        .collect();

    let digest_basis_value = serde_json::json!({
        "artifacts": normative_artifacts,
        "schema_version": "bundle_digest_basis.v1",
    });

    canonical_json_bytes(&digest_basis_value).map_err(|e| format!("{e:?}"))
}

/// Verify that JSON bytes are in canonical form (parse → re-canonicalize → compare).
fn verify_canonical_json(bytes: &[u8]) -> Result<(), ()> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| ())?;
    let recanonized = canonical_json_bytes(&value).map_err(|_| ())?;
    if recanonized == bytes {
        Ok(())
    } else {
        Err(())
    }
}

/// If both `trace.bst1` and `verification_report.json` exist, verify that
/// the report's declared `payload_hash` and `step_chain_digest` match
/// values recomputed from the trace payload.
fn verify_trace_report_binding(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    let (Some(trace_artifact), Some(report_artifact)) = (
        bundle.artifacts.get("trace.bst1"),
        bundle.artifacts.get("verification_report.json"),
    ) else {
        return Ok(());
    };

    let trace = bytes_to_trace(&trace_artifact.content).map_err(|e| {
        BundleVerifyError::TraceParseError {
            detail: format!("{e:?}"),
        }
    })?;

    let computed_payload = payload_hash(&trace).map_err(|e| BundleVerifyError::TraceHashError {
        detail: format!("{e:?}"),
    })?;

    let computed_chain = step_chain(&trace).map_err(|e| BundleVerifyError::TraceHashError {
        detail: format!("{e:?}"),
    })?;

    let report: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).map_err(|e| {
            BundleVerifyError::ReportParseError {
                detail: format!("{e:?}"),
            }
        })?;

    let declared_payload =
        report["payload_hash"]
            .as_str()
            .ok_or_else(|| BundleVerifyError::ReportFieldMissing {
                field: "payload_hash".into(),
            })?;

    if computed_payload.as_str() != declared_payload {
        return Err(BundleVerifyError::PayloadHashMismatch {
            declared: declared_payload.to_string(),
            recomputed: computed_payload.as_str().to_string(),
        });
    }

    let declared_chain = report["step_chain_digest"].as_str().ok_or_else(|| {
        BundleVerifyError::ReportFieldMissing {
            field: "step_chain_digest".into(),
        }
    })?;

    if computed_chain.digest.as_str() != declared_chain {
        return Err(BundleVerifyError::StepChainMismatch {
            declared: declared_chain.to_string(),
            recomputed: computed_chain.digest.as_str().to_string(),
        });
    }

    Ok(())
}

/// If both `search_graph.json` and `verification_report.json` exist, verify
/// that the report's `search_graph_digest` matches `search_graph.json`'s `content_hash`.
///
/// The `search_graph_digest` field is **mandatory** when both artifacts are present.
fn verify_search_graph_digest_binding(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    let (Some(graph_artifact), Some(report_artifact)) = (
        bundle.artifacts.get("search_graph.json"),
        bundle.artifacts.get("verification_report.json"),
    ) else {
        return Ok(());
    };

    let report: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).map_err(|e| {
            BundleVerifyError::ReportParseError {
                detail: format!("{e:?}"),
            }
        })?;

    // MANDATORY: search_graph_digest must be present when both artifacts exist.
    let declared_digest = report
        .get("search_graph_digest")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::SearchGraphDigestMissing)?;

    if graph_artifact.content_hash.as_str() != declared_digest {
        return Err(BundleVerifyError::SearchGraphDigestMismatch {
            declared: declared_digest.to_string(),
            recomputed: graph_artifact.content_hash.as_str().to_string(),
        });
    }

    Ok(())
}

/// Mode↔artifact coherence: `mode == "search"` ↔ `search_graph.json` exists.
fn verify_mode_artifact_coherence(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    let Some(report_artifact) = bundle.artifacts.get("verification_report.json") else {
        return Ok(());
    };

    let report: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).map_err(|e| {
            BundleVerifyError::ReportParseError {
                detail: format!("{e:?}"),
            }
        })?;

    let mode = report.get("mode").and_then(|v| v.as_str());
    let has_graph = bundle.artifacts.contains_key("search_graph.json");

    match (mode, has_graph) {
        (Some("search"), false) => Err(BundleVerifyError::SearchGraphArtifactMissing),
        (Some(m), true) if m != "search" => Err(BundleVerifyError::ModeSearchExpected {
            actual_mode: m.to_string(),
        }),
        (None, true) => Err(BundleVerifyError::ModeMissing),
        _ => Ok(()),
    }
}

/// Extract the raw hex portion of a `ContentHash` for graph-metadata binding comparisons.
///
/// Graph metadata stores raw hex (via `hex_digest()`), while verification reports
/// and artifact `content_hash` fields store the full `sha256:hex` format (via `as_str()`).
/// This helper normalizes the comparison for graph-metadata binding fields only.
/// Report-level digest fields (e.g., `search_graph_digest`) use `as_str()` format
/// and are compared directly against `content_hash.as_str()`.
fn binding_hex(hash: &ContentHash) -> &str {
    hash.hex_digest()
}

/// Cross-verify metadata bindings in `search_graph.json` against bundle artifacts.
///
/// Checks:
/// - `metadata.policy_snapshot_digest` == `policy_snapshot.json`'s `content_hash`
/// - `metadata.world_id` == `verification_report.json`'s `world_id`
/// - `metadata.fixture_digest` == `fixture.json`'s `content_hash` (required-if-present
///   in Base; mandatory in Cert)
fn verify_metadata_bindings(
    bundle: &ArtifactBundleV1,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    let Some(graph_artifact) = bundle.artifacts.get("search_graph.json") else {
        return Ok(());
    };

    let graph: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;

    // Cross-verify policy_snapshot_digest (mandatory when search_graph.json exists)
    if let Some(policy_artifact) = bundle.artifacts.get("policy_snapshot.json") {
        let graph_policy_digest = graph
            .get("metadata")
            .and_then(|m| m.get("policy_snapshot_digest"))
            .and_then(|v| v.as_str())
            .ok_or(BundleVerifyError::MetadataBindingPolicyMissing)?;

        let policy_hex = binding_hex(&policy_artifact.content_hash);
        if policy_hex != graph_policy_digest {
            return Err(BundleVerifyError::MetadataBindingPolicyMismatch {
                in_graph: graph_policy_digest.to_string(),
                in_policy: policy_hex.to_string(),
            });
        }
    }

    // Cross-verify world_id against verification report
    if let Some(report_artifact) = bundle.artifacts.get("verification_report.json") {
        let report: serde_json::Value =
            serde_json::from_slice(&report_artifact.content).map_err(|e| {
                BundleVerifyError::ReportParseError {
                    detail: format!("{e:?}"),
                }
            })?;

        let graph_world_id = graph
            .get("metadata")
            .and_then(|m| m.get("world_id"))
            .and_then(|v| v.as_str())
            .ok_or(BundleVerifyError::MetadataBindingWorldIdMissing)?;
        let report_world_id = report
            .get("world_id")
            .and_then(|v| v.as_str())
            .ok_or(BundleVerifyError::ReportWorldIdMissing)?;

        if graph_world_id != report_world_id {
            return Err(BundleVerifyError::MetadataBindingWorldIdMismatch {
                in_graph: graph_world_id.to_string(),
                in_report: report_world_id.to_string(),
            });
        }
    }

    // Cross-verify fixture_digest.
    // Base: required-if-present. Cert: mandatory.
    let graph_fixture_digest = graph
        .get("metadata")
        .and_then(|m| m.get("fixture_digest"))
        .and_then(|v| v.as_str());

    match (graph_fixture_digest, profile) {
        (Some(digest), _) => {
            // Field present: cross-check against fixture.json artifact.
            let fixture_artifact = bundle
                .artifacts
                .get("fixture.json")
                .ok_or(BundleVerifyError::MetadataBindingFixtureMissing)?;
            let fixture_hex = binding_hex(&fixture_artifact.content_hash);
            if digest != fixture_hex {
                return Err(BundleVerifyError::MetadataBindingFixtureMismatch {
                    in_graph: digest.to_string(),
                    in_fixture: fixture_hex.to_string(),
                });
            }
        }
        (None, VerificationProfile::Cert) => {
            // Cert requires fixture_digest in graph metadata.
            return Err(BundleVerifyError::MetadataBindingFixtureMissing);
        }
        (None, VerificationProfile::Base) => {
            // Base: old bundles without fixture_digest pass.
        }
    }

    Ok(())
}

/// Cross-verify compilation manifest fields against graph metadata and fixture.
///
/// Fail-closed: if `search_graph.json` exists, `compilation_manifest.json` MUST
/// exist with valid schema and payload fields. Missing artifacts or fields are
/// typed errors, not silent passes.
///
/// Check 1 — Schema coherence: `compilation_manifest.json`'s
/// `schema_id:schema_version:schema_hash` must equal graph metadata
/// `schema_descriptor`.
///
/// Check 2 — Payload coherence: recomputed
/// `canonical_hash(CompilationPayload, canonical(fixture.initial_payload_hex))`
/// must equal `compilation_manifest.json`'s `payload_hash`.
///
/// Check 3 — Registry digest coherence: `compilation_manifest.json`'s
/// `registry_hash` (stripped of `sha256:` prefix) must equal graph metadata
/// `registry_digest` (raw hex).
#[allow(clippy::too_many_lines)]
fn verify_compilation_manifest_coherence(
    bundle: &ArtifactBundleV1,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    // Only applies to search bundles.
    let Some(graph_artifact) = bundle.artifacts.get("search_graph.json") else {
        return Ok(());
    };

    // Fail-closed: manifest must exist in search bundles.
    let manifest_artifact = bundle
        .artifacts
        .get("compilation_manifest.json")
        .ok_or(BundleVerifyError::CompilationManifestMissing)?;

    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_artifact.content).map_err(|e| {
            BundleVerifyError::CompilationManifestNotJson {
                detail: format!("{e}"),
            }
        })?;

    let graph: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;

    // --- Check 1: Schema coherence ---
    let schema_id = manifest
        .get("schema_id")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "schema_id",
        })?;
    let schema_version = manifest
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "schema_version",
        })?;
    let schema_hash = manifest
        .get("schema_hash")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "schema_hash",
        })?;

    let manifest_sd = format!("{schema_id}:{schema_version}:{schema_hash}");

    let graph_sd = graph
        .get("metadata")
        .and_then(|m| m.get("schema_descriptor"))
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestSchemaMismatch {
            in_graph: "<missing>".into(),
            in_manifest: manifest_sd.clone(),
        })?;

    if graph_sd != manifest_sd {
        return Err(BundleVerifyError::CompilationManifestSchemaMismatch {
            in_graph: graph_sd.to_string(),
            in_manifest: manifest_sd,
        });
    }

    // --- Check 2: Payload coherence ---
    let fixture_artifact = bundle
        .artifacts
        .get("fixture.json")
        .ok_or(BundleVerifyError::CompilationManifestFixtureMissing)?;
    let fixture: serde_json::Value =
        serde_json::from_slice(&fixture_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;

    let payload_hex = fixture
        .get("initial_payload_hex")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestFixtureMissingField {
            field: "initial_payload_hex",
        })?;

    let manifest_payload_hash = manifest
        .get("payload_hash")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "payload_hash",
        })?;

    let payload_bytes = hex::decode(payload_hex).map_err(|e| {
        BundleVerifyError::CompilationManifestPayloadDecodeFailed {
            detail: format!("{e}"),
        }
    })?;
    let payload_json: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(|e| {
            BundleVerifyError::CompilationManifestPayloadNotJson {
                detail: format!("{e}"),
            }
        })?;
    let canonical_payload = canonical_json_bytes(&payload_json)
        .map_err(|e| BundleVerifyError::CanonError {
            detail: format!("{e}"),
        })?;
    let recomputed = canonical_hash(HashDomain::CompilationPayload, &canonical_payload);

    if recomputed.as_str() != manifest_payload_hash {
        return Err(BundleVerifyError::CompilationManifestPayloadMismatch {
            in_manifest: manifest_payload_hash.to_string(),
            recomputed: recomputed.as_str().to_string(),
        });
    }

    // --- Check 3: Registry digest coherence ---
    let manifest_registry_hash = manifest
        .get("registry_hash")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "registry_hash",
        })?;

    // INV-REGCOH-02: registry_hash MUST be ContentHash-format (sha256:<hex>).
    let manifest_registry_hex = manifest_registry_hash
        .strip_prefix("sha256:")
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "registry_hash",
        })?;

    let graph_registry_hex = graph
        .get("metadata")
        .and_then(|m| m.get("registry_digest"))
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestGraphMissingField {
            field: "registry_digest",
        })?;

    if manifest_registry_hex != graph_registry_hex {
        return Err(BundleVerifyError::CompilationManifestRegistryMismatch {
            in_manifest_hex: manifest_registry_hex.to_string(),
            in_graph_hex: graph_registry_hex.to_string(),
        });
    }

    // --- Check 4: Root identity digest coherence ---
    // --- Check 5: Root evidence digest coherence ---
    // Base: required-if-present. Cert: mandatory.
    let graph_metadata = graph.get("metadata");

    let graph_identity = graph_metadata
        .and_then(|m| m.get("root_identity_digest"))
        .and_then(|v| v.as_str());
    let graph_evidence = graph_metadata
        .and_then(|m| m.get("root_evidence_digest"))
        .and_then(|v| v.as_str());

    match (graph_identity, graph_evidence, profile) {
        // Both present: cross-check against manifest.
        (Some(g_id), Some(g_ev), _) => {
            let manifest_identity = manifest
                .get("identity_digest")
                .and_then(|v| v.as_str())
                .ok_or(BundleVerifyError::CompilationManifestMissingField {
                    field: "identity_digest",
                })?;
            let manifest_identity_hex = manifest_identity
                .strip_prefix("sha256:")
                .ok_or(BundleVerifyError::CompilationManifestMissingField {
                    field: "identity_digest",
                })?;
            if manifest_identity_hex != g_id {
                return Err(BundleVerifyError::CompilationManifestIdentityMismatch {
                    in_manifest_hex: manifest_identity_hex.to_string(),
                    in_graph_hex: g_id.to_string(),
                });
            }

            let manifest_evidence = manifest
                .get("evidence_digest")
                .and_then(|v| v.as_str())
                .ok_or(BundleVerifyError::CompilationManifestMissingField {
                    field: "evidence_digest",
                })?;
            let manifest_evidence_hex = manifest_evidence
                .strip_prefix("sha256:")
                .ok_or(BundleVerifyError::CompilationManifestMissingField {
                    field: "evidence_digest",
                })?;
            if manifest_evidence_hex != g_ev {
                return Err(BundleVerifyError::CompilationManifestEvidenceMismatch {
                    in_manifest_hex: manifest_evidence_hex.to_string(),
                    in_graph_hex: g_ev.to_string(),
                });
            }
        }
        // Cert: both must be present.
        (None, _, VerificationProfile::Cert) => {
            return Err(BundleVerifyError::CompilationManifestGraphMissingField {
                field: "root_identity_digest",
            });
        }
        (_, None, VerificationProfile::Cert) => {
            return Err(BundleVerifyError::CompilationManifestGraphMissingField {
                field: "root_evidence_digest",
            });
        }
        // Base: absent fields are ok (older bundles without root digests).
        _ => {}
    }

    Ok(())
}

/// Verify `concept_registry.json` semantic digest against corridor claims.
///
/// When `concept_registry.json` is present, recomputes the semantic digest
/// using `HashDomain::RegistrySnapshot` and checks it matches:
/// - `compilation_manifest.json.registry_hash` (full `sha256:` string)
/// - `search_graph.json.metadata.registry_digest` (raw hex)
///
/// Profile posture:
/// - Base: required-if-present (skip if artifact absent)
/// - Cert: mandatory (fail-closed if absent in search bundles)
fn verify_concept_registry_artifact(
    bundle: &ArtifactBundleV1,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    // Only applies to search bundles.
    if !bundle.artifacts.contains_key("search_graph.json") {
        return Ok(());
    }

    let registry_artifact = bundle.artifacts.get("concept_registry.json");

    match (registry_artifact, profile) {
        (None, VerificationProfile::Cert) => {
            return Err(BundleVerifyError::ConceptRegistryMissing);
        }
        (None, VerificationProfile::Base) => return Ok(()),
        _ => {}
    }

    let reg_art = registry_artifact.expect("presence checked above");

    // Recompute semantic digest from artifact bytes.
    let semantic_digest = canonical_hash(HashDomain::RegistrySnapshot, &reg_art.content);

    // Check against compilation manifest registry_hash (full sha256:hex string).
    let manifest_artifact = bundle
        .artifacts
        .get("compilation_manifest.json")
        .ok_or(BundleVerifyError::CompilationManifestMissing)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_artifact.content).map_err(|e| {
            BundleVerifyError::CompilationManifestNotJson {
                detail: format!("{e}"),
            }
        })?;
    let manifest_registry_hash = manifest
        .get("registry_hash")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "registry_hash",
        })?;

    if semantic_digest.as_str() != manifest_registry_hash {
        return Err(BundleVerifyError::ConceptRegistryDigestMismatch {
            in_artifact: semantic_digest.as_str().to_string(),
            in_manifest: manifest_registry_hash.to_string(),
        });
    }

    // Check against graph metadata registry_digest (raw hex).
    let graph_artifact = bundle.artifacts.get("search_graph.json").unwrap();
    let graph: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;
    let graph_registry_hex = graph
        .get("metadata")
        .and_then(|m| m.get("registry_digest"))
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestGraphMissingField {
            field: "registry_digest",
        })?;

    if semantic_digest.hex_digest() != graph_registry_hex {
        return Err(BundleVerifyError::ConceptRegistryDigestMismatch {
            in_artifact: semantic_digest.hex_digest().to_string(),
            in_manifest: graph_registry_hex.to_string(),
        });
    }

    Ok(())
}

/// Replay the compilation boundary from bundle contents (Cert only).
///
/// Reconstructs `compile()` inputs from shipped artifacts:
/// - `fixture.json.initial_payload_hex` → payload bytes
/// - `compilation_manifest.json` schema fields → `SchemaDescriptor`
/// - `concept_registry.json` bytes → `RegistryV1` via `from_canonical_bytes()`
///
/// Calls `compile()` and asserts the output `compilation_manifest` bytes are
/// identical to the stored `compilation_manifest.json`. This is the strongest
/// compilation boundary check: it proves the entire manifest is reproducible.
///
/// Base profile skips entirely. Cert is mandatory for search bundles.
fn verify_compilation_replay(
    bundle: &ArtifactBundleV1,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    // Only applies to search bundles.
    if !bundle.artifacts.contains_key("search_graph.json") {
        return Ok(());
    }

    // Base: skip replay entirely.
    if profile == VerificationProfile::Base {
        return Ok(());
    }

    // --- Reconstruct registry ---
    let registry_artifact = bundle
        .artifacts
        .get("concept_registry.json")
        .ok_or(BundleVerifyError::ConceptRegistryMissing)?;
    let registry = RegistryV1::from_canonical_bytes(&registry_artifact.content).map_err(|e| {
        BundleVerifyError::CompilationReplayRegistryParseFailed {
            detail: format!("{e:?}"),
        }
    })?;

    // --- Reconstruct schema descriptor ---
    let manifest_artifact = bundle
        .artifacts
        .get("compilation_manifest.json")
        .ok_or(BundleVerifyError::CompilationManifestMissing)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_artifact.content).map_err(|e| {
            BundleVerifyError::CompilationManifestNotJson {
                detail: format!("{e}"),
            }
        })?;

    let schema_id = manifest
        .get("schema_id")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "schema_id",
        })?;
    let schema_version = manifest
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "schema_version",
        })?;
    let schema_hash = manifest
        .get("schema_hash")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestMissingField {
            field: "schema_hash",
        })?;

    let schema_descriptor = SchemaDescriptor {
        id: schema_id.to_string(),
        version: schema_version.to_string(),
        hash: schema_hash.to_string(),
    };

    // --- Reconstruct payload bytes ---
    let fixture_artifact = bundle
        .artifacts
        .get("fixture.json")
        .ok_or(BundleVerifyError::CompilationManifestFixtureMissing)?;
    let fixture: serde_json::Value =
        serde_json::from_slice(&fixture_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;
    let payload_hex = fixture
        .get("initial_payload_hex")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::CompilationManifestFixtureMissingField {
            field: "initial_payload_hex",
        })?;
    let payload_bytes = hex::decode(payload_hex).map_err(|e| {
        BundleVerifyError::CompilationManifestPayloadDecodeFailed {
            detail: format!("{e}"),
        }
    })?;

    // --- Replay compile() ---
    let result = compile(&payload_bytes, &schema_descriptor, &registry).map_err(|e| {
        BundleVerifyError::CompilationReplayCompileFailed {
            detail: format!("{e:?}"),
        }
    })?;

    // --- Compare manifest bytes ---
    if result.compilation_manifest != manifest_artifact.content {
        let expected_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &manifest_artifact.content);
        let recomputed_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &result.compilation_manifest);
        return Err(BundleVerifyError::CompilationReplayManifestMismatch {
            expected_hash: expected_hash.as_str().to_string(),
            recomputed_hash: recomputed_hash.as_str().to_string(),
        });
    }

    Ok(())
}

/// Verify scorer digest binding between report and scorer artifact.
///
/// Fail-closed invariants:
/// - If report has `scorer_digest`, `scorer.json` artifact must exist and match.
/// - If `scorer.json` exists, report `scorer_digest` is mandatory and must match.
fn verify_scorer_digest_binding(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    let report_artifact = bundle.artifacts.get("verification_report.json");
    let scorer_artifact = bundle.artifacts.get("scorer.json");

    // Neither exists → nothing to check (Uniform mode).
    let (Some(report_art), scorer_opt) = (report_artifact, scorer_artifact) else {
        return Ok(());
    };

    let report: serde_json::Value = serde_json::from_slice(&report_art.content).map_err(|e| {
        BundleVerifyError::ReportParseError {
            detail: format!("{e:?}"),
        }
    })?;

    let report_scorer_digest = report.get("scorer_digest").and_then(|v| v.as_str());

    match (report_scorer_digest, scorer_opt) {
        // Report claims scorer_digest but no scorer artifact.
        (Some(_), None) => Err(BundleVerifyError::ScorerArtifactMissing),
        // Scorer artifact exists but report has no scorer_digest.
        (None, Some(_)) => Err(BundleVerifyError::ScorerDigestMissing),
        // Both exist: verify hash match.
        (Some(declared), Some(scorer_art)) => {
            if scorer_art.content_hash.as_str() != declared {
                return Err(BundleVerifyError::ScorerDigestMismatch {
                    declared: declared.to_string(),
                    recomputed: scorer_art.content_hash.as_str().to_string(),
                });
            }
            Ok(())
        }
        // Neither report digest nor artifact: Uniform mode, nothing to check.
        (None, None) => Ok(()),
    }
}

/// Cross-verify `scorer_digest` in `search_graph.json` metadata against `scorer.json`.
fn verify_metadata_scorer_binding(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    let Some(graph_artifact) = bundle.artifacts.get("search_graph.json") else {
        return Ok(());
    };

    let graph: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;

    let graph_scorer_digest = graph
        .get("metadata")
        .and_then(|m| m.get("scorer_digest"))
        .and_then(|v| v.as_str());
    let scorer_artifact = bundle.artifacts.get("scorer.json");

    match (graph_scorer_digest, scorer_artifact) {
        // One side present without the other → fail-closed.
        (Some(_), None) | (None, Some(_)) => Err(BundleVerifyError::MetadataBindingScorerMissing),
        // Both exist: verify match.
        (Some(in_graph), Some(scorer_art)) => {
            let scorer_hex = binding_hex(&scorer_art.content_hash);
            if scorer_hex != in_graph {
                return Err(BundleVerifyError::MetadataBindingScorerMismatch {
                    in_graph: in_graph.to_string(),
                    in_scorer: scorer_hex.to_string(),
                });
            }
            Ok(())
        }
        // Neither: Uniform mode.
        (None, None) => Ok(()),
    }
}

/// Scan `search_graph.json` candidate records for score source consistency.
///
/// Fail-closed invariants:
/// - If any candidate has `ModelDigest`, report/metadata/artifact scorer digests must exist.
/// - Every `ModelDigest(d)` must equal the bound scorer digest.
/// - If scorer artifact exists and no candidate references `ModelDigest`, allow only when
///   `total_expansions == 0` (root-is-goal) or termination is scorer-failure
///   (`scorer_contract_violation` or `internal_panic { stage: score_candidates }`).
fn verify_candidate_scorer_consistency(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    let Some(graph_artifact) = bundle.artifacts.get("search_graph.json") else {
        return Ok(());
    };
    let scorer_artifact = bundle.artifacts.get("scorer.json");

    let graph: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;

    // Collect all model_digest references from candidate score sources.
    let mut model_digests: Vec<String> = Vec::new();
    if let Some(expansions) = graph.get("expansions").and_then(|v| v.as_array()) {
        for expansion in expansions {
            if let Some(candidates) = expansion.get("candidates").and_then(|v| v.as_array()) {
                for candidate in candidates {
                    if let Some(source) = candidate.get("score").and_then(|s| s.get("source")) {
                        if let Some(digest) = source.get("model_digest").and_then(|v| v.as_str()) {
                            model_digests.push(digest.to_string());
                        }
                    }
                }
            }
        }
    }

    let has_model_digests = !model_digests.is_empty();

    // If scorer artifact exists but no candidate references it, check whether
    // the absence is structurally justified (root-is-goal or scorer failure).
    if scorer_artifact.is_some() && !has_model_digests {
        let metadata = &graph["metadata"];
        let total_expansions = metadata["total_expansions"].as_u64().unwrap_or(0);
        let term_reason = &metadata["termination_reason"];
        let term_type = term_reason["type"].as_str().unwrap_or("unknown");

        let scorer_failure = term_type == "scorer_contract_violation"
            || (term_type == "internal_panic"
                && term_reason["stage"].as_str() == Some("score_candidates"));

        if total_expansions > 0 && !scorer_failure {
            return Err(BundleVerifyError::ScorerEvidenceMissing {
                total_expansions,
                termination_reason: term_reason.to_string(),
            });
        }
    }

    // If candidates reference ModelDigest, scorer artifact must exist.
    if has_model_digests && scorer_artifact.is_none() {
        return Err(BundleVerifyError::ScorerArtifactMissing);
    }

    // Verify all ModelDigest values match the bound scorer artifact's content_hash.
    if let Some(scorer_art) = scorer_artifact {
        let bound_digest = scorer_art.content_hash.as_str();
        for candidate_digest in &model_digests {
            if candidate_digest != bound_digest {
                return Err(BundleVerifyError::CandidateScoreSourceScorerMismatch {
                    candidate_digest: candidate_digest.clone(),
                    bound_digest: bound_digest.to_string(),
                });
            }
        }
    }

    Ok(())
}

/// If both `policy_snapshot.json` and `verification_report.json` exist, verify
/// that the report's `policy_digest` matches the policy artifact's `content_hash`.
fn verify_policy_digest_binding(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
    let (Some(policy_artifact), Some(report_artifact)) = (
        bundle.artifacts.get("policy_snapshot.json"),
        bundle.artifacts.get("verification_report.json"),
    ) else {
        return Ok(());
    };

    let report: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).map_err(|e| {
            BundleVerifyError::ReportParseError {
                detail: format!("{e:?}"),
            }
        })?;

    let declared_policy_digest =
        report["policy_digest"]
            .as_str()
            .ok_or_else(|| BundleVerifyError::ReportFieldMissing {
                field: "policy_digest".into(),
            })?;

    if policy_artifact.content_hash.as_str() != declared_policy_digest {
        return Err(BundleVerifyError::PolicyDigestMismatch {
            declared: declared_policy_digest.to_string(),
            recomputed: policy_artifact.content_hash.as_str().to_string(),
        });
    }

    Ok(())
}

/// Verify operator set digest binding between report and `operator_registry` artifact.
///
/// Fail-closed invariants:
/// - If report has `operator_set_digest`, `operator_registry.json` must exist and match.
/// - If `operator_registry.json` exists, report `operator_set_digest` is mandatory.
fn verify_operator_set_digest_binding(
    bundle: &ArtifactBundleV1,
) -> Result<(), BundleVerifyError> {
    let report_artifact = bundle.artifacts.get("verification_report.json");
    let registry_artifact = bundle.artifacts.get("operator_registry.json");

    let (Some(report_art), registry_opt) = (report_artifact, registry_artifact) else {
        return Ok(());
    };

    let report: serde_json::Value = serde_json::from_slice(&report_art.content).map_err(|e| {
        BundleVerifyError::ReportParseError {
            detail: format!("{e:?}"),
        }
    })?;

    let report_digest = report
        .get("operator_set_digest")
        .and_then(|v| v.as_str());

    match (report_digest, registry_opt) {
        (Some(_), None) => Err(BundleVerifyError::OperatorRegistryArtifactMissing),
        (None, Some(_)) => Err(BundleVerifyError::OperatorRegistryDigestMissing),
        (Some(declared), Some(reg_art)) => {
            if reg_art.content_hash.as_str() != declared {
                return Err(BundleVerifyError::OperatorRegistryDigestMismatch {
                    declared: declared.to_string(),
                    recomputed: reg_art.content_hash.as_str().to_string(),
                });
            }
            Ok(())
        }
        (None, None) => Ok(()),
    }
}

/// Cross-verify `operator_set_digest` in `search_graph.json` metadata against
/// `operator_registry.json`.
fn verify_metadata_operator_set_binding(
    bundle: &ArtifactBundleV1,
) -> Result<(), BundleVerifyError> {
    let Some(graph_artifact) = bundle.artifacts.get("search_graph.json") else {
        return Ok(());
    };

    let graph: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;

    let graph_digest = graph
        .get("metadata")
        .and_then(|m| m.get("operator_set_digest"))
        .and_then(|v| v.as_str());
    let registry_artifact = bundle.artifacts.get("operator_registry.json");

    match (graph_digest, registry_artifact) {
        (Some(_), None) | (None, Some(_)) => {
            Err(BundleVerifyError::MetadataBindingOperatorRegistryMissing)
        }
        (Some(in_graph), Some(reg_art)) => {
            let reg_hex = binding_hex(&reg_art.content_hash);
            if reg_hex != in_graph {
                return Err(BundleVerifyError::MetadataBindingOperatorRegistryMismatch {
                    in_graph: in_graph.to_string(),
                    in_artifact: reg_hex.to_string(),
                });
            }
            Ok(())
        }
        (None, None) => Ok(()),
    }
}

/// Tape verification pipeline (profile-dependent).
///
/// Base: if tape present, verify digest binding + parse + header bindings + mode coherence.
/// Cert: require tape. Do everything Base does, plus tape→graph equivalence.
#[allow(clippy::too_many_lines)]
fn verify_tape(
    bundle: &ArtifactBundleV1,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    let tape_artifact = bundle.artifacts.get("search_tape.stap");
    let graph_artifact = bundle.artifacts.get("search_graph.json");
    let report_artifact = bundle.artifacts.get("verification_report.json");

    // Step 16a: Profile gate.
    match (tape_artifact, profile) {
        (None, VerificationProfile::Cert) => return Err(BundleVerifyError::TapeMissing),
        (None, VerificationProfile::Base) => return Ok(()),
        _ => {}
    }

    // Tape is present from here.
    let tape_art = tape_artifact.expect("tape presence checked above");

    // Step 16b: tape_digest binding (report tape_digest == tape content_hash).
    // Fail-closed: if tape is present, report MUST exist and MUST contain tape_digest.
    let report_art = report_artifact.ok_or(BundleVerifyError::ReportFieldMissing {
        field: "verification_report.json (required when tape present)".to_string(),
    })?;
    let report: serde_json::Value =
        serde_json::from_slice(&report_art.content).map_err(|e| {
            BundleVerifyError::ReportParseError {
                detail: format!("{e:?}"),
            }
        })?;
    let declared_tape_digest = report
        .get("tape_digest")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::ReportFieldMissing {
            field: "tape_digest".to_string(),
        })?;
    if tape_art.content_hash.as_str() != declared_tape_digest {
        return Err(BundleVerifyError::TapeDigestMismatch {
            declared: declared_tape_digest.to_string(),
            recomputed: tape_art.content_hash.as_str().to_string(),
        });
    }

    // Step 16c: Parse tape (chain hash verified internally by reader).
    let tape = read_tape(&tape_art.content).map_err(|e| BundleVerifyError::TapeParseFailed {
        source: format!("{e:?}"),
    })?;

    // Step 16d: Header binding against authoritative artifacts (not report).
    // Fail-closed: if tape is present, graph MUST be present (tape is a search artifact).
    let header = &tape.header.json;

    let graph_art = graph_artifact.ok_or(BundleVerifyError::TapeParseFailed {
        source: "search_graph.json required when search_tape.stap present".to_string(),
    })?;
    let graph: serde_json::Value =
        serde_json::from_slice(&graph_art.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("{e:?}"),
            }
        })?;
    let metadata = &graph["metadata"];

    // world_id (fail-closed: require both sides)
    check_tape_header_field(header, metadata, "world_id")?;

    // schema_descriptor (string, from graph metadata)
    check_tape_header_field(header, metadata, "schema_descriptor")?;

    // registry_digest (both raw hex, from graph metadata)
    check_tape_header_field(header, metadata, "registry_digest")?;

    // search_policy_digest (both raw hex, from graph metadata)
    check_tape_header_field(header, metadata, "search_policy_digest")?;

    // root_state_fingerprint (both raw hex, from graph metadata)
    check_tape_header_field(header, metadata, "root_state_fingerprint")?;

    // fixture_digest: Base=required-if-present, Cert=mandatory.
    let tape_fixture = header.get("fixture_digest").and_then(|v| v.as_str());
    let graph_fixture = metadata.get("fixture_digest").and_then(|v| v.as_str());
    match (tape_fixture, graph_fixture) {
        (Some(t), Some(g)) => {
            if t != g {
                return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                    field: "fixture_digest",
                    in_tape: t.to_string(),
                    in_artifact: g.to_string(),
                });
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field: "fixture_digest",
                in_tape: tape_fixture.unwrap_or("<absent>").to_string(),
                in_artifact: graph_fixture.unwrap_or("<absent>").to_string(),
            });
        }
        (None, None) if profile == VerificationProfile::Cert => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field: "fixture_digest",
                in_tape: "<absent>".to_string(),
                in_artifact: "<absent>".to_string(),
            });
        }
        (None, None) => {} // Base: old bundle without fixture_digest — pass.
    }

    // policy_snapshot_digest: authoritative source is policy_snapshot.json content_hash.
    // Fail-closed: require policy artifact and tape header field.
    let policy_art = bundle.artifacts.get("policy_snapshot.json").ok_or(
        BundleVerifyError::TapeParseFailed {
            source: "policy_snapshot.json required when search_tape.stap present".to_string(),
        },
    )?;
    let tape_policy_val = header
        .get("policy_snapshot_digest")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::TapeHeaderBindingMismatch {
            field: "policy_snapshot_digest",
            in_tape: "<missing>".to_string(),
            in_artifact: binding_hex(&policy_art.content_hash).to_string(),
        })?;
    let artifact_hex = binding_hex(&policy_art.content_hash);
    if tape_policy_val != artifact_hex {
        return Err(BundleVerifyError::TapeHeaderBindingMismatch {
            field: "policy_snapshot_digest",
            in_tape: tape_policy_val.to_string(),
            in_artifact: artifact_hex.to_string(),
        });
    }

    // scorer_digest: authoritative source is scorer.json content_hash.
    let scorer_artifact = bundle.artifacts.get("scorer.json");
    let tape_scorer_digest = header.get("scorer_digest").and_then(|v| v.as_str());

    match (tape_scorer_digest, scorer_artifact) {
        // Mode coherence: tape claims scorer but no artifact.
        (Some(_), None) => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field: "scorer_digest",
                in_tape: tape_scorer_digest.unwrap_or("").to_string(),
                in_artifact: "<absent>".to_string(),
            });
        }
        // Mode coherence: artifact exists but tape has no scorer_digest.
        (None, Some(_)) => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field: "scorer_digest",
                in_tape: "<absent>".to_string(),
                in_artifact: scorer_artifact
                    .map(|a| binding_hex(&a.content_hash).to_string())
                    .unwrap_or_default(),
            });
        }
        // Both present: verify match.
        (Some(tape_val), Some(scorer_art)) => {
            let artifact_hex = binding_hex(&scorer_art.content_hash);
            if tape_val != artifact_hex {
                return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                    field: "scorer_digest",
                    in_tape: tape_val.to_string(),
                    in_artifact: artifact_hex.to_string(),
                });
            }
        }
        // Neither: Uniform mode, fine.
        (None, None) => {}
    }

    // operator_set_digest: authoritative source is operator_registry.json content_hash.
    let registry_artifact = bundle.artifacts.get("operator_registry.json");
    let tape_op_digest = header
        .get("operator_set_digest")
        .and_then(|v| v.as_str());

    match (tape_op_digest, registry_artifact) {
        (Some(_), None) => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field: "operator_set_digest",
                in_tape: tape_op_digest.unwrap_or("").to_string(),
                in_artifact: "<absent>".to_string(),
            });
        }
        (None, Some(_)) => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field: "operator_set_digest",
                in_tape: "<absent>".to_string(),
                in_artifact: registry_artifact
                    .map(|a| binding_hex(&a.content_hash).to_string())
                    .unwrap_or_default(),
            });
        }
        (Some(tape_val), Some(reg_art)) => {
            let artifact_hex = binding_hex(&reg_art.content_hash);
            if tape_val != artifact_hex {
                return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                    field: "operator_set_digest",
                    in_tape: tape_val.to_string(),
                    in_artifact: artifact_hex.to_string(),
                });
            }
        }
        (None, None) => {}
    }

    // root_identity_digest: Base=required-if-present, Cert=mandatory.
    check_optional_tape_header_field(
        header,
        metadata,
        "root_identity_digest",
        profile,
    )?;

    // root_evidence_digest: Base=required-if-present, Cert=mandatory.
    check_optional_tape_header_field(
        header,
        metadata,
        "root_evidence_digest",
        profile,
    )?;

    // Step 16e: schema_version check (fail-closed: must be present and correct).
    let schema_version = header
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or(BundleVerifyError::TapeParseFailed {
            source: "tape header missing required schema_version field".to_string(),
        })?;
    if schema_version != "search_tape.v1" {
        return Err(BundleVerifyError::TapeParseFailed {
            source: format!("unexpected tape schema_version: {schema_version}"),
        });
    }

    // Step 16f (Cert only): Tape→graph canonical equivalence.
    // graph_art is already required above (fail-closed when tape present).
    if profile == VerificationProfile::Cert {
        let rendered_graph =
            render_graph(&tape).map_err(|e| BundleVerifyError::TapeRenderFailed {
                source: format!("{e:?}"),
            })?;
        let rendered_bytes = rendered_graph.to_canonical_json_bytes().map_err(|e| {
            BundleVerifyError::TapeRenderFailed {
                source: format!("{e:?}"),
            }
        })?;
        if rendered_bytes != graph_art.content {
            return Err(BundleVerifyError::TapeGraphEquivalenceMismatch);
        }
    }

    Ok(())
}

/// Tool transcript verification pipeline (profile-dependent).
///
/// Base: if `tool_transcript.json` present, verify digest binding only.
/// Cert: obligation-gated + digest binding + structural integrity +
///       equivalence render + trace-order audit.
#[allow(clippy::too_many_lines)]
fn verify_tool_transcript(
    bundle: &ArtifactBundleV1,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    let transcript_artifact = bundle.artifacts.get("tool_transcript.json");
    let report_artifact = bundle.artifacts.get("verification_report.json");
    let fixture_artifact = bundle.artifacts.get("fixture.json");
    let tape_artifact = bundle.artifacts.get("search_tape.stap");

    // Read evidence_obligations from fixture.json.
    let obligations: Vec<String> = fixture_artifact
        .and_then(|a| serde_json::from_slice::<serde_json::Value>(&a.content).ok())
        .and_then(|v| {
            v.get("evidence_obligations")
                .and_then(|arr| arr.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
        })
        .unwrap_or_default();

    let has_obligation = obligations.iter().any(|o| o == "tool_transcript_v1");

    // Step 19a: Cert obligation gating.
    if profile == VerificationProfile::Cert && has_obligation && transcript_artifact.is_none() {
        return Err(BundleVerifyError::ToolTranscriptMissing);
    }

    // If no transcript artifact, nothing more to verify (Base skips entirely,
    // Cert without obligation skips).
    let Some(transcript_art) = transcript_artifact else {
        // Cert belt-and-suspenders: check if tape contains tool ops
        // but obligation is missing.
        if profile == VerificationProfile::Cert {
            if let Some(tape_art) = tape_artifact {
                if let Ok(tape) = read_tape(&tape_art.content) {
                    if crate::transcript::tape_contains_tool_ops(&tape) && !has_obligation {
                        return Err(BundleVerifyError::ObligationMismatch {
                            detail: "tape contains tool operator frames but \
                                evidence_obligations does not include \
                                \"tool_transcript_v1\""
                                .to_string(),
                        });
                    }
                }
            }
        }
        return Ok(());
    };

    // Step 19b: Digest binding (report `tool_transcript_digest` == artifact content_hash).
    if let Some(report_art) = report_artifact {
        let report: serde_json::Value =
            serde_json::from_slice(&report_art.content).map_err(|e| {
                BundleVerifyError::ReportParseError {
                    detail: format!("{e:?}"),
                }
            })?;

        if let Some(declared) = report.get("tool_transcript_digest").and_then(|v| v.as_str()) {
            if transcript_art.content_hash.as_str() != declared {
                return Err(BundleVerifyError::ToolTranscriptDigestMismatch {
                    declared: declared.to_string(),
                    recomputed: transcript_art.content_hash.as_str().to_string(),
                });
            }
        } else {
            // Transcript artifact exists but report missing digest.
            return Err(BundleVerifyError::ToolTranscriptDigestMissing);
        }
    }

    // Step 19c: Structural integrity.
    let transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).map_err(|e| {
            BundleVerifyError::CanonError {
                detail: format!("tool_transcript.json parse: {e:?}"),
            }
        })?;

    // entry_count == entries.len()
    let entries = transcript
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or(BundleVerifyError::CanonError {
            detail: "tool_transcript.json missing entries array".to_string(),
        })?;
    #[allow(clippy::cast_possible_truncation)]
    let declared_count = transcript
        .get("entry_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize;
    if declared_count != entries.len() {
        return Err(BundleVerifyError::ToolTranscriptEntryCountMismatch {
            declared: declared_count,
            actual: entries.len(),
        });
    }

    // step_index monotonically increasing
    let mut prev_step: Option<u64> = None;
    for entry in entries {
        let step = entry
            .get("step_index")
            .and_then(serde_json::Value::as_u64)
            .ok_or(BundleVerifyError::ToolTranscriptStepIndexInvalid {
                detail: "entry missing step_index".to_string(),
            })?;
        if let Some(prev) = prev_step {
            if step <= prev {
                return Err(BundleVerifyError::ToolTranscriptStepIndexInvalid {
                    detail: format!(
                        "step_index {step} not monotonically increasing (prev {prev})"
                    ),
                });
            }
        }
        prev_step = Some(step);
    }

    // Step 19d (Cert only): Equivalence render from tape.
    if profile == VerificationProfile::Cert {
        if let Some(tape_art) = tape_artifact {
            let tape = read_tape(&tape_art.content).map_err(|e| {
                BundleVerifyError::CanonError {
                    detail: format!("tape parse for transcript equivalence: {e:?}"),
                }
            })?;

            // Use the kernel operator registry for rendering.
            // The bundle's operator_registry.json must match the kernel's
            // registry (verified by Step 16-17), so this is equivalent.
            let op_reg =
                sterling_kernel::operators::operator_registry::kernel_operator_registry();

            let world_id = transcript
                .get("world_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let rendered_bytes =
                crate::transcript::render_tool_transcript(&tape, &op_reg, world_id)
                    .map_err(|e| BundleVerifyError::CanonError {
                        detail: format!("transcript render: {e}"),
                    })?;

            if rendered_bytes != transcript_art.content {
                let rendered_hash =
                    canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &rendered_bytes);
                return Err(BundleVerifyError::ToolTranscriptEquivalenceMismatch {
                    expected_hash: transcript_art.content_hash.as_str().to_string(),
                    rendered_hash: rendered_hash.as_str().to_string(),
                });
            }

            // Cert belt-and-suspenders: if tape contains tool ops, obligation must be declared.
            if crate::transcript::tape_contains_tool_ops(&tape) && !has_obligation {
                return Err(BundleVerifyError::ObligationMismatch {
                    detail: "tape contains tool operator frames but \
                        evidence_obligations does not include \"tool_transcript_v1\""
                        .to_string(),
                });
            }

            // Step 19e (Cert only): Trace-order audit for committed-write safety.
            verify_trace_order_safety(&tape)?;
        }
    }

    Ok(())
}

/// Cert trace-order audit: verify committed-write safety from the tape.
///
/// Rules:
/// - No writes to layer 0 (via `SET_SLOT`) before `OP_COMMIT` has been applied.
/// - No writes to any layer after `OP_ROLLBACK` has been applied.
fn verify_trace_order_safety(
    tape: &sterling_search::tape::SearchTapeV1,
) -> Result<(), BundleVerifyError> {
    use sterling_kernel::operators::apply::{OP_COMMIT, OP_ROLLBACK, OP_SET_SLOT, OP_STAGE};
    use sterling_search::tape::{TapeCandidateOutcomeV1, TapeRecordV1};

    let mut commit_seen = false;
    let mut rollback_seen = false;

    for record in &tape.records {
        let TapeRecordV1::Expansion(expansion) = record else {
            continue;
        };

        for candidate in &expansion.candidates {
            if !matches!(candidate.outcome, TapeCandidateOutcomeV1::Applied { .. }) {
                continue;
            }

            let op_code =
                sterling_kernel::carrier::code32::Code32::from_le_bytes(candidate.op_code_bytes);

            // Track commit/rollback state.
            if op_code == OP_COMMIT {
                commit_seen = true;
            } else if op_code == OP_ROLLBACK {
                rollback_seen = true;
            }

            // Rule 1: No writes after rollback.
            if rollback_seen
                && (op_code == OP_SET_SLOT || op_code == OP_STAGE || op_code == OP_COMMIT)
            {
                #[allow(clippy::cast_possible_truncation)]
                return Err(BundleVerifyError::CommittedWriteOrderViolation {
                    step_index: expansion.expansion_order as usize,
                    detail: format!(
                        "write (op {op_code:?}) after OP_ROLLBACK at expansion {}",
                        expansion.expansion_order
                    ),
                });
            }

            // Rule 2: No layer 0 writes before commit.
            if op_code == OP_SET_SLOT && !commit_seen && candidate.op_args.len() >= 4 {
                let layer = u32::from_le_bytes([
                    candidate.op_args[0],
                    candidate.op_args[1],
                    candidate.op_args[2],
                    candidate.op_args[3],
                ]);
                if layer == 0 {
                    #[allow(clippy::cast_possible_truncation)]
                    return Err(BundleVerifyError::CommittedWriteOrderViolation {
                        step_index: expansion.expansion_order as usize,
                        detail: format!(
                            "SET_SLOT to layer 0 before OP_COMMIT at expansion {}",
                            expansion.expansion_order
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Compare a tape header field against the same field in graph metadata.
/// Both use raw hex format (no `sha256:` prefix).
///
/// Fail-closed: both sides must be present. Missing field on either side
/// is an error, not a skip.
fn check_tape_header_field(
    header: &serde_json::Value,
    metadata: &serde_json::Value,
    field: &'static str,
) -> Result<(), BundleVerifyError> {
    let tape_val = header.get(field).and_then(|v| v.as_str()).ok_or(
        BundleVerifyError::TapeHeaderBindingMismatch {
            field,
            in_tape: "<missing>".to_string(),
            in_artifact: metadata
                .get(field)
                .and_then(|v| v.as_str())
                .unwrap_or("<missing>")
                .to_string(),
        },
    )?;
    let graph_val = metadata.get(field).and_then(|v| v.as_str()).ok_or(
        BundleVerifyError::TapeHeaderBindingMismatch {
            field,
            in_tape: tape_val.to_string(),
            in_artifact: "<missing>".to_string(),
        },
    )?;
    if tape_val != graph_val {
        return Err(BundleVerifyError::TapeHeaderBindingMismatch {
            field,
            in_tape: tape_val.to_string(),
            in_artifact: graph_val.to_string(),
        });
    }
    Ok(())
}

/// Compare an optional tape header field against graph metadata.
///
/// Base: required-if-present (both absent is ok, one-sided is error).
/// Cert: mandatory (both absent is error).
fn check_optional_tape_header_field(
    header: &serde_json::Value,
    metadata: &serde_json::Value,
    field: &'static str,
    profile: VerificationProfile,
) -> Result<(), BundleVerifyError> {
    let tape_val = header.get(field).and_then(|v| v.as_str());
    let graph_val = metadata.get(field).and_then(|v| v.as_str());

    match (tape_val, graph_val) {
        (Some(t), Some(g)) if t != g => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field,
                in_tape: t.to_string(),
                in_artifact: g.to_string(),
            });
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field,
                in_tape: tape_val.unwrap_or("<absent>").to_string(),
                in_artifact: graph_val.unwrap_or("<absent>").to_string(),
            });
        }
        (None, None) if profile == VerificationProfile::Cert => {
            return Err(BundleVerifyError::TapeHeaderBindingMismatch {
                field,
                in_tape: "<absent>".to_string(),
                in_artifact: "<absent>".to_string(),
            });
        }
        _ => {} // Both match, or both absent in Base.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_empty_bundle() {
        let bundle = build_bundle(Vec::<ArtifactInput>::new()).unwrap();
        assert!(bundle.artifacts.is_empty());
        assert!(!bundle.manifest.is_empty());
        assert!(!bundle.digest_basis.is_empty());
    }

    #[test]
    fn normative_flag_affects_digest_basis() {
        let b1 = build_bundle(vec![
            ("a.json".into(), b"hello".to_vec(), true),
            ("b.bin".into(), b"world".to_vec(), false),
        ])
        .unwrap();

        let b2 = build_bundle(vec![
            ("a.json".into(), b"hello".to_vec(), true),
            ("b.bin".into(), b"DIFFERENT".to_vec(), false),
        ])
        .unwrap();

        // Same normative content → same digest.
        assert_eq!(b1.digest.as_str(), b2.digest.as_str());

        // Different observational content → different manifest.
        assert_ne!(b1.manifest, b2.manifest);
    }

    #[test]
    fn verify_bundle_passes_clean_build() {
        let bundle = build_bundle(vec![
            ("a.json".into(), b"{\"key\":\"value\"}".to_vec(), true),
            ("b.bin".into(), b"binary data".to_vec(), false),
        ])
        .unwrap();
        verify_bundle(&bundle).unwrap();
    }

    #[test]
    fn artifacts_sorted_by_name() {
        let bundle = build_bundle(vec![
            ("z.txt".into(), b"last".to_vec(), true),
            ("a.txt".into(), b"first".to_vec(), true),
        ])
        .unwrap();

        let names: Vec<&str> = bundle.artifacts.keys().map(String::as_str).collect();
        assert_eq!(names, vec!["a.txt", "z.txt"]);
    }

    #[test]
    fn wrong_precomputed_hash_rejected() {
        use sterling_kernel::proof::hash::ContentHash;

        let wrong_hash = ContentHash::parse(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();

        let input = ArtifactInput {
            name: "test.json".to_string(),
            content: b"{\"key\":\"value\"}".to_vec(),
            normative: true,
            precomputed_hash: Some(wrong_hash),
        };

        let err = build_bundle(vec![input]).unwrap_err();
        assert!(
            matches!(err, BundleBuildError::PrecomputedHashMismatch { .. }),
            "wrong precomputed hash must be rejected, got: {err:?}"
        );
    }

    #[test]
    fn correct_precomputed_hash_accepted() {
        let content = b"{\"key\":\"value\"}";
        let correct_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, content);

        let input = ArtifactInput {
            name: "test.json".to_string(),
            content: content.to_vec(),
            normative: true,
            precomputed_hash: Some(correct_hash),
        };

        let bundle = build_bundle(vec![input]).unwrap();
        assert_eq!(bundle.artifacts.len(), 1);
    }
}
