//! Search entry point and expansion loop.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use sterling_kernel::operators::apply;
use sterling_kernel::operators::operator_registry::OperatorRegistryV1;
use sterling_kernel::proof::hash::canonical_hash;

use crate::contract::SearchWorldV1;
use crate::error::SearchError;
use crate::frontier::BestFirstFrontier;
use crate::graph::{
    ApplyFailureKindV1, CandidateOutcomeV1, CandidateRecordV1, DeadEndReasonV1, ExpandEventV1,
    ExpansionNoteV1, FrontierInvariantStageV1, FrontierPopKeyV1, PanicStageV1, SearchGraphMetadata,
    SearchGraphNodeSummaryV1, SearchGraphV1, TerminationReasonV1,
};
use crate::node::{SearchNodeV1, DOMAIN_SEARCH_NODE};
use crate::policy::SearchPolicyV1;
use crate::scorer::{CandidateScoreV1, ScoreSourceV1, ValueScorer};
use crate::tape::{
    content_hash_to_raw, ExpansionView, NodeCreationView, TapeOutput, TapeSink, TerminationView,
};
use crate::tape_writer::TapeWriter;

/// Result of a search execution.
///
/// Always contains a complete `SearchGraphV1` audit trail regardless of how
/// the search terminated. Check [`SearchResult::is_goal_reached`] or inspect
/// `graph.metadata.termination_reason` to determine the outcome.
#[derive(Debug)]
pub struct SearchResult {
    /// The goal node (if found).
    pub goal_node: Option<SearchNodeV1>,
    /// The complete search graph audit trail.
    pub graph: SearchGraphV1,
    /// All nodes created during search, indexed by `node_id`.
    pub nodes: Vec<SearchNodeV1>,
}

impl SearchResult {
    /// Returns `true` if the search terminated because a goal was reached.
    #[must_use]
    pub fn is_goal_reached(&self) -> bool {
        matches!(
            self.graph.metadata.termination_reason,
            TerminationReasonV1::GoalReached { .. }
        )
    }
}

/// Build candidate records for the post-sort, post-cap candidate list when
/// the scorer failed (panic or arity mismatch). Each record carries a
/// deterministic placeholder score and `NotEvaluated` outcome.
fn build_not_evaluated_records(
    candidates: &[crate::node::CandidateActionV1],
) -> Vec<CandidateRecordV1> {
    candidates
        .iter()
        .enumerate()
        .map(|(i, c)| CandidateRecordV1 {
            index: i as u64,
            action: c.clone(),
            score: CandidateScoreV1 {
                bonus: 0,
                source: ScoreSourceV1::Unavailable,
            },
            outcome: CandidateOutcomeV1::NotEvaluated,
        })
        .collect()
}

/// Run best-first search from the root state.
///
/// All runtime terminations (including contract violations, caught panics, and
/// budget exhaustion) return `Ok(SearchResult)` with the audit trail preserved.
/// The `termination_reason` field in the graph metadata indicates why the search
/// stopped.
///
/// # Errors
///
/// Returns [`SearchError::UnsupportedPolicyMode`] only for pre-flight policy
/// validation failures. No `SearchGraphV1` is produced in this case because
/// no search steps were taken.
pub fn search(
    root_state: sterling_kernel::carrier::bytestate::ByteStateV1,
    world: &dyn SearchWorldV1,
    operator_registry: &OperatorRegistryV1,
    policy: &SearchPolicyV1,
    scorer: &dyn ValueScorer,
    metadata_bindings: &MetadataBindings,
) -> Result<SearchResult, SearchError> {
    search_impl(
        root_state,
        world,
        operator_registry,
        policy,
        scorer,
        metadata_bindings,
        None,
    )
}

