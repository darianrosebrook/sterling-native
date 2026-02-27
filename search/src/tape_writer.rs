//! `TapeWriter`: streaming binary tape output implementing [`TapeSink`].
//!
//! Writes framed records to an in-memory buffer, maintaining a running
//! hash chain. `on_termination()` writes the Termination record inline;
//! `finish()` writes only the footer and validates that termination was
//! already written exactly once.

use crate::graph::{CandidateOutcomeV1, ExpansionNoteV1, TerminationReasonV1};
use crate::scorer::ScoreSourceV1;
use crate::tape::{
    self, content_hash_to_raw, dead_end_to_tag, frontier_invariant_stage_to_tag, note_to_tag,
    outcome_to_tag, panic_stage_to_tag, raw_hash, raw_hash2, score_source_to_tag,
    termination_to_tag, ExpansionView, NodeCreationView, TapeOutput, TapeSink, TapeWriteError,
    TerminationView, DOMAIN_SEARCH_TAPE, DOMAIN_SEARCH_TAPE_CHAIN, FOOTER_SIZE,
    SEARCH_TAPE_FOOTER_MAGIC, SEARCH_TAPE_MAGIC, SEARCH_TAPE_VERSION,
};

/// Streaming binary tape writer.
///
/// Implements [`TapeSink`] to receive events from the search loop and
/// serialize them into the `.stap` wire format.
pub struct TapeWriter {
    /// Accumulated tape bytes (magic + version + header + records).
    buf: Vec<u8>,
    /// Running hash chain value.
    chain_hash: [u8; 32],
    /// Number of records written so far.
    record_count: u64,
    /// Whether `on_termination` has been called.
    terminated: bool,
    /// Reusable scratch buffer for building record bodies.
    ///
    /// Cleared and reused each record. Preserves transactional atomicity:
    /// if body construction fails, `buf` is untouched.
    scratch: Vec<u8>,
}

impl TapeWriter {
    /// Create a new writer, writing magic + version + header.
    ///
    /// The header is canonical JSON bytes. The chain is seeded as:
    /// `h0 = raw_hash(DOMAIN_SEARCH_TAPE, header_bytes)`.
    #[must_use]
    pub fn new(header_json_bytes: &[u8]) -> Self {
        let header_len = header_json_bytes.len();

        // Pre-allocate: magic(4) + version(2) + header_len(4) + header + estimated records
        let mut buf = Vec::with_capacity(10 + header_len + 4096);

        // Magic
        buf.extend_from_slice(&SEARCH_TAPE_MAGIC);
        // Version
        buf.extend_from_slice(&SEARCH_TAPE_VERSION.to_le_bytes());
        // Header length (u32le)
        #[allow(clippy::cast_possible_truncation)]
        let header_len_u32 = header_len as u32;
        buf.extend_from_slice(&header_len_u32.to_le_bytes());
        // Header bytes
        buf.extend_from_slice(header_json_bytes);

        // Seed the hash chain
        let chain_hash = raw_hash(DOMAIN_SEARCH_TAPE, header_json_bytes);

        Self {
            buf,
            chain_hash,
            record_count: 0,
            terminated: false,
            scratch: Vec::with_capacity(256),
        }
    }

    /// Finalize the tape: write footer, return completed bytes.
    ///
    /// The footer is: `[record_count:u64le][final_chain_hash:32][footer_magic:u32le]`.
    ///
    /// # Errors
    ///
    /// Returns [`TapeWriteError::NotTerminated`] if `on_termination()` was not called.
    pub fn finish(mut self) -> Result<TapeOutput, TapeWriteError> {
        if !self.terminated {
            return Err(TapeWriteError::NotTerminated);
        }

        let final_chain_hash = self.chain_hash;
        let record_count = self.record_count;

        // Reserve footer space
        self.buf.reserve(FOOTER_SIZE);
        // record_count: u64le
        self.buf.extend_from_slice(&record_count.to_le_bytes());
        // final_chain_hash: 32 bytes
        self.buf.extend_from_slice(&final_chain_hash);
        // footer_magic: u32le
        self.buf
            .extend_from_slice(&u32::from_le_bytes(SEARCH_TAPE_FOOTER_MAGIC).to_le_bytes());

        Ok(TapeOutput {
            bytes: self.buf,
            final_chain_hash,
            record_count,
        })
    }

