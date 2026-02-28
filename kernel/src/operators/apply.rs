//! `apply()`: apply an operator to `ByteState`, producing new state + step record.
//!
//! The single exported entry point for operator application. Requires an
//! `OperatorRegistryV1` — there is no bypass path.
//!
//! Three-phase check:
//! 1. Registry lookup (contract): is the `op_code` declared?
//! 2. Dispatch lookup (implementation): is there a handler installed?
//! 3. Post-apply validation: do the effects match the declared `EffectKind`?

use std::collections::BTreeMap;

use crate::carrier::bytestate::{ByteStateV1, SlotStatus};
use crate::carrier::code32::Code32;
use crate::operators::operator_registry::{EffectKind, OperatorRegistryV1};

/// Well-known operator code: write a value to a slot.
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
    /// The operator code is not declared in the registry.
    UnknownOperator { op_code: [u8; 4] },
    /// The operator is declared in the registry but has no implementation.
    OperatorNotImplemented { op_code: [u8; 4] },
    /// The implementation produced effects that violate the declared
    /// `EffectKind` contract.
    EffectContractViolation { op_code: [u8; 4], detail: String },
}

/// Result type for apply.
pub type ApplyResult = Result<(ByteStateV1, StepRecord), ApplyFailure>;

/// Type alias for dispatch handler functions.
type DispatchHandler = fn(&ByteStateV1, &[u8]) -> ApplyResult;

/// Build the dispatch table mapping `op_codes` to handler implementations.
///
/// This is the implementation side; the registry is the contract side.
/// Every entry here must have a corresponding registry entry, but the
/// registry may declare operators not yet implemented (caught by
/// `OperatorNotImplemented`).
fn dispatch_table() -> BTreeMap<Code32, DispatchHandler> {
    let mut map: BTreeMap<Code32, DispatchHandler> = BTreeMap::new();
    map.insert(OP_SET_SLOT, apply_set_slot);
    map
}

/// Apply an operator to the current `ByteState`.
///
/// Three-phase check:
/// 1. Registry lookup: `op_code` must be declared in `operator_registry`.
/// 2. Dispatch lookup: a handler must be installed for `op_code`.
/// 3. Post-apply validation: effects must match the declared `EffectKind`.
///
/// # Errors
///
/// Returns [`ApplyFailure`] on:
/// - `UnknownOperator`: `op_code` not in registry
/// - `OperatorNotImplemented`: in registry but no dispatch handler
/// - `ArgumentMismatch`: `op_args` length doesn't match `arg_byte_count`
/// - `PreconditionNotMet`: operator-specific precondition failure
/// - `EffectContractViolation`: handler effects don't match declared kind
pub fn apply(
    state: &ByteStateV1,
    op_code: Code32,
    op_args: &[u8],
    operator_registry: &OperatorRegistryV1,
) -> ApplyResult {
    // Phase 1: Registry lookup (contract check).
    let entry = operator_registry.get(&op_code).ok_or(ApplyFailure::UnknownOperator {
        op_code: op_code.to_le_bytes(),
    })?;

    // Check arg_byte_count before dispatch.
    if op_args.len() != entry.arg_byte_count {
        return Err(ApplyFailure::ArgumentMismatch {
            detail: format!(
                "{} expects {} arg bytes, got {}",
                entry.name, entry.arg_byte_count, op_args.len()
            ),
        });
    }

    // Phase 2: Dispatch lookup (implementation check).
    let table = dispatch_table();
    let handler = table.get(&op_code).ok_or(ApplyFailure::OperatorNotImplemented {
        op_code: op_code.to_le_bytes(),
    })?;

    // Phase 3: Execute and validate effects.
    let (new_state, record) = handler(state, op_args)?;

    validate_effect_kind(state, &new_state, op_code, entry.effect_kind)?;

    Ok((new_state, record))
}

