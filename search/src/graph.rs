//! `SearchGraphV1`: expansion-event audit log.
//!
//! The normative decision surface is the ordered list of `ExpandEventV1`
//! entries. Node summaries are a derived index for path reconstruction.

use crate::node::CandidateActionV1;
use crate::policy::{DedupKeyV1, PruneVisitedPolicyV1};
use crate::scorer::CandidateScoreV1;

/// Domain prefix for search graph content hashing.
pub const DOMAIN_SEARCH_GRAPH: &[u8] = b"STERLING::SEARCH_GRAPH::V1\0";

/// The complete search audit trail.
#[derive(Debug, Clone)]
pub struct SearchGraphV1 {
    /// Ordered expansion events (normative decision surface).
    pub expansions: Vec<ExpandEventV1>,
    /// Derived node index sorted by `node_id` ascending (INV-SC-09).
    pub node_summaries: Vec<SearchGraphNodeSummaryV1>,
    /// Aggregate metadata with snapshot bindings.
    pub metadata: SearchGraphMetadata,
}

/// A single frontier-pop + candidate-expansion event.
#[derive(Debug, Clone)]
pub struct ExpandEventV1 {
    /// Total order of frontier pops.
    pub expansion_order: u64,
    /// The node being expanded.
    pub node_id: u64,
    /// Hex fingerprint of the expanded node's state.
    pub state_fingerprint: String,
    /// The frontier key at time of pop.
    pub frontier_pop_key: FrontierPopKeyV1,
    /// Ordered candidate decision log.
    pub candidates: Vec<CandidateRecordV1>,
    /// True if max-candidates-per-node cap was hit.
    pub candidates_truncated: bool,
    /// Dead-end reason if this expansion produced zero children.
    pub dead_end_reason: Option<DeadEndReasonV1>,
    /// Expansion-level notes (prune events, budget notes).
    pub notes: Vec<ExpansionNoteV1>,
}

/// The frontier ordering key recorded at pop time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontierPopKeyV1 {
    pub f_cost: i64,
    pub depth: u32,
    pub creation_order: u64,
}

/// A candidate with its outcome recorded in the graph.
#[derive(Debug, Clone)]
pub struct CandidateRecordV1 {
    /// Index in the sorted candidate list.
    pub index: u64,
    /// The candidate action.
    pub action: CandidateActionV1,
    /// The score with provenance.
    pub score: CandidateScoreV1,
    /// What happened when this candidate was processed.
    pub outcome: CandidateOutcomeV1,
}

/// Outcome of processing a candidate during expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateOutcomeV1 {
    /// Successfully applied; created a new node.
    Applied { to_node: u64 },
    /// State already visited (first-seen-wins dedup).
    DuplicateSuppressed { existing_fingerprint: String },
    /// Op code not in registry (INV-SC-02 violation).
    IllegalOperator,
    /// Kernel `apply()` failed.
    ApplyFailed(ApplyFailureKindV1),
    /// Skipped because child would exceed `max_depth`.
    SkippedByDepthLimit,
    /// Skipped by policy (future extensibility).
    SkippedByPolicy,
}

/// Why a node was marked as a dead end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadEndReasonV1 {
    /// All candidates were tried without caps — genuinely no successors.
    Exhaustive,
    /// Expansion was truncated by budget or cap — may have successors.
    BudgetLimited,
}

/// Expansion-level notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpansionNoteV1 {
    /// Candidate generation was capped.
    CandidateCapReached { cap: u64 },
    /// Frontier was pruned during this expansion.
    FrontierPruned { pruned_node_ids: Vec<u64> },
}

/// Mirror of kernel `ApplyFailure` variants for graph serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyFailureKindV1 {
    PreconditionNotMet,
    ArgumentMismatch,
    UnknownOperator,
}

/// Derived node summary for path reconstruction.
#[derive(Debug, Clone)]
pub struct SearchGraphNodeSummaryV1 {
    pub node_id: u64,
    pub parent_id: Option<u64>,
    pub state_fingerprint: String,
    pub depth: u32,
    pub f_cost: i64,
    pub is_goal: bool,
    pub dead_end_reason: Option<DeadEndReasonV1>,
    pub expansion_order: Option<u64>,
}

