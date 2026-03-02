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

/// Well-known operator code: stage a value to a slot.
///
/// Same args and mechanical effect as `SET_SLOT`, but with distinct
/// `EffectKind::StagesOneSlot` for transcript categorization.
pub const OP_STAGE: Code32 = Code32::new(1, 1, 2);

/// Well-known operator code: commit a transaction.
///
/// Takes 1 arg slot (4 bytes): layer index (u32 LE) identifying the staging layer.
/// Writes commit marker to the last slot (`txn_marker`) on that layer.
/// Precondition: `txn_marker` must be `Hole`; at least one non-marker slot must be
/// `Provisional` on that layer.
pub const OP_COMMIT: Code32 = Code32::new(1, 1, 3);

/// Well-known operator code: roll back a transaction.
///
/// Takes 1 arg slot (4 bytes): layer index (u32 LE) identifying the staging layer.
/// Writes rollback marker to the last slot (`txn_marker`) on that layer.
/// Precondition: `txn_marker` must be `Hole`. No staged-slot precondition.
pub const OP_ROLLBACK: Code32 = Code32::new(1, 1, 4);

/// Well-known operator code: submit a guess (epistemic, agent action).
///
/// Takes K guess values as Code32 (4*K bytes). For K=2: 8 bytes.
/// Writes K slots on layer 1: the guess values for the current probe index.
/// All target slots must be Hole → Provisional (write-once).
/// The dispatch handler does NOT read layer 0 (truth).
pub const OP_GUESS: Code32 = Code32::new(1, 2, 1);

/// Well-known operator code: environment feedback (epistemic, environment action).
///
/// Takes 1 feedback value as Code32 (4 bytes).
/// Writes 1 slot on layer 1: the feedback slot for the current probe index.
/// Slot must be Hole → Provisional (write-once).
/// The dispatch handler does NOT read layer 0 or compute feedback — the value
/// is provided by the world/harness and verified by winning-path replay.
pub const OP_FEEDBACK: Code32 = Code32::new(1, 2, 2);

/// Well-known operator code: declare a solution (epistemic, agent action).
///
/// Takes K solution values as Code32 (4*K bytes). Same format as `OP_GUESS` args.
/// Writes 1 slot on layer 1: the `solved_marker` slot, set to `SOLVED_MARKER`.
/// The dispatch handler does NOT validate whether the declared solution matches
/// truth — that is checked by `is_goal()` and winning-path replay.
pub const OP_DECLARE: Code32 = Code32::new(1, 2, 3);

/// Kernel-level solved marker sentinel written by `OP_DECLARE`.
pub const SOLVED_MARKER: Code32 = Code32::new(0, 0, 3);

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
    map.insert(OP_STAGE, apply_stage);
    map.insert(OP_COMMIT, apply_commit);
    map.insert(OP_ROLLBACK, apply_rollback);
    map.insert(OP_GUESS, apply_guess);
    map.insert(OP_FEEDBACK, apply_feedback);
    map.insert(OP_DECLARE, apply_declare);
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

    validate_effect_kind(state, &new_state, op_code, entry.effect_kind, op_args)?;

    Ok((new_state, record))
}

/// Count identity-plane (4-byte slot) diffs between two states.
fn count_identity_diffs(old_state: &ByteStateV1, new_state: &ByteStateV1) -> usize {
    old_state
        .identity_bytes()
        .chunks(4)
        .zip(new_state.identity_bytes().chunks(4))
        .filter(|(a, b)| a != b)
        .count()
}

/// Count status-plane (1-byte) diffs between two states.
fn count_status_diffs(old_state: &ByteStateV1, new_state: &ByteStateV1) -> usize {
    old_state
        .status_bytes()
        .iter()
        .zip(new_state.status_bytes().iter())
        .filter(|(a, b)| a != b)
        .count()
}

/// Assert exactly 1 identity diff and 1 status diff.
fn assert_one_slot_write(
    old_state: &ByteStateV1,
    new_state: &ByteStateV1,
    op_code: Code32,
    kind_name: &str,
) -> Result<(), ApplyFailure> {
    let id_diffs = count_identity_diffs(old_state, new_state);
    let st_diffs = count_status_diffs(old_state, new_state);

    if id_diffs != 1 {
        return Err(ApplyFailure::EffectContractViolation {
            op_code: op_code.to_le_bytes(),
            detail: format!(
                "{kind_name}: expected 1 identity slot changed, got {id_diffs}"
            ),
        });
    }
    if st_diffs != 1 {
        return Err(ApplyFailure::EffectContractViolation {
            op_code: op_code.to_le_bytes(),
            detail: format!(
                "{kind_name}: expected 1 status slot changed, got {st_diffs}"
            ),
        });
    }
    Ok(())
}

