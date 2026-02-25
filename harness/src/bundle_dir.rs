//! Bundle directory persistence: write/read/verify `ArtifactBundleV1` to/from disk.
//!
//! # Directory layout (`BundleDirectoryV1`)
//!
//! ```text
//! <dir>/
//!   bundle_manifest.json         — canonical JSON, full artifact listing
//!   bundle_digest_basis.json     — canonical JSON, normative projection only
//!   bundle_digest.txt            — ASCII digest string (e.g. "sha256:...")
//!   fixture.json                 — artifact file (normative)
//!   compilation_manifest.json    — artifact file (normative)
//!   verification_report.json     — artifact file (normative)
//!   trace.bst1                   — artifact file (observational)
//! ```
//!
//! The directory path is never part of any hash surface. File ordering on disk
//! is irrelevant; the manifest's declared list is the source of truth.
//!
//! # Fail-closed semantics
//!
//! - Missing declared artifact files → error
//! - Extra undeclared files → error
//! - Content hash mismatch → error
//! - Non-canonical manifest or digest basis → error

use std::collections::BTreeSet;
use std::path::Path;

use crate::bundle::{verify_bundle, ArtifactBundleV1, BundleArtifact, BundleVerifyError};
use sterling_kernel::proof::hash::{canonical_hash, ContentHash};

use crate::bundle::DOMAIN_BUNDLE_DIGEST;

/// Fixed metadata filenames in the bundle directory.
const MANIFEST_FILENAME: &str = "bundle_manifest.json";
const DIGEST_BASIS_FILENAME: &str = "bundle_digest_basis.json";
const DIGEST_FILENAME: &str = "bundle_digest.txt";

/// The set of reserved metadata filenames (not artifact files).
const METADATA_FILENAMES: &[&str] = &[MANIFEST_FILENAME, DIGEST_BASIS_FILENAME, DIGEST_FILENAME];

/// Error writing a bundle directory.
#[derive(Debug)]
pub enum BundleDirWriteError {
    /// I/O error during write.
    Io { detail: String },
    /// Canonical JSON serialization failed.
    CanonError { detail: String },
}

impl std::fmt::Display for BundleDirWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { detail } => write!(f, "I/O error: {detail}"),
            Self::CanonError { detail } => write!(f, "canonical JSON error: {detail}"),
        }
    }
}

/// Error reading a bundle directory.
#[derive(Debug)]
pub enum BundleDirReadError {
    /// I/O error during read.
    Io { detail: String },
    /// A required metadata file is missing.
    MissingMetadata { filename: String },
    /// A declared artifact file is missing from the directory.
    MissingArtifact { name: String },
    /// An undeclared file exists in the directory.
    ExtraFile { name: String },
    /// `bundle_manifest.json` is not valid JSON.
    ManifestParseError { detail: String },
    /// Manifest `schema_version` is not recognized.
    ManifestVersionMismatch { found: String },
    /// An artifact entry in the manifest is missing a required field.
    ManifestEntryInvalid { detail: String },
    /// `bundle_digest.txt` content doesn't match the recomputed digest.
    DigestMismatch { stored: String, recomputed: String },
    /// Canonical JSON error during reconstruction.
    CanonError { detail: String },
}

impl std::fmt::Display for BundleDirReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { detail } => write!(f, "I/O error: {detail}"),
            Self::MissingMetadata { filename } => {
                write!(f, "missing metadata file: {filename}")
            }
            Self::MissingArtifact { name } => write!(f, "missing artifact: {name}"),
            Self::ExtraFile { name } => write!(f, "undeclared extra file: {name}"),
            Self::ManifestParseError { detail } => {
                write!(f, "manifest parse error: {detail}")
            }
            Self::ManifestVersionMismatch { found } => {
                write!(f, "manifest version mismatch: {found}")
            }
            Self::ManifestEntryInvalid { detail } => {
                write!(f, "manifest entry invalid: {detail}")
            }
            Self::DigestMismatch { stored, recomputed } => {
                write!(
                    f,
                    "digest mismatch: stored={stored}, recomputed={recomputed}"
                )
            }
            Self::CanonError { detail } => write!(f, "canonical JSON error: {detail}"),
        }
    }
}

