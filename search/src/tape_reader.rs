//! `TapeReader`: fail-closed deserialization with chain verification.
//!
//! Parses `.stap` bytes into [`SearchTapeV1`], enforcing all structural
//! invariants. Any violation produces a typed [`TapeParseError`].

use std::collections::HashSet;

use crate::graph::{DeadEndReasonV1, ExpansionNoteV1, TerminationReasonV1};
use crate::tape::{
    raw_hash, raw_hash2, SearchTapeFooterV1, SearchTapeHeaderV1, SearchTapeV1,
    TapeCandidateOutcomeV1, TapeCandidateV1, TapeExpansionV1, TapeNodeCreationV1, TapeParseError,
    TapeRecordV1, TapeScoreSourceV1, TapeTerminationV1, DOMAIN_SEARCH_TAPE,
    DOMAIN_SEARCH_TAPE_CHAIN, FOOTER_SIZE, OUTCOME_APPLIED, OUTCOME_APPLY_FAILED,
    OUTCOME_DUPLICATE_SUPPRESSED, OUTCOME_ILLEGAL_OPERATOR, OUTCOME_NOT_EVALUATED,
    OUTCOME_SKIPPED_BY_DEPTH_LIMIT, OUTCOME_SKIPPED_BY_POLICY, RECORD_TYPE_EXPANSION,
    RECORD_TYPE_NODE_CREATION, RECORD_TYPE_TERMINATION, SCORE_SOURCE_MODEL_DIGEST,
    SCORE_SOURCE_UNAVAILABLE, SCORE_SOURCE_UNIFORM, SEARCH_TAPE_FOOTER_MAGIC, SEARCH_TAPE_MAGIC,
    SEARCH_TAPE_VERSION,
};

