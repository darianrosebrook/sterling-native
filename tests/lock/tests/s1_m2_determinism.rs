//! S1-M2 determinism envelope tests.
//!
//! Proves that `ByteTrace` write/read, payload hashing, step chain, and
//! replay verification are deterministic across repeated invocations.
//! Also locks golden fixture values for the canonical test trace.

use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::bytetrace::{
    ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceFrameV1, ByteTraceHeaderV1, ByteTraceV1,
    ReplayVerdict, TraceBundleV1,
};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::trace_reader::bytes_to_trace;
use sterling_kernel::carrier::trace_writer::trace_to_bytes;
use sterling_kernel::operators::apply::{apply, set_slot_args, OP_SET_SLOT, SET_SLOT_ARG_COUNT};
use sterling_kernel::proof::replay::replay_verify;
use sterling_kernel::proof::trace_hash::{payload_hash, step_chain};

/// Canonical test trace used across all M2 golden fixture tests.
///
/// Dimensions: 1 layer, 2 slots, 3 arg slots.
/// Frames: initial state → `SET_SLOT`(0, 0, `Code32::new(1,1,5)`).
fn canonical_test_trace() -> ByteTraceV1 {
    let layer_count = 1;
    let slot_count = 2;
    let arg_slot_count = SET_SLOT_ARG_COUNT;

    let initial = ByteStateV1::new(layer_count, slot_count);
    let frame_0 = ByteTraceFrameV1 {
        op_code: Code32::INITIAL_STATE.to_le_bytes(),
        op_args: vec![0; arg_slot_count * 4],
        result_identity: initial.identity_bytes(),
        result_status: initial.status_bytes(),
    };

    let args = set_slot_args(0, 0, Code32::new(1, 1, 5));
    let (new_state, _) = apply(&initial, OP_SET_SLOT, &args).unwrap();
    let frame_1 = ByteTraceFrameV1 {
        op_code: OP_SET_SLOT.to_le_bytes(),
        op_args: args,
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    let header = ByteTraceHeaderV1 {
        schema_version: "1.0".into(),
        domain_id: "rome".into(),
        registry_epoch_hash: "sha256:aaa".into(),
        codebook_hash: "sha256:bbb".into(),
        fixture_hash: "sha256:ccc".into(),
        step_count: 2,
        layer_count,
        slot_count,
        arg_slot_count,
    };

    ByteTraceV1 {
        envelope: ByteTraceEnvelopeV1 {
            timestamp: "2026-01-01T00:00:00Z".into(),
            trace_id: "golden-test".into(),
            runner_version: "0.0.1".into(),
            wall_time_ms: 42,
        },
        header,
        frames: vec![frame_0, frame_1],
        footer: ByteTraceFooterV1 {
            suite_identity: "sha256:ddd".into(),
            witness_store_digest: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Determinism N=10 tests
// ---------------------------------------------------------------------------

#[test]
fn trace_write_deterministic_n10() {
    let trace = canonical_test_trace();
    let first = trace_to_bytes(&trace).unwrap();
    for _ in 0..10 {
        assert_eq!(trace_to_bytes(&trace).unwrap(), first);
    }
}

#[test]
fn trace_write_read_round_trip_deterministic_n10() {
    let trace = canonical_test_trace();
    let bytes = trace_to_bytes(&trace).unwrap();
    for _ in 0..10 {
        let parsed = bytes_to_trace(&bytes).unwrap();
        let rebytes = trace_to_bytes(&parsed).unwrap();
        assert_eq!(bytes, rebytes);
    }
}

#[test]
fn payload_hash_deterministic_n10() {
    let trace = canonical_test_trace();
    let first = payload_hash(&trace).unwrap();
    for _ in 0..10 {
        assert_eq!(payload_hash(&trace).unwrap(), first);
    }
}

#[test]
fn step_chain_deterministic_n10() {
    let trace = canonical_test_trace();
    let first = step_chain(&trace).unwrap();
    for _ in 0..10 {
        let result = step_chain(&trace).unwrap();
        assert_eq!(result.digest, first.digest);
        assert_eq!(result.chain, first.chain);
    }
}

#[test]
fn replay_deterministic_n10() {
    let trace = canonical_test_trace();
    let bundle = TraceBundleV1 {
        trace,
        compilation_manifest: vec![],
        input_payload: vec![],
    };
    for _ in 0..10 {
        let verdict = replay_verify(&bundle).unwrap();
        assert_eq!(verdict, ReplayVerdict::Match);
    }
}

// ---------------------------------------------------------------------------
// Golden fixture: wire format structure
// ---------------------------------------------------------------------------

#[test]
fn golden_wire_format_magic_offset() {
    let trace = canonical_test_trace();
    let bytes = trace_to_bytes(&trace).unwrap();

    // Envelope length is the first 2 bytes.
    let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
    // Magic starts after envelope.
    let magic_offset = 2 + env_len;
    assert_eq!(&bytes[magic_offset..magic_offset + 4], b"BST1");
}

#[test]
fn golden_wire_format_frame_stride() {
    let trace = canonical_test_trace();
    // stride = 4 + 3*4 + 1*2*4 + 1*2 = 4 + 12 + 8 + 2 = 26
    assert_eq!(trace.header.frame_stride(), Some(26));
    assert_eq!(trace.header.expected_body_len(), Some(52)); // 2 frames * 26
}

// ---------------------------------------------------------------------------
// Golden fixture: hash locks
// ---------------------------------------------------------------------------

#[test]
fn golden_payload_hash() {
    let trace = canonical_test_trace();
    let hash = payload_hash(&trace).unwrap();
    assert_eq!(hash.algorithm(), "sha256");
    // Lock the exact digest. If this changes, something in the pipeline broke.
    assert_eq!(
        hash.hex_digest(),
        golden_payload_hash_digest(),
        "payload hash golden value changed — investigate before updating"
    );
}

#[test]
fn golden_step_chain_digest() {
    let trace = canonical_test_trace();
    let result = step_chain(&trace).unwrap();
    assert_eq!(result.chain.len(), 2);
    assert_eq!(
        result.chain[0].hex_digest(),
        golden_step_chain_0_digest(),
        "step chain[0] golden value changed"
    );
    assert_eq!(
        result.digest.hex_digest(),
        golden_step_chain_final_digest(),
        "step chain final golden value changed"
    );
}

// ---------------------------------------------------------------------------
// Independence tests
// ---------------------------------------------------------------------------

#[test]
fn payload_hash_independent_of_envelope() {
    let trace1 = canonical_test_trace();
    let mut trace2 = canonical_test_trace();
    trace2.envelope.trace_id = "completely-different-id".into();
    trace2.envelope.wall_time_ms = 999_999;
    trace2.envelope.timestamp = "2099-12-31T23:59:59Z".into();

    assert_eq!(
        payload_hash(&trace1).unwrap(),
        payload_hash(&trace2).unwrap()
    );
}

#[test]
fn step_chain_independent_of_envelope_and_footer() {
    let trace1 = canonical_test_trace();
    let mut trace2 = canonical_test_trace();
    trace2.envelope.trace_id = "other-id".into();
    trace2.footer.suite_identity = "sha256:fff".into();

    assert_eq!(
        step_chain(&trace1).unwrap().digest,
        step_chain(&trace2).unwrap().digest
    );
}

// ---------------------------------------------------------------------------
// Golden value computation
// ---------------------------------------------------------------------------
// These functions compute the golden values dynamically on first run.
// Once locked, they serve as regression anchors.

fn golden_payload_hash_digest() -> &'static str {
    "da06d8cc3476cefb662351cea3c1ea21d7ffa7e0a3f11590fa6367501e41a091"
}

fn golden_step_chain_0_digest() -> &'static str {
    "6de1341581dbc47b77f035fd9348c18b9f2af2f6adcba0a9d800908838f3ecfd"
}

fn golden_step_chain_final_digest() -> &'static str {
    "e66368af840ab3f43811caf372b79b0ebb08487c4549f1b5d1b3f9b59a755c5d"
}
