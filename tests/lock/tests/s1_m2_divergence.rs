//! S1-M2 divergence localization tests.
//!
//! Builds multi-step traces, mutates specific frames, and verifies that
//! `replay_verify()` pinpoints the exact frame where divergence occurred.
//! Also tests the write→read→replay round-trip pipeline.

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

fn test_envelope() -> ByteTraceEnvelopeV1 {
    ByteTraceEnvelopeV1 {
        timestamp: "2026-01-01T00:00:00Z".into(),
        trace_id: "divergence-test".into(),
        runner_version: "0.0.1".into(),
        wall_time_ms: 0,
    }
}

fn test_footer() -> ByteTraceFooterV1 {
    ByteTraceFooterV1 {
        suite_identity: "sha256:000".into(),
        witness_store_digest: None,
    }
}

/// Build a 4-step trace: initial + 3 `SET_SLOT` operations.
fn build_four_step_trace() -> TraceBundleV1 {
    let layer_count = 1;
    let slot_count = 4;
    let arg_slot_count = SET_SLOT_ARG_COUNT;

    let initial = ByteStateV1::new(layer_count, slot_count);
    let frame_0 = ByteTraceFrameV1 {
        op_code: Code32::INITIAL_STATE.to_le_bytes(),
        op_args: vec![0; arg_slot_count * 4],
        result_identity: initial.identity_bytes(),
        result_status: initial.status_bytes(),
    };

    let mut frames = vec![frame_0];
    let mut state = initial;

    // Step 1: set slot 0 to Code32::new(1,1,1)
    let args1 = set_slot_args(0, 0, Code32::new(1, 1, 1));
    let (s1, _) = apply(&state, OP_SET_SLOT, &args1).unwrap();
    frames.push(ByteTraceFrameV1 {
        op_code: OP_SET_SLOT.to_le_bytes(),
        op_args: args1,
        result_identity: s1.identity_bytes(),
        result_status: s1.status_bytes(),
    });
    state = s1;

    // Step 2: set slot 1 to Code32::new(1,1,2)
    let args2 = set_slot_args(0, 1, Code32::new(1, 1, 2));
    let (s2, _) = apply(&state, OP_SET_SLOT, &args2).unwrap();
    frames.push(ByteTraceFrameV1 {
        op_code: OP_SET_SLOT.to_le_bytes(),
        op_args: args2,
        result_identity: s2.identity_bytes(),
        result_status: s2.status_bytes(),
    });
    state = s2;

    // Step 3: set slot 2 to Code32::new(1,1,3)
    let args3 = set_slot_args(0, 2, Code32::new(1, 1, 3));
    let (s3, _) = apply(&state, OP_SET_SLOT, &args3).unwrap();
    frames.push(ByteTraceFrameV1 {
        op_code: OP_SET_SLOT.to_le_bytes(),
        op_args: args3,
        result_identity: s3.identity_bytes(),
        result_status: s3.status_bytes(),
    });

    let header = ByteTraceHeaderV1 {
        schema_version: "1.0".into(),
        domain_id: "test".into(),
        registry_epoch_hash: "sha256:000".into(),
        codebook_hash: "sha256:000".into(),
        fixture_hash: "sha256:000".into(),
        step_count: 4,
        layer_count,
        slot_count,
        arg_slot_count,
    };

    TraceBundleV1 {
        trace: ByteTraceV1 {
            envelope: test_envelope(),
            header,
            frames,
            footer: test_footer(),
        },
        compilation_manifest: vec![],
        input_payload: vec![],
    }
}

#[test]
fn four_step_trace_replays_clean() {
    let bundle = build_four_step_trace();
    let verdict = replay_verify(&bundle).unwrap();
    assert_eq!(verdict, ReplayVerdict::Match);
}

