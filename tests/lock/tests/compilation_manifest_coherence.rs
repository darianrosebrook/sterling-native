//! Lock tests for compilation manifest coherence in the search evidence corridor.
//!
//! Proves that `compilation_manifest.json` is constrained to agree with
//! already-bound corridor surfaces:
//! - Schema identity (`schema_id:schema_version:schema_hash`) matches graph
//!   metadata `schema_descriptor`.
//! - `payload_hash` matches recomputed hash from `fixture.json.initial_payload_hex`.
//! - `registry_hash` (stripped to raw hex) matches graph metadata `registry_digest`.
//! - Missing `compilation_manifest.json` is fail-closed (not silently skipped).

use lock_tests::bundle_test_helpers::{
    rebuild_with_modified_graph, rebuild_with_modified_graph_and_tape_header,
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

// ---------------------------------------------------------------------------
// Registry digest coherence (RCOH-001)
// ---------------------------------------------------------------------------

/// Compilation manifest's `registry_hash` (stripped to raw hex) matches
/// graph metadata `registry_digest`.
#[test]
fn registry_hash_matches_graph_registry_digest() {
    let bundle = default_bundle();

    // Parse compilation manifest.
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .expect("compilation_manifest.json");
    let cm: serde_json::Value = serde_json::from_slice(&cm_art.content).expect("valid JSON");
    let registry_hash = cm["registry_hash"].as_str().expect("registry_hash");

    // Strip sha256: prefix to get raw hex.
    let manifest_hex = registry_hash
        .strip_prefix("sha256:")
        .expect("registry_hash must have sha256: prefix");

    // Parse graph metadata.
    let graph_art = bundle
        .artifacts
        .get("search_graph.json")
        .expect("search_graph.json");
    let graph: serde_json::Value =
        serde_json::from_slice(&graph_art.content).expect("valid JSON");
    let graph_hex = graph["metadata"]["registry_digest"]
        .as_str()
        .expect("registry_digest");

    assert_eq!(
        manifest_hex, graph_hex,
        "compilation manifest registry_hash (stripped) must match graph metadata registry_digest"
    );

    // Full pipeline verification passes on both profiles.
    verify_bundle(&bundle).expect("bundle must verify");
}

/// Tampered `registry_hash` in compilation manifest triggers
/// `CompilationManifestRegistryMismatch`.
#[test]
fn tampered_compilation_manifest_registry_hash_rejected() {
    let bundle = default_bundle();

    // Verify the original bundle passes first.
    verify_bundle(&bundle).expect("original must pass");

    let tampered = resign_bundle_with_modified_compilation_manifest(&bundle, |cm| {
        // Preserve sha256: prefix but change one hex nibble.
        cm["registry_hash"] =
            serde_json::json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::CompilationManifestRegistryMismatch { .. }),
        "expected CompilationManifestRegistryMismatch, got: {err:?}"
    );
}

/// Missing `registry_hash` field in compilation manifest triggers
/// `CompilationManifestMissingField`.
#[test]
fn missing_compilation_manifest_registry_hash_field_rejected() {
    let bundle = default_bundle();

    verify_bundle(&bundle).expect("original must pass");

    let tampered = resign_bundle_with_modified_compilation_manifest(&bundle, |cm| {
        cm.as_object_mut().unwrap().remove("registry_hash");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationManifestMissingField { field: "registry_hash" }
        ),
        "expected CompilationManifestMissingField for registry_hash, got: {err:?}"
    );
}

/// Missing `registry_digest` in graph metadata triggers
/// `CompilationManifestGraphMissingField`.
///
/// Uses `rebuild_with_modified_graph` so Step 10 (`search_graph_digest`) passes
/// and the failure fires in Step 12b (compilation manifest coherence).
#[test]
fn missing_graph_registry_digest_field_rejected() {
    let bundle = default_bundle();

    verify_bundle(&bundle).expect("original must pass");

    let tampered = rebuild_with_modified_graph(&bundle, |graph| {
        graph["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("registry_digest");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationManifestGraphMissingField {
                field: "registry_digest"
            }
        ),
        "expected CompilationManifestGraphMissingField for registry_digest, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Root state digest coherence (IDCOH-001)
// ---------------------------------------------------------------------------

/// Compilation manifest's `identity_digest` and `evidence_digest` (stripped to
/// raw hex) match graph metadata `root_identity_digest` and `root_evidence_digest`.
#[test]
fn root_digests_match_compilation_manifest() {
    let bundle = default_bundle();

    // Parse compilation manifest.
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .expect("compilation_manifest.json");
    let cm: serde_json::Value = serde_json::from_slice(&cm_art.content).expect("valid JSON");

    let identity_digest = cm["identity_digest"].as_str().expect("identity_digest");
    let evidence_digest = cm["evidence_digest"].as_str().expect("evidence_digest");

    let manifest_id_hex = identity_digest
        .strip_prefix("sha256:")
        .expect("identity_digest must have sha256: prefix");
    let manifest_ev_hex = evidence_digest
        .strip_prefix("sha256:")
        .expect("evidence_digest must have sha256: prefix");

    // Parse graph metadata.
    let graph_art = bundle
        .artifacts
        .get("search_graph.json")
        .expect("search_graph.json");
    let graph: serde_json::Value =
        serde_json::from_slice(&graph_art.content).expect("valid JSON");
    let graph_id = graph["metadata"]["root_identity_digest"]
        .as_str()
        .expect("root_identity_digest");
    let graph_ev = graph["metadata"]["root_evidence_digest"]
        .as_str()
        .expect("root_evidence_digest");

    assert_eq!(
        manifest_id_hex, graph_id,
        "compilation manifest identity_digest (stripped) must match graph metadata root_identity_digest"
    );
    assert_eq!(
        manifest_ev_hex, graph_ev,
        "compilation manifest evidence_digest (stripped) must match graph metadata root_evidence_digest"
    );

    // Full pipeline verification passes.
    verify_bundle(&bundle).expect("bundle must verify");
}

/// Tampered `identity_digest` in compilation manifest triggers
/// `CompilationManifestIdentityMismatch`.
#[test]
fn tampered_compilation_manifest_identity_digest_rejected() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    let tampered = resign_bundle_with_modified_compilation_manifest(&bundle, |cm| {
        cm["identity_digest"] =
            serde_json::json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::CompilationManifestIdentityMismatch { .. }),
        "expected CompilationManifestIdentityMismatch, got: {err:?}"
    );
}

/// Tampered `evidence_digest` in compilation manifest triggers
/// `CompilationManifestEvidenceMismatch`.
#[test]
fn tampered_compilation_manifest_evidence_digest_rejected() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    let tampered = resign_bundle_with_modified_compilation_manifest(&bundle, |cm| {
        cm["evidence_digest"] =
            serde_json::json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::CompilationManifestEvidenceMismatch { .. }),
        "expected CompilationManifestEvidenceMismatch, got: {err:?}"
    );
}

/// Missing `root_identity_digest` in graph metadata triggers
/// `CompilationManifestGraphMissingField` (Cert profile).
///
/// Uses `rebuild_with_modified_graph` so Step 10 (`search_graph_digest`) passes
/// and the failure fires in Step 12b.
#[test]
fn missing_graph_root_identity_digest_rejected() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    let tampered = rebuild_with_modified_graph(&bundle, |graph| {
        graph["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("root_identity_digest");
    });

    // Base: required-if-present. With one field removed and one present,
    // Step 12b's "both present" branch won't fire; Base passes silently.
    // But Cert requires both — let's verify the Cert path.
    let err = sterling_harness::bundle::verify_bundle_with_profile(
        &tampered,
        sterling_harness::bundle::VerificationProfile::Cert,
    )
    .expect_err("must fail in Cert");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationManifestGraphMissingField {
                field: "root_identity_digest"
            }
        ),
        "expected CompilationManifestGraphMissingField for root_identity_digest, got: {err:?}"
    );
}

