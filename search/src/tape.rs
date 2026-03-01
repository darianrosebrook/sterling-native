//! `SearchTapeV1`: binary event log types, constants, and traits.
//!
//! The tape is a compact binary format written during the search loop that
//! captures the complete search execution as an append-only event stream.
//! Everything else (`search_graph.json`, summaries) is a derived view.
//!
//! # Wire format
//!
//! ```text
//! [magic:4 "STAP"][version:u16le=1][header_len:u32le][header: canonical JSON]
//! [record_0][record_1]...[record_N]
//! [footer: 48 bytes fixed]
//! ```
//!
//! Records are framed as `[len:u32le][type:u8][body...]`.
//! Footer: `[record_count:u64le][final_chain_hash:32 bytes][footer_magic:u32le "PATS"]`.

use sha2::{Digest, Sha256};
use sterling_kernel::proof::hash::HashDomain;

use crate::graph::{
    ApplyFailureKindV1, CandidateOutcomeV1, DeadEndReasonV1, ExpandEventV1, ExpansionNoteV1,
    FrontierInvariantStageV1, PanicStageV1, TerminationReasonV1,
};
use crate::node::SearchNodeV1;
use crate::scorer::ScoreSourceV1;

// ---------------------------------------------------------------------------
// Magic bytes and version
// ---------------------------------------------------------------------------

/// File magic bytes for `.stap` files.
pub const SEARCH_TAPE_MAGIC: [u8; 4] = *b"STAP";

/// Footer magic bytes (reverse of header magic — truncation sentinel).
pub const SEARCH_TAPE_FOOTER_MAGIC: [u8; 4] = *b"PATS";

/// Wire format version. Readers MUST reject unknown versions.
pub const SEARCH_TAPE_VERSION: u16 = 1;

/// Fixed footer size in bytes: `u64` (`record_count`) + 32 (chain hash) + `u32` (magic).
pub const FOOTER_SIZE: usize = 8 + 32 + 4;

// ---------------------------------------------------------------------------
// Domain separators (search-originated, not kernel)
// ---------------------------------------------------------------------------

/// Domain prefix for tape header hashing (chain seed).
/// `h0 = sha256(DOMAIN_SEARCH_TAPE || header_bytes)`
pub const DOMAIN_SEARCH_TAPE: HashDomain = HashDomain::SearchTape;

/// Domain prefix for tape chain step hashing.
/// `h_i = sha256(DOMAIN_SEARCH_TAPE_CHAIN || h_{i-1} || record_frame_bytes)`
pub const DOMAIN_SEARCH_TAPE_CHAIN: HashDomain = HashDomain::SearchTapeChain;

// ---------------------------------------------------------------------------
// Record type tags
// ---------------------------------------------------------------------------

/// Record type: node creation (root + each Applied child).
pub const RECORD_TYPE_NODE_CREATION: u8 = 1;

/// Record type: expansion event (one per frontier pop).
pub const RECORD_TYPE_EXPANSION: u8 = 2;

/// Record type: termination (exactly one, always last).
pub const RECORD_TYPE_TERMINATION: u8 = 3;

// ---------------------------------------------------------------------------
// Enum tag constants
// ---------------------------------------------------------------------------

// DeadEndReason tags (0 = none/absent)
pub const DEAD_END_NONE: u8 = 0;
pub const DEAD_END_EXHAUSTIVE: u8 = 1;
pub const DEAD_END_BUDGET_LIMITED: u8 = 2;

// CandidateOutcome tags
pub const OUTCOME_APPLIED: u8 = 0;
pub const OUTCOME_DUPLICATE_SUPPRESSED: u8 = 1;
pub const OUTCOME_ILLEGAL_OPERATOR: u8 = 2;
pub const OUTCOME_APPLY_FAILED: u8 = 3;
pub const OUTCOME_SKIPPED_BY_DEPTH_LIMIT: u8 = 4;
pub const OUTCOME_SKIPPED_BY_POLICY: u8 = 5;
pub const OUTCOME_NOT_EVALUATED: u8 = 6;