/// Run best-first search with streaming tape output.
///
/// Returns both the search result and the completed tape bytes.
/// The tape's `render_graph()` output must be byte-identical to
/// `result.graph.to_canonical_json_bytes()`.
///
/// # Errors
///
/// Returns [`SearchError::TapeWrite`] if tape serialization fails.
/// Returns [`SearchError::UnsupportedPolicyMode`] for pre-flight failures.
pub fn search_with_tape(
    root_state: sterling_kernel::carrier::bytestate::ByteStateV1,
    world: &dyn SearchWorldV1,
    operator_registry: &OperatorRegistryV1,
    policy: &SearchPolicyV1,
    scorer: &dyn ValueScorer,
    metadata_bindings: &MetadataBindings,
) -> Result<(SearchResult, TapeOutput), SearchError> {
    // Build tape header
    let root_fp = canonical_hash(DOMAIN_SEARCH_NODE, &root_state.identity_bytes());
    let header_bytes = build_tape_header(metadata_bindings, root_fp.hex_digest(), policy)?;

    // Estimate tape buffer capacity from policy bounds.
    // Per expansion: ~80 bytes header + ~80 bytes per candidate.
    // Per node creation: ~73 bytes. Nodes ≈ expansions × candidates_per_node.
    // Use checked arithmetic + clamp to avoid OOM from untrusted values.
    let capacity_hint = estimate_tape_capacity(policy);

    let mut writer = TapeWriter::new_with_capacity(&header_bytes, capacity_hint);
    let result = search_impl(
        root_state,
        world,
        operator_registry,
        policy,
        scorer,
        metadata_bindings,
        Some(&mut writer),
    )?;
    let tape_output = writer.finish().map_err(SearchError::TapeWrite)?;
    Ok((result, tape_output))
}

/// Estimate tape buffer capacity from policy bounds.
///
/// Uses checked arithmetic and clamps to [`tape_writer::MAX_PREALLOC_BYTES`].
/// Falls back to 4096 if any overflow occurs (policy values are absurd).
fn estimate_tape_capacity(policy: &SearchPolicyV1) -> usize {
    const BYTES_PER_EXPANSION_HEADER: u64 = 80;
    const BYTES_PER_CANDIDATE: u64 = 80;
    const BYTES_PER_NODE: u64 = 73;
    const FALLBACK: usize = 4096;
    let max_cap = crate::tape_writer::MAX_PREALLOC_BYTES as u64;

    let expansions = policy.max_expansions;
    let cands = policy.max_candidates_per_node;

    // expansion_bytes = expansions * (header + cands * per_cand)
    let per_expansion = cands
        .checked_mul(BYTES_PER_CANDIDATE)
        .and_then(|c| c.checked_add(BYTES_PER_EXPANSION_HEADER));
    let expansion_bytes = per_expansion.and_then(|pe| expansions.checked_mul(pe));

    // node_bytes = expansions * cands * per_node (upper bound: every candidate creates a node)
    let node_count = expansions.checked_mul(cands);
    let node_bytes = node_count.and_then(|nc| nc.checked_mul(BYTES_PER_NODE));

    // total = expansion_bytes + node_bytes + footer
    let total = expansion_bytes
        .and_then(|e| node_bytes.and_then(|n| e.checked_add(n)))
        .and_then(|t| t.checked_add(64)); // footer + termination

    match total {
        Some(t) if t <= max_cap => usize::try_from(t).unwrap_or(FALLBACK),
        Some(_) => crate::tape_writer::MAX_PREALLOC_BYTES,
        None => FALLBACK,
    }
}

/// Build canonical JSON header bytes for the tape.
fn build_tape_header(
    bindings: &MetadataBindings,
    root_fp_hex: &str,
    policy: &SearchPolicyV1,
) -> Result<Vec<u8>, crate::tape::TapeWriteError> {
    let dedup_key = match policy.dedup_key {
        crate::policy::DedupKeyV1::IdentityOnly => "identity_only",
        crate::policy::DedupKeyV1::FullState => "full_state",
    };
    let prune_visited = match policy.prune_visited_policy {
        crate::policy::PruneVisitedPolicyV1::KeepVisited => "keep_visited",
        crate::policy::PruneVisitedPolicyV1::ReleaseVisited => "release_visited",
    };

    let mut obj = serde_json::json!({
        "dedup_key": dedup_key,
        "fixture_digest": bindings.fixture_digest,
        "policy_snapshot_digest": bindings.policy_snapshot_digest,
        "prune_visited_policy": prune_visited,
        "registry_digest": bindings.registry_digest,
        "root_state_fingerprint": root_fp_hex,
        "schema_descriptor": bindings.schema_descriptor,
        "schema_version": "search_tape.v1",
        "search_policy_digest": bindings.search_policy_digest,
        "world_id": bindings.world_id,
    });

    if let Some(ref digest) = bindings.operator_set_digest {
        obj["operator_set_digest"] = serde_json::json!(digest);
    }

    if let Some(ref digest) = bindings.scorer_digest {
        obj["scorer_digest"] = serde_json::json!(digest);
    }

    if let Some(ref digest) = bindings.root_identity_digest {
        obj["root_identity_digest"] = serde_json::json!(digest);
    }
    if let Some(ref digest) = bindings.root_evidence_digest {
        obj["root_evidence_digest"] = serde_json::json!(digest);
    }

    // Canonical JSON (sorted keys, compact)
    sterling_kernel::proof::canon::canonical_json_bytes(&obj)
        .map_err(|e| crate::tape::TapeWriteError::CanonError(e.to_string()))
}