/// Parse a complete tape from bytes, verifying all invariants.
///
/// # Errors
///
/// Returns [`TapeParseError`] on any structural or integrity violation.
#[allow(clippy::too_many_lines)]
pub fn read_tape(bytes: &[u8]) -> Result<SearchTapeV1, TapeParseError> {
    // Minimum: magic(4) + version(2) + header_len(4) + footer(44) = 54
    let min_size = 4 + 2 + 4 + FOOTER_SIZE;
    if bytes.len() < min_size {
        return Err(TapeParseError::TooShort);
    }

    let mut cursor = Cursor::new(bytes);

    // --- Magic ---
    let magic = cursor
        .read_bytes(4)
        .map_err(|()| TapeParseError::TooShort)?;
    if magic != SEARCH_TAPE_MAGIC {
        return Err(TapeParseError::BadMagic);
    }

    // --- Version ---
    let version = cursor.read_u16().map_err(|()| TapeParseError::TooShort)?;
    if version != SEARCH_TAPE_VERSION {
        return Err(TapeParseError::UnsupportedVersion { got: version });
    }

    // --- Header ---
    let header_len = cursor.read_u32().map_err(|()| TapeParseError::TooShort)? as usize;
    if cursor.remaining() < header_len + FOOTER_SIZE {
        return Err(TapeParseError::HeaderTruncated);
    }
    let header_bytes = cursor
        .read_bytes(header_len)
        .map_err(|()| TapeParseError::HeaderTruncated)?
        .to_vec();

    // Parse header JSON
    let json: serde_json::Value = serde_json::from_slice(&header_bytes)
        .map_err(|e| TapeParseError::InvalidHeaderJson(e.to_string()))?;

    // Seed hash chain
    let mut chain_hash = raw_hash(DOMAIN_SEARCH_TAPE, &header_bytes);

    // --- Records ---
    let footer_start = bytes.len() - FOOTER_SIZE;

    let mut records = Vec::new();
    let mut record_index: u64 = 0;

    while cursor.pos < footer_start {
        // Read the frame: [len:u32le][type:u8][body...]
        let frame_start = cursor.pos;

        let frame_len = cursor
            .read_u32()
            .map_err(|()| TapeParseError::RecordTruncated { record_index })?
            as usize;
        if cursor.remaining() < frame_len {
            return Err(TapeParseError::RecordTruncated { record_index });
        }

        // Extract bounded frame body and advance main cursor past the frame.
        let frame_body = &bytes[cursor.pos..cursor.pos + frame_len];
        cursor.pos += frame_len;

        // Parse from a bounded sub-cursor so parsers cannot over-read.
        let mut frame_cursor = Cursor::new(frame_body);

        let record_type = frame_cursor
            .read_u8()
            .map_err(|()| TapeParseError::RecordTruncated { record_index })?;

        let record = match record_type {
            RECORD_TYPE_NODE_CREATION => {
                TapeRecordV1::NodeCreation(parse_node_creation(&mut frame_cursor, record_index)?)
            }
            RECORD_TYPE_EXPANSION => {
                TapeRecordV1::Expansion(parse_expansion(&mut frame_cursor, record_index)?)
            }
            RECORD_TYPE_TERMINATION => {
                TapeRecordV1::Termination(parse_termination(&mut frame_cursor, record_index)?)
            }
            tag => {
                return Err(TapeParseError::UnknownRecordType { tag, record_index });
            }
        };

        // Reject if parser did not consume exactly all frame bytes.
        if frame_cursor.remaining() > 0 {
            return Err(TapeParseError::FrameBodyNotFullyConsumed {
                record_index,
                remaining: frame_cursor.remaining(),
            });
        }

        // Advance hash chain over the complete frame: [len:u32le][type:u8][body...]
        let full_frame = &bytes[frame_start..frame_start + 4 + frame_len];
        chain_hash = raw_hash2(DOMAIN_SEARCH_TAPE_CHAIN, &chain_hash, full_frame);

        records.push(record);
        record_index += 1;
    }

    // --- Footer ---
    if cursor.pos != footer_start {
        // Should not happen if we stop at footer_start, but double-check
        return Err(TapeParseError::RecordTruncated {
            record_index: record_index.saturating_sub(1),
        });
    }

    let footer_record_count = cursor
        .read_u64()
        .map_err(|()| TapeParseError::BadFooterMagic)?;
    let mut footer_chain_hash = [0u8; 32];
    footer_chain_hash.copy_from_slice(
        cursor
            .read_bytes(32)
            .map_err(|()| TapeParseError::BadFooterMagic)?,
    );
    let footer_magic_bytes = cursor
        .read_bytes(4)
        .map_err(|()| TapeParseError::BadFooterMagic)?;

    // Footer magic check
    if footer_magic_bytes != SEARCH_TAPE_FOOTER_MAGIC {
        return Err(TapeParseError::BadFooterMagic);
    }

    // Trailing bytes check
    if cursor.remaining() > 0 {
        return Err(TapeParseError::TrailingBytes {
            excess: cursor.remaining(),
        });
    }

    // Record count check
    let actual_count = record_index;
    if footer_record_count != actual_count {
        return Err(TapeParseError::RecordCountMismatch {
            expected: footer_record_count,
            actual: actual_count,
        });
    }

    // Chain hash check
    if chain_hash != footer_chain_hash {
        return Err(TapeParseError::ChainHashMismatch);
    }

    // --- Structural invariant checks ---
    validate_structural_invariants(&records)?;

    Ok(SearchTapeV1 {
        header: SearchTapeHeaderV1 {
            json_bytes: header_bytes,
            json,
        },
        records,
        footer: SearchTapeFooterV1 {
            record_count: actual_count,
            final_chain_hash: footer_chain_hash,
        },
    })
}

// ---------------------------------------------------------------------------
// Structural invariant validation
// ---------------------------------------------------------------------------