// ApplyFailureKind tags
pub const APPLY_FAILURE_PRECONDITION_NOT_MET: u8 = 0;
pub const APPLY_FAILURE_ARGUMENT_MISMATCH: u8 = 1;
pub const APPLY_FAILURE_UNKNOWN_OPERATOR: u8 = 2;

// ScoreSource tags
pub const SCORE_SOURCE_UNIFORM: u8 = 0;
pub const SCORE_SOURCE_MODEL_DIGEST: u8 = 1;
pub const SCORE_SOURCE_UNAVAILABLE: u8 = 2;

// TerminationReason tags
pub const TERM_GOAL_REACHED: u8 = 0;
pub const TERM_FRONTIER_EXHAUSTED: u8 = 1;
pub const TERM_EXPANSION_BUDGET_EXCEEDED: u8 = 2;
pub const TERM_DEPTH_BUDGET_EXCEEDED: u8 = 3;
pub const TERM_WORLD_CONTRACT_VIOLATION: u8 = 4;
pub const TERM_SCORER_CONTRACT_VIOLATION: u8 = 5;
pub const TERM_INTERNAL_PANIC: u8 = 6;
pub const TERM_FRONTIER_INVARIANT_VIOLATION: u8 = 7;

// PanicStage tags
pub const PANIC_ENUMERATE_CANDIDATES: u8 = 0;
pub const PANIC_SCORE_CANDIDATES: u8 = 1;
pub const PANIC_IS_GOAL_ROOT: u8 = 2;
pub const PANIC_IS_GOAL_EXPANSION: u8 = 3;

// FrontierInvariantStage tags
pub const FRONTIER_INV_POP_FROM_NON_EMPTY: u8 = 0;

// ExpansionNote tags
pub const NOTE_CANDIDATE_CAP_REACHED: u8 = 0;
pub const NOTE_FRONTIER_PRUNED: u8 = 1;

// ---------------------------------------------------------------------------
// Raw hash helpers (allocation-free)
// ---------------------------------------------------------------------------

