//! `ByteTraceV1`: append-only trace format for replay verification.
//!
//! Ported from `core/carrier/bytetrace.py` in Sterling v1.
//!
//! # Binary format
//!
//! ```text
//! [envelope_len:u16le][envelope:JSON]  -- NOT hashed (timestamps, trace_id, etc.)
//! [magic:4 = "BST1"]
//! [header_len:u16le][header:canonical JSON]  -- schema_id, schema_hash, registry, etc.
//! [body: fixed-stride frames]
//! [footer_len:u16le][footer:canonical JSON]  -- hashes, step_count
//! ```
//!
//! Each frame has fixed width:
//! ```text
//! [op_code:4][op_args:arg_slots*4][identity:layers*slots*4][status:layers*slots]
//! ```
//!
//! # M0 scope
//!
//! This module defines the logical types. Binary writer/reader is M2 scope.

/// Magic bytes identifying a `ByteTraceV1` stream.
pub const BYTETRACE_V1_MAGIC: [u8; 4] = *b"BST1";

/// Header for a `ByteTraceV1` stream.
///
/// Commits the schema and registry used for the entire trace.
/// Serialized as canonical JSON (sorted keys, no whitespace, UTF-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceHeaderV1 {
    pub schema_id: String,
    pub schema_hash: String,
    pub registry_epoch: String,
    pub registry_hash: String,
}

/// A single frame in a `ByteTraceV1` stream.
///
/// Fixed-width: `op_code` (4 bytes) + `op_args` (padded) + result identity + result status.
/// Frame width is constant across all frames in a trace (determined by schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceFrameV1 {
    /// The operator code that was applied (4 bytes, `Code32` as LE bytes).
    pub op_code: [u8; 4],
    /// Padded operator arguments (schema-dependent width, `Code32` values).
    pub op_args: Vec<u8>,
    /// Resulting identity plane bytes after this operator.
    pub result_identity: Vec<u8>,
    /// Resulting status plane bytes after this operator.
    pub result_status: Vec<u8>,
}

/// A complete `ByteTraceV1`: header + ordered frames.
///
/// Append-only: frames are added in execution order.
/// Writer/reader implementation is M2 scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceV1 {
    pub header: ByteTraceHeaderV1,
    pub frames: Vec<ByteTraceFrameV1>,
}

/// Verdict from replay verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayVerdict {
    /// Replay matches original execution bit-for-bit.
    Match,
    /// Divergence detected at the given frame index.
    Divergence { frame_index: usize, detail: String },
    /// Trace is structurally invalid (bad magic, truncated, etc.).
    Invalid { detail: String },
}

/// A trace bundle wrapping trace + manifests for verification.
///
/// This is the input to [`crate::proof::replay::replay_verify`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceBundleV1 {
    pub trace: ByteTraceV1,
    /// Canonical JSON bytes of the compilation manifest.
    pub compilation_manifest: Vec<u8>,
    /// Original input payload bytes (for re-compilation during replay).
    pub input_payload: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes_are_bst1() {
        assert_eq!(&BYTETRACE_V1_MAGIC, b"BST1");
    }
}
