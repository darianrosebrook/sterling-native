//! Lock tests for concept registry artifact coherence.
//!
//! Proves:
//! - `concept_registry.json` is present in search bundles and normative.
//! - Its semantic digest (`DOMAIN_REGISTRY_SNAPSHOT`) matches both
//!   `compilation_manifest.json.registry_hash` and
//!   `search_graph.json.metadata.registry_digest`.
//! - Tampered bytes trigger a typed digest mismatch error.
//! - Base passes without the artifact; Cert fails with `ConceptRegistryMissing`.

use lock_tests::bundle_test_helpers::rebuild_without_artifact;
use sterling_harness::bundle::{verify_bundle, BundleVerifyError};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
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

/// `concept_registry.json` exists, is normative, and its semantic digest
/// matches both `compilation_manifest.json.registry_hash` and
/// `search_graph.json.metadata.registry_digest`.
#[test]
fn concept_registry_digest_matches_corridor_claims() {
    let bundle = default_bundle();

    // Artifact present and normative.
    let reg_art = bundle
        .artifacts
        .get("concept_registry.json")
        .expect("concept_registry.json must be present");
    assert!(reg_art.normative, "concept_registry.json must be normative");

    // Recompute semantic digest from artifact bytes.
    let semantic_digest = canonical_hash(HashDomain::RegistrySnapshot, &reg_art.content);

    // Check against compilation manifest registry_hash (full sha256:hex).
    let cm_art = bundle
        .artifacts
        .get("compilation_manifest.json")
        .expect("compilation_manifest.json");
    let cm: serde_json::Value = serde_json::from_slice(&cm_art.content).expect("valid JSON");
    let registry_hash = cm["registry_hash"].as_str().expect("registry_hash");
    assert_eq!(
        semantic_digest.as_str(),
        registry_hash,
        "semantic digest must match compilation_manifest.json registry_hash"
    );

    // Check against graph metadata registry_digest (raw hex).
    let graph_art = bundle
        .artifacts
        .get("search_graph.json")
        .expect("search_graph.json");
    let graph: serde_json::Value = serde_json::from_slice(&graph_art.content).expect("valid JSON");
    let graph_hex = graph["metadata"]["registry_digest"]
        .as_str()
        .expect("registry_digest");
    assert_eq!(
        semantic_digest.hex_digest(),
        graph_hex,
        "semantic digest hex must match graph metadata registry_digest"
    );

    // Full verification passes.
    verify_bundle(&bundle).expect("bundle must verify");
}

// ---------------------------------------------------------------------------
// Negative controls
// ---------------------------------------------------------------------------

/// Tampered `concept_registry.json` triggers `ConceptRegistryDigestMismatch`.
#[test]
fn tampered_concept_registry_rejected() {
    use sterling_harness::bundle::build_bundle;
    use sterling_kernel::proof::canon::canonical_json_bytes;

    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    // Modify the concept registry bytes (add a field to the JSON).
    let reg_art = bundle.artifacts.get("concept_registry.json").unwrap();
    let mut reg_json: serde_json::Value =
        serde_json::from_slice(&reg_art.content).unwrap();
    reg_json["tampered"] = serde_json::json!(true);
    let modified_bytes = canonical_json_bytes(&reg_json).unwrap();

    // Rebuild bundle with modified concept_registry.json.
    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "concept_registry.json" {
                (a.name.clone(), modified_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let tampered = build_bundle(artifacts).unwrap();

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::ConceptRegistryDigestMismatch { .. }),
        "expected ConceptRegistryDigestMismatch, got: {err:?}"
    );
}

/// Missing `concept_registry.json` passes in Base, fails in Cert.
#[test]
fn missing_concept_registry_base_passes_cert_fails() {
    let bundle = default_bundle();
    verify_bundle(&bundle).expect("original must pass");

    let stripped = rebuild_without_artifact(&bundle, "concept_registry.json");

    // Base: passes (required-if-present, artifact absent → skip).
    sterling_harness::bundle::verify_bundle_with_profile(
        &stripped,
        sterling_harness::bundle::VerificationProfile::Base,
    )
    .expect("Base must pass without concept_registry.json");

    // Cert: fails (mandatory).
    let err = sterling_harness::bundle::verify_bundle_with_profile(
        &stripped,
        sterling_harness::bundle::VerificationProfile::Cert,
    )
    .expect_err("Cert must fail");
    assert!(
        matches!(err, BundleVerifyError::ConceptRegistryMissing),
        "expected ConceptRegistryMissing, got: {err:?}"
    );
}
