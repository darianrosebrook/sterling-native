//! S1-M5 Policy Tests: verify that bundles include policy snapshots with
//! enforcement and verification binding.

use sterling_harness::bundle::{verify_bundle, DOMAIN_BUNDLE_ARTIFACT};
use sterling_harness::bundle_dir::{verify_bundle_dir, write_bundle_dir};
use sterling_harness::policy::{PolicyConfig, PolicyViolation};
use sterling_harness::runner::{run, run_with_policy, RunError};
use sterling_harness::worlds::rome_mini::RomeMini;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;

// ---------------------------------------------------------------------------
// S1-M5-POLICY-IN-BUNDLE: policy snapshot present, normative, verifiable
// ---------------------------------------------------------------------------

#[test]
fn policy_snapshot_is_normative_and_verifies() {
    let bundle = run(&RomeMini).unwrap();

    let policy = bundle
        .artifacts
        .get("policy_snapshot.json")
        .expect("missing policy_snapshot.json");
    assert!(policy.normative, "policy_snapshot.json should be normative");

    // Verify the full bundle passes.
    verify_bundle(&bundle).unwrap();
}

#[test]
fn policy_snapshot_content_is_valid_json() {
    let bundle = run(&RomeMini).unwrap();
    let policy = bundle.artifacts.get("policy_snapshot.json").unwrap();
    let json: serde_json::Value = serde_json::from_slice(&policy.content).unwrap();

    assert_eq!(json["schema_version"], "policy.v1");
    assert_eq!(json["world_id"], "rome_mini");
    assert!(json["allowed_ops"].is_array());
    assert!(json["budgets"].is_object());
    assert!(json["determinism_contract"].is_object());
}

#[test]
fn policy_snapshot_is_canonical_json() {
    let bundle = run(&RomeMini).unwrap();
    let policy = bundle.artifacts.get("policy_snapshot.json").unwrap();
    let value: serde_json::Value = serde_json::from_slice(&policy.content).unwrap();
    let recanonicalized = canonical_json_bytes(&value).unwrap();
    assert_eq!(
        policy.content, recanonicalized,
        "policy_snapshot.json is not in canonical form"
    );
}

#[test]
fn policy_snapshot_in_digest_basis() {
    let bundle = run(&RomeMini).unwrap();

    // Parse digest_basis and verify policy_snapshot.json is listed.
    let basis: serde_json::Value = serde_json::from_slice(&bundle.digest_basis).unwrap();
    let artifacts = basis["artifacts"].as_array().unwrap();
    let policy_entry = artifacts
        .iter()
        .find(|a| a["name"] == "policy_snapshot.json");
    assert!(
        policy_entry.is_some(),
        "policy_snapshot.json not found in digest_basis"
    );
}

// ---------------------------------------------------------------------------
// S1-M5-POLICY-DIGEST-IN-REPORT: verification report carries policy_digest
// ---------------------------------------------------------------------------

#[test]
fn verification_report_contains_policy_digest() {
    let bundle = run(&RomeMini).unwrap();
    let report = bundle.artifacts.get("verification_report.json").unwrap();
    let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();

    let policy_digest = json["policy_digest"].as_str().unwrap();
    assert!(policy_digest.starts_with("sha256:"));

    // Must match the policy artifact's content_hash.
    let policy = bundle.artifacts.get("policy_snapshot.json").unwrap();
    assert_eq!(
        policy_digest,
        policy.content_hash.as_str(),
        "policy_digest in report does not match policy artifact content_hash"
    );
}