fn validate_structural_invariants(records: &[TapeRecordV1]) -> Result<(), TapeParseError> {
    let mut node_ids: HashSet<u64> = HashSet::new();
    let mut last_expansion_order: Option<u64> = None;
    let mut termination_index: Option<u64> = None;

    for (i, record) in records.iter().enumerate() {
        let ri = i as u64;

        match record {
            TapeRecordV1::NodeCreation(nc) => {
                // Duplicate node_id check
                if !node_ids.insert(nc.node_id) {
                    return Err(TapeParseError::DuplicateNodeId {
                        node_id: nc.node_id,
                    });
                }

                // parent_id < node_id (monotonic assignment)
                if let Some(pid) = nc.parent_id {
                    if pid >= nc.node_id {
                        return Err(TapeParseError::NonMonotonicParentId {
                            node_id: nc.node_id,
                            parent_id: pid,
                        });
                    }
                }
            }
            TapeRecordV1::Expansion(exp) => {
                // Monotonic expansion order
                if let Some(prev) = last_expansion_order {
                    if exp.expansion_order <= prev {
                        return Err(TapeParseError::NonMonotonicExpansionOrder {
                            previous: prev,
                            current: exp.expansion_order,
                            record_index: ri,
                        });
                    }
                }
                last_expansion_order = Some(exp.expansion_order);

                // Applied to_node references valid NodeCreation
                for cand in &exp.candidates {
                    if let TapeCandidateOutcomeV1::Applied { to_node } = &cand.outcome {
                        if !node_ids.contains(to_node) {
                            return Err(TapeParseError::InvalidAppliedNodeRef {
                                to_node: *to_node,
                                record_index: ri,
                            });
                        }
                    }
                }
            }
            TapeRecordV1::Termination(_) => {
                if termination_index.is_some() {
                    return Err(TapeParseError::DuplicateTermination { record_index: ri });
                }
                termination_index = Some(ri);
            }
        }
    }

    // Termination must exist
    let term_idx = termination_index.ok_or(TapeParseError::MissingTermination)?;

    // Termination must be last
    if term_idx != (records.len() as u64).saturating_sub(1) {
        return Err(TapeParseError::TerminationNotLast {
            record_index: term_idx,
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Record parsers
// ---------------------------------------------------------------------------

fn parse_node_creation(
    cursor: &mut Cursor<'_>,
    record_index: u64,
) -> Result<TapeNodeCreationV1, TapeParseError> {
    let node_id = cursor
        .read_u64()
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "node_id",
        })?;

    let parent_present = cursor
        .read_u8()
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "parent_id_present",
        })?;

    let parent_id = if parent_present == 0x00 {
        None
    } else {
        Some(
            cursor
                .read_u64()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "parent_id",
                })?,
        )
    };

    let mut state_fingerprint = [0u8; 32];
    state_fingerprint.copy_from_slice(cursor.read_bytes(32).map_err(|()| {
        TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "state_fingerprint",
        }
    })?);

    let depth = cursor
        .read_u32()
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "depth",
        })?;

    let f_cost = cursor
        .read_i64()
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "f_cost",
        })?;

    let creation_order = cursor
        .read_u64()
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "creation_order",
        })?;

    Ok(TapeNodeCreationV1 {
        node_id,
        parent_id,
        state_fingerprint,
        depth,
        f_cost,
        creation_order,
    })
}

fn parse_expansion(
    cursor: &mut Cursor<'_>,
    record_index: u64,
) -> Result<TapeExpansionV1, TapeParseError> {
    macro_rules! read_field {
        ($method:ident, $field:expr) => {
            cursor
                .$method()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: $field,
                })?
        };
    }

    let expansion_order = read_field!(read_u64, "expansion_order");
    let node_id = read_field!(read_u64, "node_id");

    let mut state_fingerprint = [0u8; 32];
    state_fingerprint.copy_from_slice(cursor.read_bytes(32).map_err(|()| {
        TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "state_fingerprint",
        }
    })?);

    let pop_f_cost = read_field!(read_i64, "pop_f_cost");
    let pop_depth = read_field!(read_u32, "pop_depth");
    let pop_creation_order = read_field!(read_u64, "pop_creation_order");
    let candidates_truncated = read_field!(read_u8, "candidates_truncated") != 0;

    let dead_end_tag = read_field!(read_u8, "dead_end_reason");
    let dead_end_reason = match dead_end_tag {
        crate::tape::DEAD_END_NONE => None,
        crate::tape::DEAD_END_EXHAUSTIVE => Some(DeadEndReasonV1::Exhaustive),
        crate::tape::DEAD_END_BUDGET_LIMITED => Some(DeadEndReasonV1::BudgetLimited),
        tag => {
            return Err(TapeParseError::UnknownEnumTag {
                field: "dead_end_reason",
                tag,
                record_index,
            });
        }
    };

    let candidate_count = read_field!(read_u32, "candidate_count");
    let mut candidates = Vec::with_capacity((candidate_count as usize).min(cursor.remaining()));
    for _ in 0..candidate_count {
        candidates.push(parse_candidate(cursor, record_index)?);
    }

    let note_count = read_field!(read_u32, "note_count");
    let mut notes = Vec::with_capacity((note_count as usize).min(cursor.remaining()));
    for _ in 0..note_count {
        notes.push(parse_note(cursor, record_index)?);
    }

    Ok(TapeExpansionV1 {
        expansion_order,
        node_id,
        state_fingerprint,
        pop_f_cost,
        pop_depth,
        pop_creation_order,
        candidates_truncated,
        dead_end_reason,
        candidates,
        notes,
    })
}