    /// Commit the scratch buffer as a framed record and advance the hash chain.
    ///
    /// Frame format: `[len:u32le][type:u8][body...]`
    /// where `len` includes the type byte + body (NOT the len field itself).
    ///
    /// Transactional: if body construction fails (before calling this),
    /// `self.buf` is untouched because the body was built in `self.scratch`.
    fn commit_record(&mut self, record_type: u8) {
        // Frame length = 1 (type byte) + scratch.len()
        #[allow(clippy::cast_possible_truncation)]
        let frame_len = (1 + self.scratch.len()) as u32;

        let frame_start = self.buf.len();

        // Write frame
        self.buf.extend_from_slice(&frame_len.to_le_bytes());
        self.buf.push(record_type);
        self.buf.extend_from_slice(&self.scratch);

        // Advance hash chain over the complete frame bytes
        let frame_bytes = &self.buf[frame_start..];
        self.chain_hash = raw_hash2(DOMAIN_SEARCH_TAPE_CHAIN, &self.chain_hash, frame_bytes);

        self.record_count += 1;
    }
}

impl TapeSink for TapeWriter {
    fn on_node_created(&mut self, view: &NodeCreationView<'_>) -> Result<(), TapeWriteError> {
        if self.terminated {
            return Err(TapeWriteError::AlreadyTerminated);
        }

        self.scratch.clear();

        // node_id: u64le
        self.scratch.extend_from_slice(&view.node_id.to_le_bytes());

        // parent_id_present: u8, then optional parent_id: u64le
        match view.parent_id {
            None => {
                self.scratch.push(0x00);
            }
            Some(pid) => {
                self.scratch.push(0x01);
                self.scratch.extend_from_slice(&pid.to_le_bytes());
            }
        }

        // state_fingerprint: 32 bytes raw
        self.scratch.extend_from_slice(&view.state_fingerprint_raw);

        // depth: u32le
        self.scratch.extend_from_slice(&view.depth.to_le_bytes());

        // f_cost: i64le
        self.scratch.extend_from_slice(&view.f_cost.to_le_bytes());

        // creation_order: u64le
        self.scratch
            .extend_from_slice(&view.creation_order.to_le_bytes());

        self.commit_record(tape::RECORD_TYPE_NODE_CREATION);
        Ok(())
    }

    fn on_expansion(&mut self, view: &ExpansionView<'_>) -> Result<(), TapeWriteError> {
        if self.terminated {
            return Err(TapeWriteError::AlreadyTerminated);
        }

        let exp = view.expansion;
        self.scratch.clear();

        // expansion_order: u64le
        self.scratch
            .extend_from_slice(&exp.expansion_order.to_le_bytes());

        // node_id: u64le
        self.scratch.extend_from_slice(&exp.node_id.to_le_bytes());

        // state_fingerprint: 32 bytes raw
        self.scratch.extend_from_slice(&view.state_fingerprint_raw);

        // pop_f_cost: i64le
        self.scratch
            .extend_from_slice(&exp.frontier_pop_key.f_cost.to_le_bytes());

        // pop_depth: u32le
        self.scratch
            .extend_from_slice(&exp.frontier_pop_key.depth.to_le_bytes());

        // pop_creation_order: u64le
        self.scratch
            .extend_from_slice(&exp.frontier_pop_key.creation_order.to_le_bytes());

        // candidates_truncated: u8
        self.scratch.push(u8::from(exp.candidates_truncated));

        // dead_end_reason: u8
        self.scratch.push(dead_end_to_tag(exp.dead_end_reason));

        // candidate_count: u32le
        #[allow(clippy::cast_possible_truncation)]
        let candidate_count = exp.candidates.len() as u32;
        self.scratch
            .extend_from_slice(&candidate_count.to_le_bytes());

        // Candidates
        for cand in &exp.candidates {
            write_candidate(&mut self.scratch, cand)?;
        }

        // note_count: u32le
        #[allow(clippy::cast_possible_truncation)]
        let note_count = exp.notes.len() as u32;
        self.scratch.extend_from_slice(&note_count.to_le_bytes());

        // Notes
        for note in &exp.notes {
            write_note(&mut self.scratch, note);
        }

        self.commit_record(tape::RECORD_TYPE_EXPANSION);
        Ok(())
    }

