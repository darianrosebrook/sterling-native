//! `ByteTraceV1`: append-only trace format for replay verification.
//!
//! Ported from `core/carrier/bytetrace.py` in Sterling v1.
//!
//! # Binary format (`.bst1`)
//!
//! ```text
//! [envelope_len:u16le][envelope:JSON]       -- NOT hashed (observability only)
//! [magic:4 = "BST1"]                        -- hashed
//! [header_len:u16le][header:canonical JSON]  -- hashed
//! [body: fixed-stride frames]               -- hashed
//! [footer_len:u16le][footer:canonical JSON]  -- hashed
//! ```
//!
//! Each frame has fixed width (determined by header dimensions):
//! ```text
//! [op_code:4][op_args:arg_slot_count*4][identity:layers*slots*4][status:layers*slots]
//! ```
//!
//! # Hashing
//!
//! - **Payload hash** (V1-compatible): `sha256(DOMAIN_BYTETRACE || magic || header_json || body || footer_json)`
//! - **Step chain** (Native-originated): `chain_0 = sha256(DOMAIN_TRACE_STEP || frame_0)`,
//!   `chain_i = sha256(DOMAIN_TRACE_STEP_CHAIN || chain_{i-1} || frame_i)`
//!
//! These are separate claim surfaces. Do not conflate them.
//!
//! # Invariants
//!
//! - No JSON/serde inside frames (S1-M2-NO-SECOND-TRUTH).
//! - Header/footer canonical JSON produced by `proof::canon::canonical_json_bytes` only.
//! - Envelope is excluded from all hashes.
//! - Frame 0 uses `INITIAL_STATE` sentinel for `op_code`, zero-filled `op_args`.

/// Magic bytes identifying a `ByteTraceV1` stream.
pub const BYTETRACE_V1_MAGIC: [u8; 4] = *b"BST1";

/// Maximum length for envelope/header/footer JSON sections (u16 max).
pub const MAX_SECTION_LEN: usize = u16::MAX as usize;

// ---------------------------------------------------------------------------
// Envelope (NOT hashed)
// ---------------------------------------------------------------------------

/// Non-hashed observability metadata.
///
/// Excluded from all trace hashes. Contains wall-clock time, trace ID,
/// and other non-deterministic runner metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceEnvelopeV1 {
    /// ISO 8601 timestamp of trace creation.
    pub timestamp: String,
    /// Unique trace identifier.
    pub trace_id: String,
    /// Version of the runner that produced this trace.
    pub runner_version: String,
    /// Wall-clock time for the traced run, in milliseconds.
    pub wall_time_ms: u64,
}

// ---------------------------------------------------------------------------
// Header (hashed, canonical JSON)
// ---------------------------------------------------------------------------

/// Hashed trace header. Commits schema, registry, and dimensions.
///
/// Serialized as canonical JSON via `proof::canon::canonical_json_bytes`.
/// All keys are ASCII. No floats. This is a **descriptor only** — it does
/// not encode state content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceHeaderV1 {
    /// Schema version string (e.g., "1.0").
    pub schema_version: String,
    /// Domain this trace belongs to (e.g., "rome").
    pub domain_id: String,
    /// Content hash of the registry epoch snapshot.
    pub registry_epoch_hash: String,
    /// Content hash of the operator codebook.
    pub codebook_hash: String,
    /// Content hash of the fixture being traced.
    pub fixture_hash: String,
    /// Total number of frames (including initial state frame 0).
    pub step_count: usize,
    /// Layers per `ByteState`.
    pub layer_count: usize,
    /// Slots per layer.
    pub slot_count: usize,
    /// Fixed number of operator argument slots per frame.
    pub arg_slot_count: usize,
}

impl ByteTraceHeaderV1 {
    /// Fixed frame stride in bytes, derived from header dimensions.
    ///
    /// `stride = 4 + arg_slot_count*4 + layer_count*slot_count*4 + layer_count*slot_count`
    ///
    /// Returns `None` on arithmetic overflow.
    #[must_use]
    pub fn frame_stride(&self) -> Option<usize> {
        let total_slots = self.layer_count.checked_mul(self.slot_count)?;
        let identity_bytes = total_slots.checked_mul(4)?;
        let status_bytes = total_slots;
        let arg_bytes = self.arg_slot_count.checked_mul(4)?;
        let stride = 4usize
            .checked_add(arg_bytes)?
            .checked_add(identity_bytes)?
            .checked_add(status_bytes)?;
        Some(stride)
    }

    /// Expected body length in bytes: `step_count * frame_stride`.
    ///
    /// Returns `None` on arithmetic overflow or if `frame_stride` overflows.
    #[must_use]
    pub fn expected_body_len(&self) -> Option<usize> {
        let stride = self.frame_stride()?;
        self.step_count.checked_mul(stride)
    }
}

// ---------------------------------------------------------------------------
// Footer (hashed, canonical JSON)
// ---------------------------------------------------------------------------

/// Hashed trace footer.
///
/// Serialized as canonical JSON via `proof::canon::canonical_json_bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceFooterV1 {
    /// Suite identity hash linking this trace to a suite run.
    pub suite_identity: String,
    /// Optional SHA-256 digest of the step witness store.
    /// When `None`, omitted from canonical JSON (not serialized as null).
    pub witness_store_digest: Option<String>,
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