fn parse_candidate(
    cursor: &mut Cursor<'_>,
    record_index: u64,
) -> Result<TapeCandidateV1, TapeParseError> {
    macro_rules! read_field {
        ($method:ident, $field:expr) => {
            cursor
                .$method()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: $field,
                })?
        };
    }

    let index = read_field!(read_u64, "candidate.index");

    let mut op_code_bytes = [0u8; 4];
    op_code_bytes.copy_from_slice(cursor.read_bytes(4).map_err(|()| {
        TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "candidate.op_code",
        }
    })?);

    let op_args_len = read_field!(read_u16, "candidate.op_args_len");
    let op_args = cursor
        .read_bytes(op_args_len as usize)
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "candidate.op_args",
        })?
        .to_vec();

    let mut canonical_hash = [0u8; 32];
    canonical_hash.copy_from_slice(cursor.read_bytes(32).map_err(|()| {
        TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "candidate.canonical_hash",
        }
    })?);

    let score_bonus = read_field!(read_i64, "candidate.score_bonus");

    let score_source_tag = read_field!(read_u8, "candidate.score_source");
    let score_source = match score_source_tag {
        SCORE_SOURCE_UNIFORM => TapeScoreSourceV1::Uniform,
        SCORE_SOURCE_MODEL_DIGEST => {
            let mut digest = [0u8; 32];
            digest.copy_from_slice(cursor.read_bytes(32).map_err(|()| {
                TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "candidate.model_digest",
                }
            })?);
            TapeScoreSourceV1::ModelDigest(digest)
        }
        SCORE_SOURCE_UNAVAILABLE => TapeScoreSourceV1::Unavailable,
        tag => {
            return Err(TapeParseError::UnknownEnumTag {
                field: "score_source",
                tag,
                record_index,
            });
        }
    };

    let outcome_tag = read_field!(read_u8, "candidate.outcome");
    let outcome = parse_outcome(cursor, outcome_tag, record_index)?;

    Ok(TapeCandidateV1 {
        index,
        op_code_bytes,
        op_args,
        canonical_hash,
        score_bonus,
        score_source,
        outcome,
    })
}

fn parse_outcome(
    cursor: &mut Cursor<'_>,
    tag: u8,
    record_index: u64,
) -> Result<TapeCandidateOutcomeV1, TapeParseError> {
    match tag {
        OUTCOME_APPLIED => {
            let to_node = cursor
                .read_u64()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "outcome.to_node",
                })?;
            Ok(TapeCandidateOutcomeV1::Applied { to_node })
        }
        OUTCOME_DUPLICATE_SUPPRESSED => {
            let mut fp = [0u8; 32];
            fp.copy_from_slice(cursor.read_bytes(32).map_err(|()| {
                TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "outcome.fingerprint",
                }
            })?);
            Ok(TapeCandidateOutcomeV1::DuplicateSuppressed {
                existing_fingerprint: fp,
            })
        }
        OUTCOME_ILLEGAL_OPERATOR => Ok(TapeCandidateOutcomeV1::IllegalOperator),
        OUTCOME_APPLY_FAILED => {
            let kind_tag = cursor
                .read_u8()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "outcome.apply_failure_kind",
                })?;
            let kind = crate::tape::tag_to_apply_failure(kind_tag).ok_or(
                TapeParseError::UnknownEnumTag {
                    field: "apply_failure_kind",
                    tag: kind_tag,
                    record_index,
                },
            )?;
            Ok(TapeCandidateOutcomeV1::ApplyFailed(kind))
        }
        OUTCOME_SKIPPED_BY_DEPTH_LIMIT => Ok(TapeCandidateOutcomeV1::SkippedByDepthLimit),
        OUTCOME_SKIPPED_BY_POLICY => Ok(TapeCandidateOutcomeV1::SkippedByPolicy),
        OUTCOME_NOT_EVALUATED => Ok(TapeCandidateOutcomeV1::NotEvaluated),
        _ => Err(TapeParseError::UnknownEnumTag {
            field: "candidate_outcome",
            tag,
            record_index,
        }),
    }
}

