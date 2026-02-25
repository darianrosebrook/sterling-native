//! S1-M3 Harness Tests: verify that the runner produces a valid bundle.
//!
//! These tests prove that `run(RomeMini)` produces a well-formed
//! `ArtifactBundleV1` with the expected artifacts, correct hashes,
//! and normative/observational artifact classification.

use sterling_harness::bundle::{
    verify_bundle, BundleVerifyError, DOMAIN_BUNDLE_ARTIFACT, DOMAIN_BUNDLE_DIGEST,
};
use sterling_harness::runner::run;
use sterling_harness::worlds::rome_mini::RomeMini;
use sterling_kernel::carrier::trace_reader::bytes_to_trace;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;

// ---------------------------------------------------------------------------
// S1-M3: runner produces bundle
// ---------------------------------------------------------------------------

#[test]
fn harness_run_produces_bundle() {
    let bundle = run(&RomeMini).unwrap();
    assert!(!bundle.artifacts.is_empty());
    assert!(!bundle.manifest.is_empty());
    assert!(!bundle.digest_basis.is_empty());
    assert_eq!(bundle.digest.algorithm(), "sha256");
}

#[test]
fn bundle_contains_four_artifacts() {
    let bundle = run(&RomeMini).unwrap();
    assert_eq!(bundle.artifacts.len(), 4);

    let expected = [
        "compilation_manifest.json",
        "fixture.json",
        "trace.bst1",
        "verification_report.json",
    ];
    for name in expected {
        assert!(
            bundle.artifacts.contains_key(name),
            "missing artifact: {name}"
        );
    }
}

#[test]
fn trace_bst1_parses() {
    let bundle = run(&RomeMini).unwrap();
    let trace_artifact = bundle.artifacts.get("trace.bst1").unwrap();
    let trace = bytes_to_trace(&trace_artifact.content);
    assert!(trace.is_ok(), "trace.bst1 failed to parse: {trace:?}");
}

#[test]
fn verification_report_shows_match() {
    let bundle = run(&RomeMini).unwrap();
    let report = bundle.artifacts.get("verification_report.json").unwrap();
    let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();
    assert_eq!(json["replay_verdict"], "Match");
}

#[test]
fn bundle_manifest_hashes_match_content() {
    let bundle = run(&RomeMini).unwrap();

    for artifact in bundle.artifacts.values() {
        let recomputed = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &artifact.content);
        assert_eq!(
            artifact.content_hash.as_str(),
            recomputed.as_str(),
            "content hash mismatch for {}",
            artifact.name
        );
    }
}

#[test]
fn bundle_digest_matches_digest_basis() {
    let bundle = run(&RomeMini).unwrap();

    // Recompute digest from digest_basis.
    let recomputed = canonical_hash(DOMAIN_BUNDLE_DIGEST, &bundle.digest_basis);
    assert_eq!(
        bundle.digest.as_str(),
        recomputed.as_str(),
        "bundle digest does not match recomputed digest from digest_basis"
    );
}

#[test]
fn fixture_json_is_valid_canonical() {
    let bundle = run(&RomeMini).unwrap();
    let fixture = bundle.artifacts.get("fixture.json").unwrap();

    // Parse as JSON.
    let value: serde_json::Value = serde_json::from_slice(&fixture.content).unwrap();

    // Re-canonicalize and compare — fixture must already be canonical.
    let recanon = canonical_json_bytes(&value).unwrap();
    assert_eq!(
        fixture.content, recanon,
        "fixture.json is not in canonical form"
    );

    // Check expected fields.
    assert_eq!(value["schema_version"], "fixture.v1");
    assert_eq!(value["world_id"], "rome_mini");
    assert!(value["dimensions"].is_object());
    assert!(value["initial_payload_hex"].is_string());
    assert!(value["program"].is_array());
}

#[test]
fn verification_report_declares_planes_verified() {
    let bundle = run(&RomeMini).unwrap();
    let report = bundle.artifacts.get("verification_report.json").unwrap();
    let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();

    let planes = json["planes_verified"].as_array().unwrap();
    assert_eq!(planes.len(), 2);
    assert_eq!(planes[0], "identity");
    assert_eq!(planes[1], "status");
}

#[test]
fn normative_observational_classification() {
    let bundle = run(&RomeMini).unwrap();

    let normative = [
        "fixture.json",
        "compilation_manifest.json",
        "verification_report.json",
    ];
    for name in normative {
        let artifact = bundle.artifacts.get(name).unwrap();
        assert!(artifact.normative, "{name} should be normative");
    }

    let observational = ["trace.bst1"];
    for name in observational {
        let artifact = bundle.artifacts.get(name).unwrap();
        assert!(!artifact.normative, "{name} should be observational");
    }
}

// ---------------------------------------------------------------------------
// S1-M3: verify_bundle() integrity verification
// ---------------------------------------------------------------------------

#[test]
fn verify_bundle_passes_clean_bundle() {
    let bundle = run(&RomeMini).unwrap();
    verify_bundle(&bundle).unwrap();
}

#[test]
fn verify_bundle_detects_trace_report_payload_hash_mismatch() {
    let mut bundle = run(&RomeMini).unwrap();

    // Mutate trace.bst1 body bytes (not just envelope) so payload_hash changes.
    let trace_artifact = bundle.artifacts.get("trace.bst1").unwrap();
    let mut mutated_bytes = trace_artifact.content.clone();

    // The envelope is at the start; body comes after magic+header.
    // Flip a byte deep in the body to ensure payload changes.
    let body_offset = mutated_bytes.len() - 10;
    mutated_bytes[body_offset] ^= 0xFF;

    // Recompute the trace artifact's content_hash to match the mutated bytes.
    let new_content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &mutated_bytes);

    // Update the artifact in the bundle.
    let trace_entry = bundle.artifacts.get_mut("trace.bst1").unwrap();
    trace_entry.content = mutated_bytes;
    trace_entry.content_hash = new_content_hash;

    // Recompute the manifest to reflect the new content_hash for trace.bst1.
    // (trace.bst1 is observational, so digest_basis stays the same.)
    let manifest_artifacts: Vec<serde_json::Value> = bundle
        .artifacts
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
    bundle.manifest = canonical_json_bytes(&manifest_value).unwrap();

    // Now verify_bundle should fail specifically on the trace↔report binding,
    // not on content hash or manifest checks.
    let err = verify_bundle(&bundle).unwrap_err();
    match err {
        BundleVerifyError::PayloadHashMismatch { .. }
        | BundleVerifyError::StepChainMismatch { .. }
        | BundleVerifyError::TraceParseError { .. } => {
            // Expected: trace body mutation detected via report binding.
        }
        other => panic!(
            "expected PayloadHashMismatch, StepChainMismatch, or TraceParseError, got {other:?}"
        ),
    }
}
