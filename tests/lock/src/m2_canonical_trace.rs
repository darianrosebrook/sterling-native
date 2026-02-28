//! Single source of truth for the M2 canonical test trace.
//!
//! Used by both the `trace_fixture` binary and the `s1_m2_determinism`
//! integration tests. Any change here changes both, preventing silent
//! drift between what the cross-proc harness produces and what the
//! in-process tests expect.
//!
//! Dimensions: 1 layer, 2 slots, 3 arg slots.
//! Frames: initial state → `SET_SLOT`(0, 0, `Code32::new(1,1,5)`).

use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::bytetrace::{
    ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceFrameV1, ByteTraceHeaderV1, ByteTraceV1,
};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::operators::apply::{apply, set_slot_args, OP_SET_SLOT, SET_SLOT_ARG_COUNT};
use sterling_kernel::operators::operator_registry::kernel_operator_registry;

/// Build the M2 canonical test trace.
///
/// This trace is the oracle anchor for golden hash locks, cross-proc
/// determinism, and v1 parity. Do not change without updating all
/// downstream fixtures.
///
/// # Panics
///
/// Panics if `apply()` fails on the known-good `SET_SLOT` operation
/// (indicates a kernel bug, not a usage error).
#[must_use]
pub fn canonical_test_trace() -> ByteTraceV1 {
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
    let (new_state, _) = apply(&initial, OP_SET_SLOT, &args, &kernel_operator_registry()).unwrap();
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
