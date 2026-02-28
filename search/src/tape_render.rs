//! Tape → `SearchGraphV1` renderer.
//!
//! Converts a parsed [`SearchTapeV1`] into a [`SearchGraphV1`] that must
//! produce byte-identical `to_canonical_json_bytes()` output as the existing
//! `build_graph()` path. This is the T1 equivalence gate.

use std::collections::HashMap;

use sterling_kernel::carrier::code32::Code32;

use crate::graph::{
    CandidateOutcomeV1, CandidateRecordV1, DeadEndReasonV1, ExpandEventV1, ExpansionNoteV1,
    FrontierPopKeyV1, SearchGraphMetadata, SearchGraphNodeSummaryV1, SearchGraphV1,
    TerminationReasonV1,
};
use crate::node::CandidateActionV1;
use crate::policy::{DedupKeyV1, PruneVisitedPolicyV1};
use crate::scorer::{CandidateScoreV1, ScoreSourceV1};
use crate::tape::{
    raw_to_content_hash, raw_to_hex_str, SearchTapeV1, TapeCandidateOutcomeV1, TapeCandidateV1,
    TapeRecordV1, TapeRenderError, TapeScoreSourceV1,
};

/// Render a parsed tape into a `SearchGraphV1`.
///
/// The resulting graph's `to_canonical_json_bytes()` must be byte-identical
/// to the `SearchGraphV1` built by the existing `build_graph()` path for
/// the same search execution.
///
/// # Errors
///
/// Returns [`TapeRenderError`] if required header fields are missing or invalid.
#[allow(clippy::too_many_lines)]
pub fn render_graph(tape: &SearchTapeV1) -> Result<SearchGraphV1, TapeRenderError> {
    let header = &tape.header.json;

    // Extract header bindings
    let world_id = header_str(header, "world_id")?;
    let schema_descriptor = header_str(header, "schema_descriptor")?;
    let registry_digest = header_str(header, "registry_digest")?;
    let policy_snapshot_digest = header_str(header, "policy_snapshot_digest")?;
    let search_policy_digest = header_str(header, "search_policy_digest")?;
    let root_state_fingerprint = header_str(header, "root_state_fingerprint")?;
    let operator_set_digest = header
        .get("operator_set_digest")
        .and_then(|v| v.as_str())
        .map(String::from);
    let scorer_digest = header
        .get("scorer_digest")
        .and_then(|v| v.as_str())
        .map(String::from);

    let dedup_key = match header_str(header, "dedup_key")?.as_str() {
        "identity_only" => DedupKeyV1::IdentityOnly,
        "full_state" => DedupKeyV1::FullState,
        other => {
            return Err(TapeRenderError::InvalidHeaderField {
                field: "dedup_key",
                detail: format!("unknown dedup_key: {other}"),
            });
        }
    };

    let prune_visited_policy = match header_str(header, "prune_visited_policy")?.as_str() {
        "keep_visited" => PruneVisitedPolicyV1::KeepVisited,
        "release_visited" => PruneVisitedPolicyV1::ReleaseVisited,
        other => {
            return Err(TapeRenderError::InvalidHeaderField {
                field: "prune_visited_policy",
                detail: format!("unknown prune_visited_policy: {other}"),
            });
        }
    };

    // Build expansion index: node_id → first expansion index (first-wins)
    let mut expansion_index: HashMap<u64, usize> = HashMap::new();
    let mut expansions_out: Vec<ExpandEventV1> = Vec::new();
    let mut total_candidates_generated: u64 = 0;
    let mut total_duplicates_suppressed: u64 = 0;
    let mut total_dead_ends_exhaustive: u64 = 0;
    let mut total_dead_ends_budget_limited: u64 = 0;

    let mut termination_reason: Option<TerminationReasonV1> = None;
    let mut frontier_high_water: u64 = 0;

    for record in &tape.records {
        match record {
            TapeRecordV1::NodeCreation(_) => {
                // Node summaries are built separately below
            }
            TapeRecordV1::Expansion(exp) => {
                let candidates: Vec<CandidateRecordV1> =
                    exp.candidates.iter().map(render_candidate).collect();

                // Accumulate counters
                total_candidates_generated += candidates.len() as u64;
                for cand in &candidates {
                    if matches!(cand.outcome, CandidateOutcomeV1::DuplicateSuppressed { .. }) {
                        total_duplicates_suppressed += 1;
                    }
                }

                if let Some(reason) = &exp.dead_end_reason {
                    match reason {
                        DeadEndReasonV1::Exhaustive => total_dead_ends_exhaustive += 1,
                        DeadEndReasonV1::BudgetLimited => total_dead_ends_budget_limited += 1,
                    }
                }

                let notes: Vec<ExpansionNoteV1> = exp.notes.clone();

                let expansion_event = ExpandEventV1 {
                    expansion_order: exp.expansion_order,
                    node_id: exp.node_id,
                    state_fingerprint: raw_to_hex_str(&exp.state_fingerprint),
                    frontier_pop_key: FrontierPopKeyV1 {
                        f_cost: exp.pop_f_cost,
                        depth: exp.pop_depth,
                        creation_order: exp.pop_creation_order,
                    },
                    candidates,
                    candidates_truncated: exp.candidates_truncated,
                    dead_end_reason: exp.dead_end_reason,
                    notes,
                };

                expansion_index
                    .entry(exp.node_id)
                    .or_insert(expansions_out.len());
                expansions_out.push(expansion_event);
            }
            TapeRecordV1::Termination(term) => {
                termination_reason = Some(term.reason.clone());
                frontier_high_water = term.frontier_high_water;
            }
        }
    }

    let termination_reason = termination_reason.ok_or(TapeRenderError::NoTermination)?;
    let total_expansions = expansions_out.len() as u64;

    // Build node summaries sorted by node_id ascending
    let mut node_summaries: Vec<SearchGraphNodeSummaryV1> = Vec::new();
    for record in &tape.records {
        if let TapeRecordV1::NodeCreation(nc) = record {
            let exp = expansion_index
                .get(&nc.node_id)
                .map(|&idx| &expansions_out[idx]);
            let expansion_order = exp.map(|e| e.expansion_order);
            let dead_end_reason = exp.and_then(|e| e.dead_end_reason);
            let is_goal = matches!(
                &termination_reason,
                TerminationReasonV1::GoalReached { node_id } if *node_id == nc.node_id
            );

            node_summaries.push(SearchGraphNodeSummaryV1 {
                node_id: nc.node_id,
                parent_id: nc.parent_id,
                state_fingerprint: raw_to_hex_str(&nc.state_fingerprint),
                depth: nc.depth,
                f_cost: nc.f_cost,
                is_goal,
                dead_end_reason,
                expansion_order,
            });
        }
    }
    node_summaries.sort_by_key(|n| n.node_id);

    Ok(SearchGraphV1 {
        expansions: expansions_out,
        node_summaries,
        metadata: SearchGraphMetadata {
            world_id,
            schema_descriptor,
            registry_digest,
            policy_snapshot_digest,
            search_policy_digest,
            root_state_fingerprint,
            scorer_digest,
            operator_set_digest,
            total_expansions,
            total_candidates_generated,
            total_duplicates_suppressed,
            total_dead_ends_exhaustive,
            total_dead_ends_budget_limited,
            termination_reason,
            frontier_high_water,
            dedup_key,
            prune_visited_policy,
        },
    })
}

