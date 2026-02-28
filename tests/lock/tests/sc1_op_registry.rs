//! SC-001 operator registry lock tests.
//!
//! Proves:
//! 1. No bypass path for `apply()` (source grep)
//! 2. `operator_registry.json` present and normative in bundles
//! 3. `operator_set_digest` in graph metadata matches artifact
//! 4. `operator_set_digest` in tape header matches artifact
//! 5. Clean bundle passes full verification (regression guard)
//! 6. `apply()` fail-closed: unknown operator, declared-not-implemented,
//!    effect contract violation
//! 7. Registry canonical bytes are stable across calls
//! 8. Tamper detection on `operator_registry.json` artifact
//! 9. Tamper detection on `operator_set_digest` in report
//! 10. Contract mismatch detection (registry with/without `SET_SLOT`)

use sterling_harness::bundle::{verify_bundle, BundleVerifyError};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::operators::apply::{apply, set_slot_args, ApplyFailure, OP_SET_SLOT};
use sterling_kernel::operators::operator_registry::{
    kernel_operator_registry, EffectKind, OperatorEntry, OperatorRegistryV1,
};
use sterling_kernel::operators::signature::{IdentityMaskV1, OperatorCategory, StatusMaskV1};
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_search::policy::SearchPolicyV1;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn search_bundle() -> sterling_harness::bundle::ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform)
        .expect("run_search should succeed")
}

// ---------------------------------------------------------------------------
// 1. No apply bypass exported
// ---------------------------------------------------------------------------

/// Source-grep: no `pub fn apply_unchecked`, `pub fn apply_raw`, or any
/// other exported apply-like entrypoint in apply.rs besides `apply()`.
///
/// ACCEPTANCE: SC1-OPREG-NO-BYPASS
#[test]
fn no_apply_bypass_exported() {
    let source = include_str!("../../../kernel/src/operators/apply.rs");

    // Only one `pub fn` that starts with `apply` should exist.
    let apply_pub_fns: Vec<&str> = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("pub fn apply")
                || trimmed.starts_with("pub fn apply_unchecked")
                || trimmed.starts_with("pub fn apply_raw")
                || trimmed.starts_with("pub fn apply_no_registry")
        })
        .collect();

    assert_eq!(
        apply_pub_fns.len(),
        1,
        "expected exactly one pub fn apply*, got: {apply_pub_fns:?}"
    );
    assert!(
        apply_pub_fns[0].trim().starts_with("pub fn apply("),
        "the only exported apply fn must be apply(), got: {}",
        apply_pub_fns[0]
    );

    // Dispatch table function must be private.
    assert!(
        !source.contains("pub fn dispatch_table"),
        "dispatch_table must not be public"
    );
}

// ---------------------------------------------------------------------------
// 2. Registry digest in bundle
// ---------------------------------------------------------------------------

/// `operator_registry.json` present, normative; report `operator_set_digest`
/// matches content hash.
///
/// ACCEPTANCE: SC1-OPREG-DIGEST-IN-BUNDLE
#[test]
fn registry_digest_in_bundle() {
    let bundle = search_bundle();

    let reg_artifact = bundle
        .artifacts
        .get("operator_registry.json")
        .expect("operator_registry.json must be present");
    assert!(reg_artifact.normative, "operator_registry.json must be normative");

    // Report must declare operator_set_digest matching artifact content hash.
    let report_art = bundle
        .artifacts
        .get("verification_report.json")
        .expect("verification_report.json must be present");
    let report: serde_json::Value =
        serde_json::from_slice(&report_art.content).expect("report parse");
    let report_digest = report
        .get("operator_set_digest")
        .and_then(|v| v.as_str())
        .expect("report must have operator_set_digest");

    assert_eq!(
        report_digest,
        reg_artifact.content_hash.as_str(),
        "report operator_set_digest must match artifact content_hash"
    );
}

// ---------------------------------------------------------------------------
// 3. operator_set_digest in graph metadata
// ---------------------------------------------------------------------------

/// Graph metadata `operator_set_digest` matches artifact (raw hex).
///
/// ACCEPTANCE: SC1-OPREG-DIGEST-IN-GRAPH
#[test]
fn operator_set_digest_in_graph_metadata() {
    let bundle = search_bundle();

    let graph_art = bundle.artifacts.get("search_graph.json").unwrap();
    let graph: serde_json::Value =
        serde_json::from_slice(&graph_art.content).expect("graph parse");
    let graph_digest = graph
        .get("metadata")
        .and_then(|m| m.get("operator_set_digest"))
        .and_then(|v| v.as_str())
        .expect("graph metadata must have operator_set_digest");

    let reg_art = bundle.artifacts.get("operator_registry.json").unwrap();
    assert_eq!(
        graph_digest,
        reg_art.content_hash.hex_digest(),
        "graph metadata operator_set_digest must match artifact hex digest"
    );
}