/// SHA-256 with domain prefix, returning raw 32 bytes.
///
/// Equivalent to `canonical_hash(domain, data)` but returns `[u8; 32]` instead
/// of `ContentHash`. Used internally for tape chain integrity where raw bytes
/// are needed. Also used by lock-tests for tape binary surgery tooling.
///
/// Stable only insofar as the tape format remains v1.
#[must_use]
pub fn raw_hash(domain: HashDomain, data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// SHA-256 with domain prefix and two data slices, returning raw 32 bytes.
///
/// Used for hash chain: `h_i = raw_hash2(DOMAIN, h_{i-1}, record_frame_bytes)`.
/// Public for the same reason as [`raw_hash`]: lock-tests tape surgery tooling.
#[must_use]
pub fn raw_hash2(domain: HashDomain, a: &[u8], b: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update(a);
    hasher.update(b);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

// ---------------------------------------------------------------------------
// ContentHash ↔ raw bytes conversion helpers
// ---------------------------------------------------------------------------

/// Extract the raw 32-byte SHA-256 digest from a `ContentHash`.
///
/// Decodes the hex digest string. Returns a tape-local error if the
/// algorithm is not `sha256` or the digest is not valid 64-char lowercase hex.
pub(crate) fn content_hash_to_raw(
    hash: &sterling_kernel::proof::hash::ContentHash,
) -> Result<[u8; 32], TapeWriteError> {
    if hash.algorithm() != "sha256" {
        return Err(TapeWriteError::UnsupportedHashAlgorithm);
    }
    hex_str_to_raw(hash.hex_digest())
}

/// Reconstruct a `ContentHash` from raw 32-byte SHA-256 digest.
///
/// Produces `"sha256:<lowercase_hex>"` format.
pub(crate) fn raw_to_content_hash(bytes: &[u8; 32]) -> sterling_kernel::proof::hash::ContentHash {
    let hex = hex::encode(bytes);
    // Safety: we construct a valid "sha256:<hex>" string from known-good bytes.
    sterling_kernel::proof::hash::ContentHash::parse(&format!("sha256:{hex}"))
        .expect("raw_to_content_hash: constructed hash is always valid")
}

/// Decode a lowercase hex string (64 chars) to raw 32 bytes.
pub(crate) fn hex_str_to_raw(hex_str: &str) -> Result<[u8; 32], TapeWriteError> {
    let mut out = [0u8; 32];
    hex::decode_to_slice(hex_str, &mut out).map_err(|_| TapeWriteError::InvalidHexDigest)?;
    Ok(out)
}

/// Encode raw 32 bytes to lowercase hex string (64 chars).
pub(crate) fn raw_to_hex_str(bytes: &[u8; 32]) -> String {
    hex::encode(bytes)
}

// ---------------------------------------------------------------------------
// Tape write errors
// ---------------------------------------------------------------------------

/// Errors during tape writing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapeWriteError {
    /// Operator args exceed `u16::MAX` bytes.
    OpArgsTooLong { len: usize },
    /// Hash algorithm is not `sha256` (only sha256 is supported in V1).
    UnsupportedHashAlgorithm,
    /// Hex digest string is not valid lowercase hex or wrong length.
    InvalidHexDigest,
    /// Canonical JSON serialization failed.
    CanonError(String),
    /// Termination was already written.
    AlreadyTerminated,
    /// Writer not yet terminated when `finish()` was called.
    NotTerminated,
}

impl std::fmt::Display for TapeWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpArgsTooLong { len } => {
                write!(f, "op_args length {len} exceeds u16::MAX")
            }
            Self::UnsupportedHashAlgorithm => {
                write!(f, "only sha256 hashes are supported in tape V1")
            }
            Self::InvalidHexDigest => write!(f, "invalid hex digest string"),
            Self::CanonError(detail) => write!(f, "canonical JSON error: {detail}"),
            Self::AlreadyTerminated => write!(f, "termination already written"),
            Self::NotTerminated => {
                write!(f, "finish() called before termination was written")
            }
        }
    }
}

impl std::error::Error for TapeWriteError {}

// ---------------------------------------------------------------------------
// Tape parse errors
// ---------------------------------------------------------------------------

/// Errors during tape parsing (fail-closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapeParseError {
    /// File is too short for minimum valid tape.
    TooShort,
    /// Invalid magic bytes.
    BadMagic,
    /// Unsupported version.
    UnsupportedVersion { got: u16 },
    /// Header length exceeds available data.
    HeaderTruncated,
    /// Header is not valid canonical JSON.
    InvalidHeaderJson(String),
    /// Record frame truncated (not enough bytes for declared length).
    RecordTruncated { record_index: u64 },
    /// Unknown record type tag.
    UnknownRecordType { tag: u8, record_index: u64 },
    /// Unknown enum tag within a record.
    UnknownEnumTag {
        field: &'static str,
        tag: u8,
        record_index: u64,
    },
    /// Record body is shorter than expected for the declared fields.
    RecordBodyTruncated {
        record_index: u64,
        detail: &'static str,
    },
    /// Footer magic mismatch.
    BadFooterMagic,
    /// Footer `record_count` does not match decoded count.
    RecordCountMismatch { expected: u64, actual: u64 },
    /// Footer chain hash does not match recomputed hash.
    ChainHashMismatch,
    /// Trailing bytes after footer.
    TrailingBytes { excess: usize },
    /// Duplicate `node_id` in `NodeCreation` records.
    DuplicateNodeId { node_id: u64 },
    /// Applied outcome references a `node_id` not in any `NodeCreation` record.
    InvalidAppliedNodeRef { to_node: u64, record_index: u64 },
    /// `NodeCreation` `parent_id` is not less than `node_id`.
    NonMonotonicParentId { node_id: u64, parent_id: u64 },
    /// Expansion orders are not monotonically increasing.
    NonMonotonicExpansionOrder {
        previous: u64,
        current: u64,
        record_index: u64,
    },
    /// Termination record is not the last record.
    TerminationNotLast { record_index: u64 },
    /// No termination record found.
    MissingTermination,
    /// Multiple termination records found.
    DuplicateTermination { record_index: u64 },
    /// Record parser did not consume all bytes in the frame body.
    FrameBodyNotFullyConsumed { record_index: u64, remaining: usize },
    /// `NodeCreation` references a `parent_id` that has no corresponding
    /// `NodeCreation` record (dangling link in graph topology).
    DanglingParentLink { node_id: u64, parent_id: u64 },
    /// `parent_id_present` flag byte is not `0x00` (absent) or `0x01` (present).
    InvalidParentPresenceFlag { flag: u8, record_index: u64 },
}