// ---------------------------------------------------------------------------
// Candidate rendering
// ---------------------------------------------------------------------------

fn render_candidate(tc: &TapeCandidateV1) -> CandidateRecordV1 {
    let op_code = Code32::from_le_bytes(tc.op_code_bytes);
    let canonical_hash = raw_to_content_hash(&tc.canonical_hash);
    let action = CandidateActionV1::from_parts(op_code, tc.op_args.clone(), canonical_hash);

    let source = match &tc.score_source {
        TapeScoreSourceV1::Uniform => ScoreSourceV1::Uniform,
        TapeScoreSourceV1::ModelDigest(digest) => {
            ScoreSourceV1::ModelDigest(raw_to_content_hash(digest))
        }
        TapeScoreSourceV1::Unavailable => ScoreSourceV1::Unavailable,
    };

    let score = CandidateScoreV1 {
        bonus: tc.score_bonus,
        source,
    };

    let outcome = match &tc.outcome {
        TapeCandidateOutcomeV1::Applied { to_node } => {
            CandidateOutcomeV1::Applied { to_node: *to_node }
        }
        TapeCandidateOutcomeV1::DuplicateSuppressed {
            existing_fingerprint,
        } => CandidateOutcomeV1::DuplicateSuppressed {
            existing_fingerprint: raw_to_hex_str(existing_fingerprint),
        },
        TapeCandidateOutcomeV1::IllegalOperator => CandidateOutcomeV1::IllegalOperator,
        TapeCandidateOutcomeV1::ApplyFailed(kind) => CandidateOutcomeV1::ApplyFailed(*kind),
        TapeCandidateOutcomeV1::SkippedByDepthLimit => CandidateOutcomeV1::SkippedByDepthLimit,
        TapeCandidateOutcomeV1::SkippedByPolicy => CandidateOutcomeV1::SkippedByPolicy,
        TapeCandidateOutcomeV1::NotEvaluated => CandidateOutcomeV1::NotEvaluated,
    };

    CandidateRecordV1 {
        index: tc.index,
        action,
        score,
        outcome,
    }
}

