//! S1-M4 lock tests: bundle directory persistence round-trip and verification.
//!
//! These tests exercise the `BundleDirectoryV1` format through the full harness
//! pipeline: produce bundle → write directory → read directory → verify.

use sterling_harness::bundle::verify_bundle;
use sterling_harness::bundle_dir::{
    read_bundle_dir, verify_bundle_dir, write_bundle_dir, BundleDirReadError, BundleDirVerifyError,
};
use sterling_harness::runner::run;
use sterling_harness::worlds::rome_mini::RomeMini;

// ---------------------------------------------------------------------------
// S1-M4-DIR-WRITE-READ-ROUNDTRIP
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_produces_equivalent_bundle() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // Digest must match.
    assert_eq!(loaded.digest.as_str(), bundle.digest.as_str());

    // Manifest and digest_basis must be byte-identical.
    assert_eq!(loaded.manifest, bundle.manifest);
    assert_eq!(loaded.digest_basis, bundle.digest_basis);

    // All artifacts must match.
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

// ---------------------------------------------------------------------------
// S1-M4-DIR-VERIFY-PASSES
// ---------------------------------------------------------------------------

#[test]
fn verify_bundle_dir_passes_clean_directory() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    verify_bundle_dir(dir.path()).unwrap();
}

#[test]
fn loaded_bundle_passes_verify_bundle() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();
    verify_bundle(&loaded).unwrap();
}

// ---------------------------------------------------------------------------
// S1-M4-FAIL-CLOSED-MISSING-FILE
// ---------------------------------------------------------------------------

#[test]
fn fail_closed_missing_artifact_file() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Remove a declared artifact.
    std::fs::remove_file(dir.path().join("verification_report.json")).unwrap();

    let err = read_bundle_dir(dir.path()).unwrap_err();
    assert!(
        matches!(err, BundleDirReadError::MissingArtifact { ref name } if name == "verification_report.json"),
        "expected MissingArtifact for verification_report.json, got {err}"
    );
}

#[test]
fn fail_closed_missing_manifest() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    std::fs::remove_file(dir.path().join("bundle_manifest.json")).unwrap();

    let err = read_bundle_dir(dir.path()).unwrap_err();
    assert!(matches!(err, BundleDirReadError::MissingMetadata { .. }));
}

// ---------------------------------------------------------------------------
// S1-M4-FAIL-CLOSED-EXTRA-FILE
// ---------------------------------------------------------------------------

#[test]
fn fail_closed_extra_file() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Add an undeclared file.
    std::fs::write(dir.path().join("rogue.txt"), b"surprise").unwrap();

    let err = read_bundle_dir(dir.path()).unwrap_err();
    assert!(
        matches!(err, BundleDirReadError::ExtraFile { ref name } if name == "rogue.txt"),
        "expected ExtraFile for rogue.txt, got {err}"
    );
}

// ---------------------------------------------------------------------------
// S1-M4-FAIL-CLOSED-HASH-MISMATCH
// ---------------------------------------------------------------------------

#[test]
fn fail_closed_content_hash_mismatch_at_read_boundary() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Tamper with an artifact file without updating the manifest.
    let fixture_path = dir.path().join("fixture.json");
    std::fs::write(&fixture_path, b"{\"tampered\":true}").unwrap();

    // read_bundle_dir now enforces content hash at the read boundary.
    // Tampering is rejected before verify_bundle() is ever called.
    let err = verify_bundle_dir(dir.path()).unwrap_err();
    match err {
        BundleDirVerifyError::ReadError(ref re) => {
            assert!(
                format!("{re:?}").contains("ContentHashMismatch"),
                "expected ContentHashMismatch, got {re:?}"
            );
        }
        BundleDirVerifyError::VerifyError(ve) => {
            panic!("expected ReadError(ContentHashMismatch), got VerifyError({ve:?})")
        }
    }
}

// ---------------------------------------------------------------------------
// S1-M4-FAIL-CLOSED-NONCANONICAL-MANIFEST
// ---------------------------------------------------------------------------

#[test]
fn fail_closed_noncanonical_manifest() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Rewrite manifest with pretty-printed (non-canonical) JSON.
    let manifest_value: serde_json::Value = serde_json::from_slice(&bundle.manifest).unwrap();
    let pretty = serde_json::to_vec_pretty(&manifest_value).unwrap();
    std::fs::write(dir.path().join("bundle_manifest.json"), &pretty).unwrap();

    // verify_bundle_dir should detect the non-canonical manifest.
    let err = verify_bundle_dir(dir.path()).unwrap_err();
    match err {
        BundleDirVerifyError::VerifyError(ref ve) => {
            assert!(
                format!("{ve:?}").contains("ManifestMismatch")
                    || format!("{ve:?}").contains("ManifestNotCanonical"),
                "expected manifest-related error, got {ve:?}"
            );
        }
        BundleDirVerifyError::ReadError(re) => panic!("expected VerifyError, got ReadError({re})"),
    }
}

// ---------------------------------------------------------------------------
// S1-M4-NO-PATH-LEAKAGE
// ---------------------------------------------------------------------------

#[test]
fn no_path_leakage_in_normative_surfaces() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Read back and check normative surfaces for path fragments.
    let loaded = read_bundle_dir(dir.path()).unwrap();

    let dir_str = dir.path().to_string_lossy();
    let surfaces_to_check: Vec<(&str, &[u8])> = vec![
        ("manifest", &loaded.manifest),
        ("digest_basis", &loaded.digest_basis),
    ];

    for (label, bytes) in &surfaces_to_check {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains(dir_str.as_ref()),
            "{label} contains directory path: {dir_str}"
        );
        assert!(
            !text.contains("/Users/"),
            "{label} contains absolute path fragment /Users/"
        );
        assert!(
            !text.contains("/home/"),
            "{label} contains absolute path fragment /home/"
        );
    }

    // Check normative artifact contents too.
    for artifact in loaded.artifacts.values() {
        if artifact.normative {
            let text = String::from_utf8_lossy(&artifact.content);
            assert!(
                !text.contains(dir_str.as_ref()),
                "normative artifact {} contains directory path",
                artifact.name
            );
        }
    }
}