impl std::fmt::Display for TapeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TapeParseError {}

// ---------------------------------------------------------------------------
// Tape render errors
// ---------------------------------------------------------------------------

/// Errors during tape → `SearchGraphV1` rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapeRenderError {
    /// Missing required header field.
    MissingHeaderField(&'static str),
    /// Header field has invalid format.
    InvalidHeaderField { field: &'static str, detail: String },
    /// Tape has no termination record (should be caught by reader).
    NoTermination,
}

impl std::fmt::Display for TapeRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TapeRenderError {}

// ---------------------------------------------------------------------------
// TapeSink trait
// ---------------------------------------------------------------------------

/// View of a node creation event for the tape sink.
///
/// Borrowed references to data already computed by the search loop.
pub struct NodeCreationView<'a> {
    pub node_id: u64,
    pub parent_id: Option<u64>,
    /// Raw 32-byte SHA-256 fingerprint digest.
    pub state_fingerprint_raw: [u8; 32],
    pub depth: u32,
    pub f_cost: i64,
    pub creation_order: u64,
    /// The full node reference (for debug assertions only, not serialized).
    #[allow(dead_code)]
    pub node: &'a SearchNodeV1,
}

/// View of an expansion event for the tape sink.
///
/// Borrowed references to data already computed by the search loop.
pub struct ExpansionView<'a> {
    pub expansion: &'a ExpandEventV1,
    /// Raw 32-byte fingerprint of the expanded node.
    pub state_fingerprint_raw: [u8; 32],
}

/// View of a termination event for the tape sink.
pub struct TerminationView {
    pub termination_reason: TerminationReasonV1,
    pub frontier_high_water: u64,
}

