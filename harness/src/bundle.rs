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

use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, ContentHash};

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

    // Build full manifest (all artifacts).
    let manifest_artifacts: Vec<serde_json::Value> = artifact_map
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

    let manifest =
        canonical_json_bytes(&manifest_value).map_err(|e| BundleBuildError::CanonError {
            detail: format!("{e:?}"),
        })?;

    // Build digest basis (normative artifacts only).
    let normative_artifacts: Vec<serde_json::Value> = artifact_map
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

    let digest_basis =
        canonical_json_bytes(&digest_basis_value).map_err(|e| BundleBuildError::CanonError {
            detail: format!("{e:?}"),
        })?;

    let digest = canonical_hash(DOMAIN_BUNDLE_DIGEST, &digest_basis);

    Ok(ArtifactBundleV1 {
        artifacts: artifact_map,
        manifest,
        digest_basis,
        digest,
    })
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