/// Aggregate metadata with snapshot bindings.
#[derive(Debug, Clone)]
pub struct SearchGraphMetadata {
    // Snapshot bindings
    pub world_id: String,
    pub schema_descriptor: String,
    pub registry_digest: String,
    pub policy_snapshot_digest: String,
    pub search_policy_digest: String,
    pub root_state_fingerprint: String,

    // Counters
    pub total_expansions: u64,
    pub total_candidates_generated: u64,
    pub total_duplicates_suppressed: u64,
    pub total_dead_ends_exhaustive: u64,
    pub total_dead_ends_budget_limited: u64,
    pub termination_reason: TerminationReasonV1,
    pub frontier_high_water: u64,

    // Policy echo
    pub dedup_key: DedupKeyV1,
    pub prune_visited_policy: PruneVisitedPolicyV1,
}

/// Why the search terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationReasonV1 {
    /// Search found a goal state.
    GoalReached { node_id: u64 },
    /// Frontier emptied without finding a goal.
    FrontierExhausted,
    /// `max_expansions` budget was hit.
    ExpansionBudgetExceeded,
    /// `max_depth` budget was hit for all candidates.
    DepthBudgetExceeded,
    /// A candidate's `op_code` was not in the registry (INV-SC-02).
    WorldContractViolation,
    /// Scorer returned wrong number of scores.
    ScorerContractViolation { expected: u64, actual: u64 },
    /// A panic was caught in a world or scorer callback.
    InternalPanic { stage: PanicStageV1 },
    /// An internal search-loop invariant was violated without panicking.
    FrontierInvariantViolation,
}

/// Stage at which a panic was caught.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanicStageV1 {
    /// `SearchWorldV1::enumerate_candidates()` panicked.
    EnumerateCandidates,
    /// `ValueScorer::score_candidates()` panicked.
    ScoreCandidates,
    /// `SearchWorldV1::is_goal()` panicked on the root node.
    IsGoalRoot,
    /// `SearchWorldV1::is_goal()` panicked during expansion.
    IsGoalExpansion,
}

// ---------------------------------------------------------------------------
// Canonical JSON serialization
// ---------------------------------------------------------------------------

impl SearchGraphV1 {
    /// Serialize the graph to canonical JSON bytes.
    ///
    /// Uses `sterling_kernel::proof::canon::canonical_json_bytes` for
    /// deterministic output (sorted keys, compact separators).
    ///
    /// # Errors
    ///
    /// Returns [`sterling_kernel::proof::canon::CanonError`] if serialization fails.
    pub fn to_canonical_json_bytes(
        &self,
    ) -> Result<Vec<u8>, sterling_kernel::proof::canon::CanonError> {
        let value = self.to_json_value();
        sterling_kernel::proof::canon::canonical_json_bytes(&value)
    }

    /// Convert to a `serde_json::Value` for canonical serialization.
    #[must_use]
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "expansions": self.expansions.iter().map(expand_event_to_json).collect::<Vec<_>>(),
            "metadata": metadata_to_json(&self.metadata),
            "node_summaries": self.node_summaries.iter().map(node_summary_to_json).collect::<Vec<_>>(),
        })
    }
}

fn expand_event_to_json(e: &ExpandEventV1) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "candidates": e.candidates.iter().map(candidate_record_to_json).collect::<Vec<_>>(),
        "candidates_truncated": e.candidates_truncated,
        "expansion_order": e.expansion_order,
        "frontier_pop_key": {
            "creation_order": e.frontier_pop_key.creation_order,
            "depth": e.frontier_pop_key.depth,
            "f_cost": e.frontier_pop_key.f_cost,
        },
        "node_id": e.node_id,
        "notes": e.notes.iter().map(note_to_json).collect::<Vec<_>>(),
        "state_fingerprint": e.state_fingerprint,
    });

    if let Some(reason) = &e.dead_end_reason {
        obj["dead_end_reason"] = dead_end_reason_to_json(*reason);
    } else {
        obj["dead_end_reason"] = serde_json::Value::Null;
    }

    obj
}

fn candidate_record_to_json(r: &CandidateRecordV1) -> serde_json::Value {
    serde_json::json!({
        "action": {
            "canonical_hash": r.action.canonical_hash.as_str(),
            "op_args_hex": hex::encode(&r.action.op_args),
            "op_code_hex": hex::encode(r.action.op_code.to_le_bytes()),
        },
        "index": r.index,
        "outcome": outcome_to_json(&r.outcome),
        "score": {
            "bonus": r.score.bonus,
            "source": score_source_to_json(&r.score.source),
        },
    })
}