    fn on_termination(&mut self, view: &TerminationView) -> Result<(), TapeWriteError> {
        if self.terminated {
            return Err(TapeWriteError::AlreadyTerminated);
        }

        self.scratch.clear();

        // termination_tag: u8
        self.scratch
            .push(termination_to_tag(&view.termination_reason));

        // termination_payload: variable
        write_termination_payload(&mut self.scratch, &view.termination_reason);

        // frontier_high_water: u64le
        self.scratch
            .extend_from_slice(&view.frontier_high_water.to_le_bytes());

        self.commit_record(tape::RECORD_TYPE_TERMINATION);
        self.terminated = true;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Candidate serialization
// ---------------------------------------------------------------------------

fn write_candidate(
    buf: &mut Vec<u8>,
    cand: &crate::graph::CandidateRecordV1,
) -> Result<(), TapeWriteError> {
    // index: u64le
    buf.extend_from_slice(&cand.index.to_le_bytes());

    // op_code: 4 bytes LE
    buf.extend_from_slice(&cand.action.op_code.to_le_bytes());

    // op_args_len: u16le, op_args: variable
    let args_len = cand.action.op_args.len();
    if args_len > usize::from(u16::MAX) {
        return Err(TapeWriteError::OpArgsTooLong { len: args_len });
    }
    #[allow(clippy::cast_possible_truncation)]
    let args_len_u16 = args_len as u16;
    buf.extend_from_slice(&args_len_u16.to_le_bytes());
    buf.extend_from_slice(&cand.action.op_args);

    // canonical_hash: 32 bytes raw
    let hash_raw = content_hash_to_raw(&cand.action.canonical_hash)?;
    buf.extend_from_slice(&hash_raw);

    // score_bonus: i64le
    buf.extend_from_slice(&cand.score.bonus.to_le_bytes());

    // score_source: u8
    let source_tag = score_source_to_tag(&cand.score.source);
    buf.push(source_tag);

    // model_digest: 32 bytes (only if score_source == ModelDigest)
    if let ScoreSourceV1::ModelDigest(ref digest) = cand.score.source {
        let digest_raw = content_hash_to_raw(digest)?;
        buf.extend_from_slice(&digest_raw);
    }

    // outcome_tag: u8
    buf.push(outcome_to_tag(&cand.outcome));

    // outcome_payload: variable
    write_outcome_payload(buf, &cand.outcome)?;

    Ok(())
}

fn write_outcome_payload(
    buf: &mut Vec<u8>,
    outcome: &CandidateOutcomeV1,
) -> Result<(), TapeWriteError> {
    match outcome {
        CandidateOutcomeV1::Applied { to_node } => {
            buf.extend_from_slice(&to_node.to_le_bytes());
        }
        CandidateOutcomeV1::DuplicateSuppressed {
            existing_fingerprint,
        } => {
            let raw = tape::hex_str_to_raw(existing_fingerprint)?;
            buf.extend_from_slice(&raw);
        }
        CandidateOutcomeV1::IllegalOperator
        | CandidateOutcomeV1::SkippedByDepthLimit
        | CandidateOutcomeV1::SkippedByPolicy
        | CandidateOutcomeV1::NotEvaluated => {
            // Empty payload
        }
        CandidateOutcomeV1::ApplyFailed(kind) => {
            buf.push(tape::apply_failure_to_tag(*kind));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Note serialization
// ---------------------------------------------------------------------------

fn write_note(buf: &mut Vec<u8>, note: &ExpansionNoteV1) {
    buf.push(note_to_tag(note));
    match note {
        ExpansionNoteV1::CandidateCapReached { cap } => {
            buf.extend_from_slice(&cap.to_le_bytes());
        }
        ExpansionNoteV1::FrontierPruned { pruned_node_ids } => {
            #[allow(clippy::cast_possible_truncation)]
            let count = pruned_node_ids.len() as u32;
            buf.extend_from_slice(&count.to_le_bytes());
            for id in pruned_node_ids {
                buf.extend_from_slice(&id.to_le_bytes());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Termination payload serialization
// ---------------------------------------------------------------------------

fn write_termination_payload(buf: &mut Vec<u8>, reason: &TerminationReasonV1) {
    match reason {
        TerminationReasonV1::GoalReached { node_id } => {
            buf.extend_from_slice(&node_id.to_le_bytes());
        }
        TerminationReasonV1::FrontierExhausted
        | TerminationReasonV1::ExpansionBudgetExceeded
        | TerminationReasonV1::DepthBudgetExceeded
        | TerminationReasonV1::WorldContractViolation => {
            // Empty payload
        }
        TerminationReasonV1::ScorerContractViolation { expected, actual } => {
            buf.extend_from_slice(&expected.to_le_bytes());
            buf.extend_from_slice(&actual.to_le_bytes());
        }
        TerminationReasonV1::InternalPanic { stage } => {
            buf.push(panic_stage_to_tag(*stage));
        }
        TerminationReasonV1::FrontierInvariantViolation { stage } => {
            buf.push(frontier_invariant_stage_to_tag(*stage));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{
        CandidateRecordV1, DeadEndReasonV1, ExpandEventV1, ExpansionNoteV1, FrontierPopKeyV1,
        PanicStageV1,
    };
    use crate::node::CandidateActionV1;
    use crate::scorer::{CandidateScoreV1, ScoreSourceV1};
    use sterling_kernel::carrier::bytestate::ByteStateV1;
    use sterling_kernel::carrier::code32::Code32;

    /// Build a simple header JSON for tests.
    fn test_header_bytes() -> Vec<u8> {
        b"{\"schema_version\":\"search_tape.v1\",\"world_id\":\"test\"}".to_vec()
    }

    /// Build a minimal `NodeCreationView` for the root node.
    fn root_node_view(node: &crate::node::SearchNodeV1) -> NodeCreationView<'_> {
        NodeCreationView {
            node_id: node.node_id,
            parent_id: None,
            state_fingerprint_raw: [0xAA; 32],
            depth: 0,
            f_cost: 0,
            creation_order: 0,
            node,
        }
    }

    /// Build a test `SearchNodeV1` (minimal, for view construction).
    fn make_test_node(node_id: u64) -> crate::node::SearchNodeV1 {
        crate::node::SearchNodeV1 {
            node_id,
            parent_id: None,
            state: ByteStateV1::new(1, 2),
            state_fingerprint: sterling_kernel::proof::hash::ContentHash::parse(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
            depth: 0,
            g_cost: 0,
            h_cost: 0,
            creation_order: 0,
            producing_action: None,
        }
    }

    #[test]
    fn write_single_node_creation() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);

        let node = make_test_node(0);
        writer.on_node_created(&root_node_view(&node)).unwrap();

        // Write termination
        writer
            .on_termination(&TerminationView {
                termination_reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 1,
            })
            .unwrap();

        let output = writer.finish().unwrap();

        assert_eq!(output.record_count, 2);
        assert!(!output.bytes.is_empty());

        // Check magic bytes at start
        assert_eq!(&output.bytes[..4], b"STAP");
        // Check version
        assert_eq!(
            u16::from_le_bytes([output.bytes[4], output.bytes[5]]),
            SEARCH_TAPE_VERSION
        );
        // Check footer magic at end
        let len = output.bytes.len();
        assert_eq!(&output.bytes[len - 4..], b"PATS");
    }

    #[test]
    fn write_expansion_with_candidates() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);

        let node = make_test_node(0);
        writer.on_node_created(&root_node_view(&node)).unwrap();

        let expansion = ExpandEventV1 {
            expansion_order: 0,
            node_id: 0,
            state_fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .into(),
            frontier_pop_key: FrontierPopKeyV1 {
                f_cost: 10,
                depth: 0,
                creation_order: 0,
            },
            candidates: vec![CandidateRecordV1 {
                index: 0,
                action: CandidateActionV1::new(Code32::new(2, 1, 3), vec![0, 1]),
                score: CandidateScoreV1 {
                    bonus: 0,
                    source: ScoreSourceV1::Uniform,
                },
                outcome: CandidateOutcomeV1::Applied { to_node: 1 },
            }],
            candidates_truncated: false,
            dead_end_reason: None,
            notes: vec![],
        };

        writer
            .on_expansion(&ExpansionView {
                expansion: &expansion,
                state_fingerprint_raw: [0xAA; 32],
            })
            .unwrap();

        writer
            .on_termination(&TerminationView {
                termination_reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 1,
            })
            .unwrap();

        let output = writer.finish().unwrap();
        assert_eq!(output.record_count, 3); // node + expansion + termination
    }

    #[test]
    fn chain_integrity_across_records() {
        let header = test_header_bytes();

        // Write two tapes with the same events — chain must be identical.
        let mut outputs = Vec::new();
        for _ in 0..2 {
            let mut writer = TapeWriter::new(&header);
            let node = make_test_node(0);
            writer.on_node_created(&root_node_view(&node)).unwrap();
            writer
                .on_termination(&TerminationView {
                    termination_reason: TerminationReasonV1::FrontierExhausted,
                    frontier_high_water: 0,
                })
                .unwrap();
            outputs.push(writer.finish().unwrap());
        }

        assert_eq!(outputs[0].final_chain_hash, outputs[1].final_chain_hash);
        assert_eq!(outputs[0].bytes, outputs[1].bytes);
    }

    #[test]
    fn finish_without_termination_fails() {
        let header = test_header_bytes();
        let writer = TapeWriter::new(&header);
        assert_eq!(writer.finish(), Err(TapeWriteError::NotTerminated));
    }

    #[test]
    fn double_termination_fails() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);
        writer
            .on_termination(&TerminationView {
                termination_reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 0,
            })
            .unwrap();
        let result = writer.on_termination(&TerminationView {
            termination_reason: TerminationReasonV1::FrontierExhausted,
            frontier_high_water: 0,
        });
        assert_eq!(result, Err(TapeWriteError::AlreadyTerminated));
    }

    #[test]
    fn node_after_termination_fails() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);
        writer
            .on_termination(&TerminationView {
                termination_reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 0,
            })
            .unwrap();
        let node = make_test_node(0);
        let result = writer.on_node_created(&root_node_view(&node));
        assert_eq!(result, Err(TapeWriteError::AlreadyTerminated));
    }

    #[test]
    fn expansion_notes_serialized() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);

        let node = make_test_node(0);
        writer.on_node_created(&root_node_view(&node)).unwrap();

        let expansion = ExpandEventV1 {
            expansion_order: 0,
            node_id: 0,
            state_fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .into(),
            frontier_pop_key: FrontierPopKeyV1 {
                f_cost: 0,
                depth: 0,
                creation_order: 0,
            },
            candidates: vec![],
            candidates_truncated: true,
            dead_end_reason: Some(DeadEndReasonV1::Exhaustive),
            notes: vec![
                ExpansionNoteV1::CandidateCapReached { cap: 5 },
                ExpansionNoteV1::FrontierPruned {
                    pruned_node_ids: vec![10, 20, 30],
                },
            ],
        };

        writer
            .on_expansion(&ExpansionView {
                expansion: &expansion,
                state_fingerprint_raw: [0xAA; 32],
            })
            .unwrap();

        writer
            .on_termination(&TerminationView {
                termination_reason: TerminationReasonV1::GoalReached { node_id: 0 },
                frontier_high_water: 5,
            })
            .unwrap();

        let output = writer.finish().unwrap();
        assert_eq!(output.record_count, 3);
    }

    #[test]
    fn all_termination_reasons_writable() {
        let reasons: Vec<TerminationReasonV1> = vec![
            TerminationReasonV1::GoalReached { node_id: 42 },
            TerminationReasonV1::FrontierExhausted,
            TerminationReasonV1::ExpansionBudgetExceeded,
            TerminationReasonV1::DepthBudgetExceeded,
            TerminationReasonV1::WorldContractViolation,
            TerminationReasonV1::ScorerContractViolation {
                expected: 5,
                actual: 3,
            },
            TerminationReasonV1::InternalPanic {
                stage: PanicStageV1::EnumerateCandidates,
            },
            TerminationReasonV1::FrontierInvariantViolation {
                stage: crate::graph::FrontierInvariantStageV1::PopFromNonEmptyFrontier,
            },
        ];

        for reason in reasons {
            let header = test_header_bytes();
            let mut writer = TapeWriter::new(&header);
            writer
                .on_termination(&TerminationView {
                    termination_reason: reason,
                    frontier_high_water: 0,
                })
                .unwrap();
            let output = writer.finish().unwrap();
            assert_eq!(output.record_count, 1);
        }
    }

    #[test]
    fn all_outcome_types_writable() {
        let outcomes: Vec<CandidateOutcomeV1> = vec![
            CandidateOutcomeV1::Applied { to_node: 1 },
            CandidateOutcomeV1::DuplicateSuppressed {
                existing_fingerprint:
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            },
            CandidateOutcomeV1::IllegalOperator,
            CandidateOutcomeV1::ApplyFailed(crate::graph::ApplyFailureKindV1::PreconditionNotMet),
            CandidateOutcomeV1::SkippedByDepthLimit,
            CandidateOutcomeV1::SkippedByPolicy,
            CandidateOutcomeV1::NotEvaluated,
        ];

        for outcome in outcomes {
            let header = test_header_bytes();
            let mut writer = TapeWriter::new(&header);

            let node = make_test_node(0);
            writer.on_node_created(&root_node_view(&node)).unwrap();

            let expansion = ExpandEventV1 {
                expansion_order: 0,
                node_id: 0,
                state_fingerprint:
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                frontier_pop_key: FrontierPopKeyV1 {
                    f_cost: 0,
                    depth: 0,
                    creation_order: 0,
                },
                candidates: vec![CandidateRecordV1 {
                    index: 0,
                    action: CandidateActionV1::new(Code32::new(2, 1, 3), vec![0, 1]),
                    score: CandidateScoreV1 {
                        bonus: 0,
                        source: ScoreSourceV1::Uniform,
                    },
                    outcome: outcome.clone(),
                }],
                candidates_truncated: false,
                dead_end_reason: None,
                notes: vec![],
            };

            writer
                .on_expansion(&ExpansionView {
                    expansion: &expansion,
                    state_fingerprint_raw: [0xAA; 32],
                })
                .unwrap();

            writer
                .on_termination(&TerminationView {
                    termination_reason: TerminationReasonV1::FrontierExhausted,
                    frontier_high_water: 0,
                })
                .unwrap();

            let output = writer.finish().unwrap();
            assert_eq!(output.record_count, 3);
        }
    }

    #[test]
    fn op_args_too_long_rejected() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);

        let node = make_test_node(0);
        writer.on_node_created(&root_node_view(&node)).unwrap();

        // Build a candidate with op_args > u16::MAX
        let big_args = vec![0u8; usize::from(u16::MAX) + 1];
        let expansion = ExpandEventV1 {
            expansion_order: 0,
            node_id: 0,
            state_fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .into(),
            frontier_pop_key: FrontierPopKeyV1 {
                f_cost: 0,
                depth: 0,
                creation_order: 0,
            },
            candidates: vec![CandidateRecordV1 {
                index: 0,
                action: CandidateActionV1::new(Code32::new(2, 1, 3), big_args),
                score: CandidateScoreV1 {
                    bonus: 0,
                    source: ScoreSourceV1::Uniform,
                },
                outcome: CandidateOutcomeV1::Applied { to_node: 1 },
            }],
            candidates_truncated: false,
            dead_end_reason: None,
            notes: vec![],
        };

        let result = writer.on_expansion(&ExpansionView {
            expansion: &expansion,
            state_fingerprint_raw: [0xAA; 32],
        });

        assert!(matches!(result, Err(TapeWriteError::OpArgsTooLong { .. })));
    }
}
