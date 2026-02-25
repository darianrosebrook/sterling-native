//! Search entry point and expansion loop.

use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply;
use sterling_kernel::proof::hash::canonical_hash;

use crate::contract::SearchWorldV1;
use crate::error::SearchError;
use crate::frontier::BestFirstFrontier;
use crate::graph::{
    ApplyFailureKindV1, CandidateOutcomeV1, CandidateRecordV1, DeadEndReasonV1, ExpandEventV1,
    ExpansionNoteV1, FrontierPopKeyV1, SearchGraphMetadata, SearchGraphNodeSummaryV1,
    SearchGraphV1, TerminationReasonV1,
};
use crate::node::{SearchNodeV1, DOMAIN_SEARCH_NODE};
use crate::policy::SearchPolicyV1;
use crate::scorer::{CandidateScoreV1, ValueScorer};

/// Result of a successful search.
#[derive(Debug)]
pub struct SearchResult {
    /// The goal node (if found).
    pub goal_node: Option<SearchNodeV1>,
    /// The complete search graph audit trail.
    pub graph: SearchGraphV1,
    /// All nodes created during search, indexed by `node_id`.
    pub nodes: Vec<SearchNodeV1>,
}

/// Run best-first search from the root state.
///
/// # Errors
///
/// Returns [`SearchError::UnsupportedPolicyMode`] if reserved policy options are selected.
/// Returns [`SearchError::WorldContractViolation`] if a world produces an illegal candidate.
///
/// # Panics
///
/// Panics if the scorer returns a different number of scores than candidates.
#[allow(clippy::too_many_lines)]
pub fn search(
    root_state: sterling_kernel::carrier::bytestate::ByteStateV1,
    world: &dyn SearchWorldV1,
    registry: &RegistryV1,
    policy: &SearchPolicyV1,
    scorer: &dyn ValueScorer,
    // Snapshot bindings for graph metadata
    metadata_bindings: &MetadataBindings,
) -> Result<SearchResult, SearchError> {
    // INV-SC-10: validate M1 policy constraints
    policy.validate_m1()?;

    let mut frontier = BestFirstFrontier::new();
    let mut expansions: Vec<ExpandEventV1> = Vec::new();
    let mut all_nodes: Vec<SearchNodeV1> = Vec::new();
    let mut next_node_id: u64 = 0;
    let mut next_creation_order: u64 = 0;
    let mut expansion_count: u64 = 0;
    let mut total_candidates_generated: u64 = 0;
    let mut total_duplicates_suppressed: u64 = 0;
    let mut total_dead_ends_exhaustive: u64 = 0;
    let mut total_dead_ends_budget_limited: u64 = 0;

    // Create root node
    let root_fp = canonical_hash(DOMAIN_SEARCH_NODE, &root_state.identity_bytes());
    let root_node = SearchNodeV1 {
        node_id: next_node_id,
        parent_id: None,
        state: root_state,
        state_fingerprint: root_fp.clone(),
        depth: 0,
        g_cost: 0,
        h_cost: 0,
        creation_order: next_creation_order,
        producing_action: None,
    };
    next_node_id += 1;
    next_creation_order += 1;

    let root_fp_hex = root_fp.hex_digest().to_string();

    // Check if root is already a goal
    if world.is_goal(&root_node.state) {
        all_nodes.push(root_node.clone());
        let graph = build_graph(
            expansions,
            &all_nodes,
            TerminationReasonV1::GoalReached { node_id: 0 },
            frontier.high_water(),
            total_candidates_generated,
            total_duplicates_suppressed,
            total_dead_ends_exhaustive,
            total_dead_ends_budget_limited,
            metadata_bindings,
            &root_fp_hex,
            policy,
        );
        return Ok(SearchResult {
            goal_node: Some(all_nodes[0].clone()),
            graph,
            nodes: all_nodes,
        });
    }

    all_nodes.push(root_node.clone());
    frontier.push(root_node);

    let termination_reason;

    // Main search loop
    loop {
        // Check frontier exhaustion
        if frontier.is_empty() {
            termination_reason = TerminationReasonV1::FrontierExhausted;
            break;
        }

        // Check expansion budget
        if expansion_count >= policy.max_expansions {
            termination_reason = TerminationReasonV1::ExpansionBudgetExceeded;
            break;
        }

        // Pop best node
        // Safety: we checked `!frontier.is_empty()` above
        let Some(current) = frontier.pop() else {
            unreachable!("frontier was checked non-empty")
        };
        let current_fp_hex = current.state_fingerprint.hex_digest().to_string();
        let pop_key = FrontierPopKeyV1 {
            f_cost: current.f_cost(),
            depth: current.depth,
            creation_order: current.creation_order,
        };

        // Enumerate candidates from world
        let mut candidates = world.enumerate_candidates(&current.state, registry);

        // Sort by canonical_hash for deterministic enumeration (INV-SC-01)
        candidates.sort();

        // Apply candidate cap
        let mut notes = Vec::new();
        let candidates_truncated = candidates.len() as u64 > policy.max_candidates_per_node;
        if candidates_truncated {
            #[allow(clippy::cast_possible_truncation)]
            candidates.truncate(policy.max_candidates_per_node as usize);
            notes.push(ExpansionNoteV1::CandidateCapReached {
                cap: policy.max_candidates_per_node,
            });
        }

        // Score candidates
        let candidate_scores = scorer.score_candidates(&current, &candidates);
        assert_eq!(
            candidate_scores.len(),
            candidates.len(),
            "scorer must return one score per candidate"
        );

        // Build scored + sorted candidate list for expansion
        // Sort by (-bonus, canonical_hash) for deterministic expansion order
        let mut scored_candidates: Vec<(
            usize,
            &crate::node::CandidateActionV1,
            &CandidateScoreV1,
        )> = candidates
            .iter()
            .zip(candidate_scores.iter())
            .enumerate()
            .map(|(i, (c, s))| (i, c, s))
            .collect();
        scored_candidates.sort_by(|a, b| {
            b.2.bonus
                .cmp(&a.2.bonus)
                .then_with(|| a.1.canonical_hash.cmp(&b.1.canonical_hash))
        });

        total_candidates_generated += candidates.len() as u64;

        let mut candidate_records = Vec::new();
        let mut children_created = 0u64;
        let mut found_goal = false;
        let mut goal_node_id = 0u64;

        for (sorted_idx, &(_orig_idx, candidate, score)) in scored_candidates.iter().enumerate() {
            // INV-SC-02: check candidate legality
            if !registry.contains(&candidate.op_code) {
                candidate_records.push(CandidateRecordV1 {
                    index: sorted_idx as u64,
                    action: candidate.clone(),
                    score: score.clone(),
                    outcome: CandidateOutcomeV1::IllegalOperator,
                });

                // Record the expansion event before terminating
                expansions.push(ExpandEventV1 {
                    expansion_order: expansion_count,
                    node_id: current.node_id,
                    state_fingerprint: current_fp_hex.clone(),
                    frontier_pop_key: pop_key,
                    candidates: candidate_records,
                    candidates_truncated,
                    dead_end_reason: None,
                    notes,
                });
                // Build and drop the graph — it's not returned in the error
                // but building it validates invariants and exercises the code path.
                let _graph = build_graph(
                    expansions,
                    &all_nodes,
                    TerminationReasonV1::WorldContractViolation,
                    frontier.high_water(),
                    total_candidates_generated,
                    total_duplicates_suppressed,
                    total_dead_ends_exhaustive,
                    total_dead_ends_budget_limited,
                    metadata_bindings,
                    &root_fp_hex,
                    policy,
                );
                return Err(SearchError::WorldContractViolation {
                    detail: format!(
                        "candidate op_code {} not in registry",
                        hex::encode(candidate.op_code.to_le_bytes())
                    ),
                });
            }

            // Check depth limit
            if current.depth + 1 > policy.max_depth {
                candidate_records.push(CandidateRecordV1 {
                    index: sorted_idx as u64,
                    action: candidate.clone(),
                    score: score.clone(),
                    outcome: CandidateOutcomeV1::SkippedByDepthLimit,
                });
                continue;
            }

            // Apply the candidate
            let apply_result = apply::apply(&current.state, candidate.op_code, &candidate.op_args);

            match apply_result {
                Err(fail) => {
                    let kind = match &fail {
                        apply::ApplyFailure::PreconditionNotMet { .. } => {
                            ApplyFailureKindV1::PreconditionNotMet
                        }
                        apply::ApplyFailure::ArgumentMismatch { .. } => {
                            ApplyFailureKindV1::ArgumentMismatch
                        }
                        apply::ApplyFailure::UnknownOperator { .. } => {
                            ApplyFailureKindV1::UnknownOperator
                        }
                    };
                    candidate_records.push(CandidateRecordV1 {
                        index: sorted_idx as u64,
                        action: candidate.clone(),
                        score: score.clone(),
                        outcome: CandidateOutcomeV1::ApplyFailed(kind),
                    });
                }
                Ok((new_state, _step_record)) => {
                    // Compute fingerprint for dedup
                    let child_fp = canonical_hash(DOMAIN_SEARCH_NODE, &new_state.identity_bytes());
                    let child_fp_hex = child_fp.hex_digest().to_string();

                    // Check visited set (first-seen-wins)
                    if frontier.is_visited(&child_fp_hex) {
                        total_duplicates_suppressed += 1;
                        candidate_records.push(CandidateRecordV1 {
                            index: sorted_idx as u64,
                            action: candidate.clone(),
                            score: score.clone(),
                            outcome: CandidateOutcomeV1::DuplicateSuppressed {
                                existing_fingerprint: child_fp_hex,
                            },
                        });
                        continue;
                    }

                    // Create child node
                    let child = SearchNodeV1 {
                        node_id: next_node_id,
                        parent_id: Some(current.node_id),
                        state: new_state,
                        state_fingerprint: child_fp,
                        depth: current.depth + 1,
                        g_cost: current.g_cost + 1,
                        h_cost: 0, // M1 default: no heuristic
                        creation_order: next_creation_order,
                        producing_action: Some(candidate.clone()),
                    };
                    next_node_id += 1;
                    next_creation_order += 1;

                    let child_node_id = child.node_id;

                    candidate_records.push(CandidateRecordV1 {
                        index: sorted_idx as u64,
                        action: candidate.clone(),
                        score: score.clone(),
                        outcome: CandidateOutcomeV1::Applied {
                            to_node: child_node_id,
                        },
                    });

                    // Check goal before pushing to frontier
                    if world.is_goal(&child.state) {
                        found_goal = true;
                        goal_node_id = child_node_id;
                    }

                    all_nodes.push(child.clone());
                    frontier.push(child);
                    children_created += 1;
                }
            }
        }

        // Dead-end detection (INV-SC-07)
        let dead_end_reason = if children_created == 0 {
            let reason = if candidates_truncated {
                DeadEndReasonV1::BudgetLimited
            } else {
                DeadEndReasonV1::Exhaustive
            };
            frontier.mark_dead_end(&current_fp_hex);
            match reason {
                DeadEndReasonV1::Exhaustive => total_dead_ends_exhaustive += 1,
                DeadEndReasonV1::BudgetLimited => total_dead_ends_budget_limited += 1,
            }
            Some(reason)
        } else {
            None
        };

        // Frontier pruning
        if frontier.len() as u64 > policy.max_frontier_size {
            #[allow(clippy::cast_possible_truncation)]
            let pruned_ids = frontier.prune_to(policy.max_frontier_size as usize);
            if !pruned_ids.is_empty() {
                notes.push(ExpansionNoteV1::FrontierPruned {
                    pruned_node_ids: pruned_ids,
                });
            }
        }

        // Record the expansion event (INV-SC-03, INV-SC-06)
        expansions.push(ExpandEventV1 {
            expansion_order: expansion_count,
            node_id: current.node_id,
            state_fingerprint: current_fp_hex,
            frontier_pop_key: pop_key,
            candidates: candidate_records,
            candidates_truncated,
            dead_end_reason,
            notes,
        });
        expansion_count += 1;

        // If goal was found, terminate
        if found_goal {
            termination_reason = TerminationReasonV1::GoalReached {
                node_id: goal_node_id,
            };
            break;
        }
    }

    let goal_node = match &termination_reason {
        TerminationReasonV1::GoalReached { node_id } => {
            all_nodes.iter().find(|n| n.node_id == *node_id).cloned()
        }
        _ => None,
    };

    let graph = build_graph(
        expansions,
        &all_nodes,
        termination_reason,
        frontier.high_water(),
        total_candidates_generated,
        total_duplicates_suppressed,
        total_dead_ends_exhaustive,
        total_dead_ends_budget_limited,
        metadata_bindings,
        &root_fp_hex,
        policy,
    );

    Ok(SearchResult {
        goal_node,
        graph,
        nodes: all_nodes,
    })
}