#[allow(clippy::too_many_lines)]
fn search_impl(
    root_state: sterling_kernel::carrier::bytestate::ByteStateV1,
    world: &dyn SearchWorldV1,
    operator_registry: &OperatorRegistryV1,
    policy: &SearchPolicyV1,
    scorer: &dyn ValueScorer,
    metadata_bindings: &MetadataBindings,
    mut tape_sink: Option<&mut dyn TapeSink>,
) -> Result<SearchResult, SearchError> {
    // INV-SC-10: validate M1 policy constraints (pre-flight only)
    policy.validate_m1()?;

    let mut frontier = BestFirstFrontier::new();
    let mut expansions: Vec<ExpandEventV1> = Vec::new();
    let mut expansion_index: HashMap<u64, usize> = HashMap::new();
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

    // Check if root is already a goal (with panic protection)
    let root_is_goal = catch_unwind(AssertUnwindSafe(|| world.is_goal(&root_node.state)));
    match root_is_goal {
        Ok(true) => {
            // Emit root node creation + termination to tape
            if let Some(sink) = &mut tape_sink {
                let fp_raw = content_hash_to_raw(&root_node.state_fingerprint)?;
                sink.on_node_created(&NodeCreationView {
                    node_id: root_node.node_id,
                    parent_id: None,
                    state_fingerprint_raw: fp_raw,
                    depth: root_node.depth,
                    f_cost: root_node.f_cost(),
                    creation_order: root_node.creation_order,
                    node: &root_node,
                })
                .map_err(SearchError::TapeWrite)?;
                sink.on_termination(&TerminationView {
                    termination_reason: TerminationReasonV1::GoalReached { node_id: 0 },
                    frontier_high_water: frontier.high_water(),
                })
                .map_err(SearchError::TapeWrite)?;
            }
            all_nodes.push(root_node.clone());
            let graph = build_graph(
                expansions,
                &expansion_index,
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
        Err(_) => {
            // is_goal panicked on root — preserve evidence
            if let Some(sink) = &mut tape_sink {
                let fp_raw = content_hash_to_raw(&root_node.state_fingerprint)?;
                sink.on_node_created(&NodeCreationView {
                    node_id: root_node.node_id,
                    parent_id: None,
                    state_fingerprint_raw: fp_raw,
                    depth: root_node.depth,
                    f_cost: root_node.f_cost(),
                    creation_order: root_node.creation_order,
                    node: &root_node,
                })
                .map_err(SearchError::TapeWrite)?;
                sink.on_termination(&TerminationView {
                    termination_reason: TerminationReasonV1::InternalPanic {
                        stage: PanicStageV1::IsGoalRoot,
                    },
                    frontier_high_water: frontier.high_water(),
                })
                .map_err(SearchError::TapeWrite)?;
            }
            all_nodes.push(root_node);
            let graph = build_graph(
                expansions,
                &expansion_index,
                &all_nodes,
                TerminationReasonV1::InternalPanic {
                    stage: PanicStageV1::IsGoalRoot,
                },
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
                goal_node: None,
                graph,
                nodes: all_nodes,
            });
        }
        Ok(false) => {} // continue normally
    }

    // Emit root node creation to tape
    if let Some(sink) = &mut tape_sink {
        let fp_raw = content_hash_to_raw(&root_node.state_fingerprint)?;
        sink.on_node_created(&NodeCreationView {
            node_id: root_node.node_id,
            parent_id: None,
            state_fingerprint_raw: fp_raw,
            depth: root_node.depth,
            f_cost: root_node.f_cost(),
            creation_order: root_node.creation_order,
            node: &root_node,
        })
        .map_err(SearchError::TapeWrite)?;
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

        // Pop best node — frontier was checked non-empty above
        let Some(current) = frontier.pop() else {
            termination_reason = TerminationReasonV1::FrontierInvariantViolation {
                stage: FrontierInvariantStageV1::PopFromNonEmptyFrontier,
            };
            break;
        };
        let current_fp_hex = current.state_fingerprint.hex_digest().to_string();
        let pop_key = FrontierPopKeyV1 {
            f_cost: current.f_cost(),
            depth: current.depth,
            creation_order: current.creation_order,
        };

        // Enumerate candidates from world (with panic protection)
        let candidates_result = catch_unwind(AssertUnwindSafe(|| {
            world.enumerate_candidates(&current.state, operator_registry)
        }));

        let Ok(mut candidates) = candidates_result else {
            // enumerate_candidates panicked — record partial expand event
            let panic_expansion = ExpandEventV1 {
                expansion_order: expansion_count,
                node_id: current.node_id,
                state_fingerprint: current_fp_hex,
                frontier_pop_key: pop_key,
                candidates: Vec::new(),
                candidates_truncated: false,
                dead_end_reason: None,
                notes: Vec::new(),
            };
            if let Some(sink) = &mut tape_sink {
                sink.on_expansion(&ExpansionView {
                    expansion: &panic_expansion,
                    state_fingerprint_raw: content_hash_to_raw(&current.state_fingerprint)?,
                })
                .map_err(SearchError::TapeWrite)?;
            }
            expansions.push(panic_expansion);
            // Index for O(1) lookup in build_graph; first-wins matches .find() semantics.
            expansion_index
                .entry(current.node_id)
                .or_insert(expansions.len() - 1);
            termination_reason = TerminationReasonV1::InternalPanic {
                stage: PanicStageV1::EnumerateCandidates,
            };
            break;
        };

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

        // Score candidates (with panic protection)
        #[allow(clippy::similar_names)]
        let scoring_output = catch_unwind(AssertUnwindSafe(|| {
            scorer.score_candidates(&current, &candidates)
        }));

        let candidate_scores = match scoring_output {
            Ok(cs) if cs.len() == candidates.len() => cs,
            Ok(cs) => {
                // Scorer returned wrong arity — record expansion with candidate identity
                let actual_len = cs.len() as u64;
                total_candidates_generated += candidates.len() as u64;
                let arity_expansion = ExpandEventV1 {
                    expansion_order: expansion_count,
                    node_id: current.node_id,
                    state_fingerprint: current_fp_hex,
                    frontier_pop_key: pop_key,
                    candidates: build_not_evaluated_records(&candidates),
                    candidates_truncated,
                    dead_end_reason: None,
                    notes,
                };
                if let Some(sink) = &mut tape_sink {
                    sink.on_expansion(&ExpansionView {
                        expansion: &arity_expansion,
                        state_fingerprint_raw: content_hash_to_raw(&current.state_fingerprint)?,
                    })
                    .map_err(SearchError::TapeWrite)?;
                }
                expansions.push(arity_expansion);
                // Index for O(1) lookup in build_graph; first-wins matches .find() semantics.
                expansion_index
                    .entry(current.node_id)
                    .or_insert(expansions.len() - 1);
                termination_reason = TerminationReasonV1::ScorerContractViolation {
                    expected: candidates.len() as u64,
                    actual: actual_len,
                };
                break;
            }
            Err(_) => {
                // Scorer panicked — record expansion with candidate identity
                total_candidates_generated += candidates.len() as u64;
                let scorer_panic_expansion = ExpandEventV1 {
                    expansion_order: expansion_count,
                    node_id: current.node_id,
                    state_fingerprint: current_fp_hex,
                    frontier_pop_key: pop_key,
                    candidates: build_not_evaluated_records(&candidates),
                    candidates_truncated,
                    dead_end_reason: None,
                    notes,
                };
                if let Some(sink) = &mut tape_sink {
                    sink.on_expansion(&ExpansionView {
                        expansion: &scorer_panic_expansion,
                        state_fingerprint_raw: content_hash_to_raw(&current.state_fingerprint)?,
                    })
                    .map_err(SearchError::TapeWrite)?;
                }
                expansions.push(scorer_panic_expansion);
                // Index for O(1) lookup in build_graph; first-wins matches .find() semantics.
                expansion_index
                    .entry(current.node_id)
                    .or_insert(expansions.len() - 1);
                termination_reason = TerminationReasonV1::InternalPanic {
                    stage: PanicStageV1::ScoreCandidates,
                };
                break;
            }
        };

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
        let mut contract_violation = false;

        for (sorted_idx, &(_orig_idx, candidate, score)) in scored_candidates.iter().enumerate() {
            // INV-SC-02: check candidate legality
            if !operator_registry.contains(&candidate.op_code) {
                candidate_records.push(CandidateRecordV1 {
                    index: sorted_idx as u64,
                    action: candidate.clone(),
                    score: score.clone(),
                    outcome: CandidateOutcomeV1::IllegalOperator,
                });
                contract_violation = true;
                break; // exit candidate loop
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
            let apply_result = apply::apply(
                &current.state,
                candidate.op_code,
                &candidate.op_args,
                operator_registry,
            );

            match apply_result {
                Err(fail) => {
                    let kind = match &fail {
                        apply::ApplyFailure::PreconditionNotMet { .. }
                        | apply::ApplyFailure::EffectContractViolation { .. } => {
                            ApplyFailureKindV1::PreconditionNotMet
                        }
                        apply::ApplyFailure::ArgumentMismatch { .. } => {
                            ApplyFailureKindV1::ArgumentMismatch
                        }
                        apply::ApplyFailure::UnknownOperator { .. }
                        | apply::ApplyFailure::OperatorNotImplemented { .. } => {
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

                    // Check goal before pushing to frontier (with panic protection)
                    let is_goal_result =
                        catch_unwind(AssertUnwindSafe(|| world.is_goal(&child.state)));
                    match is_goal_result {
                        Ok(true) => {
                            found_goal = true;
                            goal_node_id = child_node_id;
                        }
                        Ok(false) => {}
                        Err(_) => {
                            // is_goal panicked during expansion — record and terminate
                            // Emit child node creation to tape
                            if let Some(sink) = &mut tape_sink {
                                let child_fp_raw = content_hash_to_raw(&child.state_fingerprint)?;
                                sink.on_node_created(&NodeCreationView {
                                    node_id: child.node_id,
                                    parent_id: child.parent_id,
                                    state_fingerprint_raw: child_fp_raw,
                                    depth: child.depth,
                                    f_cost: child.f_cost(),
                                    creation_order: child.creation_order,
                                    node: &child,
                                })
                                .map_err(SearchError::TapeWrite)?;
                            }

                            all_nodes.push(child.clone());
                            frontier.push(child);

                            // Record expansion event before terminating
                            let panic_expansion = ExpandEventV1 {
                                expansion_order: expansion_count,
                                node_id: current.node_id,
                                state_fingerprint: current_fp_hex.clone(),
                                frontier_pop_key: pop_key,
                                candidates: candidate_records,
                                candidates_truncated,
                                dead_end_reason: None,
                                notes,
                            };

                            // Emit expansion + termination to tape
                            if let Some(sink) = &mut tape_sink {
                                sink.on_expansion(&ExpansionView {
                                    expansion: &panic_expansion,
                                    state_fingerprint_raw: content_hash_to_raw(
                                        &current.state_fingerprint,
                                    )?,
                                })
                                .map_err(SearchError::TapeWrite)?;
                                sink.on_termination(&TerminationView {
                                    termination_reason: TerminationReasonV1::InternalPanic {
                                        stage: PanicStageV1::IsGoalExpansion,
                                    },
                                    frontier_high_water: frontier.high_water(),
                                })
                                .map_err(SearchError::TapeWrite)?;
                            }

                            expansions.push(panic_expansion);
                            // Index for O(1) lookup in build_graph; first-wins matches .find() semantics.
                            expansion_index
                                .entry(current.node_id)
                                .or_insert(expansions.len() - 1);

                            termination_reason = TerminationReasonV1::InternalPanic {
                                stage: PanicStageV1::IsGoalExpansion,
                            };
                            // Use a nested return to exit both for-loop and main loop
                            let goal_node = None;
                            let graph = build_graph(
                                expansions,
                                &expansion_index,
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
                            return Ok(SearchResult {
                                goal_node,
                                graph,
                                nodes: all_nodes,
                            });
                        }
                    }

                    // Emit child node creation to tape
                    if let Some(sink) = &mut tape_sink {
                        let child_fp_raw = content_hash_to_raw(&child.state_fingerprint)?;
                        sink.on_node_created(&NodeCreationView {
                            node_id: child.node_id,
                            parent_id: child.parent_id,
                            state_fingerprint_raw: child_fp_raw,
                            depth: child.depth,
                            f_cost: child.f_cost(),
                            creation_order: child.creation_order,
                            node: &child,
                        })
                        .map_err(SearchError::TapeWrite)?;
                    }

                    all_nodes.push(child.clone());
                    frontier.push(child);
                    children_created += 1;
                }
            }
        }

        // If WorldContractViolation, record expansion and terminate
        if contract_violation {
            let violation_expansion = ExpandEventV1 {
                expansion_order: expansion_count,
                node_id: current.node_id,
                state_fingerprint: current_fp_hex,
                frontier_pop_key: pop_key,
                candidates: candidate_records,
                candidates_truncated,
                dead_end_reason: None,
                notes,
            };
            if let Some(sink) = &mut tape_sink {
                sink.on_expansion(&ExpansionView {
                    expansion: &violation_expansion,
                    state_fingerprint_raw: content_hash_to_raw(&current.state_fingerprint)?,
                })
                .map_err(SearchError::TapeWrite)?;
            }
            expansions.push(violation_expansion);
            // Index for O(1) lookup in build_graph; first-wins matches .find() semantics.
            expansion_index
                .entry(current.node_id)
                .or_insert(expansions.len() - 1);
            termination_reason = TerminationReasonV1::WorldContractViolation;
            break;
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
        let normal_expansion = ExpandEventV1 {
            expansion_order: expansion_count,
            node_id: current.node_id,
            state_fingerprint: current_fp_hex,
            frontier_pop_key: pop_key,
            candidates: candidate_records,
            candidates_truncated,
            dead_end_reason,
            notes,
        };
        if let Some(sink) = &mut tape_sink {
            sink.on_expansion(&ExpansionView {
                expansion: &normal_expansion,
                state_fingerprint_raw: content_hash_to_raw(&current.state_fingerprint)?,
            })
            .map_err(SearchError::TapeWrite)?;
        }
        expansions.push(normal_expansion);
        // Index for O(1) lookup in build_graph; first-wins matches .find() semantics.
        expansion_index
            .entry(current.node_id)
            .or_insert(expansions.len() - 1);
        expansion_count += 1;

        // If goal was found, terminate
        if found_goal {
            termination_reason = TerminationReasonV1::GoalReached {
                node_id: goal_node_id,
            };
            break;
        }
    }

    // Emit termination to tape (for all break-path terminations)
    if let Some(sink) = &mut tape_sink {
        sink.on_termination(&TerminationView {
            termination_reason: termination_reason.clone(),
            frontier_high_water: frontier.high_water(),
        })
        .map_err(SearchError::TapeWrite)?;
    }

    let goal_node = match &termination_reason {
        TerminationReasonV1::GoalReached { node_id } => {
            all_nodes.iter().find(|n| n.node_id == *node_id).cloned()
        }
        _ => None,
    };

    let graph = build_graph(
        expansions,
        &expansion_index,
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
    /// `fixture.json` artifact content-hash digest (raw hex, always present).
    pub fixture_digest: String,
    /// Scorer artifact digest (Table mode only; `None` for Uniform).
    pub scorer_digest: Option<String>,
    /// Operator registry content-hash digest (`None` until M2b wires it).
    pub operator_set_digest: Option<String>,
    /// Root `ByteStateV1` identity plane digest (raw hex). `None` for pre-IDCOH-001 bundles.
    pub root_identity_digest: Option<String>,
    /// Root `ByteStateV1` evidence plane digest (raw hex). `None` for pre-IDCOH-001 bundles.
    pub root_evidence_digest: Option<String>,
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
    expansion_index: &HashMap<u64, usize>,
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
    // O(N) via expansion_index lookup instead of O(N×E) linear scan.
    let mut node_summaries: Vec<SearchGraphNodeSummaryV1> = all_nodes
        .iter()
        .map(|n| {
            let exp = expansion_index.get(&n.node_id).map(|&idx| &expansions[idx]);
            let expansion_order = exp.map(|e| e.expansion_order);
            let dead_end_reason = exp.and_then(|e| e.dead_end_reason);
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
            fixture_digest: bindings.fixture_digest.clone(),
            scorer_digest: bindings.scorer_digest.clone(),
            operator_set_digest: bindings.operator_set_digest.clone(),
            root_identity_digest: bindings.root_identity_digest.clone(),
            root_evidence_digest: bindings.root_evidence_digest.clone(),
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