/// Error verifying a bundle directory.
#[derive(Debug)]
pub enum BundleDirVerifyError {
    /// Error reading the directory.
    ReadError(BundleDirReadError),
    /// Bundle integrity verification failed.
    VerifyError(BundleVerifyError),
}

impl std::fmt::Display for BundleDirVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadError(e) => write!(f, "read error: {e}"),
            Self::VerifyError(e) => write!(f, "verify error: {e:?}"),
        }
    }
}

/// Write an `ArtifactBundleV1` to a directory in `BundleDirectoryV1` format.
///
/// Creates the directory if it does not exist. Writes each artifact file,
/// plus the three metadata files (`bundle_manifest.json`, `bundle_digest_basis.json`,
/// `bundle_digest.txt`).
///
/// # Errors
///
/// Returns [`BundleDirWriteError`] on I/O failure or canonical JSON error.
pub fn write_bundle_dir(bundle: &ArtifactBundleV1, dir: &Path) -> Result<(), BundleDirWriteError> {
    // Create directory.
    std::fs::create_dir_all(dir).map_err(|e| BundleDirWriteError::Io {
        detail: format!("create_dir_all: {e}"),
    })?;

    // Write artifact files.
    for artifact in bundle.artifacts.values() {
        let path = dir.join(&artifact.name);
        write_atomic(path, &artifact.content)?;
    }

    // Write metadata files.
    write_atomic(dir.join(MANIFEST_FILENAME), &bundle.manifest)?;
    write_atomic(dir.join(DIGEST_BASIS_FILENAME), &bundle.digest_basis)?;
    write_atomic(dir.join(DIGEST_FILENAME), bundle.digest.as_str().as_bytes())?;

    Ok(())
}

