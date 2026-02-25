//! `apply()`: apply an operator to `ByteState`, producing new state + step record.
//!
//! # M0 scope
//!
//! Types and signature only. Logic is M1/M2 scope.

use crate::carrier::bytestate::ByteStateV1;

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
    /// Operator arguments do not match the declared descriptor.
    ArgumentMismatch { detail: String },
    /// The operator code is not in the registry.
    UnknownOperator { op_code: [u8; 4] },
}

/// Result type for apply.
pub type ApplyResult = Result<(ByteStateV1, StepRecord), ApplyFailure>;

/// Apply an operator to the current `ByteState`.
///
/// # Arguments
///
/// * `state` - The current `ByteState`.
/// * `op_args_bytes` - Canonical bytes of operator arguments.
/// * `op_args_descriptor` - Schema descriptor for the operator arguments (type + version).
///
/// # Errors
///
/// Returns [`ApplyFailure`] on precondition/mismatch.
///
/// # Panics
///
/// M0 stub. Will panic until M1/M2 implementation.
pub fn apply(
    _state: &ByteStateV1,
    _op_args_bytes: &[u8],
    _op_args_descriptor: &str,
) -> ApplyResult {
    todo!("M1/M2: implement operator application logic")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "M1")]
    fn apply_stub_panics() {
        let state = ByteStateV1::new(4, 32);
        let _ = apply(&state, b"{}", "test_op_v1");
    }
}