/// Missing `root_evidence_digest` in graph metadata triggers
/// `CompilationManifestGraphMissingField` (Cert profile).
#[test]
fn missing_graph_root_evidence_digest_rejected() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    let tampered = rebuild_with_modified_graph(&bundle, |graph| {
        graph["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("root_evidence_digest");
    });

    let err = sterling_harness::bundle::verify_bundle_with_profile(
        &tampered,
        sterling_harness::bundle::VerificationProfile::Cert,
    )
    .expect_err("must fail in Cert");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationManifestGraphMissingField {
                field: "root_evidence_digest"
            }
        ),
        "expected CompilationManifestGraphMissingField for root_evidence_digest, got: {err:?}"
    );
}

/// Tape header carries `root_identity_digest` and `root_evidence_digest`
/// matching graph metadata values.
#[test]
fn tape_header_carries_root_digests() {
    let bundle = default_bundle();

    let tape_art = bundle
        .artifacts
        .get("search_tape.stap")
        .expect("search_tape.stap");
    let tape = sterling_search::tape_reader::read_tape(&tape_art.content).expect("parse tape");
    let header = &tape.header.json;

    let graph_art = bundle
        .artifacts
        .get("search_graph.json")
        .expect("search_graph.json");
    let graph: serde_json::Value =
        serde_json::from_slice(&graph_art.content).expect("valid JSON");

    let tape_id = header["root_identity_digest"]
        .as_str()
        .expect("tape header root_identity_digest");
    let tape_ev = header["root_evidence_digest"]
        .as_str()
        .expect("tape header root_evidence_digest");
    let graph_id = graph["metadata"]["root_identity_digest"]
        .as_str()
        .expect("graph metadata root_identity_digest");
    let graph_ev = graph["metadata"]["root_evidence_digest"]
        .as_str()
        .expect("graph metadata root_evidence_digest");

    assert_eq!(tape_id, graph_id, "tape header root_identity_digest must match graph metadata");
    assert_eq!(tape_ev, graph_ev, "tape header root_evidence_digest must match graph metadata");
}