#[test]
fn mutation_at_frame_1_localized() {
    let mut bundle = build_four_step_trace();
    // Corrupt frame 1's identity: flip first byte.
    bundle.trace.frames[1].result_identity[0] ^= 0xFF;
    let verdict = replay_verify(&bundle).unwrap();
    match verdict {
        ReplayVerdict::Divergence { frame_index, .. } => assert_eq!(frame_index, 1),
        _ => panic!("expected divergence at frame 1"),
    }
}

#[test]
fn mutation_at_frame_2_localized() {
    let mut bundle = build_four_step_trace();
    // Corrupt frame 2's identity.
    bundle.trace.frames[2].result_identity[0] ^= 0xFF;
    let verdict = replay_verify(&bundle).unwrap();
    match verdict {
        ReplayVerdict::Divergence { frame_index, .. } => assert_eq!(frame_index, 2),
        _ => panic!("expected divergence at frame 2"),
    }
}

#[test]
fn mutation_at_frame_3_localized() {
    let mut bundle = build_four_step_trace();
    // Corrupt frame 3's status byte.
    bundle.trace.frames[3].result_status[0] ^= 0xFF;
    let verdict = replay_verify(&bundle).unwrap();
    match verdict {
        ReplayVerdict::Divergence { frame_index, .. } => assert_eq!(frame_index, 3),
        _ => panic!("expected divergence at frame 3"),
    }
}

#[test]
fn write_read_replay_round_trip() {
    let bundle = build_four_step_trace();
    // Serialize → deserialize → replay.
    let bytes = trace_to_bytes(&bundle.trace).unwrap();
    let parsed = bytes_to_trace(&bytes).unwrap();
    let round_trip_bundle = TraceBundleV1 {
        trace: parsed,
        compilation_manifest: vec![],
        input_payload: vec![],
    };
    let verdict = replay_verify(&round_trip_bundle).unwrap();
    assert_eq!(verdict, ReplayVerdict::Match);
}

#[test]
fn payload_hash_changes_on_frame_mutation() {
    let bundle = build_four_step_trace();
    let hash_clean = payload_hash(&bundle.trace).unwrap();

    let mut mutated = bundle.trace.clone();
    mutated.frames[2].result_identity[0] ^= 0xFF;
    let hash_mutated = payload_hash(&mutated).unwrap();

    assert_ne!(hash_clean, hash_mutated);
}

#[test]
fn step_chain_diverges_at_mutated_frame() {
    let bundle = build_four_step_trace();
    let chain_clean = step_chain(&bundle.trace).unwrap();

    let mut mutated = bundle.trace.clone();
    mutated.frames[2].result_identity[0] ^= 0xFF;
    let chain_mutated = step_chain(&mutated).unwrap();

    // Frames 0 and 1 should still match.
    assert_eq!(chain_clean.chain[0], chain_mutated.chain[0]);
    assert_eq!(chain_clean.chain[1], chain_mutated.chain[1]);
    // Frame 2 onward should diverge.
    assert_ne!(chain_clean.chain[2], chain_mutated.chain[2]);
    assert_ne!(chain_clean.chain[3], chain_mutated.chain[3]);
}

// ACCEPTANCE: S1-M2-DIV-LOCALIZE
#[test]
fn step_chain_localizes_mutation_to_exact_frame() {
    let bundle = build_four_step_trace();
    let chain_clean = step_chain(&bundle.trace).unwrap();

    // Mutate frame 3 only.
    let mut mutated = bundle.trace.clone();
    mutated.frames[3].result_status[0] ^= 0xFF;
    let chain_mutated = step_chain(&mutated).unwrap();

    // Frames 0, 1, 2 should still match.
    assert_eq!(chain_clean.chain[0], chain_mutated.chain[0]);
    assert_eq!(chain_clean.chain[1], chain_mutated.chain[1]);
    assert_eq!(chain_clean.chain[2], chain_mutated.chain[2]);
    // Frame 3 should diverge.
    assert_ne!(chain_clean.chain[3], chain_mutated.chain[3]);
}