/// Validate that the handler's effects match the declared `EffectKind`.
fn validate_effect_kind(
    old_state: &ByteStateV1,
    new_state: &ByteStateV1,
    op_code: Code32,
    effect_kind: EffectKind,
    op_args: &[u8],
) -> Result<(), ApplyFailure> {
    match effect_kind {
        EffectKind::WritesOneSlotFromArgs => {
            assert_one_slot_write(old_state, new_state, op_code, "WritesOneSlotFromArgs")
        }

        EffectKind::StagesOneSlot => {
            assert_one_slot_write(old_state, new_state, op_code, "StagesOneSlot")
        }

        EffectKind::CommitsTransaction => {
            // Must write exactly one slot (the txn_marker).
            assert_one_slot_write(old_state, new_state, op_code, "CommitsTransaction")?;

            // Additional: at least one non-marker slot on the target layer
            // must be Provisional (something was staged).
            let layer =
                u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;
            let slot_count = old_state.slot_count();
            // The txn_marker is the last slot on the layer.
            let marker_slot = slot_count.saturating_sub(1);

            let has_staged = (0..slot_count)
                .filter(|&s| s != marker_slot)
                .any(|s| old_state.get_status(layer, s) == SlotStatus::Provisional);

            if !has_staged {
                return Err(ApplyFailure::EffectContractViolation {
                    op_code: op_code.to_le_bytes(),
                    detail: "CommitsTransaction: no non-marker slot on target layer is Provisional (empty commit)".into(),
                });
            }
            Ok(())
        }

        EffectKind::RollsBackTransaction => {
            // Must write exactly one slot (the txn_marker). No staged-slot precondition.
            assert_one_slot_write(old_state, new_state, op_code, "RollsBackTransaction")
        }

        EffectKind::WritesGuess => {
            // K values written: K identity diffs and K status diffs.
            // K = (arg_byte_count - 8) / 4 (subtract layer + start_slot header).
            let k = (op_args.len().saturating_sub(8)) / 4;
            let id_diffs = count_identity_diffs(old_state, new_state);
            let st_diffs = count_status_diffs(old_state, new_state);

            if id_diffs != k {
                return Err(ApplyFailure::EffectContractViolation {
                    op_code: op_code.to_le_bytes(),
                    detail: format!(
                        "WritesGuess: expected {k} identity slots changed, got {id_diffs}"
                    ),
                });
            }
            if st_diffs != k {
                return Err(ApplyFailure::EffectContractViolation {
                    op_code: op_code.to_le_bytes(),
                    detail: format!(
                        "WritesGuess: expected {k} status slots changed, got {st_diffs}"
                    ),
                });
            }
            Ok(())
        }

        EffectKind::WritesFeedback => {
            // Exactly 1 identity diff and 1 status diff on layer 1.
            assert_one_slot_write(old_state, new_state, op_code, "WritesFeedback")
        }

        EffectKind::DeclaresSolution => {
            // Exactly 1 identity diff and 1 status diff (the solved_marker slot).
            assert_one_slot_write(old_state, new_state, op_code, "DeclaresSolution")
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

/// Build the `op_args` bytes for a `STAGE` operation (same layout as `SET_SLOT`).
#[must_use]
pub fn stage_args(layer: u32, slot: u32, value: Code32) -> Vec<u8> {
    set_slot_args(layer, slot, value)
}

/// Build the `op_args` bytes for a `COMMIT` operation.
#[must_use]
pub fn commit_args(layer: u32) -> Vec<u8> {
    layer.to_le_bytes().to_vec()
}

/// Build the `op_args` bytes for a `ROLLBACK` operation.
#[must_use]
pub fn rollback_args(layer: u32) -> Vec<u8> {
    layer.to_le_bytes().to_vec()
}

/// `STAGE` operator: stage a value to (layer, slot).
///
/// Mechanically identical to `SET_SLOT`: writes identity and promotes status
/// to `Provisional`. Distinguished at the `EffectKind` level for transcript
/// categorization.
fn apply_stage(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
    let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;
    let slot = u32::from_le_bytes([op_args[4], op_args[5], op_args[6], op_args[7]]) as usize;
    let value = Code32::from_le_bytes([op_args[8], op_args[9], op_args[10], op_args[11]]);

    if layer >= state.layer_count() || slot >= state.slot_count() {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "STAGE target ({layer}, {slot}) out of bounds for {}x{} state",
                state.layer_count(),
                state.slot_count()
            ),
        });
    }

    let mut new_state = state.clone();
    new_state.set_identity(layer, slot, value);
    new_state.set_status(layer, slot, SlotStatus::Provisional);

    let record = StepRecord {
        op_code: OP_STAGE.to_le_bytes(),
        op_args: op_args.to_vec(),
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    Ok((new_state, record))
}

/// `COMMIT` operator: write commit marker to the `txn_marker` slot on a layer.
///
/// The `txn_marker` is the last slot on the target layer. The commit marker
/// value is determined by the world (not the kernel) — here we write a
/// well-known sentinel. The kernel only validates the effect contract.
///
/// Precondition: `txn_marker` must be `Hole`. At least one non-marker slot
/// must be `Provisional` (validated in `validate_effect_kind`).
fn apply_commit(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
    let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;

    if layer >= state.layer_count() {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "COMMIT target layer {layer} out of bounds for {} layers",
                state.layer_count()
            ),
        });
    }

    let marker_slot = state.slot_count().saturating_sub(1);

    if state.get_status(layer, marker_slot) != SlotStatus::Hole {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "COMMIT: txn_marker slot ({layer}, {marker_slot}) is not Hole (already finalized)"
            ),
        });
    }

    let mut new_state = state.clone();
    // Write the commit marker identity. Use a well-known sentinel: Code32(0, 0, 1).
    // The actual marker value is semantic (world-defined concept); the kernel writes
    // a fixed sentinel. Worlds using OP_COMMIT map this sentinel to their concept
    // (e.g., kv:commit).
    new_state.set_identity(layer, marker_slot, COMMIT_MARKER);
    new_state.set_status(layer, marker_slot, SlotStatus::Provisional);

    let record = StepRecord {
        op_code: OP_COMMIT.to_le_bytes(),
        op_args: op_args.to_vec(),
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    Ok((new_state, record))
}