/// Read a `BundleDirectoryV1` directory into an `ArtifactBundleV1`.
///
/// Fail-closed:
/// - Missing declared artifact files → error
/// - Extra undeclared files → error
/// - Manifest must be valid canonical JSON with `schema_version: "bundle.v1"`
///
/// The stored `bundle_digest.txt` is verified against the recomputed digest.
///
/// # Errors
///
/// Returns [`BundleDirReadError`] on any validation failure.
pub fn read_bundle_dir(dir: &Path) -> Result<ArtifactBundleV1, BundleDirReadError> {
    // Read metadata files.
    let manifest_bytes = read_required(dir, MANIFEST_FILENAME)?;
    let digest_basis_bytes = read_required(dir, DIGEST_BASIS_FILENAME)?;
    let digest_str = read_required(dir, DIGEST_FILENAME)?;

    // Parse manifest to discover artifact entries.
    let manifest_value: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).map_err(|e| {
            BundleDirReadError::ManifestParseError {
                detail: format!("{e}"),
            }
        })?;

    let schema_version = manifest_value["schema_version"].as_str().unwrap_or("");
    if schema_version != "bundle.v1" {
        return Err(BundleDirReadError::ManifestVersionMismatch {
            found: schema_version.to_string(),
        });
    }

    let artifact_entries = manifest_value["artifacts"].as_array().ok_or_else(|| {
        BundleDirReadError::ManifestParseError {
            detail: "\"artifacts\" is not an array".into(),
        }
    })?;

    // Build artifact map from manifest declarations.
    let mut artifacts = std::collections::BTreeMap::new();
    let mut declared_filenames: BTreeSet<String> = BTreeSet::new();

    for entry in artifact_entries {
        let name = entry["name"]
            .as_str()
            .ok_or_else(|| BundleDirReadError::ManifestEntryInvalid {
                detail: "missing \"name\" field".into(),
            })?
            .to_string();

        let content_hash_str = entry["content_hash"].as_str().ok_or_else(|| {
            BundleDirReadError::ManifestEntryInvalid {
                detail: format!("missing \"content_hash\" for {name}"),
            }
        })?;

        let normative = entry["normative"].as_bool().ok_or_else(|| {
            BundleDirReadError::ManifestEntryInvalid {
                detail: format!("missing \"normative\" for {name}"),
            }
        })?;

        // Read artifact file.
        let content = read_file(dir, &name)
            .map_err(|_| BundleDirReadError::MissingArtifact { name: name.clone() })?;

        let content_hash = ContentHash::parse(content_hash_str).ok_or_else(|| {
            BundleDirReadError::ManifestEntryInvalid {
                detail: format!("invalid content_hash format for {name}: {content_hash_str}"),
            }
        })?;

        declared_filenames.insert(name.clone());

        artifacts.insert(
            name.clone(),
            BundleArtifact {
                name,
                content,
                content_hash,
                normative,
            },
        );
    }

    // Check for extra files.
    let actual_files = list_files(dir)?;
    let metadata_set: BTreeSet<String> = METADATA_FILENAMES
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    for filename in &actual_files {
        if !declared_filenames.contains(filename) && !metadata_set.contains(filename) {
            return Err(BundleDirReadError::ExtraFile {
                name: filename.clone(),
            });
        }
    }

    // Recompute digest from digest_basis and verify against stored.
    let recomputed_digest = canonical_hash(DOMAIN_BUNDLE_DIGEST, &digest_basis_bytes);
    let stored_digest = String::from_utf8_lossy(&digest_str).trim().to_string();
    if recomputed_digest.as_str() != stored_digest {
        return Err(BundleDirReadError::DigestMismatch {
            stored: stored_digest,
            recomputed: recomputed_digest.as_str().to_string(),
        });
    }

    Ok(ArtifactBundleV1 {
        artifacts,
        manifest: manifest_bytes,
        digest_basis: digest_basis_bytes,
        digest: recomputed_digest,
    })
}

