//! `apply()`: apply an operator to `ByteState`, producing new state + step record.
//!
//! # M2 scope
//!
//! Minimal operator dispatch: one toy operator (`SET_SLOT`) for replay
//! verification testing. Full operator catalog is M3+.

use crate::carrier::bytestate::{ByteStateV1, SlotStatus};
use crate::carrier::code32::Code32;

/// Well-known operator code for M2: write a value to a slot.
///
/// `SET_SLOT` takes 3 arg slots (12 bytes):
/// - arg 0: layer index (u32 LE)
/// - arg 1: slot index (u32 LE)
/// - arg 2: Code32 value to write (4 bytes LE)
///
/// Effect: sets `identity[layer][slot] = value` and promotes status to
/// `SlotStatus::Provisional`.
pub const OP_SET_SLOT: Code32 = Code32::new(1, 1, 1);

/// Number of arg slots for `SET_SLOT`.
pub const SET_SLOT_ARG_COUNT: usize = 3;

/// A step record produced by applying an operator.
///
/// Records what happened for inclusion in `ByteTrace`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    /// The operator code that was applied (4 bytes, `Code32` as LE bytes).
    pub op_code: [u8; 4],
    /// Serialized operator arguments.
    pub op_args: Vec<u8>,
    /// Identity plane bytes after applying the operator.
    pub result_identity: Vec<u8>,
    /// Status plane bytes after applying the operator.
    pub result_status: Vec<u8>,
}

/// Typed failure for operator application. Fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyFailure {
    /// Operator preconditions not met (INV-CORE-06).
    PreconditionNotMet { detail: String },
    /// Operator arguments do not match the expected format.
    ArgumentMismatch { detail: String },
    /// The operator code is not in the registry.
    UnknownOperator { op_code: [u8; 4] },
}

/// Result type for apply.
pub type ApplyResult = Result<(ByteStateV1, StepRecord), ApplyFailure>;

/// Apply an operator to the current `ByteState`.
///
/// Dispatches on `op_code` to the appropriate operator implementation.
/// For M2, only `OP_SET_SLOT` is supported.
///
/// # Errors
///
/// Returns [`ApplyFailure`] on precondition/mismatch or unknown operator.
pub fn apply(state: &ByteStateV1, op_code: Code32, op_args: &[u8]) -> ApplyResult {
    if op_code == OP_SET_SLOT {
        apply_set_slot(state, op_args)
    } else {
        Err(ApplyFailure::UnknownOperator {
            op_code: op_code.to_le_bytes(),
        })
    }
}

/// `SET_SLOT` operator: write a Code32 value to (layer, slot) and promote.
fn apply_set_slot(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
    if op_args.len() != SET_SLOT_ARG_COUNT * 4 {
        return Err(ApplyFailure::ArgumentMismatch {
            detail: format!(
                "SET_SLOT expects {} bytes, got {}",
                SET_SLOT_ARG_COUNT * 4,
                op_args.len()
            ),
        });
    }

    let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;
    let slot = u32::from_le_bytes([op_args[4], op_args[5], op_args[6], op_args[7]]) as usize;
    let value = Code32::from_le_bytes([op_args[8], op_args[9], op_args[10], op_args[11]]);

    if layer >= state.layer_count() || slot >= state.slot_count() {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "SET_SLOT target ({layer}, {slot}) out of bounds for {}x{} state",
                state.layer_count(),
                state.slot_count()
            ),
        });
    }

    let mut new_state = state.clone();
    new_state.set_identity(layer, slot, value);
    new_state.set_status(layer, slot, SlotStatus::Provisional);

    let record = StepRecord {
        op_code: OP_SET_SLOT.to_le_bytes(),
        op_args: op_args.to_vec(),
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    Ok((new_state, record))
}

/// Build the `op_args` bytes for a `SET_SLOT` operation.
#[must_use]
pub fn set_slot_args(layer: u32, slot: u32, value: Code32) -> Vec<u8> {
    let mut args = Vec::with_capacity(12);
    args.extend_from_slice(&layer.to_le_bytes());
    args.extend_from_slice(&slot.to_le_bytes());
    args.extend_from_slice(&value.to_le_bytes());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_slot_basic() {
        let state = ByteStateV1::new(1, 2);
        let args = set_slot_args(0, 1, Code32::new(1, 1, 5));
        let (new_state, record) = apply(&state, OP_SET_SLOT, &args).unwrap();

        assert_eq!(new_state.get_identity(0, 1), Code32::new(1, 1, 5));
        assert_eq!(new_state.get_status(0, 1), SlotStatus::Provisional);
        // Untouched slot stays default.
        assert_eq!(new_state.get_identity(0, 0), Code32::PADDING);
        assert_eq!(new_state.get_status(0, 0), SlotStatus::Hole);

        assert_eq!(record.op_code, OP_SET_SLOT.to_le_bytes());
        assert_eq!(record.op_args, args);
    }

    #[test]
    fn set_slot_rejects_wrong_arg_length() {
        let state = ByteStateV1::new(1, 2);
        let err = apply(&state, OP_SET_SLOT, &[0; 8]).unwrap_err();
        assert!(matches!(err, ApplyFailure::ArgumentMismatch { .. }));
    }

    #[test]
    fn set_slot_rejects_out_of_bounds() {
        let state = ByteStateV1::new(1, 2);
        let args = set_slot_args(0, 5, Code32::new(1, 1, 1)); // slot 5 out of bounds
        let err = apply(&state, OP_SET_SLOT, &args).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn unknown_operator_rejected() {
        let state = ByteStateV1::new(1, 2);
        let err = apply(&state, Code32::new(9, 9, 9), &[]).unwrap_err();
        assert!(matches!(err, ApplyFailure::UnknownOperator { .. }));
    }

    #[test]
    fn set_slot_deterministic() {
        let state = ByteStateV1::new(1, 2);
        let args = set_slot_args(0, 0, Code32::new(2, 1, 3));
        let (first_state, first_record) = apply(&state, OP_SET_SLOT, &args).unwrap();
        for _ in 0..10 {
            let (s, r) = apply(&state, OP_SET_SLOT, &args).unwrap();
            assert!(s.bitwise_eq(&first_state));
            assert_eq!(r, first_record);
        }
    }
}