/// `ROLLBACK` operator: write rollback marker to the `txn_marker` slot on a layer.
///
/// The `txn_marker` is the last slot on the target layer. No staged-slot
/// precondition — empty rollbacks are permitted.
///
/// Precondition: `txn_marker` must be `Hole`.
fn apply_rollback(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
    let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;

    if layer >= state.layer_count() {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "ROLLBACK target layer {layer} out of bounds for {} layers",
                state.layer_count()
            ),
        });
    }

    let marker_slot = state.slot_count().saturating_sub(1);

    if state.get_status(layer, marker_slot) != SlotStatus::Hole {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "ROLLBACK: txn_marker slot ({layer}, {marker_slot}) is not Hole (already finalized)"
            ),
        });
    }

    let mut new_state = state.clone();
    new_state.set_identity(layer, marker_slot, ROLLBACK_MARKER);
    new_state.set_status(layer, marker_slot, SlotStatus::Provisional);

    let record = StepRecord {
        op_code: OP_ROLLBACK.to_le_bytes(),
        op_args: op_args.to_vec(),
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    Ok((new_state, record))
}

/// Kernel-level commit marker sentinel written by `OP_COMMIT`.
///
/// Worlds map this to their domain concept (e.g., `kv:commit`). The kernel
/// uses a fixed sentinel so the operator doesn't need to carry the marker
/// value as an argument — the commit/rollback distinction is in the `op_code`.
pub const COMMIT_MARKER: Code32 = Code32::new(0, 0, 1);

/// Kernel-level rollback marker sentinel written by `OP_ROLLBACK`.
pub const ROLLBACK_MARKER: Code32 = Code32::new(0, 0, 2);

