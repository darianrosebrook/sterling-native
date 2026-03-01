//! Lock tests for compilation boundary replay verification.
//!
//! Proves:
//! - Cert-mode replay reconstructs `compile()` inputs from bundle artifacts
//!   and asserts the output `compilation_manifest` bytes are byte-identical.
//! - Coherently forged corridor surfaces (manifest + graph + tape agree but
//!   don't match actual `compile()` output) are rejected by replay.
//! - Tampered `concept_registry.json` causes replay failure.
//! - Base profile skips replay entirely.

use lock_tests::bundle_test_helpers::rebuild_with_modified_graph_and_tape_header;
use sterling_harness::bundle::{
    build_bundle, verify_bundle, verify_bundle_with_profile, BundleVerifyError,
    VerificationProfile,
};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;
use sterling_search::policy::SearchPolicyV1;

/// Produce a search bundle from `RomeMiniSearch` with default policy.
fn default_bundle() -> sterling_harness::bundle::ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("search run")
}

// ---------------------------------------------------------------------------
// Positive controls
// ---------------------------------------------------------------------------

/// Cert replay passes on a valid `RomeMiniSearch` bundle.
#[test]
fn cert_replay_passes_valid_bundle() {
    let bundle = default_bundle();
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert).expect("Cert must pass");
}

// ---------------------------------------------------------------------------
// Negative controls
// ---------------------------------------------------------------------------

/// A coherently forged bundle where `compilation_manifest.json`, graph metadata,
/// and tape header all agree on a wrong `identity_digest` passes Steps 12b/12c/18
/// but fails Step 12d (replay).
///
/// This is the unique value proposition of replay: field-level coherence checks
/// only prove that corridor surfaces agree, not that they were produced by
/// `compile()`.
#[test]
fn coherently_forged_manifest_rejected_by_replay() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    let forged_digest =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let forged_hex = "0000000000000000000000000000000000000000000000000000000000000000";

    // Forge identity_digest in manifest.
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .unwrap();
    let mut cm_json: serde_json::Value =
        serde_json::from_slice(&cm_art.content).unwrap();
    cm_json["identity_digest"] = serde_json::json!(forged_digest);
    let modified_cm_bytes = canonical_json_bytes(&cm_json).unwrap();

    // Forge root_identity_digest in both graph metadata AND tape header.
    let forged_bundle = rebuild_with_modified_graph_and_tape_header(
        &bundle,
        |graph| {
            graph["metadata"]["root_identity_digest"] = serde_json::json!(forged_hex);
        },
        |header| {
            header["root_identity_digest"] = serde_json::json!(forged_hex);
        },
    );

    // Now replace compilation_manifest.json in the forged bundle.
    let artifacts: Vec<(String, Vec<u8>, bool)> = forged_bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "compilation_manifest.json" {
                (a.name.clone(), modified_cm_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let forged = build_bundle(artifacts).unwrap();

    // Base: passes (replay skipped; all field-level coherence checks see agreement).
    verify_bundle_with_profile(&forged, VerificationProfile::Base)
        .expect("Base must pass on coherently forged bundle");

    // Cert: replay detects the forgery — compile() produces the real identity_digest.
    let err = verify_bundle_with_profile(&forged, VerificationProfile::Cert)
        .expect_err("Cert must fail on coherently forged bundle");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationReplayManifestMismatch { .. }
        ),
        "expected CompilationReplayManifestMismatch, got: {err:?}"
    );
}