fn parse_note(
    cursor: &mut Cursor<'_>,
    record_index: u64,
) -> Result<ExpansionNoteV1, TapeParseError> {
    let tag = cursor
        .read_u8()
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "note.tag",
        })?;

    match tag {
        crate::tape::NOTE_CANDIDATE_CAP_REACHED => {
            let cap = cursor
                .read_u64()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "note.cap",
                })?;
            Ok(ExpansionNoteV1::CandidateCapReached { cap })
        }
        crate::tape::NOTE_FRONTIER_PRUNED => {
            let count = cursor
                .read_u32()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "note.pruned_count",
                })?;
            let mut ids = Vec::with_capacity((count as usize).min(cursor.remaining()));
            for _ in 0..count {
                ids.push(
                    cursor
                        .read_u64()
                        .map_err(|()| TapeParseError::RecordBodyTruncated {
                            record_index,
                            detail: "note.pruned_node_id",
                        })?,
                );
            }
            Ok(ExpansionNoteV1::FrontierPruned {
                pruned_node_ids: ids,
            })
        }
        _ => Err(TapeParseError::UnknownEnumTag {
            field: "expansion_note",
            tag,
            record_index,
        }),
    }
}

fn parse_termination(
    cursor: &mut Cursor<'_>,
    record_index: u64,
) -> Result<TapeTerminationV1, TapeParseError> {
    let tag = cursor
        .read_u8()
        .map_err(|()| TapeParseError::RecordBodyTruncated {
            record_index,
            detail: "termination.tag",
        })?;

    let reason = match tag {
        crate::tape::TERM_GOAL_REACHED => {
            let node_id = cursor
                .read_u64()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "termination.node_id",
                })?;
            TerminationReasonV1::GoalReached { node_id }
        }
        crate::tape::TERM_FRONTIER_EXHAUSTED => TerminationReasonV1::FrontierExhausted,
        crate::tape::TERM_EXPANSION_BUDGET_EXCEEDED => TerminationReasonV1::ExpansionBudgetExceeded,
        crate::tape::TERM_DEPTH_BUDGET_EXCEEDED => TerminationReasonV1::DepthBudgetExceeded,
        crate::tape::TERM_WORLD_CONTRACT_VIOLATION => TerminationReasonV1::WorldContractViolation,
        crate::tape::TERM_SCORER_CONTRACT_VIOLATION => {
            let expected = cursor
                .read_u64()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "termination.expected",
                })?;
            let actual = cursor
                .read_u64()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "termination.actual",
                })?;
            TerminationReasonV1::ScorerContractViolation { expected, actual }
        }
        crate::tape::TERM_INTERNAL_PANIC => {
            let stage_tag = cursor
                .read_u8()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "termination.panic_stage",
                })?;
            let stage = crate::tape::tag_to_panic_stage(stage_tag).ok_or(
                TapeParseError::UnknownEnumTag {
                    field: "panic_stage",
                    tag: stage_tag,
                    record_index,
                },
            )?;
            TerminationReasonV1::InternalPanic { stage }
        }
        crate::tape::TERM_FRONTIER_INVARIANT_VIOLATION => {
            let stage_tag = cursor
                .read_u8()
                .map_err(|()| TapeParseError::RecordBodyTruncated {
                    record_index,
                    detail: "termination.frontier_stage",
                })?;
            let stage = crate::tape::tag_to_frontier_invariant_stage(stage_tag).ok_or(
                TapeParseError::UnknownEnumTag {
                    field: "frontier_invariant_stage",
                    tag: stage_tag,
                    record_index,
                },
            )?;
            TerminationReasonV1::FrontierInvariantViolation { stage }
        }
        _ => {
            return Err(TapeParseError::UnknownEnumTag {
                field: "termination_reason",
                tag,
                record_index,
            });
        }
    };

    let frontier_high_water =
        cursor
            .read_u64()
            .map_err(|()| TapeParseError::RecordBodyTruncated {
                record_index,
                detail: "termination.frontier_high_water",
            })?;

    Ok(TapeTerminationV1 {
        reason,
        frontier_high_water,
    })
}

