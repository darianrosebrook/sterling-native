//! `replay_verify()`: verify a trace bundle by deterministic replay.
//!
//! Parses the trace, extracts initial state from frame 0, applies each
//! operator in sequence, and compares frame-by-frame. Returns
//! [`ReplayVerdict::Match`] if all frames agree, or
//! [`ReplayVerdict::Divergence`] at the first mismatch.

use crate::carrier::bytestate::ByteStateV1;
use crate::carrier::bytetrace::{ReplayVerdict, TraceBundleV1};
use crate::carrier::code32::Code32;
use crate::operators::apply::{apply, ApplyFailure};

/// Error during replay (distinct from a divergence verdict).
///
/// A `ReplayError` means the trace could not be replayed at all.
/// A `ReplayVerdict::Divergence` means it was replayed but did not match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// The trace bundle is structurally malformed.
    MalformedBundle { detail: String },
    /// Schema or registry referenced by the trace is unavailable.
    MissingDependency { detail: String },
    /// Operator application failed during replay.
    OperatorFailed { frame_index: usize, detail: String },
}

/// Result type for replay verification.
pub type ReplayResult = Result<ReplayVerdict, ReplayError>;

/// Verify a trace bundle by deterministic replay.
///
/// Re-executes the trace from the initial state, comparing each frame's
/// result against the recorded result. If all frames match, returns
/// [`ReplayVerdict::Match`]. Otherwise returns [`ReplayVerdict::Divergence`]
/// pointing to the first differing frame.
///
/// # Errors
///
/// Returns [`ReplayError`] if the bundle is malformed or an operator
/// fails during replay.
pub fn replay_verify(trace_bundle: &TraceBundleV1) -> ReplayResult {
    let trace = &trace_bundle.trace;

    if trace.frames.is_empty() {
        return Err(ReplayError::MalformedBundle {
            detail: "trace has no frames".into(),
        });
    }

    let header = &trace.header;

    // Extract initial state from frame 0.
    let frame_0 = &trace.frames[0];

    // Frame 0 must use INITIAL_STATE sentinel.
    let op_code_0 = Code32::from_le_bytes(frame_0.op_code);
    if op_code_0 != Code32::INITIAL_STATE {
        return Err(ReplayError::MalformedBundle {
            detail: format!(
                "frame 0 op_code is not INITIAL_STATE: got {:?}",
                frame_0.op_code
            ),
        });
    }

    // Reconstruct initial state from frame 0's result bytes.
    let mut evidence =
        Vec::with_capacity(frame_0.result_identity.len() + frame_0.result_status.len());
    evidence.extend_from_slice(&frame_0.result_identity);
    evidence.extend_from_slice(&frame_0.result_status);

    let mut current_state =
        ByteStateV1::from_evidence_bytes(header.layer_count, header.slot_count, &evidence)
            .ok_or_else(|| ReplayError::MalformedBundle {
                detail: "frame 0 result bytes do not form a valid ByteState".into(),
            })?;

    // Replay frames 1..n.
    for (i, frame) in trace.frames.iter().enumerate().skip(1) {
        let op_code = Code32::from_le_bytes(frame.op_code);

        let (new_state, _record) =
            apply(&current_state, op_code, &frame.op_args).map_err(|e| match e {
                ApplyFailure::UnknownOperator { op_code: oc } => ReplayError::OperatorFailed {
                    frame_index: i,
                    detail: format!("unknown operator: {oc:?}"),
                },
                ApplyFailure::PreconditionNotMet { detail } => ReplayError::OperatorFailed {
                    frame_index: i,
                    detail: format!("precondition not met: {detail}"),
                },
                ApplyFailure::ArgumentMismatch { detail } => ReplayError::OperatorFailed {
                    frame_index: i,
                    detail: format!("argument mismatch: {detail}"),
                },
            })?;

        // Compare identity and status planes.
        let expected_identity = &frame.result_identity;
        let expected_status = &frame.result_status;
        let actual_identity = new_state.identity_bytes();
        let actual_status = new_state.status_bytes();

        if actual_identity != *expected_identity || actual_status != *expected_status {
            return Ok(ReplayVerdict::Divergence {
                frame_index: i,
                expected_identity_hex: hex::encode(expected_identity),
                actual_identity_hex: hex::encode(&actual_identity),
                expected_status_hex: hex::encode(expected_status),
                actual_status_hex: hex::encode(&actual_status),
                detail: format!("frame {i} result mismatch"),
            });
        }

        current_state = new_state;
    }

    Ok(ReplayVerdict::Match)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carrier::bytetrace::{
        ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceFrameV1, ByteTraceHeaderV1, ByteTraceV1,
    };
    use crate::operators::apply::{set_slot_args, OP_SET_SLOT};

    fn test_envelope() -> ByteTraceEnvelopeV1 {
        ByteTraceEnvelopeV1 {
            timestamp: "2026-01-01T00:00:00Z".into(),
            trace_id: "replay-test".into(),
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

    /// Build a valid single-step trace: initial state → `SET_SLOT`.
    fn build_single_step_trace() -> TraceBundleV1 {
        let layer_count = 1;
        let slot_count = 2;
        let arg_slot_count = 3; // SET_SLOT uses 3 arg slots

        // Initial state: all PADDING/Hole.
        let initial = ByteStateV1::new(layer_count, slot_count);
        let frame_0 = ByteTraceFrameV1 {
            op_code: Code32::INITIAL_STATE.to_le_bytes(),
            op_args: vec![0; arg_slot_count * 4],
            result_identity: initial.identity_bytes(),
            result_status: initial.status_bytes(),
        };

        // Apply SET_SLOT(0, 0, Code32::new(1,1,5)).
        let args = set_slot_args(0, 0, Code32::new(1, 1, 5));
        let (new_state, _record) = apply(&initial, OP_SET_SLOT, &args).unwrap();
        let frame_1 = ByteTraceFrameV1 {
            op_code: OP_SET_SLOT.to_le_bytes(),
            op_args: args,
            result_identity: new_state.identity_bytes(),
            result_status: new_state.status_bytes(),
        };

        let header = ByteTraceHeaderV1 {
            schema_version: "1.0".into(),
            domain_id: "test".into(),
            registry_epoch_hash: "sha256:000".into(),
            codebook_hash: "sha256:000".into(),
            fixture_hash: "sha256:000".into(),
            step_count: 2,
            layer_count,
            slot_count,
            arg_slot_count,
        };

        TraceBundleV1 {
            trace: ByteTraceV1 {
                envelope: test_envelope(),
                header,
                frames: vec![frame_0, frame_1],
                footer: test_footer(),
            },
            compilation_manifest: vec![],
            input_payload: vec![],
        }
    }

    // ACCEPTANCE: S1-M2-REPLAY-1STEP
    #[test]
    fn replay_match_single_step() {
        let bundle = build_single_step_trace();
        let verdict = replay_verify(&bundle).unwrap();
        assert_eq!(verdict, ReplayVerdict::Match);
    }

    #[test]
    fn replay_divergence_on_corrupted_frame() {
        let mut bundle = build_single_step_trace();
        // Corrupt frame 1's identity bytes.
        bundle.trace.frames[1].result_identity[0] = 0xFF;
        let verdict = replay_verify(&bundle).unwrap();
        assert!(matches!(
            verdict,
            ReplayVerdict::Divergence { frame_index: 1, .. }
        ));
    }

    #[test]
    fn replay_rejects_empty_trace() {
        let bundle = TraceBundleV1 {
            trace: ByteTraceV1 {
                envelope: test_envelope(),
                header: ByteTraceHeaderV1 {
                    schema_version: "1.0".into(),
                    domain_id: "test".into(),
                    registry_epoch_hash: "sha256:000".into(),
                    codebook_hash: "sha256:000".into(),
                    fixture_hash: "sha256:000".into(),
                    step_count: 0,
                    layer_count: 1,
                    slot_count: 1,
                    arg_slot_count: 0,
                },
                frames: vec![],
                footer: test_footer(),
            },
            compilation_manifest: vec![],
            input_payload: vec![],
        };
        let err = replay_verify(&bundle).unwrap_err();
        assert!(matches!(err, ReplayError::MalformedBundle { .. }));
    }

    #[test]
    fn replay_rejects_bad_frame_0_sentinel() {
        let mut bundle = build_single_step_trace();
        // Change frame 0 op_code to something that isn't INITIAL_STATE.
        bundle.trace.frames[0].op_code = Code32::new(1, 1, 1).to_le_bytes();
        let err = replay_verify(&bundle).unwrap_err();
        assert!(matches!(err, ReplayError::MalformedBundle { .. }));
    }

    #[test]
    fn replay_reports_unknown_operator() {
        let mut bundle = build_single_step_trace();
        // Change frame 1's op_code to unknown operator.
        bundle.trace.frames[1].op_code = Code32::new(9, 9, 9).to_le_bytes();
        let err = replay_verify(&bundle).unwrap_err();
        assert!(matches!(
            err,
            ReplayError::OperatorFailed { frame_index: 1, .. }
        ));
    }

    #[test]
    fn replay_deterministic_n10() {
        let bundle = build_single_step_trace();
        let first = replay_verify(&bundle).unwrap();
        for _ in 0..10 {
            assert_eq!(replay_verify(&bundle).unwrap(), first);
        }
    }

    #[test]
    fn replay_divergence_on_status_corruption() {
        let mut bundle = build_single_step_trace();
        // Corrupt frame 1's status bytes — change Provisional to Certified.
        bundle.trace.frames[1].result_status[0] = 255; // Certified instead of Provisional
        let verdict = replay_verify(&bundle).unwrap();
        assert!(matches!(
            verdict,
            ReplayVerdict::Divergence { frame_index: 1, .. }
        ));
    }
}