/// Tampered `concept_registry.json` with modified epoch causes compilation
/// replay to fail because `compile()` with the wrong registry produces a
/// manifest with a different `registry_epoch`, triggering byte mismatch.
#[test]
fn tampered_registry_epoch_fails_replay() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    // Change the registry epoch — this changes the registry_epoch field
    // in compile()'s output manifest.
    let reg_art = bundle.artifacts.get("concept_registry.json").unwrap();
    let mut reg_json: serde_json::Value =
        serde_json::from_slice(&reg_art.content).unwrap();
    reg_json["epoch"] = serde_json::json!("tampered-epoch");
    let modified_reg_bytes = canonical_json_bytes(&reg_json).unwrap();

    // Recompute semantic digest for the tampered registry.
    let new_reg_digest = canonical_hash(
        sterling_kernel::proof::hash::HashDomain::RegistrySnapshot,
        &modified_reg_bytes,
    );

    // Patch manifest registry_hash to match tampered registry (so Step 12c passes).
    // Do NOT change registry_epoch in manifest — that's what replay will catch.
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .unwrap();
    let mut cm_json: serde_json::Value =
        serde_json::from_slice(&cm_art.content).unwrap();
    cm_json["registry_hash"] = serde_json::json!(new_reg_digest.as_str());
    let modified_cm_bytes = canonical_json_bytes(&cm_json).unwrap();

    // Patch graph metadata registry_digest AND tape header registry_digest.
    let forged_bundle = rebuild_with_modified_graph_and_tape_header(
        &bundle,
        |graph| {
            graph["metadata"]["registry_digest"] =
                serde_json::json!(new_reg_digest.hex_digest());
        },
        |header| {
            header["registry_digest"] =
                serde_json::json!(new_reg_digest.hex_digest());
        },
    );

    // Replace concept_registry.json and compilation_manifest.json.
    let artifacts: Vec<(String, Vec<u8>, bool)> = forged_bundle
        .artifacts
        .values()
        .map(|a| match a.name.as_str() {
            "concept_registry.json" => {
                (a.name.clone(), modified_reg_bytes.clone(), a.normative)
            }
            "compilation_manifest.json" => {
                (a.name.clone(), modified_cm_bytes.clone(), a.normative)
            }
            _ => (a.name.clone(), a.content.clone(), a.normative),
        })
        .collect();
    let tampered = build_bundle(artifacts).unwrap();

    // Cert: replay uses tampered registry (tampered-epoch) → compile() produces
    // manifest with registry_epoch="tampered-epoch", but stored manifest has
    // original epoch. Byte mismatch.
    let err = verify_bundle_with_profile(&tampered, VerificationProfile::Cert)
        .expect_err("Cert must fail with tampered registry epoch");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationReplayManifestMismatch { .. }
        ),
        "expected CompilationReplayManifestMismatch, got: {err:?}"
    );
}

/// Base profile skips compilation replay entirely.
///
/// A bundle with a forged `identity_digest` (coherently patched across all
/// corridor surfaces) passes Base but fails Cert.
#[test]
fn base_skips_replay() {
    let bundle = default_bundle();

    let forged_digest =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let forged_hex = "0000000000000000000000000000000000000000000000000000000000000000";

    // Forge identity_digest in manifest.
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .unwrap();
    let mut cm_json: serde_json::Value =
        serde_json::from_slice(&cm_art.content).unwrap();
    cm_json["identity_digest"] = serde_json::json!(forged_digest);
    let modified_cm_bytes = canonical_json_bytes(&cm_json).unwrap();

    // Forge root_identity_digest in graph metadata + tape header.
    let forged_bundle = rebuild_with_modified_graph_and_tape_header(
        &bundle,
        |graph| {
            graph["metadata"]["root_identity_digest"] = serde_json::json!(forged_hex);
        },
        |header| {
            header["root_identity_digest"] = serde_json::json!(forged_hex);
        },
    );

    // Replace compilation_manifest.json.
    let artifacts: Vec<(String, Vec<u8>, bool)> = forged_bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "compilation_manifest.json" {
                (a.name.clone(), modified_cm_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let forged = build_bundle(artifacts).unwrap();

    // Base: must pass (replay not executed).
    verify_bundle_with_profile(&forged, VerificationProfile::Base)
        .expect("Base must pass — replay is not executed");

    // Cert: must fail (replay catches the forgery).
    let err = verify_bundle_with_profile(&forged, VerificationProfile::Cert)
        .expect_err("Cert must fail");
    assert!(
        matches!(
            err,
            BundleVerifyError::CompilationReplayManifestMismatch { .. }
        ),
        "expected CompilationReplayManifestMismatch, got: {err:?}"
    );
}