// ---------------------------------------------------------------------------
// 4. operator_set_digest in tape header
// ---------------------------------------------------------------------------

/// Tape header `operator_set_digest` matches artifact (raw hex).
///
/// ACCEPTANCE: SC1-OPREG-DIGEST-IN-TAPE
#[test]
fn operator_set_digest_in_tape_header() {
    let bundle = search_bundle();

    let tape_art = bundle.artifacts.get("search_tape.stap").unwrap();
    let tape =
        sterling_search::tape_reader::read_tape(&tape_art.content).expect("tape parse");
    let tape_digest = tape
        .header
        .json
        .get("operator_set_digest")
        .and_then(|v| v.as_str())
        .expect("tape header must have operator_set_digest");

    let reg_art = bundle.artifacts.get("operator_registry.json").unwrap();
    assert_eq!(
        tape_digest,
        reg_art.content_hash.hex_digest(),
        "tape header operator_set_digest must match artifact hex digest"
    );
}

// ---------------------------------------------------------------------------
// 5. verify_bundle passes clean
// ---------------------------------------------------------------------------

/// Clean `run_search()` bundle passes full verification (regression guard).
///
/// ACCEPTANCE: SC1-OPREG-VERIFY-CLEAN
#[test]
fn verify_bundle_passes_clean() {
    let bundle = search_bundle();
    verify_bundle(&bundle).expect("clean bundle must pass verification");
}

// ---------------------------------------------------------------------------
// 6a. Unknown operator fail-closed
// ---------------------------------------------------------------------------