/// Snapshot bindings for `SearchGraphMetadata`.
#[derive(Debug, Clone)]
pub struct MetadataBindings {
    pub world_id: String,
    pub schema_descriptor: String,
    pub registry_digest: String,
    pub policy_snapshot_digest: String,
    pub search_policy_digest: String,
}

/// Reconstruct the path from root to a goal node.
#[must_use]
pub fn reconstruct_path(nodes: &[SearchNodeV1], goal_node_id: u64) -> Vec<u64> {
    let mut path = Vec::new();
    let mut current_id = Some(goal_node_id);

    while let Some(id) = current_id {
        path.push(id);
        current_id = nodes
            .iter()
            .find(|n| n.node_id == id)
            .and_then(|n| n.parent_id);
    }

    path.reverse();
    path
}

#[allow(clippy::too_many_arguments)]
fn build_graph(
    expansions: Vec<ExpandEventV1>,
    all_nodes: &[SearchNodeV1],
    termination_reason: TerminationReasonV1,
    frontier_high_water: u64,
    total_candidates_generated: u64,
    total_duplicates_suppressed: u64,
    total_dead_ends_exhaustive: u64,
    total_dead_ends_budget_limited: u64,
    bindings: &MetadataBindings,
    root_fp_hex: &str,
    policy: &SearchPolicyV1,
) -> SearchGraphV1 {
    let total_expansions = expansions.len() as u64;

    // Build node summaries sorted by node_id ascending (INV-SC-09)
    let mut node_summaries: Vec<SearchGraphNodeSummaryV1> = all_nodes
        .iter()
        .map(|n| {
            let expansion_order = expansions
                .iter()
                .find(|e| e.node_id == n.node_id)
                .map(|e| e.expansion_order);
            let dead_end_reason = expansions
                .iter()
                .find(|e| e.node_id == n.node_id)
                .and_then(|e| e.dead_end_reason);
            let is_goal = matches!(
                &termination_reason,
                TerminationReasonV1::GoalReached { node_id } if *node_id == n.node_id
            );

            SearchGraphNodeSummaryV1 {
                node_id: n.node_id,
                parent_id: n.parent_id,
                state_fingerprint: n.state_fingerprint.hex_digest().to_string(),
                depth: n.depth,
                f_cost: n.f_cost(),
                is_goal,
                dead_end_reason,
                expansion_order,
            }
        })
        .collect();
    node_summaries.sort_by_key(|n| n.node_id);

    SearchGraphV1 {
        expansions,
        node_summaries,
        metadata: SearchGraphMetadata {
            world_id: bindings.world_id.clone(),
            schema_descriptor: bindings.schema_descriptor.clone(),
            registry_digest: bindings.registry_digest.clone(),
            policy_snapshot_digest: bindings.policy_snapshot_digest.clone(),
            search_policy_digest: bindings.search_policy_digest.clone(),
            root_state_fingerprint: root_fp_hex.to_string(),
            total_expansions,
            total_candidates_generated,
            total_duplicates_suppressed,
            total_dead_ends_exhaustive,
            total_dead_ends_budget_limited,
            termination_reason,
            frontier_high_water,
            dedup_key: policy.dedup_key,
            prune_visited_policy: policy.prune_visited_policy,
        },
    }
}