/// Trait for streaming tape event sinks.
///
/// The search loop calls these methods at each data-emitting site.
/// `TapeWriter` implements this trait.
pub(crate) trait TapeSink {
    /// Record a node creation (root or Applied child).
    fn on_node_created(&mut self, view: &NodeCreationView<'_>) -> Result<(), TapeWriteError>;

    /// Record an expansion event (one per frontier pop).
    fn on_expansion(&mut self, view: &ExpansionView<'_>) -> Result<(), TapeWriteError>;

    /// Record termination. Marks the writer as terminated.
    /// Must be called exactly once, as the last event before `finish()`.
    fn on_termination(&mut self, view: &TerminationView) -> Result<(), TapeWriteError>;
}

// ---------------------------------------------------------------------------
// Parsed tape types (reader output)
// ---------------------------------------------------------------------------

/// Parsed tape header.
#[derive(Debug, Clone)]
pub struct SearchTapeHeaderV1 {
    /// Raw canonical JSON bytes of the header.
    pub json_bytes: Vec<u8>,
    /// Parsed JSON value for field access.
    pub json: serde_json::Value,
}

/// Parsed tape footer.
#[derive(Debug, Clone)]
pub struct SearchTapeFooterV1 {
    pub record_count: u64,
    pub final_chain_hash: [u8; 32],
}

/// A complete parsed tape.
#[derive(Debug, Clone)]
pub struct SearchTapeV1 {
    pub header: SearchTapeHeaderV1,
    pub records: Vec<TapeRecordV1>,
    pub footer: SearchTapeFooterV1,
}

/// A single parsed record from the tape.
#[derive(Debug, Clone)]
pub enum TapeRecordV1 {
    NodeCreation(TapeNodeCreationV1),
    Expansion(TapeExpansionV1),
    Termination(TapeTerminationV1),
}

/// Parsed node creation record.
#[derive(Debug, Clone)]
pub struct TapeNodeCreationV1 {
    pub node_id: u64,
    pub parent_id: Option<u64>,
    pub state_fingerprint: [u8; 32],
    pub depth: u32,
    pub f_cost: i64,
    pub creation_order: u64,
}

/// Parsed expansion record.
#[derive(Debug, Clone)]
pub struct TapeExpansionV1 {
    pub expansion_order: u64,
    pub node_id: u64,
    pub state_fingerprint: [u8; 32],
    pub pop_f_cost: i64,
    pub pop_depth: u32,
    pub pop_creation_order: u64,
    pub candidates_truncated: bool,
    pub dead_end_reason: Option<DeadEndReasonV1>,
    pub candidates: Vec<TapeCandidateV1>,
    pub notes: Vec<ExpansionNoteV1>,
}

/// Parsed candidate record within an expansion.
#[derive(Debug, Clone)]
pub struct TapeCandidateV1 {
    pub index: u64,
    pub op_code_bytes: [u8; 4],
    pub op_args: Vec<u8>,
    pub canonical_hash: [u8; 32],
    pub score_bonus: i64,
    pub score_source: TapeScoreSourceV1,
    pub outcome: TapeCandidateOutcomeV1,
}

/// Parsed score source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapeScoreSourceV1 {
    Uniform,
    ModelDigest([u8; 32]),
    Unavailable,
}

/// Parsed candidate outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapeCandidateOutcomeV1 {
    Applied { to_node: u64 },
    DuplicateSuppressed { existing_fingerprint: [u8; 32] },
    IllegalOperator,
    ApplyFailed(ApplyFailureKindV1),
    SkippedByDepthLimit,
    SkippedByPolicy,
    NotEvaluated,
}

/// Parsed termination record.
#[derive(Debug, Clone)]
pub struct TapeTerminationV1 {
    pub reason: TerminationReasonV1,
    pub frontier_high_water: u64,
}

/// Output from a completed tape write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapeOutput {
    /// Complete tape bytes (magic through footer).
    pub bytes: Vec<u8>,
    /// Final hash chain value (same as footer's `final_chain_hash`).
    pub final_chain_hash: [u8; 32],
    /// Total number of records written.
    pub record_count: u64,
}

// ---------------------------------------------------------------------------
// Tag encoding/decoding helpers
// ---------------------------------------------------------------------------

/// Encode a `DeadEndReasonV1` option to its tape tag.
pub(crate) fn dead_end_to_tag(reason: Option<DeadEndReasonV1>) -> u8 {
    match reason {
        None => DEAD_END_NONE,
        Some(DeadEndReasonV1::Exhaustive) => DEAD_END_EXHAUSTIVE,
        Some(DeadEndReasonV1::BudgetLimited) => DEAD_END_BUDGET_LIMITED,
    }
}

/// Encode a `CandidateOutcomeV1` to its tape tag.
pub(crate) fn outcome_to_tag(outcome: &CandidateOutcomeV1) -> u8 {
    match outcome {
        CandidateOutcomeV1::Applied { .. } => OUTCOME_APPLIED,
        CandidateOutcomeV1::DuplicateSuppressed { .. } => OUTCOME_DUPLICATE_SUPPRESSED,
        CandidateOutcomeV1::IllegalOperator => OUTCOME_ILLEGAL_OPERATOR,
        CandidateOutcomeV1::ApplyFailed(_) => OUTCOME_APPLY_FAILED,
        CandidateOutcomeV1::SkippedByDepthLimit => OUTCOME_SKIPPED_BY_DEPTH_LIMIT,
        CandidateOutcomeV1::SkippedByPolicy => OUTCOME_SKIPPED_BY_POLICY,
        CandidateOutcomeV1::NotEvaluated => OUTCOME_NOT_EVALUATED,
    }
}

