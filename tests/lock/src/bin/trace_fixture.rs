//! Tiny binary that builds the M2 canonical test trace, serializes it,
//! and prints deterministic output lines for cross-process verification.
//!
//! Used by S1-M2-DETERMINISM-CROSSPROC to verify that trace write, read,
//! payload hash, and step chain are identical across different process
//! environments (cwd, locale, env).
//!
//! Usage: `trace_fixture`
//! Output: five lines, each `key=value`:
//!   `bst1_hex`=...
//!   `payload_hash`=sha256:...
//!   `step_chain_0`=sha256:...
//!   `step_chain_final`=sha256:...
//!   `replay_verdict`=Match

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

/// Build the canonical test trace (identical to `s1_m2_determinism.rs`).
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

fn main() {
    let trace = canonical_test_trace();

    // Serialize
    let bst1_bytes = trace_to_bytes(&trace).unwrap();

    // Round-trip through reader
    let parsed = bytes_to_trace(&bst1_bytes).unwrap();
    let rebytes = trace_to_bytes(&parsed).unwrap();
    assert_eq!(
        bst1_bytes, rebytes,
        "round-trip produced different bytes — non-determinism detected"
    );

    // Payload hash
    let ph = payload_hash(&trace).unwrap();

    // Step chain
    let sc = step_chain(&trace).unwrap();

    // Replay
    let bundle = TraceBundleV1 {
        trace,
        compilation_manifest: vec![],
        input_payload: vec![],
    };
    let verdict = replay_verify(&bundle).unwrap();
    let verdict_str = match verdict {
        ReplayVerdict::Match => "Match",
        _ => "FAIL",
    };

    println!("bst1_hex={}", hex::encode(&bst1_bytes));
    println!("payload_hash={}", ph.as_str());
    println!("step_chain_0={}", sc.chain[0].as_str());
    println!("step_chain_final={}", sc.digest.as_str());
    println!("replay_verdict={verdict_str}");
}
