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
    /// `policy_snapshot_digest` in `search_graph.json` metadata does not match
    /// `policy_snapshot.json`'s `content_hash`.
    MetadataBindingPolicyMismatch { in_graph: String, in_policy: String },
    /// `world_id` in `search_graph.json` metadata does not match
    /// `verification_report.json`'s `world_id`.
    MetadataBindingWorldIdMismatch { in_graph: String, in_report: String },
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
        _ => Ok(()),
    }
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

    // Cross-verify policy_snapshot_digest
    if let Some(policy_artifact) = bundle.artifacts.get("policy_snapshot.json") {
        if let Some(graph_policy_digest) = graph
            .get("metadata")
            .and_then(|m| m.get("policy_snapshot_digest"))
            .and_then(|v| v.as_str())
        {
            // Graph metadata stores raw hex digest; content_hash stores "sha256:hex".
            // Compare the hex portion.
            let policy_hex = policy_artifact.content_hash.hex_digest();
            if policy_hex != graph_policy_digest {
                return Err(BundleVerifyError::MetadataBindingPolicyMismatch {
                    in_graph: graph_policy_digest.to_string(),
                    in_policy: policy_hex.to_string(),
                });
            }
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
            .and_then(|v| v.as_str());
        let report_world_id = report.get("world_id").and_then(|v| v.as_str());

        if let (Some(gw), Some(rw)) = (graph_world_id, report_world_id) {
            if gw != rw {
                return Err(BundleVerifyError::MetadataBindingWorldIdMismatch {
                    in_graph: gw.to_string(),
                    in_report: rw.to_string(),
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