/// `GUESS` operator: write K guess values to consecutive slots on a layer.
///
/// Arg layout: `[layer: u32 LE, start_slot: u32 LE, value_0..value_{K-1}: Code32 LE]`
/// Writes K consecutive slots starting at `start_slot` on `layer`.
/// All target slots must be Hole → Provisional (write-once).
/// Does NOT read layer 0 (truth). Pure bounded-write primitive.
fn apply_guess(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
    let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;
    let start_slot =
        u32::from_le_bytes([op_args[4], op_args[5], op_args[6], op_args[7]]) as usize;
    let k = (op_args.len() - 8) / 4;

    if k == 0 {
        return Err(ApplyFailure::ArgumentMismatch {
            detail: "GUESS: no values in args (need at least 1)".into(),
        });
    }

    // Bounds check all target slots.
    if layer >= state.layer_count() || start_slot + k > state.slot_count() {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "GUESS target ({layer}, {start_slot}..{}) out of bounds for {}x{} state",
                start_slot + k,
                state.layer_count(),
                state.slot_count()
            ),
        });
    }

    let mut new_state = state.clone();
    for i in 0..k {
        let offset = 8 + i * 4;
        let value =
            Code32::from_le_bytes([op_args[offset], op_args[offset + 1], op_args[offset + 2], op_args[offset + 3]]);
        new_state.set_identity(layer, start_slot + i, value);
        new_state.set_status(layer, start_slot + i, SlotStatus::Provisional);
    }

    let record = StepRecord {
        op_code: OP_GUESS.to_le_bytes(),
        op_args: op_args.to_vec(),
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    Ok((new_state, record))
}

/// `FEEDBACK` operator: write 1 feedback value to a slot on a layer.
///
/// Arg layout: [layer: u32 LE, slot: u32 LE, value: Code32 LE]
/// Mechanically identical to `SET_SLOT` but with distinct `EffectKind`.
/// Does NOT read layer 0 or compute feedback — the value is provided by the
/// world/harness and verified by winning-path replay.
fn apply_feedback(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
    let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;
    let slot = u32::from_le_bytes([op_args[4], op_args[5], op_args[6], op_args[7]]) as usize;
    let value = Code32::from_le_bytes([op_args[8], op_args[9], op_args[10], op_args[11]]);

    if layer >= state.layer_count() || slot >= state.slot_count() {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "FEEDBACK target ({layer}, {slot}) out of bounds for {}x{} state",
                state.layer_count(),
                state.slot_count()
            ),
        });
    }

    let mut new_state = state.clone();
    new_state.set_identity(layer, slot, value);
    new_state.set_status(layer, slot, SlotStatus::Provisional);

    let record = StepRecord {
        op_code: OP_FEEDBACK.to_le_bytes(),
        op_args: op_args.to_vec(),
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    Ok((new_state, record))
}

/// `DECLARE` operator: write the solved marker to the `solved_marker` slot.
///
/// Arg layout: `[layer: u32 LE, solved_marker_slot: u32 LE, value_0..value_{K-1}: Code32 LE]`
/// The declared solution values (K Code32s) are carried in the args for the
/// record/transcript but are NOT written to state — only the `SOLVED_MARKER`
/// is written to the `solved_marker_slot`. The declared solution is validated
/// by `is_goal()` and winning-path replay, not by the kernel.
fn apply_declare(state: &ByteStateV1, op_args: &[u8]) -> ApplyResult {
    let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]) as usize;
    let solved_slot =
        u32::from_le_bytes([op_args[4], op_args[5], op_args[6], op_args[7]]) as usize;

    if layer >= state.layer_count() || solved_slot >= state.slot_count() {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "DECLARE target ({layer}, {solved_slot}) out of bounds for {}x{} state",
                state.layer_count(),
                state.slot_count()
            ),
        });
    }

    if state.get_status(layer, solved_slot) != SlotStatus::Hole {
        return Err(ApplyFailure::PreconditionNotMet {
            detail: format!(
                "DECLARE: solved_marker slot ({layer}, {solved_slot}) is not Hole (already declared)"
            ),
        });
    }

    let mut new_state = state.clone();
    new_state.set_identity(layer, solved_slot, SOLVED_MARKER);
    new_state.set_status(layer, solved_slot, SlotStatus::Provisional);

    let record = StepRecord {
        op_code: OP_DECLARE.to_le_bytes(),
        op_args: op_args.to_vec(),
        result_identity: new_state.identity_bytes(),
        result_status: new_state.status_bytes(),
    };

    Ok((new_state, record))
}

