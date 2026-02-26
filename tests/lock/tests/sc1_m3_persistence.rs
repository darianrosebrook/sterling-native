//! SEARCH-CORE-001 M3.0 lock tests: search bundle persistence round-trip.
//!
//! These tests exercise `write_bundle_dir → read_bundle_dir → verify_bundle_dir`
//! for search bundles (5 artifacts including `search_graph.json`).
//! Positive controls prove round-trip idempotence; negative controls prove that
//! each falsifier mutation triggers the *specific* verifier failure path it targets.

use sterling_harness::bundle::{build_bundle, verify_bundle, ArtifactBundleV1, BundleVerifyError};
use sterling_harness::bundle_dir::{
    read_bundle_dir, verify_bundle_dir, write_bundle_dir, BundleDirReadError, BundleDirVerifyError,
};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_search::policy::SearchPolicyV1;

/// Produce a search bundle via `run_search(RomeMiniSearch)`.
fn search_bundle() -> ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("search run failed")
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-DIR-ROUNDTRIP
// ---------------------------------------------------------------------------

#[test]
fn search_bundle_roundtrip_produces_equivalent_bundle() {
    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // Bundle digest must match.
    assert_eq!(loaded.digest.as_str(), bundle.digest.as_str());

    // Manifest and digest_basis must be byte-identical.
    assert_eq!(loaded.manifest, bundle.manifest);
    assert_eq!(loaded.digest_basis, bundle.digest_basis);

    // All 5 artifacts must match.
    assert_eq!(loaded.artifacts.len(), bundle.artifacts.len());
    assert_eq!(
        loaded.artifacts.len(),
        5,
        "search bundle should have 5 artifacts"
    );

    for (name, artifact) in &bundle.artifacts {
        let loaded_artifact = loaded
            .artifacts
            .get(name)
            .unwrap_or_else(|| panic!("missing artifact after round-trip: {name}"));
        assert_eq!(
            loaded_artifact.content, artifact.content,
            "content mismatch: {name}"
        );
        assert_eq!(
            loaded_artifact.content_hash.as_str(),
            artifact.content_hash.as_str(),
            "content_hash mismatch: {name}"
        );
        assert_eq!(
            loaded_artifact.normative, artifact.normative,
            "normative flag mismatch: {name}"
        );
    }
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-DIR-VERIFY
// ---------------------------------------------------------------------------

#[test]
fn search_verify_bundle_dir_passes_clean_directory() {
    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    verify_bundle_dir(dir.path()).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-LOADED-VERIFY
// ---------------------------------------------------------------------------

#[test]
fn search_loaded_bundle_passes_verify_bundle() {
    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // verify_bundle runs search-specific checks: graph digest binding,
    // mode coherence, and metadata bindings.
    verify_bundle(&loaded).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-GRAPH-CANONICAL-STABLE
// ---------------------------------------------------------------------------

#[test]
fn search_graph_canonical_stable_across_roundtrip() {
    let bundle = search_bundle();
    let original_graph_bytes = bundle
        .artifacts
        .get("search_graph.json")
        .expect("missing search_graph.json")
        .content
        .clone();

    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    let loaded_graph_bytes = &loaded
        .artifacts
        .get("search_graph.json")
        .expect("missing search_graph.json after round-trip")
        .content;

    assert_eq!(
        &original_graph_bytes, loaded_graph_bytes,
        "search_graph.json bytes changed across persistence round-trip"
    );
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-TAMPER-GRAPH-BYTES
// ---------------------------------------------------------------------------

#[test]
fn search_fail_closed_tampered_graph_bytes() {
    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Tamper: flip one byte in search_graph.json on disk.
    let graph_path = dir.path().join("search_graph.json");
    let mut bytes = std::fs::read(&graph_path).unwrap();
    assert!(!bytes.is_empty());
    bytes[0] ^= 0xFF;
    std::fs::write(&graph_path, &bytes).unwrap();

    // read_bundle_dir catches tamper at the read boundary.
    let err = read_bundle_dir(dir.path()).unwrap_err();
    assert!(
        matches!(err, BundleDirReadError::ContentHashMismatch { ref name, .. } if name == "search_graph.json"),
        "expected ContentHashMismatch for search_graph.json, got {err}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-TAMPER-REPORT-BYTES
// ---------------------------------------------------------------------------

#[test]
fn search_fail_closed_tampered_report_bytes() {
    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Tamper: flip one byte in verification_report.json on disk.
    let report_path = dir.path().join("verification_report.json");
    let mut bytes = std::fs::read(&report_path).unwrap();
    assert!(!bytes.is_empty());
    bytes[0] ^= 0xFF;
    std::fs::write(&report_path, &bytes).unwrap();

    let err = read_bundle_dir(dir.path()).unwrap_err();
    assert!(
        matches!(err, BundleDirReadError::ContentHashMismatch { ref name, .. } if name == "verification_report.json"),
        "expected ContentHashMismatch for verification_report.json, got {err}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-DIGEST-BINDING-MISMATCH
// ---------------------------------------------------------------------------

#[test]
fn search_fail_closed_graph_digest_binding_mismatch() {
    let bundle = search_bundle();

    // Modify graph content (change a metadata field) but keep the original
    // report, which still has the old search_graph_digest.
    let graph_art = bundle.artifacts.get("search_graph.json").unwrap();
    let mut graph_json: serde_json::Value = serde_json::from_slice(&graph_art.content).unwrap();
    graph_json["metadata"]["total_expansions"] = serde_json::json!(999_999);
    let modified_graph = canonical_json_bytes(&graph_json).unwrap();

    // Rebuild bundle with modified graph but stale report.
    // build_bundle recomputes content hashes but doesn't validate semantic coherence.
    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "search_graph.json" {
                (a.name.clone(), modified_graph.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let broken_bundle = build_bundle(artifacts).unwrap();

    // Write and read back — read-boundary passes (hashes are self-consistent).
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&broken_bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // verify_bundle catches the binding mismatch: report's search_graph_digest
    // points to the old graph, but the loaded graph has different content_hash.
    let err = verify_bundle(&loaded).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::SearchGraphDigestMismatch { .. }),
        "expected SearchGraphDigestMismatch, got {err:?}"
    );

    // verify_bundle_dir should surface the same semantic error.
    let err_dir = verify_bundle_dir(dir.path()).unwrap_err();
    assert!(
        matches!(
            err_dir,
            BundleDirVerifyError::VerifyError(BundleVerifyError::SearchGraphDigestMismatch { .. })
        ),
        "expected VerifyError(SearchGraphDigestMismatch), got {err_dir}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-MISSING-GRAPH-FILE
// ---------------------------------------------------------------------------

#[test]
fn search_fail_closed_missing_graph_file() {
    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Remove search_graph.json from disk (manifest still declares it).
    std::fs::remove_file(dir.path().join("search_graph.json")).unwrap();

    let err = read_bundle_dir(dir.path()).unwrap_err();
    assert!(
        matches!(err, BundleDirReadError::MissingArtifact { ref name } if name == "search_graph.json"),
        "expected MissingArtifact for search_graph.json, got {err}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-MISSING-GRAPH-DIGEST-FIELD
// ---------------------------------------------------------------------------

#[test]
fn search_fail_closed_missing_graph_digest_field() {
    let bundle = search_bundle();

    // Build a bundle where the report lacks search_graph_digest.
    let report_art = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value = serde_json::from_slice(&report_art.content).unwrap();
    report_json
        .as_object_mut()
        .unwrap()
        .remove("search_graph_digest");
    let modified_report = canonical_json_bytes(&report_json).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "verification_report.json" {
                (a.name.clone(), modified_report.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let broken_bundle = build_bundle(artifacts).unwrap();

    // Write and read back — read-boundary passes.
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&broken_bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // verify_bundle catches the missing mandatory field.
    let err = verify_bundle(&loaded).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::SearchGraphDigestMissing),
        "expected SearchGraphDigestMissing, got {err:?}"
    );

    // verify_bundle_dir should surface the same error.
    let err_dir = verify_bundle_dir(dir.path()).unwrap_err();
    assert!(
        matches!(
            err_dir,
            BundleDirVerifyError::VerifyError(BundleVerifyError::SearchGraphDigestMissing)
        ),
        "expected VerifyError(SearchGraphDigestMissing), got {err_dir}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M3.0-SEARCH-NO-PATH-LEAKAGE
// ---------------------------------------------------------------------------

#[test]
fn search_no_path_leakage_in_normative_surfaces() {
    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    let loaded = read_bundle_dir(dir.path()).unwrap();
    let dir_str = dir.path().to_string_lossy();

    // Check metadata surfaces.
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

    // Check normative artifact contents.
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