/// `apply()` with an operator code not in the registry → `UnknownOperator`.
///
/// ACCEPTANCE: SC1-OPREG-UNKNOWN-OP
#[test]
fn unknown_op_fail_closed() {
    let state = ByteStateV1::new(1, 2);
    let registry = kernel_operator_registry();
    let fake_op = Code32::new(9, 9, 9);
    let err = apply(&state, fake_op, &[], &registry).unwrap_err();
    assert!(
        matches!(err, ApplyFailure::UnknownOperator { .. }),
        "expected UnknownOperator, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 6b. Declared but not implemented
// ---------------------------------------------------------------------------

/// Registry with a fake `op_id` (no dispatch entry) → `OperatorNotImplemented`.
///
/// ACCEPTANCE: SC1-OPREG-NOT-IMPLEMENTED
#[test]
fn declared_but_not_implemented() {
    let fake_op = Code32::new(9, 9, 9);
    let entry = OperatorEntry {
        op_id: fake_op,
        name: "FAKE_OP".into(),
        category: OperatorCategory::Seek,
        arg_byte_count: 0,
        effect_kind: EffectKind::WritesOneSlotFromArgs,
        precondition_mask: IdentityMaskV1::new(0, 0),
        effect_mask: IdentityMaskV1::new(0, 0),
        status_effect_mask: StatusMaskV1::new(0, 0),
        cost_model: "unit".into(),
        contract_epoch: "v1".into(),
    };
    let registry = OperatorRegistryV1::new("test.v1".into(), vec![entry]).unwrap();
    let state = ByteStateV1::new(1, 2);

    let err = apply(&state, fake_op, &[], &registry).unwrap_err();
    assert!(
        matches!(err, ApplyFailure::OperatorNotImplemented { .. }),
        "expected OperatorNotImplemented, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 6c. Effect contract violation
// ---------------------------------------------------------------------------

/// If `SET_SLOT` is applied to a slot that already has the same value
/// (so no identity plane diff), post-check catches it.
///
/// ACCEPTANCE: SC1-OPREG-EFFECT-VIOLATION
#[test]
fn effect_contract_violation_detected() {
    let registry = kernel_operator_registry();
    let mut state = ByteStateV1::new(1, 2);
    let value = Code32::new(1, 1, 5);

    // Set slot (0,0) to value first.
    let args = set_slot_args(0, 0, value);
    let (new_state, _) = apply(&state, OP_SET_SLOT, &args, &registry).unwrap();
    state = new_state;

    // Now apply SET_SLOT with the same value to the same slot.
    // The identity plane will have 0 diffs → EffectContractViolation.
    let err = apply(&state, OP_SET_SLOT, &args, &registry).unwrap_err();
    assert!(
        matches!(err, ApplyFailure::EffectContractViolation { .. }),
        "expected EffectContractViolation when writing same value, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. Registry stable ordering
// ---------------------------------------------------------------------------

/// Two `kernel_operator_registry()` calls → byte-identical canonical JSON.
///
/// ACCEPTANCE: SC1-OPREG-STABLE-ORDERING
#[test]
fn registry_stable_ordering() {
    let r1 = kernel_operator_registry();
    let r2 = kernel_operator_registry();
    let b1 = r1.canonical_bytes().unwrap();
    let b2 = r2.canonical_bytes().unwrap();
    assert_eq!(b1, b2, "canonical bytes must be deterministic");
}

// ---------------------------------------------------------------------------
// 8. Tamper operator_registry artifact caught
// ---------------------------------------------------------------------------

/// Modified `operator_registry.json` artifact → verification failure.
///
/// ACCEPTANCE: SC1-OPREG-TAMPER-ARTIFACT
#[test]
fn tamper_operator_registry_artifact_caught() {
    use sterling_harness::bundle::{build_bundle, ArtifactInput};

    let bundle = search_bundle();

    // Rebuild bundle with tampered operator_registry.json.
    let mut artifacts: Vec<ArtifactInput> = Vec::new();
    for (name, art) in &bundle.artifacts {
        if name == "operator_registry.json" {
            // Add an extra byte to the content → content_hash mismatch.
            let mut tampered = art.content.clone();
            tampered.push(b' ');
            artifacts.push(ArtifactInput {
                name: name.clone(),
                content: tampered,
                normative: art.normative,
                precomputed_hash: None, // force recomputation
            });
        } else {
            artifacts.push(ArtifactInput {
                name: name.clone(),
                content: art.content.clone(),
                normative: art.normative,
                precomputed_hash: Some(art.content_hash.clone()),
            });
        }
    }

    let tampered_bundle = build_bundle(artifacts).expect("build_bundle should succeed");

    // Verification must fail — content changed but report still has old digest.
    let err = verify_bundle(&tampered_bundle).unwrap_err();

    // Tamper detection may fire at different pipeline stages:
    // - ArtifactNotCanonical (Step 7): appending bytes breaks canonical form
    // - OperatorRegistryDigestMismatch (Step 16): content hash doesn't match report
    // - MetadataBindingOperatorRegistryMismatch (Step 17): graph metadata mismatch
    // Any of these proves the tamper was caught fail-closed.
    let is_detected = matches!(
        &err,
        BundleVerifyError::OperatorRegistryDigestMismatch { .. }
            | BundleVerifyError::MetadataBindingOperatorRegistryMismatch { .. }
            | BundleVerifyError::ArtifactNotCanonical { .. }
    );
    assert!(
        is_detected,
        "tampered operator_registry.json must be detected, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 9. Tamper operator_set_digest in report caught
// ---------------------------------------------------------------------------

/// Corrupted report `operator_set_digest` field → verification failure.
///
/// ACCEPTANCE: SC1-OPREG-TAMPER-REPORT
#[test]
fn tamper_operator_set_digest_in_report_caught() {
    use sterling_harness::bundle::{build_bundle, ArtifactInput};

    let bundle = search_bundle();

    // Rebuild bundle with modified report (operator_set_digest → bogus value).
    let report_art = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report: serde_json::Value =
        serde_json::from_slice(&report_art.content).expect("parse");
    report["operator_set_digest"] = serde_json::json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    let tampered_report_bytes = canonical_json_bytes(&report).expect("canon");

    let mut artifacts: Vec<ArtifactInput> = Vec::new();
    for (name, art) in &bundle.artifacts {
        if name == "verification_report.json" {
            artifacts.push(ArtifactInput {
                name: name.clone(),
                content: tampered_report_bytes.clone(),
                normative: art.normative,
                precomputed_hash: None,
            });
        } else {
            artifacts.push(ArtifactInput {
                name: name.clone(),
                content: art.content.clone(),
                normative: art.normative,
                precomputed_hash: Some(art.content_hash.clone()),
            });
        }
    }

    let tampered_bundle = build_bundle(artifacts).expect("build_bundle");
    let err = verify_bundle(&tampered_bundle).unwrap_err();

    assert!(
        matches!(err, BundleVerifyError::OperatorRegistryDigestMismatch { .. }),
        "tampered report operator_set_digest must be detected, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 10. Contract mismatch detection
// ---------------------------------------------------------------------------

/// Registry with `SET_SLOT` → `apply()` Ok; empty registry → `UnknownOperator`.
///
/// ACCEPTANCE: SC1-OPREG-CONTRACT-MISMATCH
#[test]
fn contract_mismatch_detection() {
    let state = ByteStateV1::new(1, 2);
    let args = set_slot_args(0, 0, Code32::new(1, 1, 5));

    // Full registry: apply succeeds.
    let full_reg = kernel_operator_registry();
    assert!(apply(&state, OP_SET_SLOT, &args, &full_reg).is_ok());

    // Empty registry: apply fails with UnknownOperator.
    let empty_reg = OperatorRegistryV1::new("empty.v1".into(), vec![]).unwrap();
    let err = apply(&state, OP_SET_SLOT, &args, &empty_reg).unwrap_err();
    assert!(
        matches!(err, ApplyFailure::UnknownOperator { .. }),
        "empty registry must reject SET_SLOT, got {err:?}"
    );
}