/// A single frame in a `ByteTraceV1` stream.
///
/// Fixed-width: `op_code` (4 bytes) + `op_args` (padded) + result identity + result status.
/// Frame width is constant across all frames in a trace (determined by header).
///
/// Frame 0: `op_code` = `INITIAL_STATE` sentinel, `op_args` zero-filled.
///
/// **No JSON/serde in frames.** Frames are pure byte arrays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceFrameV1 {
    /// The operator code that was applied (4 bytes, `Code32` as LE bytes).
    pub op_code: [u8; 4],
    /// Padded operator arguments (`arg_slot_count * 4` bytes).
    pub op_args: Vec<u8>,
    /// Resulting identity plane bytes after this operator.
    pub result_identity: Vec<u8>,
    /// Resulting status plane bytes after this operator.
    pub result_status: Vec<u8>,
}

impl ByteTraceFrameV1 {
    /// Serialize this frame to its fixed-stride byte representation.
    ///
    /// Returns `op_code || op_args || identity || status`.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            4 + self.op_args.len() + self.result_identity.len() + self.result_status.len(),
        );
        buf.extend_from_slice(&self.op_code);
        buf.extend_from_slice(&self.op_args);
        buf.extend_from_slice(&self.result_identity);
        buf.extend_from_slice(&self.result_status);
        buf
    }
}

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

/// A complete `ByteTraceV1`: envelope + header + frames + footer.
///
/// Append-only: frames are added in execution order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteTraceV1 {
    pub envelope: ByteTraceEnvelopeV1,
    pub header: ByteTraceHeaderV1,
    pub frames: Vec<ByteTraceFrameV1>,
    pub footer: ByteTraceFooterV1,
}

// ---------------------------------------------------------------------------
// Verdict
// ---------------------------------------------------------------------------

/// Verdict from replay verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayVerdict {
    /// Replay matches original execution bit-for-bit.
    Match,
    /// Divergence detected at the given frame index.
    Divergence {
        frame_index: usize,
        expected_identity_hex: String,
        actual_identity_hex: String,
        expected_status_hex: String,
        actual_status_hex: String,
        detail: String,
    },
    /// Trace is structurally invalid (bad magic, truncated, etc.).
    Invalid { detail: String },
}

// ---------------------------------------------------------------------------
// Trace bundle
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Parse errors
// ---------------------------------------------------------------------------

/// Typed error for trace parsing. Fail-closed: no partial frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceParseError {
    /// Input is too short to contain the expected section.
    Truncated { detail: String },
    /// Magic bytes do not match "BST1".
    BadMagic { found: [u8; 4] },
    /// A section length field exceeds `MAX_SECTION_LEN`.
    SectionTooLong { section: String, len: usize },
    /// Header JSON is invalid or missing required fields.
    InvalidHeader { detail: String },
    /// Footer JSON is invalid or missing required fields.
    InvalidFooter { detail: String },
    /// Envelope JSON is invalid or missing required fields.
    InvalidEnvelope { detail: String },
    /// Body length does not match `step_count * frame_stride`.
    BodyLengthMismatch { expected: usize, actual: usize },
    /// A frame contains an invalid `SlotStatus` byte.
    InvalidSlotStatus { frame_index: usize, byte_value: u8 },
    /// Header dimensions would cause arithmetic overflow.
    DimensionOverflow { detail: String },
    /// Frame 0 does not use `INITIAL_STATE` sentinel or has non-zero `op_args`.
    BadInitialFrame { detail: String },
    /// Trailing bytes after footer section.
    TrailingBytes { excess: usize },
    /// Header or footer bytes are not in canonical JSON form.
    NonCanonical { section: String, detail: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes_are_bst1() {
        assert_eq!(&BYTETRACE_V1_MAGIC, b"BST1");
    }

    #[test]
    fn frame_stride_basic() {
        let header = ByteTraceHeaderV1 {
            schema_version: "1.0".into(),
            domain_id: "test".into(),
            registry_epoch_hash: String::new(),
            codebook_hash: String::new(),
            fixture_hash: String::new(),
            step_count: 2,
            layer_count: 2,
            slot_count: 4,
            arg_slot_count: 2,
        };
        // 4 + 2*4 + 2*4*4 + 2*4 = 4 + 8 + 32 + 8 = 52
        assert_eq!(header.frame_stride(), Some(52));
        assert_eq!(header.expected_body_len(), Some(104));
    }

    #[test]
    fn frame_stride_overflow_returns_none() {
        let header = ByteTraceHeaderV1 {
            schema_version: "1.0".into(),
            domain_id: "test".into(),
            registry_epoch_hash: String::new(),
            codebook_hash: String::new(),
            fixture_hash: String::new(),
            step_count: usize::MAX,
            layer_count: usize::MAX,
            slot_count: usize::MAX,
            arg_slot_count: 0,
        };
        assert!(header.frame_stride().is_none());
    }

    #[test]
    fn frame_to_bytes_round_trip() {
        let frame = ByteTraceFrameV1 {
            op_code: [0, 0, 1, 0], // INITIAL_STATE
            op_args: vec![0; 8],
            result_identity: vec![1, 0, 0, 0, 0, 0, 0, 0],
            result_status: vec![0, 0],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes.len(), 4 + 8 + 8 + 2);
        assert_eq!(&bytes[..4], &[0, 0, 1, 0]);
    }
}