/// Validate that the handler's effects match the declared `EffectKind`.
fn validate_effect_kind(
    old_state: &ByteStateV1,
    new_state: &ByteStateV1,
    op_code: Code32,
    effect_kind: EffectKind,
) -> Result<(), ApplyFailure> {
    match effect_kind {
        EffectKind::WritesOneSlotFromArgs => {
            // Count identity-plane diffs.
            let old_id = old_state.identity_bytes();
            let new_id = new_state.identity_bytes();
            let old_st = old_state.status_bytes();
            let new_st = new_state.status_bytes();

            // Identity plane: count 4-byte slots that changed.
            let id_diffs = old_id
                .chunks(4)
                .zip(new_id.chunks(4))
                .filter(|(a, b)| a != b)
                .count();

            // Status plane: count bytes that changed.
            let st_diffs = old_st
                .iter()
                .zip(new_st.iter())
                .filter(|(a, b)| a != b)
                .count();

            if id_diffs != 1 {
                return Err(ApplyFailure::EffectContractViolation {
                    op_code: op_code.to_le_bytes(),
                    detail: format!(
                        "WritesOneSlotFromArgs: expected 1 identity slot changed, got {id_diffs}"
                    ),
                });
            }
            if st_diffs != 1 {
                return Err(ApplyFailure::EffectContractViolation {
                    op_code: op_code.to_le_bytes(),
                    detail: format!(
                        "WritesOneSlotFromArgs: expected 1 status slot changed, got {st_diffs}"
                    ),
                });
            }
            Ok(())
        }
    }
}

/// `SET_SLOT` operator: write a Code32 value to (layer, slot) and promote.
///
/// Arg byte count check is performed by `apply()` before dispatch.
fn apply_set_slot(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
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
    use crate::operators::operator_registry::kernel_operator_registry;

    fn reg() -> OperatorRegistryV1 {
        kernel_operator_registry()
    }

    #[test]
    fn set_slot_basic() {
        let state = ByteStateV1::new(1, 2);
        let args = set_slot_args(0, 1, Code32::new(1, 1, 5));
        let (new_state, record) = apply(&state, OP_SET_SLOT, &args, &reg()).unwrap();

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
        let err = apply(&state, OP_SET_SLOT, &[0; 8], &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::ArgumentMismatch { .. }));
    }

    #[test]
    fn set_slot_rejects_out_of_bounds() {
        let state = ByteStateV1::new(1, 2);
        let args = set_slot_args(0, 5, Code32::new(1, 1, 1)); // slot 5 out of bounds
        let err = apply(&state, OP_SET_SLOT, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn unknown_operator_rejected() {
        let state = ByteStateV1::new(1, 2);
        let err = apply(&state, Code32::new(9, 9, 9), &[], &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::UnknownOperator { .. }));
    }

    #[test]
    fn set_slot_deterministic() {
        let state = ByteStateV1::new(1, 2);
        let args = set_slot_args(0, 0, Code32::new(2, 1, 3));
        let r = reg();
        let (first_state, first_record) = apply(&state, OP_SET_SLOT, &args, &r).unwrap();
        for _ in 0..10 {
            let (s, rec) = apply(&state, OP_SET_SLOT, &args, &r).unwrap();
            assert!(s.bitwise_eq(&first_state));
            assert_eq!(rec, first_record);
        }
    }

    #[test]
    fn operator_not_implemented_distinct_from_unknown() {
        // Build a registry with a fake operator that has no dispatch handler.
        use crate::operators::operator_registry::{EffectKind, OperatorEntry, OperatorRegistryV1};
        use crate::operators::signature::{IdentityMaskV1, OperatorCategory, StatusMaskV1};

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
        let registry =
            OperatorRegistryV1::new("test.v1".into(), vec![entry]).unwrap();

        let state = ByteStateV1::new(1, 2);
        let err = apply(&state, fake_op, &[], &registry).unwrap_err();
        assert!(
            matches!(err, ApplyFailure::OperatorNotImplemented { .. }),
            "expected OperatorNotImplemented, got {err:?}"
        );
    }
}