// ---------------------------------------------------------------------------
// Base-compat: legacy bundles without root digests (IDCOH-001-BASECOMPAT)
// ---------------------------------------------------------------------------

/// A "legacy" bundle with root digest fields stripped from both graph metadata
/// and tape header passes Base verification but fails Cert.
///
/// This is the only proof that Base tolerance actually works and prevents
/// future refactors from silently converting "required-if-present" into
/// "mandatory everywhere."
#[test]
fn legacy_bundle_without_root_digests_passes_base_fails_cert() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    let strip_root_digests = |obj: &mut serde_json::Value| {
        if let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            meta.remove("root_identity_digest");
            meta.remove("root_evidence_digest");
        } else if let Some(obj) = obj.as_object_mut() {
            // Tape header: fields are top-level.
            obj.remove("root_identity_digest");
            obj.remove("root_evidence_digest");
        }
    };

    let legacy = rebuild_with_modified_graph_and_tape_header(
        &bundle,
        strip_root_digests,
        |header| {
            header
                .as_object_mut()
                .unwrap()
                .remove("root_identity_digest");
            header
                .as_object_mut()
                .unwrap()
                .remove("root_evidence_digest");
        },
    );

    // Base: passes (fields absent on both sides → required-if-present → ok).
    sterling_harness::bundle::verify_bundle_with_profile(
        &legacy,
        sterling_harness::bundle::VerificationProfile::Base,
    )
    .expect("Base must pass for legacy bundle without root digests");

    // Cert: fails (mandatory fields absent).
    let err = sterling_harness::bundle::verify_bundle_with_profile(
        &legacy,
        sterling_harness::bundle::VerificationProfile::Cert,
    )
    .expect_err("Cert must fail for legacy bundle");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationManifestGraphMissingField {
                field: "root_identity_digest"
            }
        ),
        "expected CompilationManifestGraphMissingField for root_identity_digest, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// check_optional_tape_header_field arm coverage (IDCOH-001-ARMS)
// ---------------------------------------------------------------------------

/// Tape header has `root_identity_digest` but graph metadata does not → error
/// in both profiles (one-sided presence is always invalid).
#[test]
fn tape_header_one_sided_root_digest_rejected() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    // Strip from graph metadata only; tape header retains the field.
    let tampered = rebuild_with_modified_graph(&bundle, |graph| {
        graph["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("root_identity_digest");
        graph["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("root_evidence_digest");
    });

    // Step 12b passes in Base (both graph fields absent → skip).
    // Step 18 sees tape header has root_identity_digest but graph doesn't → error.
    let err = sterling_harness::bundle::verify_bundle_with_profile(
        &tampered,
        sterling_harness::bundle::VerificationProfile::Base,
    )
    .expect_err("must fail: one-sided tape header field");
    assert!(
        matches!(err, BundleVerifyError::TapeHeaderBindingMismatch { field: "root_identity_digest", .. }),
        "expected TapeHeaderBindingMismatch for root_identity_digest, got: {err:?}"
    );
}

/// Both tape header and graph metadata absent for root digests → Cert rejects
/// at Step 12b (before Step 18 can fire).
#[test]
fn both_absent_root_digests_cert_rejects_at_step_12b() {
    let bundle = default_bundle();

    let legacy = rebuild_with_modified_graph_and_tape_header(
        &bundle,
        |graph| {
            graph["metadata"]
                .as_object_mut()
                .unwrap()
                .remove("root_identity_digest");
            graph["metadata"]
                .as_object_mut()
                .unwrap()
                .remove("root_evidence_digest");
        },
        |header| {
            header
                .as_object_mut()
                .unwrap()
                .remove("root_identity_digest");
            header
                .as_object_mut()
                .unwrap()
                .remove("root_evidence_digest");
        },
    );

    // Cert rejects at Step 12b (graph missing mandatory field).
    let err = sterling_harness::bundle::verify_bundle_with_profile(
        &legacy,
        sterling_harness::bundle::VerificationProfile::Cert,
    )
    .expect_err("Cert must fail");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationManifestGraphMissingField {
                field: "root_identity_digest"
            }
        ),
        "expected CompilationManifestGraphMissingField, got: {err:?}"
    );
}