// ---------------------------------------------------------------------------
// Header field helpers
// ---------------------------------------------------------------------------

fn header_str(header: &serde_json::Value, field: &'static str) -> Result<String, TapeRenderError> {
    header
        .get(field)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or(TapeRenderError::MissingHeaderField(field))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tape_reader::read_tape;

    #[test]
    fn render_minimal_tape() {
        // Build a minimal tape: root node + termination
        use crate::tape::{NodeCreationView, TapeSink, TerminationView};
        use crate::tape_writer::TapeWriter;
        use sterling_kernel::carrier::bytestate::ByteStateV1;

        let header = br#"{"dedup_key":"identity_only","policy_snapshot_digest":"test_policy","prune_visited_policy":"keep_visited","registry_digest":"test_registry","root_state_fingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","schema_descriptor":"test:v1:sha256:abc","search_policy_digest":"test_search_policy","world_id":"test_world"}"#;

        let mut writer = TapeWriter::new(header);

        let node = crate::node::SearchNodeV1 {
            node_id: 0,
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
        };

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

        let output = writer.finish().unwrap();
        let tape = read_tape(&output.bytes).unwrap();
        let graph = render_graph(&tape).unwrap();

        // Verify metadata
        assert_eq!(graph.metadata.world_id, "test_world");
        assert_eq!(graph.metadata.total_expansions, 0);
        assert_eq!(
            graph.metadata.termination_reason,
            TerminationReasonV1::FrontierExhausted
        );
        assert_eq!(graph.metadata.frontier_high_water, 1);

        // Verify node summaries
        assert_eq!(graph.node_summaries.len(), 1);
        assert_eq!(graph.node_summaries[0].node_id, 0);
        assert!(!graph.node_summaries[0].is_goal);

        // Verify it produces valid canonical JSON
        let bytes = graph.to_canonical_json_bytes().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn render_missing_header_field() {
        use crate::tape::{TapeSink, TerminationView};
        use crate::tape_writer::TapeWriter;

        // Header missing "world_id"
        let header = br#"{"dedup_key":"identity_only"}"#;
        let mut writer = TapeWriter::new(header);
        writer
            .on_termination(&TerminationView {
                termination_reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 0,
            })
            .unwrap();

        let output = writer.finish().unwrap();
        let tape = read_tape(&output.bytes).unwrap();
        let err = render_graph(&tape).unwrap_err();
        assert!(matches!(
            err,
            TapeRenderError::MissingHeaderField("world_id")
        ));
    }
}