/// Encode an `ApplyFailureKindV1` to its tape tag.
pub(crate) fn apply_failure_to_tag(kind: ApplyFailureKindV1) -> u8 {
    match kind {
        ApplyFailureKindV1::PreconditionNotMet => APPLY_FAILURE_PRECONDITION_NOT_MET,
        ApplyFailureKindV1::ArgumentMismatch => APPLY_FAILURE_ARGUMENT_MISMATCH,
        ApplyFailureKindV1::UnknownOperator => APPLY_FAILURE_UNKNOWN_OPERATOR,
    }
}

/// Decode a tape tag to `ApplyFailureKindV1`.
pub(crate) fn tag_to_apply_failure(tag: u8) -> Option<ApplyFailureKindV1> {
    match tag {
        APPLY_FAILURE_PRECONDITION_NOT_MET => Some(ApplyFailureKindV1::PreconditionNotMet),
        APPLY_FAILURE_ARGUMENT_MISMATCH => Some(ApplyFailureKindV1::ArgumentMismatch),
        APPLY_FAILURE_UNKNOWN_OPERATOR => Some(ApplyFailureKindV1::UnknownOperator),
        _ => None,
    }
}

/// Encode a `ScoreSourceV1` to its tape tag.
pub(crate) fn score_source_to_tag(source: &ScoreSourceV1) -> u8 {
    match source {
        ScoreSourceV1::Uniform => SCORE_SOURCE_UNIFORM,
        ScoreSourceV1::ModelDigest(_) => SCORE_SOURCE_MODEL_DIGEST,
        ScoreSourceV1::Unavailable => SCORE_SOURCE_UNAVAILABLE,
    }
}

/// Encode a `TerminationReasonV1` to its tape tag.
pub(crate) fn termination_to_tag(reason: &TerminationReasonV1) -> u8 {
    match reason {
        TerminationReasonV1::GoalReached { .. } => TERM_GOAL_REACHED,
        TerminationReasonV1::FrontierExhausted => TERM_FRONTIER_EXHAUSTED,
        TerminationReasonV1::ExpansionBudgetExceeded => TERM_EXPANSION_BUDGET_EXCEEDED,
        TerminationReasonV1::DepthBudgetExceeded => TERM_DEPTH_BUDGET_EXCEEDED,
        TerminationReasonV1::WorldContractViolation => TERM_WORLD_CONTRACT_VIOLATION,
        TerminationReasonV1::ScorerContractViolation { .. } => TERM_SCORER_CONTRACT_VIOLATION,
        TerminationReasonV1::InternalPanic { .. } => TERM_INTERNAL_PANIC,
        TerminationReasonV1::FrontierInvariantViolation { .. } => TERM_FRONTIER_INVARIANT_VIOLATION,
    }
}

/// Encode a `PanicStageV1` to its tape tag.
pub(crate) fn panic_stage_to_tag(stage: PanicStageV1) -> u8 {
    match stage {
        PanicStageV1::EnumerateCandidates => PANIC_ENUMERATE_CANDIDATES,
        PanicStageV1::ScoreCandidates => PANIC_SCORE_CANDIDATES,
        PanicStageV1::IsGoalRoot => PANIC_IS_GOAL_ROOT,
        PanicStageV1::IsGoalExpansion => PANIC_IS_GOAL_EXPANSION,
    }
}

/// Decode a tape tag to `PanicStageV1`.
pub(crate) fn tag_to_panic_stage(tag: u8) -> Option<PanicStageV1> {
    match tag {
        PANIC_ENUMERATE_CANDIDATES => Some(PanicStageV1::EnumerateCandidates),
        PANIC_SCORE_CANDIDATES => Some(PanicStageV1::ScoreCandidates),
        PANIC_IS_GOAL_ROOT => Some(PanicStageV1::IsGoalRoot),
        PANIC_IS_GOAL_EXPANSION => Some(PanicStageV1::IsGoalExpansion),
        _ => None,
    }
}

