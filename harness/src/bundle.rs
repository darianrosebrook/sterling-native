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

use sterling_kernel::carrier::trace_reader::bytes_to_trace;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, ContentHash};
use sterling_kernel::proof::trace_hash::{payload_hash, step_chain};

/// Domain prefix for bundle artifact content hashing (harness-originated).
pub const DOMAIN_BUNDLE_ARTIFACT: &[u8] = b"STERLING::BUNDLE_ARTIFACT::V1\0";

/// Domain prefix for bundle digest computation (harness-originated).
pub const DOMAIN_BUNDLE_DIGEST: &[u8] = b"STERLING::BUNDLE_DIGEST::V1\0";

/// Domain prefix for harness fixture hashing (harness-originated).
pub const DOMAIN_HARNESS_FIXTURE: &[u8] = b"STERLING::HARNESS_FIXTURE::V1\0";

/// Domain prefix for codebook hash computation (harness-originated).
pub const DOMAIN_CODEBOOK_HASH: &[u8] = b"STERLING::CODEBOOK_HASH::V1\0";

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
}

/// Build an `ArtifactBundleV1` from a list of `(name, content, normative)` tuples.
///
/// Computes content hashes, builds the sorted manifest and digest basis,
/// and derives the bundle digest. All JSON via kernel's `canonical_json_bytes`.
///
/// # Errors
///
/// Returns [`BundleBuildError`] if canonical JSON serialization fails.
pub fn build_bundle(
    artifacts: Vec<(String, Vec<u8>, bool)>,
) -> Result<ArtifactBundleV1, BundleBuildError> {
    let mut artifact_map = BTreeMap::new();

    for (name, content, normative) in artifacts {
        let content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &content);
        artifact_map.insert(
            name.clone(),
            BundleArtifact {
                name,
                content,
                content_hash,
                normative,
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
    /// Canonical JSON error during verification.
    CanonError { detail: String },
}

/// Verify the internal consistency of a bundle.
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
///
/// # Errors
///
/// Returns the first [`BundleVerifyError`] encountered.
pub fn verify_bundle(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
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
    verify_metadata_bindings(bundle)?;

    // Step 13: Scorer digest binding (report ↔ scorer.json).
    verify_scorer_digest_binding(bundle)?;

    // Step 14: Scorer digest in graph metadata ↔ scorer.json.
    verify_metadata_scorer_binding(bundle)?;

    // Step 15: Candidate score source consistency with scorer artifact.
    verify_candidate_scorer_consistency(bundle)?;

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
fn verify_metadata_bindings(bundle: &ArtifactBundleV1) -> Result<(), BundleVerifyError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_empty_bundle() {
        let bundle = build_bundle(vec![]).unwrap();
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
}