fn outcome_to_json(o: &CandidateOutcomeV1) -> serde_json::Value {
    match o {
        CandidateOutcomeV1::Applied { to_node } => {
            serde_json::json!({"type": "applied", "to_node": to_node})
        }
        CandidateOutcomeV1::DuplicateSuppressed {
            existing_fingerprint,
        } => {
            serde_json::json!({"existing_fingerprint": existing_fingerprint, "type": "duplicate_suppressed"})
        }
        CandidateOutcomeV1::IllegalOperator => {
            serde_json::json!({"type": "illegal_operator"})
        }
        CandidateOutcomeV1::ApplyFailed(kind) => {
            serde_json::json!({"kind": apply_failure_kind_str(*kind), "type": "apply_failed"})
        }
        CandidateOutcomeV1::SkippedByDepthLimit => {
            serde_json::json!({"type": "skipped_by_depth_limit"})
        }
        CandidateOutcomeV1::SkippedByPolicy => {
            serde_json::json!({"type": "skipped_by_policy"})
        }
    }
}

fn apply_failure_kind_str(k: ApplyFailureKindV1) -> &'static str {
    match k {
        ApplyFailureKindV1::PreconditionNotMet => "precondition_not_met",
        ApplyFailureKindV1::ArgumentMismatch => "argument_mismatch",
        ApplyFailureKindV1::UnknownOperator => "unknown_operator",
    }
}

fn score_source_to_json(s: &crate::scorer::ScoreSourceV1) -> serde_json::Value {
    match s {
        crate::scorer::ScoreSourceV1::Uniform => serde_json::json!("uniform"),
        crate::scorer::ScoreSourceV1::ModelDigest(h) => {
            serde_json::json!({"model_digest": h.as_str()})
        }
    }
}

fn dead_end_reason_to_json(r: DeadEndReasonV1) -> serde_json::Value {
    match r {
        DeadEndReasonV1::Exhaustive => serde_json::json!("exhaustive"),
        DeadEndReasonV1::BudgetLimited => serde_json::json!("budget_limited"),
    }
}

fn note_to_json(n: &ExpansionNoteV1) -> serde_json::Value {
    match n {
        ExpansionNoteV1::CandidateCapReached { cap } => {
            serde_json::json!({"cap": cap, "type": "candidate_cap_reached"})
        }
        ExpansionNoteV1::FrontierPruned { pruned_node_ids } => {
            serde_json::json!({"pruned_node_ids": pruned_node_ids, "type": "frontier_pruned"})
        }
    }
}

fn node_summary_to_json(n: &SearchGraphNodeSummaryV1) -> serde_json::Value {
    serde_json::json!({
        "dead_end_reason": n.dead_end_reason.map(dead_end_reason_to_json),
        "depth": n.depth,
        "expansion_order": n.expansion_order,
        "f_cost": n.f_cost,
        "is_goal": n.is_goal,
        "node_id": n.node_id,
        "parent_id": n.parent_id,
        "state_fingerprint": n.state_fingerprint,
    })
}

fn metadata_to_json(m: &SearchGraphMetadata) -> serde_json::Value {
    serde_json::json!({
        "dedup_key": dedup_key_str(m.dedup_key),
        "frontier_high_water": m.frontier_high_water,
        "policy_snapshot_digest": m.policy_snapshot_digest,
        "prune_visited_policy": prune_policy_str(m.prune_visited_policy),
        "registry_digest": m.registry_digest,
        "root_state_fingerprint": m.root_state_fingerprint,
        "schema_descriptor": m.schema_descriptor,
        "search_policy_digest": m.search_policy_digest,
        "termination_reason": termination_reason_to_json(&m.termination_reason),
        "total_candidates_generated": m.total_candidates_generated,
        "total_dead_ends_budget_limited": m.total_dead_ends_budget_limited,
        "total_dead_ends_exhaustive": m.total_dead_ends_exhaustive,
        "total_duplicates_suppressed": m.total_duplicates_suppressed,
        "total_expansions": m.total_expansions,
        "world_id": m.world_id,
    })
}