#[test]
fn verify_bundle_detects_policy_digest_mismatch() {
    let mut bundle = run(&RomeMini).unwrap();

    // Tamper with the policy artifact content (add a byte) and update its
    // content_hash, but leave the verification_report unchanged.
    let policy = bundle.artifacts.get_mut("policy_snapshot.json").unwrap();
    policy.content.push(b' ');
    policy.content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &policy.content);

    // Recompute manifest (since content_hash changed).
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

    // Recompute digest_basis (policy is normative, so it changes).
    let normative_artifacts: Vec<serde_json::Value> = bundle
        .artifacts
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
    bundle.digest_basis = canonical_json_bytes(&digest_basis_value).unwrap();
    bundle.digest = canonical_hash(
        sterling_harness::bundle::DOMAIN_BUNDLE_DIGEST,
        &bundle.digest_basis,
    );

    let err = verify_bundle(&bundle).unwrap_err();
    // The policy artifact is no longer canonical JSON (trailing space), so it
    // may fail on ArtifactNotCanonical or PolicyDigestMismatch. Either is
    // correct detection.
    match err {
        sterling_harness::bundle::BundleVerifyError::ArtifactNotCanonical { .. }
        | sterling_harness::bundle::BundleVerifyError::PolicyDigestMismatch { .. } => {}
        other => panic!("expected ArtifactNotCanonical or PolicyDigestMismatch, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// S1-M5-BUDGET-STEP-OVERFLOW: step budget enforcement
// ---------------------------------------------------------------------------

#[test]
fn fails_on_step_budget_overflow() {
    let config = PolicyConfig {
        max_steps: Some(1), // RomeMini needs 2 frames
        ..PolicyConfig::default()
    };
    let err = run_with_policy(&RomeMini, &config).unwrap_err();
    match err {
        RunError::PolicyViolation(PolicyViolation::StepBudgetExceeded {
            max_steps: 1,
            actual: 2,
        }) => {}
        other => panic!("expected StepBudgetExceeded(1, 2), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// S1-M5-ALLOWLIST-VIOLATION: op allowlist enforcement
// ---------------------------------------------------------------------------

#[test]
fn fails_on_allowlist_violation() {
    let config = PolicyConfig {
        allowed_ops: Some(vec![Code32::new(99, 99, 99)]),
        ..PolicyConfig::default()
    };
    let err = run_with_policy(&RomeMini, &config).unwrap_err();
    match err {
        RunError::PolicyViolation(PolicyViolation::AllowlistViolation { .. }) => {}
        other => panic!("expected AllowlistViolation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// S1-M5-DETERMINISM-INPROC: in-process determinism
// ---------------------------------------------------------------------------

#[test]
fn policy_snapshot_deterministic_n10() {
    let first = run(&RomeMini).unwrap();
    let first_policy = first.artifacts.get("policy_snapshot.json").unwrap();

    for _ in 1..10 {
        let other = run(&RomeMini).unwrap();
        let other_policy = other.artifacts.get("policy_snapshot.json").unwrap();
        assert_eq!(
            first_policy.content, other_policy.content,
            "policy_snapshot bytes differ across runs"
        );
        assert_eq!(
            first.digest.as_str(),
            other.digest.as_str(),
            "bundle digest differs across runs"
        );
    }
}

// ---------------------------------------------------------------------------
// S1-M5-POLICY-CANONICAL: noncanonical policy detected by verify
// ---------------------------------------------------------------------------

#[test]
fn rejects_noncanonical_policy_snapshot() {
    let bundle = run(&RomeMini).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();

    // Pretty-print the policy snapshot (making it noncanonical).
    let policy_path = dir.path().join("policy_snapshot.json");
    let content = std::fs::read(&policy_path).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&content).unwrap();
    let pretty = serde_json::to_vec_pretty(&value).unwrap();
    std::fs::write(&policy_path, &pretty).unwrap();

    // Update manifest with new content hash so read_bundle_dir() doesn't fail
    // on content hash mismatch.
    let new_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &pretty);
    let manifest_path = dir.path().join("bundle_manifest.json");
    let manifest_bytes = std::fs::read(&manifest_path).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for artifact in manifest["artifacts"].as_array_mut().unwrap() {
        if artifact["name"] == "policy_snapshot.json" {
            artifact["content_hash"] = serde_json::Value::String(new_hash.as_str().to_string());
        }
    }
    let new_manifest = canonical_json_bytes(&manifest).unwrap();
    std::fs::write(&manifest_path, &new_manifest).unwrap();

    // verify_bundle_dir should fail (either read-time or verify-time).
    let err = verify_bundle_dir(dir.path());
    assert!(err.is_err(), "expected failure for noncanonical policy");
}

// ---------------------------------------------------------------------------
// Trace byte budget
// ---------------------------------------------------------------------------

#[test]
fn fails_on_trace_byte_budget() {
    let config = PolicyConfig {
        max_trace_bytes: Some(1), // impossibly small
        ..PolicyConfig::default()
    };
    let err = run_with_policy(&RomeMini, &config).unwrap_err();
    match err {
        RunError::PolicyViolation(PolicyViolation::TraceByteBudgetExceeded { .. }) => {}
        other => panic!("expected TraceByteBudgetExceeded, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// No path leakage in policy snapshot
// ---------------------------------------------------------------------------

#[test]
fn no_path_leakage_in_policy_snapshot() {
    let bundle = run(&RomeMini).unwrap();
    let policy = bundle.artifacts.get("policy_snapshot.json").unwrap();
    let content = String::from_utf8_lossy(&policy.content);

    let forbidden = ["/Users/", "/home/", "/tmp/", "\\Users\\"];
    for fragment in &forbidden {
        assert!(
            !content.contains(fragment),
            "policy_snapshot.json contains path fragment: {fragment}"
        );
    }
}
