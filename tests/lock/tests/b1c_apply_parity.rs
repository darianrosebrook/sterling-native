//! B1c apply parity lock tests.
//!
//! Proves:
//! 1. `apply()` from compiled state produces post-state matching golden bytes
//! 2. `apply()` step record fields match golden bytes
//! 3. Post-state identity/evidence digests match golden hashes
//! 4. Pre-state digests match B1b golden hashes (cross-fixture consistency)
//! 5. Fixture inputs are self-consistent (`op_code` matches `step_record_op_code`, etc.)
//! 6. Expected hashes has exactly the expected keys (fail-closed)

use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::operators::apply::{apply, OP_SET_SLOT};
use sterling_kernel::operators::operator_registry::kernel_operator_registry;
use sterling_kernel::proof::hash::{canonical_hash, HashDomain};

/// Path to the committed golden apply fixture directory.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/b1c_apply_rome_mini_search"
);

/// Path to the B1b compile fixture (for cross-fixture consistency check).
const B1B_FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/b1b_compile_rome_mini_search"
);

/// Load a fixture file as bytes.
fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = format!("{FIXTURE_DIR}/{name}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// Load a B1b fixture file as bytes.
fn b1b_fixture_bytes(name: &str) -> Vec<u8> {
    let path = format!("{B1B_FIXTURE_DIR}/{name}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// Parse the expected hashes JSON fixture.
fn expected_hashes() -> serde_json::Value {
    let bytes = fixture_bytes("expected_hashes.json");
    serde_json::from_slice(&bytes).expect("invalid expected_hashes.json")
}

/// Compile the `RomeMiniSearch` initial state and apply the golden `SET_SLOT`.
fn compile_and_apply() -> (
    sterling_kernel::carrier::bytestate::ByteStateV1,
    sterling_kernel::operators::apply::StepRecord,
) {
    let world = RomeMiniSearch;
    let payload_bytes = world.encode_payload().expect("encode_payload failed");
    let schema = world.schema_descriptor();
    let concept_registry = world.registry().expect("registry failed");
    let compiled = compile(&payload_bytes, &schema, &concept_registry).expect("compile failed");

    let op_code_bytes = fixture_bytes("op_code.bin");
    let op_code = Code32::from_le_bytes([
        op_code_bytes[0],
        op_code_bytes[1],
        op_code_bytes[2],
        op_code_bytes[3],
    ]);
    let op_args = fixture_bytes("op_args.bin");
    let operator_registry = kernel_operator_registry();

    let (post_state, step_record) =
        apply(&compiled.state, op_code, &op_args, &operator_registry).expect("apply failed");
    (post_state, step_record)
}

// ---------------------------------------------------------------------------
// 1. Apply produces post-state matching golden bytes
// ---------------------------------------------------------------------------

#[test]
fn apply_post_state_matches_golden_bytes() {
    let (post_state, _) = compile_and_apply();

    let expected_identity = fixture_bytes("post_state_identity.bin");
    let expected_status = fixture_bytes("post_state_status.bin");

    assert_eq!(
        post_state.identity_bytes(),
        expected_identity,
        "post-state identity bytes differ from golden"
    );
    assert_eq!(
        post_state.status_bytes(),
        expected_status,
        "post-state status bytes differ from golden"
    );
}

// ---------------------------------------------------------------------------
// 2. Step record fields match golden bytes
// ---------------------------------------------------------------------------

#[test]
fn apply_step_record_matches_golden_bytes() {
    let (_, step_record) = compile_and_apply();

    assert_eq!(
        &step_record.op_code,
        fixture_bytes("step_record_op_code.bin").as_slice(),
        "step_record.op_code differs from golden"
    );
    assert_eq!(
        step_record.op_args,
        fixture_bytes("step_record_op_args.bin"),
        "step_record.op_args differs from golden"
    );
    assert_eq!(
        step_record.result_identity,
        fixture_bytes("step_record_result_identity.bin"),
        "step_record.result_identity differs from golden"
    );
    assert_eq!(
        step_record.result_status,
        fixture_bytes("step_record_result_status.bin"),
        "step_record.result_status differs from golden"
    );
}

// ---------------------------------------------------------------------------
// 3. Post-state digests match golden hashes
// ---------------------------------------------------------------------------

#[test]
fn apply_post_state_digests_match_golden_hashes() {
    let (post_state, _) = compile_and_apply();
    let hashes = expected_hashes();

    let post_identity_digest =
        canonical_hash(HashDomain::IdentityPlane, &post_state.identity_bytes());
    let post_evidence_digest =
        canonical_hash(HashDomain::EvidencePlane, &post_state.evidence_bytes());

    assert_eq!(
        post_identity_digest.as_str(),
        hashes["post_identity_digest"].as_str().unwrap(),
        "post_identity_digest mismatch"
    );
    assert_eq!(
        post_evidence_digest.as_str(),
        hashes["post_evidence_digest"].as_str().unwrap(),
        "post_evidence_digest mismatch"
    );
}

// ---------------------------------------------------------------------------
// 4. Pre-state digests match B1b golden hashes (cross-fixture consistency)
// ---------------------------------------------------------------------------

#[test]
fn pre_state_digests_match_b1b_golden() {
    let hashes = expected_hashes();
    let b1b_hashes: serde_json::Value = {
        let bytes = b1b_fixture_bytes("expected_hashes.json");
        serde_json::from_slice(&bytes).expect("invalid B1b expected_hashes.json")
    };

    assert_eq!(
        hashes["pre_identity_digest"].as_str().unwrap(),
        b1b_hashes["identity_digest"].as_str().unwrap(),
        "B1c pre_identity_digest must match B1b identity_digest"
    );
    assert_eq!(
        hashes["pre_evidence_digest"].as_str().unwrap(),
        b1b_hashes["evidence_digest"].as_str().unwrap(),
        "B1c pre_evidence_digest must match B1b evidence_digest"
    );
}

// ---------------------------------------------------------------------------
// 5. Fixture inputs are self-consistent
// ---------------------------------------------------------------------------

#[test]
fn fixture_inputs_self_consistent() {
    // op_code.bin must be OP_SET_SLOT bytes.
    let op_code_bytes = fixture_bytes("op_code.bin");
    assert_eq!(
        op_code_bytes,
        OP_SET_SLOT.to_le_bytes(),
        "op_code.bin must be OP_SET_SLOT"
    );

    // op_args.bin must be 12 bytes (3 arg slots × 4 bytes).
    let op_args = fixture_bytes("op_args.bin");
    assert_eq!(op_args.len(), 12, "op_args.bin must be 12 bytes");

    // step_record_op_code must equal op_code.
    assert_eq!(
        fixture_bytes("step_record_op_code.bin"),
        op_code_bytes,
        "step_record_op_code.bin must equal op_code.bin"
    );

    // step_record_op_args must equal op_args.
    assert_eq!(
        fixture_bytes("step_record_op_args.bin"),
        op_args,
        "step_record_op_args.bin must equal op_args.bin"
    );

    // step_record_result_identity must equal post_state_identity.
    assert_eq!(
        fixture_bytes("step_record_result_identity.bin"),
        fixture_bytes("post_state_identity.bin"),
        "step_record_result_identity.bin must equal post_state_identity.bin"
    );

    // step_record_result_status must equal post_state_status.
    assert_eq!(
        fixture_bytes("step_record_result_status.bin"),
        fixture_bytes("post_state_status.bin"),
        "step_record_result_status.bin must equal post_state_status.bin"
    );
}

// ---------------------------------------------------------------------------
// 6. Expected hashes has exactly the expected keys (fail-closed)
// ---------------------------------------------------------------------------

#[test]
fn expected_hashes_has_exact_keys() {
    let hashes = expected_hashes();
    let expected_keys = [
        "post_evidence_digest",
        "post_identity_digest",
        "pre_evidence_digest",
        "pre_identity_digest",
    ];

    let obj = hashes
        .as_object()
        .expect("expected_hashes must be a JSON object");

    for key in obj.keys() {
        assert!(
            expected_keys.contains(&key.as_str()),
            "unexpected key in expected_hashes: {key}"
        );
    }
    for key in &expected_keys {
        assert!(
            obj.contains_key(*key),
            "missing expected key in expected_hashes: {key}"
        );
    }
}