fn dedup_key_str(k: DedupKeyV1) -> &'static str {
    match k {
        DedupKeyV1::IdentityOnly => "identity_only",
        DedupKeyV1::FullState => "full_state",
    }
}

fn prune_policy_str(p: PruneVisitedPolicyV1) -> &'static str {
    match p {
        PruneVisitedPolicyV1::KeepVisited => "keep_visited",
        PruneVisitedPolicyV1::ReleaseVisited => "release_visited",
    }
}

fn termination_reason_to_json(r: &TerminationReasonV1) -> serde_json::Value {
    match r {
        TerminationReasonV1::GoalReached { node_id } => {
            serde_json::json!({"node_id": node_id, "type": "goal_reached"})
        }
        TerminationReasonV1::FrontierExhausted => serde_json::json!({"type": "frontier_exhausted"}),
        TerminationReasonV1::ExpansionBudgetExceeded => {
            serde_json::json!({"type": "expansion_budget_exceeded"})
        }
        TerminationReasonV1::DepthBudgetExceeded => {
            serde_json::json!({"type": "depth_budget_exceeded"})
        }
        TerminationReasonV1::WorldContractViolation => {
            serde_json::json!({"type": "world_contract_violation"})
        }
        TerminationReasonV1::ScorerContractViolation { expected, actual } => {
            serde_json::json!({"actual": actual, "expected": expected, "type": "scorer_contract_violation"})
        }
        TerminationReasonV1::InternalPanic { stage } => {
            serde_json::json!({"stage": panic_stage_str(*stage), "type": "internal_panic"})
        }
        TerminationReasonV1::FrontierInvariantViolation => {
            serde_json::json!({"type": "frontier_invariant_violation"})
        }
    }
}

fn panic_stage_str(s: PanicStageV1) -> &'static str {
    match s {
        PanicStageV1::EnumerateCandidates => "enumerate_candidates",
        PanicStageV1::ScoreCandidates => "score_candidates",
        PanicStageV1::IsGoalRoot => "is_goal_root",
        PanicStageV1::IsGoalExpansion => "is_goal_expansion",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_is_deterministic() {
        let graph = SearchGraphV1 {
            expansions: Vec::new(),
            node_summaries: Vec::new(),
            metadata: SearchGraphMetadata {
                world_id: "test".into(),
                schema_descriptor: "test:v1".into(),
                registry_digest: "abc123".into(),
                policy_snapshot_digest: "def456".into(),
                search_policy_digest: "ghi789".into(),
                root_state_fingerprint: "root_fp".into(),
                total_expansions: 0,
                total_candidates_generated: 0,
                total_duplicates_suppressed: 0,
                total_dead_ends_exhaustive: 0,
                total_dead_ends_budget_limited: 0,
                termination_reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 0,
                dedup_key: DedupKeyV1::IdentityOnly,
                prune_visited_policy: PruneVisitedPolicyV1::KeepVisited,
            },
        };

        let bytes1 = graph.to_canonical_json_bytes().unwrap();
        let bytes2 = graph.to_canonical_json_bytes().unwrap();
        assert_eq!(bytes1, bytes2, "canonical JSON must be deterministic");

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn termination_reason_serializes_correctly() {
        let goal = termination_reason_to_json(&TerminationReasonV1::GoalReached { node_id: 42 });
        assert_eq!(goal["type"], "goal_reached");
        assert_eq!(goal["node_id"], 42);

        let exhausted = termination_reason_to_json(&TerminationReasonV1::FrontierExhausted);
        assert_eq!(exhausted["type"], "frontier_exhausted");

        let scorer = termination_reason_to_json(&TerminationReasonV1::ScorerContractViolation {
            expected: 5,
            actual: 3,
        });
        assert_eq!(scorer["type"], "scorer_contract_violation");
        assert_eq!(scorer["expected"], 5);
        assert_eq!(scorer["actual"], 3);

        let panic = termination_reason_to_json(&TerminationReasonV1::InternalPanic {
            stage: PanicStageV1::EnumerateCandidates,
        });
        assert_eq!(panic["type"], "internal_panic");
        assert_eq!(panic["stage"], "enumerate_candidates");

        let frontier_inv =
            termination_reason_to_json(&TerminationReasonV1::FrontierInvariantViolation);
        assert_eq!(frontier_inv["type"], "frontier_invariant_violation");
    }
}