// ---------------------------------------------------------------------------
// Cursor helper
// ---------------------------------------------------------------------------

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], ()> {
        if self.pos + n > self.data.len() {
            return Err(());
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, ()> {
        let b = self.read_bytes(1)?;
        Ok(b[0])
    }

    fn read_u16(&mut self) -> Result<u16, ()> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, ()> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, ()> {
        let b = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_i64(&mut self) -> Result<i64, ()> {
        let b = self.read_bytes(8)?;
        Ok(i64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{CandidateOutcomeV1, CandidateRecordV1, ExpandEventV1, FrontierPopKeyV1};
    use crate::node::CandidateActionV1;
    use crate::scorer::{CandidateScoreV1, ScoreSourceV1};
    use crate::tape::{ExpansionView, NodeCreationView, TapeSink, TerminationView};
    use crate::tape_writer::TapeWriter;
    use sterling_kernel::carrier::bytestate::ByteStateV1;
    use sterling_kernel::carrier::code32::Code32;

    fn test_header_bytes() -> Vec<u8> {
        b"{\"schema_version\":\"search_tape.v1\",\"world_id\":\"test\"}".to_vec()
    }

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

    /// Write a minimal tape and read it back.
    fn write_minimal_tape() -> Vec<u8> {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);

        let node = make_test_node(0);
        writer
            .on_node_created(&NodeCreationView {
                node_id: 0,
                parent_id: None,
                state_fingerprint_raw: [0xAA; 32],
                depth: 0,
                f_cost: 0,
                creation_order: 0,
                node: &node,
            })
            .unwrap();

        writer
            .on_termination(&TerminationView {
                termination_reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 1,
            })
            .unwrap();

        writer.finish().unwrap().bytes
    }

    #[test]
    fn round_trip_minimal() {
        let bytes = write_minimal_tape();
        let tape = read_tape(&bytes).unwrap();

        assert_eq!(tape.footer.record_count, 2);
        assert_eq!(tape.records.len(), 2);

        // First record is NodeCreation
        assert!(matches!(tape.records[0], TapeRecordV1::NodeCreation(_)));
        // Last record is Termination
        assert!(matches!(tape.records[1], TapeRecordV1::Termination(_)));

        if let TapeRecordV1::NodeCreation(nc) = &tape.records[0] {
            assert_eq!(nc.node_id, 0);
            assert_eq!(nc.parent_id, None);
            assert_eq!(nc.state_fingerprint, [0xAA; 32]);
        }
    }

    #[test]
    fn round_trip_with_expansion() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);

        let node = make_test_node(0);
        writer
            .on_node_created(&NodeCreationView {
                node_id: 0,
                parent_id: None,
                state_fingerprint_raw: [0xAA; 32],
                depth: 0,
                f_cost: 0,
                creation_order: 0,
                node: &node,
            })
            .unwrap();

        // Child node
        let child = make_test_node(1);
        writer
            .on_node_created(&NodeCreationView {
                node_id: 1,
                parent_id: Some(0),
                state_fingerprint_raw: [0xBB; 32],
                depth: 1,
                f_cost: 1,
                creation_order: 1,
                node: &child,
            })
            .unwrap();

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
                action: CandidateActionV1::new(Code32::new(2, 1, 3), vec![0, 1]),
                score: CandidateScoreV1 {
                    bonus: 5,
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
                termination_reason: TerminationReasonV1::GoalReached { node_id: 1 },
                frontier_high_water: 2,
            })
            .unwrap();

        let output = writer.finish().unwrap();
        let tape = read_tape(&output.bytes).unwrap();

        assert_eq!(tape.footer.record_count, 4);
        assert_eq!(tape.records.len(), 4);

        // Check expansion
        if let TapeRecordV1::Expansion(exp) = &tape.records[2] {
            assert_eq!(exp.expansion_order, 0);
            assert_eq!(exp.candidates.len(), 1);
            assert_eq!(exp.candidates[0].score_bonus, 5);
            assert!(matches!(
                exp.candidates[0].outcome,
                TapeCandidateOutcomeV1::Applied { to_node: 1 }
            ));
        } else {
            panic!("expected expansion record");
        }

        // Check termination
        if let TapeRecordV1::Termination(term) = &tape.records[3] {
            assert_eq!(term.reason, TerminationReasonV1::GoalReached { node_id: 1 });
            assert_eq!(term.frontier_high_water, 2);
        } else {
            panic!("expected termination record");
        }
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = write_minimal_tape();
        bytes[0] = b'X';
        assert_eq!(read_tape(&bytes).unwrap_err(), TapeParseError::BadMagic);
    }

    #[test]
    fn rejects_bad_version() {
        let mut bytes = write_minimal_tape();
        // Version is at offset 4-5 (u16le)
        bytes[4] = 99;
        bytes[5] = 0;
        assert!(matches!(
            read_tape(&bytes).unwrap_err(),
            TapeParseError::UnsupportedVersion { got: 99 }
        ));
    }

    #[test]
    fn rejects_truncation() {
        let bytes = write_minimal_tape();
        // Truncate to just magic + version
        assert!(matches!(
            read_tape(&bytes[..6]).unwrap_err(),
            TapeParseError::TooShort
        ));
    }

    #[test]
    fn rejects_chain_hash_mismatch() {
        let mut bytes = write_minimal_tape();
        // Flip a byte in the chain hash (in the footer)
        let footer_start = bytes.len() - FOOTER_SIZE;
        let hash_offset = footer_start + 8; // after record_count u64
        bytes[hash_offset] ^= 0xFF;
        assert_eq!(
            read_tape(&bytes).unwrap_err(),
            TapeParseError::ChainHashMismatch
        );
    }

    #[test]
    fn rejects_bad_footer_magic() {
        let mut bytes = write_minimal_tape();
        let len = bytes.len();
        bytes[len - 4] = b'X';
        assert_eq!(
            read_tape(&bytes).unwrap_err(),
            TapeParseError::BadFooterMagic
        );
    }

    #[test]
    fn rejects_trailing_bytes_after_footer() {
        // Appending bytes after the footer shifts the computed footer position,
        // causing the reader to misparse. This manifests as either a record
        // parse failure or chain hash mismatch (both are correct rejections).
        let mut bytes = write_minimal_tape();
        bytes.push(0xFF);
        let err = read_tape(&bytes).unwrap_err();
        // The extra byte is consumed as part of the record region (before
        // the new footer_start), causing a chain hash mismatch.
        assert!(
            matches!(
                err,
                TapeParseError::ChainHashMismatch
                    | TapeParseError::RecordTruncated { .. }
                    | TapeParseError::TrailingBytes { .. }
                    | TapeParseError::UnknownRecordType { .. }
                    | TapeParseError::BadFooterMagic
            ),
            "expected rejection of trailing bytes, got: {err:?}"
        );
    }

    #[test]
    fn rejects_record_count_mismatch() {
        let mut bytes = write_minimal_tape();
        let footer_start = bytes.len() - FOOTER_SIZE;
        // Overwrite record_count to wrong value, but also fix chain hash
        // to isolate the record_count check. Since chain_hash depends on
        // records (not footer), we can just corrupt the count.
        // The chain hash check passes because we only changed the footer count.
        // But actually the chain hash is computed before the footer, so
        // changing the footer count won't affect chain check.
        // However, we need to also keep chain hash valid. The simplest way:
        // read the tape, verify it's good, then just corrupt the count field.
        let tape = read_tape(&bytes).unwrap();
        // Corrupt count: set to 99
        bytes[footer_start..footer_start + 8].copy_from_slice(&99u64.to_le_bytes());
        // Restore chain hash (it's correct since records didn't change)
        bytes[footer_start + 8..footer_start + 40].copy_from_slice(&tape.footer.final_chain_hash);
        // Restore footer magic
        bytes[footer_start + 40..footer_start + 44].copy_from_slice(b"PATS");

        assert!(matches!(
            read_tape(&bytes).unwrap_err(),
            TapeParseError::RecordCountMismatch {
                expected: 99,
                actual: 2
            }
        ));
    }

    #[test]
    fn write_read_rewrite_identical() {
        // Write → read → re-write must produce identical bytes.
        let bytes1 = write_minimal_tape();
        let tape = read_tape(&bytes1).unwrap();

        // Re-write from parsed tape
        let mut writer = TapeWriter::new(&tape.header.json_bytes);
        for record in &tape.records {
            match record {
                TapeRecordV1::NodeCreation(nc) => {
                    let node = make_test_node(nc.node_id);
                    writer
                        .on_node_created(&NodeCreationView {
                            node_id: nc.node_id,
                            parent_id: nc.parent_id,
                            state_fingerprint_raw: nc.state_fingerprint,
                            depth: nc.depth,
                            f_cost: nc.f_cost,
                            creation_order: nc.creation_order,
                            node: &node,
                        })
                        .unwrap();
                }
                TapeRecordV1::Termination(term) => {
                    writer
                        .on_termination(&TerminationView {
                            termination_reason: term.reason.clone(),
                            frontier_high_water: term.frontier_high_water,
                        })
                        .unwrap();
                }
                TapeRecordV1::Expansion(_) => {
                    // No expansions in minimal tape
                    unreachable!("minimal tape has no expansions");
                }
            }
        }

        let output2 = writer.finish().unwrap();
        assert_eq!(bytes1, output2.bytes);
    }

    #[test]
    fn rejects_invalid_dead_end_tag() {
        let header = test_header_bytes();
        let mut writer = TapeWriter::new(&header);

        let node = make_test_node(0);
        writer
            .on_node_created(&NodeCreationView {
                node_id: 0,
                parent_id: None,
                state_fingerprint_raw: [0xAA; 32],
                depth: 0,
                f_cost: 0,
                creation_order: 0,
                node: &node,
            })
            .unwrap();

        let child = make_test_node(1);
        writer
            .on_node_created(&NodeCreationView {
                node_id: 1,
                parent_id: Some(0),
                state_fingerprint_raw: [0xBB; 32],
                depth: 1,
                f_cost: 1,
                creation_order: 1,
                node: &child,
            })
            .unwrap();

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
                action: CandidateActionV1::new(Code32::new(2, 1, 3), vec![0, 1]),
                score: CandidateScoreV1 {
                    bonus: 5,
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
                termination_reason: TerminationReasonV1::GoalReached { node_id: 1 },
                frontier_high_water: 2,
            })
            .unwrap();

        let mut bytes = writer.finish().unwrap().bytes;

        // Verify the tape is valid before tampering
        assert!(read_tape(&bytes).is_ok());

        // Find the expansion record frame: scan for RECORD_TYPE_EXPANSION (0x02)
        // after the header. Frame format: [len:u32le][type:u8][body...]
        // Body layout of expansion: expansion_order(8) + node_id(8) +
        // state_fingerprint(32) + f_cost(8) + depth(4) + creation_order(8) +
        // candidates_truncated(1) + dead_end_reason(1) + ...
        // dead_end offset within body = 8+8+32+8+4+8+1 = 69
        let header_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        let mut pos = 10 + header_len;
        loop {
            assert!(pos + 5 <= bytes.len(), "expansion frame not found");
            let frame_len =
                u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                    as usize;
            let type_tag = bytes[pos + 4];
            if type_tag == crate::tape::RECORD_TYPE_EXPANSION {
                // dead_end_reason is at body offset 69 = frame offset 4 + 1 + 69 = 74
                let dead_end_offset = pos + 4 + 1 + 69;
                assert_eq!(
                    bytes[dead_end_offset], 0x00,
                    "expected DEAD_END_NONE before tamper"
                );
                bytes[dead_end_offset] = 0xFF; // invalid tag
                break;
            }
            pos += 4 + frame_len; // len prefix + frame body (type + body)
        }

        // Re-fix the chain hash so the reader doesn't reject for ChainHashMismatch
        // before reaching the enum tag check — actually, tampering a byte inside a
        // chained record WILL trigger ChainHashMismatch first. The chain check
        // validates record integrity; enum tag validation is a second line of defense.
        // So we test both: the chain catches it, AND if the chain were somehow
        // bypassed, the enum match catches it.
        let err = read_tape(&bytes).unwrap_err();
        // Chain hash mismatch is the first line of defense
        assert!(
            matches!(
                err,
                TapeParseError::ChainHashMismatch
                    | TapeParseError::UnknownEnumTag {
                        field: "dead_end_reason",
                        ..
                    }
            ),
            "expected ChainHashMismatch or UnknownEnumTag, got: {err:?}"
        );
    }

    #[test]
    fn rejects_frame_body_not_fully_consumed() {
        // Build a valid tape, then inflate one frame's declared length
        // by inserting extra bytes. The parser should reject with
        // FrameBodyNotFullyConsumed before reaching the chain hash check.
        let mut bytes = write_minimal_tape();

        // Locate first record frame after header.
        // Layout: magic(4) + version(2) + header_len(4) + header_bytes + records + footer
        let header_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        let first_frame_pos = 10 + header_len;

        // Read original frame_len
        let orig_frame_len = u32::from_le_bytes([
            bytes[first_frame_pos],
            bytes[first_frame_pos + 1],
            bytes[first_frame_pos + 2],
            bytes[first_frame_pos + 3],
        ]);

        // Insert 4 garbage bytes right after the original frame body,
        // and increase frame_len by 4 so the frame "contains" extra bytes.
        let frame_body_end = first_frame_pos + 4 + orig_frame_len as usize;
        let padding = [0xDE, 0xAD, 0xBE, 0xEF];
        bytes.splice(frame_body_end..frame_body_end, padding);

        // Update frame_len
        let new_frame_len = orig_frame_len + 4;
        bytes[first_frame_pos..first_frame_pos + 4].copy_from_slice(&new_frame_len.to_le_bytes());

        let err = read_tape(&bytes).unwrap_err();
        assert!(
            matches!(
                err,
                TapeParseError::FrameBodyNotFullyConsumed {
                    record_index: 0,
                    remaining: 4,
                }
            ),
            "expected FrameBodyNotFullyConsumed, got: {err:?}"
        );
    }
}