/// Verify a bundle directory: read from disk, then run `verify_bundle()`.
///
/// This is the primary offline verification entrypoint.
///
/// # Errors
///
/// Returns [`BundleDirVerifyError`] on read failure or integrity mismatch.
pub fn verify_bundle_dir(dir: &Path) -> Result<(), BundleDirVerifyError> {
    let bundle = read_bundle_dir(dir).map_err(BundleDirVerifyError::ReadError)?;
    verify_bundle(&bundle).map_err(BundleDirVerifyError::VerifyError)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Write bytes to a path via temp file + rename (best-effort atomicity on Unix).
fn write_atomic(path: impl AsRef<Path>, content: &[u8]) -> Result<(), BundleDirWriteError> {
    let path = path.as_ref();
    let dir = path.parent().ok_or_else(|| BundleDirWriteError::Io {
        detail: "no parent directory".into(),
    })?;

    // Write to a temp file in the same directory, then rename.
    let temp_name = format!(
        ".tmp_{}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    let temp_path = dir.join(temp_name);

    std::fs::write(&temp_path, content).map_err(|e| BundleDirWriteError::Io {
        detail: format!("write {}: {e}", temp_path.display()),
    })?;

    std::fs::rename(&temp_path, path).map_err(|e| BundleDirWriteError::Io {
        detail: format!("rename {} → {}: {e}", temp_path.display(), path.display()),
    })?;

    Ok(())
}

/// Read a required metadata file; return error if missing.
fn read_required(dir: &Path, filename: &str) -> Result<Vec<u8>, BundleDirReadError> {
    read_file(dir, filename).map_err(|_| BundleDirReadError::MissingMetadata {
        filename: filename.to_string(),
    })
}

/// Read a file from the directory, returning I/O error.
fn read_file(dir: &Path, filename: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(dir.join(filename))
}

/// List all regular files in the directory (filenames only, no paths).
fn list_files(dir: &Path) -> Result<BTreeSet<String>, BundleDirReadError> {
    let mut files = BTreeSet::new();
    let entries = std::fs::read_dir(dir).map_err(|e| BundleDirReadError::Io {
        detail: format!("read_dir: {e}"),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| BundleDirReadError::Io {
            detail: format!("dir entry: {e}"),
        })?;

        let file_type = entry.file_type().map_err(|e| BundleDirReadError::Io {
            detail: format!("file_type: {e}"),
        })?;

        if file_type.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                // Skip hidden temp files from write_atomic.
                if !name.starts_with(".tmp_") {
                    files.insert(name.to_string());
                }
            }
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::build_bundle;

    fn test_bundle() -> ArtifactBundleV1 {
        build_bundle(vec![
            ("a.json".into(), b"{\"key\":\"value\"}".to_vec(), true),
            ("b.bin".into(), b"binary data".to_vec(), false),
        ])
        .unwrap()
    }

    #[test]
    fn write_read_roundtrip() {
        let bundle = test_bundle();
        let dir = tempfile::tempdir().unwrap();
        write_bundle_dir(&bundle, dir.path()).unwrap();
        let loaded = read_bundle_dir(dir.path()).unwrap();

        assert_eq!(loaded.manifest, bundle.manifest);
        assert_eq!(loaded.digest_basis, bundle.digest_basis);
        assert_eq!(loaded.digest.as_str(), bundle.digest.as_str());
        assert_eq!(loaded.artifacts.len(), bundle.artifacts.len());

        for (name, artifact) in &bundle.artifacts {
            let loaded_artifact = loaded.artifacts.get(name).unwrap();
            assert_eq!(loaded_artifact.content, artifact.content);
            assert_eq!(
                loaded_artifact.content_hash.as_str(),
                artifact.content_hash.as_str()
            );
            assert_eq!(loaded_artifact.normative, artifact.normative);
        }
    }

    #[test]
    fn verify_bundle_dir_passes_clean() {
        let bundle = test_bundle();
        let dir = tempfile::tempdir().unwrap();
        write_bundle_dir(&bundle, dir.path()).unwrap();
        verify_bundle_dir(dir.path()).unwrap();
    }

    #[test]
    fn read_fails_on_missing_manifest() {
        let dir = tempfile::tempdir().unwrap();
        // Write nothing.
        let err = read_bundle_dir(dir.path()).unwrap_err();
        assert!(matches!(err, BundleDirReadError::MissingMetadata { .. }));
    }

    #[test]
    fn read_fails_on_extra_file() {
        let bundle = test_bundle();
        let dir = tempfile::tempdir().unwrap();
        write_bundle_dir(&bundle, dir.path()).unwrap();

        // Add an undeclared file.
        std::fs::write(dir.path().join("rogue.txt"), b"surprise").unwrap();

        let err = read_bundle_dir(dir.path()).unwrap_err();
        assert!(matches!(err, BundleDirReadError::ExtraFile { .. }));
    }

    #[test]
    fn read_fails_on_missing_artifact() {
        let bundle = test_bundle();
        let dir = tempfile::tempdir().unwrap();
        write_bundle_dir(&bundle, dir.path()).unwrap();

        // Remove a declared artifact.
        std::fs::remove_file(dir.path().join("a.json")).unwrap();

        let err = read_bundle_dir(dir.path()).unwrap_err();
        assert!(matches!(err, BundleDirReadError::MissingArtifact { .. }));
    }

    #[test]
    fn read_fails_on_tampered_digest() {
        let bundle = test_bundle();
        let dir = tempfile::tempdir().unwrap();
        write_bundle_dir(&bundle, dir.path()).unwrap();

        // Tamper with digest file.
        std::fs::write(dir.path().join(DIGEST_FILENAME), b"sha256:0000").unwrap();

        let err = read_bundle_dir(dir.path()).unwrap_err();
        assert!(matches!(err, BundleDirReadError::DigestMismatch { .. }));
    }
}