/// Build the `op_args` bytes for a `GUESS` operation.
///
/// Args layout: `[layer: u32, start_slot: u32, value_0..value_{K-1}: Code32]`
#[must_use]
pub fn guess_args(layer: u32, start_slot: u32, values: &[Code32]) -> Vec<u8> {
    let mut args = Vec::with_capacity(8 + values.len() * 4);
    args.extend_from_slice(&layer.to_le_bytes());
    args.extend_from_slice(&start_slot.to_le_bytes());
    for v in values {
        args.extend_from_slice(&v.to_le_bytes());
    }
    args
}

/// Build the `op_args` bytes for a `FEEDBACK` operation.
///
/// Args layout: [layer: u32, slot: u32, value: Code32]
#[must_use]
pub fn feedback_args(layer: u32, slot: u32, value: Code32) -> Vec<u8> {
    let mut args = Vec::with_capacity(12);
    args.extend_from_slice(&layer.to_le_bytes());
    args.extend_from_slice(&slot.to_le_bytes());
    args.extend_from_slice(&value.to_le_bytes());
    args
}

/// Build the `op_args` bytes for a `DECLARE` operation.
///
/// Args layout: `[layer: u32, solved_marker_slot: u32, value_0..value_{K-1}: Code32]`
/// The values are the declared solution — carried for the record/transcript
/// but only the `solved_marker_slot` is written to state.
#[must_use]
pub fn declare_args(layer: u32, solved_marker_slot: u32, values: &[Code32]) -> Vec<u8> {
    let mut args = Vec::with_capacity(8 + values.len() * 4);
    args.extend_from_slice(&layer.to_le_bytes());
    args.extend_from_slice(&solved_marker_slot.to_le_bytes());
    for v in values {
        args.extend_from_slice(&v.to_le_bytes());
    }
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

    // -----------------------------------------------------------------------
    // STAGE tests
    // -----------------------------------------------------------------------

    #[test]
    fn stage_basic() {
        let state = ByteStateV1::new(2, 4); // 2 layers, 4 slots
        let args = stage_args(1, 0, Code32::new(2, 1, 0));
        let (new_state, record) = apply(&state, OP_STAGE, &args, &reg()).unwrap();

        assert_eq!(new_state.get_identity(1, 0), Code32::new(2, 1, 0));
        assert_eq!(new_state.get_status(1, 0), SlotStatus::Provisional);
        assert_eq!(record.op_code, OP_STAGE.to_le_bytes());
    }

    #[test]
    fn stage_rejects_out_of_bounds() {
        let state = ByteStateV1::new(2, 4);
        let args = stage_args(0, 10, Code32::new(2, 1, 0));
        let err = apply(&state, OP_STAGE, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn stage_rejects_wrong_arg_length() {
        let state = ByteStateV1::new(2, 4);
        let err = apply(&state, OP_STAGE, &[0; 4], &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::ArgumentMismatch { .. }));
    }

    // -----------------------------------------------------------------------
    // COMMIT tests
    // -----------------------------------------------------------------------

    #[test]
    fn commit_basic() {
        // Stage a slot first, then commit.
        let state = ByteStateV1::new(2, 4);
        let stg = stage_args(1, 0, Code32::new(2, 1, 0));
        let (staged, _) = apply(&state, OP_STAGE, &stg, &reg()).unwrap();

        let args = commit_args(1);
        let (committed, record) = apply(&staged, OP_COMMIT, &args, &reg()).unwrap();

        // txn_marker (last slot) should have commit marker.
        assert_eq!(committed.get_identity(1, 3), COMMIT_MARKER);
        assert_eq!(committed.get_status(1, 3), SlotStatus::Provisional);
        assert_eq!(record.op_code, OP_COMMIT.to_le_bytes());
    }

    #[test]
    fn commit_rejects_empty_staging() {
        // No slots staged → commit should fail at effect validation.
        let state = ByteStateV1::new(2, 4);
        let args = commit_args(1);
        let err = apply(&state, OP_COMMIT, &args, &reg()).unwrap_err();
        assert!(
            matches!(err, ApplyFailure::EffectContractViolation { .. }),
            "expected EffectContractViolation for empty commit, got {err:?}"
        );
    }

    #[test]
    fn commit_rejects_double_commit() {
        let state = ByteStateV1::new(2, 4);
        let stg = stage_args(1, 0, Code32::new(2, 1, 0));
        let (staged, _) = apply(&state, OP_STAGE, &stg, &reg()).unwrap();
        let args = commit_args(1);
        let (committed, _) = apply(&staged, OP_COMMIT, &args, &reg()).unwrap();

        // Second commit → txn_marker already Provisional.
        let err = apply(&committed, OP_COMMIT, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn commit_rejects_out_of_bounds_layer() {
        let state = ByteStateV1::new(2, 4);
        let args = commit_args(5);
        let err = apply(&state, OP_COMMIT, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn commit_rejects_wrong_arg_length() {
        let state = ByteStateV1::new(2, 4);
        let err = apply(&state, OP_COMMIT, &[0; 12], &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::ArgumentMismatch { .. }));
    }

    // -----------------------------------------------------------------------
    // ROLLBACK tests
    // -----------------------------------------------------------------------

    #[test]
    fn rollback_basic() {
        let state = ByteStateV1::new(2, 4);
        let stg = stage_args(1, 0, Code32::new(2, 1, 0));
        let (staged, _) = apply(&state, OP_STAGE, &stg, &reg()).unwrap();

        let args = rollback_args(1);
        let (rolled_back, record) = apply(&staged, OP_ROLLBACK, &args, &reg()).unwrap();

        assert_eq!(rolled_back.get_identity(1, 3), ROLLBACK_MARKER);
        assert_eq!(rolled_back.get_status(1, 3), SlotStatus::Provisional);
        assert_eq!(record.op_code, OP_ROLLBACK.to_le_bytes());
    }

    #[test]
    fn rollback_empty_staging_allowed() {
        // No slots staged → rollback is still allowed (no-op rollback).
        let state = ByteStateV1::new(2, 4);
        let args = rollback_args(1);
        let (rolled_back, _) = apply(&state, OP_ROLLBACK, &args, &reg()).unwrap();

        assert_eq!(rolled_back.get_identity(1, 3), ROLLBACK_MARKER);
        assert_eq!(rolled_back.get_status(1, 3), SlotStatus::Provisional);
    }

    #[test]
    fn rollback_rejects_double_rollback() {
        let state = ByteStateV1::new(2, 4);
        let args = rollback_args(1);
        let (rolled_back, _) = apply(&state, OP_ROLLBACK, &args, &reg()).unwrap();

        let err = apply(&rolled_back, OP_ROLLBACK, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn commit_after_rollback_rejected() {
        let state = ByteStateV1::new(2, 4);
        let stg = stage_args(1, 0, Code32::new(2, 1, 0));
        let (staged, _) = apply(&state, OP_STAGE, &stg, &reg()).unwrap();
        let (rolled_back, _) = apply(&staged, OP_ROLLBACK, &rollback_args(1), &reg()).unwrap();

        // Commit after rollback → txn_marker already Provisional.
        let err = apply(&rolled_back, OP_COMMIT, &commit_args(1), &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn rollback_after_commit_rejected() {
        let state = ByteStateV1::new(2, 4);
        let stg = stage_args(1, 0, Code32::new(2, 1, 0));
        let (staged, _) = apply(&state, OP_STAGE, &stg, &reg()).unwrap();
        let (committed, _) = apply(&staged, OP_COMMIT, &commit_args(1), &reg()).unwrap();

        // Rollback after commit → txn_marker already Provisional.
        let err = apply(&committed, OP_ROLLBACK, &rollback_args(1), &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    // -----------------------------------------------------------------------
    // Validator-negative tests (out-of-contract effects)
    // -----------------------------------------------------------------------

    #[test]
    fn stage_write_once_enforced() {
        // Stage slot 0, then try to stage slot 0 again → 0 status diffs (already Provisional).
        let state = ByteStateV1::new(2, 4);
        let args = stage_args(1, 0, Code32::new(2, 1, 0));
        let (staged, _) = apply(&state, OP_STAGE, &args, &reg()).unwrap();

        // Same slot, different value: identity changes but status stays Provisional.
        let args2 = stage_args(1, 0, Code32::new(2, 1, 1));
        let err = apply(&staged, OP_STAGE, &args2, &reg()).unwrap_err();
        assert!(
            matches!(err, ApplyFailure::EffectContractViolation { .. }),
            "expected EffectContractViolation for double-write, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Existing tests
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // GUESS tests
    // -----------------------------------------------------------------------

    #[test]
    fn guess_basic() {
        // 2 layers, 13 slots (K=2, max 4 probes: 4*(2+1)+1 = 13)
        let state = ByteStateV1::new(2, 13);
        let values = [Code32::new(3, 0, 0), Code32::new(3, 0, 1)];
        let args = guess_args(1, 0, &values); // layer 1, start_slot 0
        let (new_state, record) = apply(&state, OP_GUESS, &args, &reg()).unwrap();

        assert_eq!(new_state.get_identity(1, 0), Code32::new(3, 0, 0));
        assert_eq!(new_state.get_identity(1, 1), Code32::new(3, 0, 1));
        assert_eq!(new_state.get_status(1, 0), SlotStatus::Provisional);
        assert_eq!(new_state.get_status(1, 1), SlotStatus::Provisional);
        // Untouched slots stay Hole.
        assert_eq!(new_state.get_status(1, 2), SlotStatus::Hole);
        assert_eq!(record.op_code, OP_GUESS.to_le_bytes());
    }

    #[test]
    fn guess_rejects_wrong_arg_length() {
        let state = ByteStateV1::new(2, 13);
        // Registry says 16 bytes, give 8.
        let err = apply(&state, OP_GUESS, &[0; 8], &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::ArgumentMismatch { .. }));
    }

    #[test]
    fn guess_rejects_out_of_bounds() {
        let state = ByteStateV1::new(2, 4);
        let values = [Code32::new(3, 0, 0), Code32::new(3, 0, 1)];
        let args = guess_args(1, 3, &values); // slots 3,4 but only 4 slots (0..3)
        let err = apply(&state, OP_GUESS, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn guess_write_once_enforced() {
        let state = ByteStateV1::new(2, 13);
        let values = [Code32::new(3, 0, 0), Code32::new(3, 0, 1)];
        let args = guess_args(1, 0, &values);
        let (guessed, _) = apply(&state, OP_GUESS, &args, &reg()).unwrap();

        // Try to guess same slots again — status already Provisional.
        let values2 = [Code32::new(3, 0, 2), Code32::new(3, 0, 0)];
        let args2 = guess_args(1, 0, &values2);
        let err = apply(&guessed, OP_GUESS, &args2, &reg()).unwrap_err();
        assert!(
            matches!(err, ApplyFailure::EffectContractViolation { .. }),
            "expected EffectContractViolation for double-write guess, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // FEEDBACK tests
    // -----------------------------------------------------------------------

    #[test]
    fn feedback_basic() {
        let state = ByteStateV1::new(2, 13);
        let args = feedback_args(1, 2, Code32::new(3, 2, 1)); // feedback slot 2
        let (new_state, record) = apply(&state, OP_FEEDBACK, &args, &reg()).unwrap();

        assert_eq!(new_state.get_identity(1, 2), Code32::new(3, 2, 1));
        assert_eq!(new_state.get_status(1, 2), SlotStatus::Provisional);
        assert_eq!(record.op_code, OP_FEEDBACK.to_le_bytes());
    }

    #[test]
    fn feedback_rejects_wrong_arg_length() {
        let state = ByteStateV1::new(2, 13);
        let err = apply(&state, OP_FEEDBACK, &[0; 4], &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::ArgumentMismatch { .. }));
    }

    #[test]
    fn feedback_rejects_out_of_bounds() {
        let state = ByteStateV1::new(2, 4);
        let args = feedback_args(1, 10, Code32::new(3, 2, 0));
        let err = apply(&state, OP_FEEDBACK, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn feedback_write_once_enforced() {
        let state = ByteStateV1::new(2, 13);
        let args = feedback_args(1, 2, Code32::new(3, 2, 1));
        let (fed, _) = apply(&state, OP_FEEDBACK, &args, &reg()).unwrap();

        let args2 = feedback_args(1, 2, Code32::new(3, 2, 0));
        let err = apply(&fed, OP_FEEDBACK, &args2, &reg()).unwrap_err();
        assert!(
            matches!(err, ApplyFailure::EffectContractViolation { .. }),
            "expected EffectContractViolation for double-write feedback, got {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // DECLARE tests
    // -----------------------------------------------------------------------

    #[test]
    fn declare_basic() {
        let state = ByteStateV1::new(2, 13);
        let solution = [Code32::new(3, 0, 0), Code32::new(3, 0, 1)];
        let args = declare_args(1, 12, &solution); // solved_marker at slot 12
        let (new_state, record) = apply(&state, OP_DECLARE, &args, &reg()).unwrap();

        assert_eq!(new_state.get_identity(1, 12), SOLVED_MARKER);
        assert_eq!(new_state.get_status(1, 12), SlotStatus::Provisional);
        // Slots 0-11 untouched.
        assert_eq!(new_state.get_status(1, 0), SlotStatus::Hole);
        assert_eq!(record.op_code, OP_DECLARE.to_le_bytes());
    }

    #[test]
    fn declare_rejects_wrong_arg_length() {
        let state = ByteStateV1::new(2, 13);
        let err = apply(&state, OP_DECLARE, &[0; 8], &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::ArgumentMismatch { .. }));
    }

    #[test]
    fn declare_rejects_double_declare() {
        let state = ByteStateV1::new(2, 13);
        let solution = [Code32::new(3, 0, 0), Code32::new(3, 0, 1)];
        let args = declare_args(1, 12, &solution);
        let (declared, _) = apply(&state, OP_DECLARE, &args, &reg()).unwrap();

        let err = apply(&declared, OP_DECLARE, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    #[test]
    fn declare_rejects_out_of_bounds() {
        let state = ByteStateV1::new(2, 4);
        let solution = [Code32::new(3, 0, 0), Code32::new(3, 0, 1)];
        let args = declare_args(1, 10, &solution);
        let err = apply(&state, OP_DECLARE, &args, &reg()).unwrap_err();
        assert!(matches!(err, ApplyFailure::PreconditionNotMet { .. }));
    }

    // -----------------------------------------------------------------------
    // Cross-operator tests: epistemic probe cycle
    // -----------------------------------------------------------------------

    #[test]
    fn full_probe_cycle() {
        // Simulate: guess → feedback → guess → feedback → declare
        let state = ByteStateV1::new(2, 13);

        // Probe 0: guess slots 0,1
        let g0 = guess_args(1, 0, &[Code32::new(3, 0, 0), Code32::new(3, 0, 1)]);
        let (s1, _) = apply(&state, OP_GUESS, &g0, &reg()).unwrap();

        // Probe 0: feedback slot 2
        let f0 = feedback_args(1, 2, Code32::new(3, 2, 1)); // 1 exact match
        let (s2, _) = apply(&s1, OP_FEEDBACK, &f0, &reg()).unwrap();

        // Probe 1: guess slots 3,4
        let g1 = guess_args(1, 3, &[Code32::new(3, 0, 2), Code32::new(3, 0, 1)]);
        let (s3, _) = apply(&s2, OP_GUESS, &g1, &reg()).unwrap();

        // Probe 1: feedback slot 5
        let f1 = feedback_args(1, 5, Code32::new(3, 2, 2)); // 2 exact matches
        let (s4, _) = apply(&s3, OP_FEEDBACK, &f1, &reg()).unwrap();

        // Declare: solved_marker at slot 12
        let d = declare_args(1, 12, &[Code32::new(3, 0, 2), Code32::new(3, 0, 1)]);
        let (final_state, _) = apply(&s4, OP_DECLARE, &d, &reg()).unwrap();

        // Verify final state: guess, feedback, and solved_marker all present
        assert_eq!(final_state.get_identity(1, 0), Code32::new(3, 0, 0));
        assert_eq!(final_state.get_identity(1, 1), Code32::new(3, 0, 1));
        assert_eq!(final_state.get_identity(1, 2), Code32::new(3, 2, 1));
        assert_eq!(final_state.get_identity(1, 3), Code32::new(3, 0, 2));
        assert_eq!(final_state.get_identity(1, 4), Code32::new(3, 0, 1));
        assert_eq!(final_state.get_identity(1, 5), Code32::new(3, 2, 2));
        assert_eq!(final_state.get_identity(1, 12), SOLVED_MARKER);

        // Layer 0 untouched (all Hole).
        for s in 0..13 {
            assert_eq!(final_state.get_status(0, s), SlotStatus::Hole);
        }
    }

    // -----------------------------------------------------------------------
    // Existing tests
    // -----------------------------------------------------------------------

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
