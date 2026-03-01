//! Lock tests for compilation manifest coherence in the search evidence corridor.
//!
//! Proves that `compilation_manifest.json` is constrained to agree with
//! already-bound corridor surfaces:
//! - Schema identity (`schema_id:schema_version:schema_hash`) matches graph
//!   metadata `schema_descriptor`.
//! - `payload_hash` matches recomputed hash from `fixture.json.initial_payload_hex`.
//! - Missing `compilation_manifest.json` is fail-closed (not silently skipped).

use lock_tests::bundle_test_helpers::{
    rebuild_without_artifact, resign_bundle_with_modified_compilation_manifest,
};
use sterling_harness::bundle::{verify_bundle, BundleVerifyError};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, HashDomain};
use sterling_search::policy::SearchPolicyV1;

/// Produce a search bundle from `RomeMiniSearch` with default policy.
fn default_bundle() -> sterling_harness::bundle::ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("search run")
}

// ---------------------------------------------------------------------------
// Positive controls
// ---------------------------------------------------------------------------

/// Compilation manifest's `schema_id:schema_version:schema_hash` matches
/// graph metadata `schema_descriptor`.
#[test]
fn compilation_manifest_schema_matches_graph_metadata() {
    let bundle = default_bundle();

    // Parse compilation manifest.
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .expect("compilation_manifest.json");
    let cm: serde_json::Value = serde_json::from_slice(&cm_art.content).expect("valid JSON");

    let schema_id = cm["schema_id"].as_str().expect("schema_id");
    let schema_version = cm["schema_version"].as_str().expect("schema_version");
    let schema_hash = cm["schema_hash"].as_str().expect("schema_hash");
    let manifest_sd = format!("{schema_id}:{schema_version}:{schema_hash}");

    // Parse graph metadata.
    let graph_art = bundle
        .artifacts
        .get("search_graph.json")
        .expect("search_graph.json");
    let graph: serde_json::Value = serde_json::from_slice(&graph_art.content).expect("valid JSON");
    let graph_sd = graph["metadata"]["schema_descriptor"]
        .as_str()
        .expect("schema_descriptor");

    assert_eq!(
        manifest_sd, graph_sd,
        "compilation manifest schema must match graph metadata schema_descriptor"
    );
}

/// Compilation manifest's `payload_hash` matches recomputed hash from
/// `fixture.json.initial_payload_hex`.
#[test]
fn compilation_manifest_payload_hash_matches_fixture() {
    let bundle = default_bundle();

    // Parse compilation manifest.
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .expect("compilation_manifest.json");
    let cm: serde_json::Value = serde_json::from_slice(&cm_art.content).expect("valid JSON");
    let manifest_payload_hash = cm["payload_hash"].as_str().expect("payload_hash");

    // Parse fixture and recompute payload_hash.
    let fixture_art = bundle
        .artifacts
        .get("fixture.json")
        .expect("fixture.json");
    let fixture: serde_json::Value =
        serde_json::from_slice(&fixture_art.content).expect("valid JSON");
    let payload_hex = fixture["initial_payload_hex"]
        .as_str()
        .expect("initial_payload_hex");

    let payload_bytes = hex::decode(payload_hex).expect("hex decode");
    let payload_json: serde_json::Value =
        serde_json::from_slice(&payload_bytes).expect("payload JSON");
    let canonical_payload = canonical_json_bytes(&payload_json).expect("canonicalize");
    let recomputed = canonical_hash(HashDomain::CompilationPayload, &canonical_payload);

    assert_eq!(
        manifest_payload_hash,
        recomputed.as_str(),
        "compilation manifest payload_hash must match recomputed hash from fixture"
    );
}

// ---------------------------------------------------------------------------
// Negative controls
// ---------------------------------------------------------------------------

/// Tampered `schema_id` in compilation manifest triggers
/// `CompilationManifestSchemaMismatch`.
#[test]
fn tampered_compilation_manifest_schema_rejected() {
    let bundle = default_bundle();

    // Verify the original bundle passes first.
    verify_bundle(&bundle).expect("original must pass");

    let tampered = resign_bundle_with_modified_compilation_manifest(&bundle, |cm| {
        cm["schema_id"] = serde_json::json!("tampered_schema_id");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::CompilationManifestSchemaMismatch { .. }),
        "expected CompilationManifestSchemaMismatch, got: {err:?}"
    );
}

/// Tampered `payload_hash` in compilation manifest triggers
/// `CompilationManifestPayloadMismatch`.
#[test]
fn tampered_compilation_manifest_payload_hash_rejected() {
    let bundle = default_bundle();

    // Verify the original bundle passes first.
    verify_bundle(&bundle).expect("original must pass");

    let tampered = resign_bundle_with_modified_compilation_manifest(&bundle, |cm| {
        cm["payload_hash"] = serde_json::json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::CompilationManifestPayloadMismatch { .. }),
        "expected CompilationManifestPayloadMismatch, got: {err:?}"
    );
}

/// Missing `compilation_manifest.json` triggers `CompilationManifestMissing`.
/// Proves fail-closed on omission — no bypass by artifact removal.
#[test]
fn missing_compilation_manifest_rejected() {
    let bundle = default_bundle();

    // Verify the original bundle passes first.
    verify_bundle(&bundle).expect("original must pass");

    let stripped = rebuild_without_artifact(&bundle, "compilation_manifest.json");

    let err = verify_bundle(&stripped).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::CompilationManifestMissing),
        "expected CompilationManifestMissing, got: {err:?}"
    );
}