/// Decode a tape tag to `FrontierInvariantStageV1`.
pub(crate) fn tag_to_frontier_invariant_stage(tag: u8) -> Option<FrontierInvariantStageV1> {
    match tag {
        FRONTIER_INV_POP_FROM_NON_EMPTY => Some(FrontierInvariantStageV1::PopFromNonEmptyFrontier),
        _ => None,
    }
}

/// Encode a `FrontierInvariantStageV1` to its tape tag.
pub(crate) fn frontier_invariant_stage_to_tag(stage: FrontierInvariantStageV1) -> u8 {
    match stage {
        FrontierInvariantStageV1::PopFromNonEmptyFrontier => FRONTIER_INV_POP_FROM_NON_EMPTY,
    }
}

/// Encode an `ExpansionNoteV1` to its tape tag.
pub(crate) fn note_to_tag(note: &ExpansionNoteV1) -> u8 {
    match note {
        ExpansionNoteV1::CandidateCapReached { .. } => NOTE_CANDIDATE_CAP_REACHED,
        ExpansionNoteV1::FrontierPruned { .. } => NOTE_FRONTIER_PRUNED,
    }
}

// ---------------------------------------------------------------------------
// Candidate/expansion view helpers for the writer
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sterling_kernel::proof::hash::{canonical_hash, ContentHash};

    #[test]
    fn raw_hash_matches_canonical_hash() {
        let domain = HashDomain::SearchTape;
        let data = b"test data for hash verification";

        let raw = raw_hash(domain, data);
        let content = canonical_hash(domain, data);

        // Decode the canonical_hash hex digest to raw bytes and compare.
        let expected_hex = content.hex_digest();
        let mut expected_raw = [0u8; 32];
        hex::decode_to_slice(expected_hex, &mut expected_raw).unwrap();

        assert_eq!(raw, expected_raw, "raw_hash must match canonical_hash");
    }

    #[test]
    fn raw_hash2_matches_concatenated_canonical_hash() {
        let domain = DOMAIN_SEARCH_TAPE_CHAIN;
        let a = [1u8; 32]; // simulated previous chain hash
        let b = b"record frame bytes here";

        let raw = raw_hash2(domain, &a, b);

        // Manually concatenate and compute via canonical_hash.
        let mut combined = Vec::new();
        combined.extend_from_slice(&a);
        combined.extend_from_slice(b);
        let content = canonical_hash(domain, &combined);

        let expected_hex = content.hex_digest();
        let mut expected_raw = [0u8; 32];
        hex::decode_to_slice(expected_hex, &mut expected_raw).unwrap();

        assert_eq!(
            raw, expected_raw,
            "raw_hash2 must match canonical_hash(domain, a || b)"
        );
    }

    #[test]
    fn content_hash_roundtrip() {
        let original = ContentHash::parse(
            "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        )
        .unwrap();
        let raw = content_hash_to_raw(&original).unwrap();
        let reconstructed = raw_to_content_hash(&raw);
        assert_eq!(original, reconstructed);
    }

    #[test]
    fn hex_str_roundtrip() {
        let hex_str = "deadbeef01234567deadbeef01234567deadbeef01234567deadbeef01234567";
        let raw = hex_str_to_raw(hex_str).unwrap();
        let back = raw_to_hex_str(&raw);
        assert_eq!(hex_str, back);
    }

    #[test]
    fn hex_str_to_raw_rejects_invalid() {
        // Wrong length
        assert!(hex_str_to_raw("abcd").is_err());
        // Invalid hex chars
        assert!(
            hex_str_to_raw("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ")
                .is_err()
        );
    }

    #[test]
    fn content_hash_to_raw_rejects_non_sha256() {
        let hash = ContentHash::parse(
            "blake3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        )
        .unwrap();
        assert_eq!(
            content_hash_to_raw(&hash),
            Err(TapeWriteError::UnsupportedHashAlgorithm)
        );
    }

    /// Lock INV-TO-01 + INV-TO-03: short sha256 digests are accepted by
    /// `ContentHash::parse()` but rejected by the tape path with a tape-local
    /// error (`InvalidHexDigest`), not a kernel error type.
    #[test]
    fn short_sha256_accepted_by_parse_rejected_by_tape() {
        // parse() accepts short sha256 digests (permissive).
        let short = ContentHash::parse("sha256:abcdef0123456789").unwrap();
        assert_eq!(short.algorithm(), "sha256");

        // Tape path rejects with tape-local error (hex decode fails on short input).
        assert_eq!(
            content_hash_to_raw(&short),
            Err(TapeWriteError::InvalidHexDigest)
        );
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn tag_constants_are_distinct() {
        // Record types
        assert_ne!(RECORD_TYPE_NODE_CREATION, RECORD_TYPE_EXPANSION);
        assert_ne!(RECORD_TYPE_EXPANSION, RECORD_TYPE_TERMINATION);
        assert_ne!(RECORD_TYPE_NODE_CREATION, RECORD_TYPE_TERMINATION);

        // Outcome tags (0..6 contiguous)
        let outcomes: [u8; 7] = [
            OUTCOME_APPLIED,
            OUTCOME_DUPLICATE_SUPPRESSED,
            OUTCOME_ILLEGAL_OPERATOR,
            OUTCOME_APPLY_FAILED,
            OUTCOME_SKIPPED_BY_DEPTH_LIMIT,
            OUTCOME_SKIPPED_BY_POLICY,
            OUTCOME_NOT_EVALUATED,
        ];
        for (i, a) in outcomes.iter().enumerate() {
            assert_eq!(*a, i as u8, "outcome tags should be contiguous from 0");
        }

        // Termination tags (0..7 contiguous)
        let terms: [u8; 8] = [
            TERM_GOAL_REACHED,
            TERM_FRONTIER_EXHAUSTED,
            TERM_EXPANSION_BUDGET_EXCEEDED,
            TERM_DEPTH_BUDGET_EXCEEDED,
            TERM_WORLD_CONTRACT_VIOLATION,
            TERM_SCORER_CONTRACT_VIOLATION,
            TERM_INTERNAL_PANIC,
            TERM_FRONTIER_INVARIANT_VIOLATION,
        ];
        for (i, a) in terms.iter().enumerate() {
            assert_eq!(*a, i as u8, "termination tags should be contiguous from 0");
        }
    }

    #[test]
    fn dead_end_tag_roundtrip() {
        assert_eq!(dead_end_to_tag(None), DEAD_END_NONE);
        assert_eq!(
            dead_end_to_tag(Some(DeadEndReasonV1::Exhaustive)),
            DEAD_END_EXHAUSTIVE
        );
        assert_eq!(
            dead_end_to_tag(Some(DeadEndReasonV1::BudgetLimited)),
            DEAD_END_BUDGET_LIMITED
        );
        // Tags are distinct and contiguous from 0
        assert_eq!(DEAD_END_NONE, 0);
        assert_eq!(DEAD_END_EXHAUSTIVE, 1);
        assert_eq!(DEAD_END_BUDGET_LIMITED, 2);
    }

    #[test]
    fn panic_stage_tag_roundtrip() {
        for stage in [
            PanicStageV1::EnumerateCandidates,
            PanicStageV1::ScoreCandidates,
            PanicStageV1::IsGoalRoot,
            PanicStageV1::IsGoalExpansion,
        ] {
            let tag = panic_stage_to_tag(stage);
            assert_eq!(tag_to_panic_stage(tag), Some(stage));
        }
        assert_eq!(tag_to_panic_stage(255), None);
    }

    #[test]
    fn apply_failure_tag_roundtrip() {
        for kind in [
            ApplyFailureKindV1::PreconditionNotMet,
            ApplyFailureKindV1::ArgumentMismatch,
            ApplyFailureKindV1::UnknownOperator,
        ] {
            let tag = apply_failure_to_tag(kind);
            assert_eq!(tag_to_apply_failure(tag), Some(kind));
        }
        assert_eq!(tag_to_apply_failure(255), None);
    }
}
