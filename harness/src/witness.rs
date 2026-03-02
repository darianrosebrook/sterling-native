//! Winning-path replay witness.
//!
//! `replay_winning_path()` is a generic verification primitive that
//! re-executes the goal-path operator sequence from a compiled root
//! `ByteState`, verifying state fingerprints at every step and invoking
//! a world-specific [`ReplayInvariantChecker`] for semantic checks.
//!
//! This facility is Cert-only, gated by `evidence_obligations` containing
//! `"winning_path_replay_v1"`. It lives in harness (verifier side), not
//! search (recorder side).
//!
//! # Inputs
//!
//! All inputs come from the bundle — no out-of-band access:
//! - Compiled root `ByteStateV1` (from compilation replay)
//! - Parsed `SearchTapeV1` (goal path edges: parent chain + `op_code`/`op_args`)
//! - `OperatorRegistryV1` (already verified by bundle Steps 16-17)
//! - World-specific `ReplayInvariantChecker` (trait callback)

use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::operators::apply::apply;
use sterling_kernel::operators::operator_registry::OperatorRegistryV1;
use sterling_kernel::proof::hash::{canonical_hash, HashDomain};
use sterling_search::tape::{
    SearchTapeV1, TapeCandidateOutcomeV1, TapeRecordV1,
};

/// Evidence obligation string that gates winning-path replay.
pub const OBLIGATION_WINNING_PATH_REPLAY: &str = "winning_path_replay_v1";

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors produced by winning-path replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// No `GoalReached` termination in tape — replay is vacuously ok.
    NoGoalReached,

    /// Goal node ID not found among tape `NodeCreation` records.
    GoalNodeNotFound { node_id: u64 },

    /// A node on the path has no `NodeCreation` record in the tape.
    PathNodeMissing { node_id: u64 },

    /// No `Applied` candidate in the parent's expansion produces the child.
    ReplayEdgeMissing { parent_node: u64, child_node: u64 },

    /// More than one `Applied` candidate produces the same child.
    ReplayEdgeAmbiguous {
        parent_node: u64,
        child_node: u64,
        count: usize,
    },

    /// No expansion record found for a node on the winning path.
    ExpansionMissing { node_id: u64 },

    /// `apply()` failed when re-executing an operator.
    ReplayApplyFailed { step_index: usize, detail: String },

    /// State fingerprint after apply does not match the tape's recorded
    /// fingerprint for the child node.
    ReplayFingerprintMismatch {
        step_index: usize,
        expected: String,
        actual: String,
    },

    /// A world-specific invariant was violated.
    InvariantViolation {
        step_index: usize,
        detail: String,
    },
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoGoalReached => write!(f, "no GoalReached termination in tape"),
            Self::GoalNodeNotFound { node_id } => {
                write!(f, "goal node {node_id} not found in tape NodeCreation records")
            }
            Self::PathNodeMissing { node_id } => {
                write!(f, "path node {node_id} has no NodeCreation record")
            }
            Self::ReplayEdgeMissing {
                parent_node,
                child_node,
            } => write!(
                f,
                "no Applied candidate in expansion of node {parent_node} produces child {child_node}"
            ),
            Self::ReplayEdgeAmbiguous {
                parent_node,
                child_node,
                count,
            } => write!(
                f,
                "{count} Applied candidates in expansion of node {parent_node} produce child {child_node}"
            ),
            Self::ExpansionMissing { node_id } => {
                write!(f, "no expansion record for winning-path node {node_id}")
            }
            Self::ReplayApplyFailed { step_index, detail } => {
                write!(f, "apply failed at step {step_index}: {detail}")
            }
            Self::ReplayFingerprintMismatch {
                step_index,
                expected,
                actual,
            } => write!(
                f,
                "fingerprint mismatch at step {step_index}: expected {expected}, got {actual}"
            ),
            Self::InvariantViolation { step_index, detail } => {
                write!(f, "invariant violation at step {step_index}: {detail}")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

// ---------------------------------------------------------------------------
// Invariant checker trait
// ---------------------------------------------------------------------------

/// World-specific invariant checker invoked at each step of winning-path
/// replay.
///
/// Implementors verify semantic properties that the kernel cannot check
/// (e.g., feedback correctness, belief monotonicity). The checker receives
/// pre-state and post-state at each step, along with the operator that was
/// applied.
pub trait ReplayInvariantChecker {
    /// Check invariants for one replay step.
    ///
    /// Called after `apply()` succeeds and fingerprint is verified.
    ///
    /// - `step_index`: 0-based position in the winning-path edge sequence
    /// - `pre_state`: state before the operator was applied
    /// - `post_state`: state after the operator was applied
    /// - `op_code`: the operator code that was applied
    /// - `op_args`: the operator arguments
    ///
    /// # Errors
    ///
    /// Returns an error description if any invariant is violated.
    fn check(
        &mut self,
        step_index: usize,
        pre_state: &ByteStateV1,
        post_state: &ByteStateV1,
        op_code: Code32,
        op_args: &[u8],
    ) -> Result<(), String>;
}

/// A no-op invariant checker that accepts all steps. Useful for worlds
/// that opt into replay but have no world-specific invariants beyond
/// fingerprint verification.
pub struct NoopInvariantChecker;

impl ReplayInvariantChecker for NoopInvariantChecker {
    fn check(
        &mut self,
        _step_index: usize,
        _pre_state: &ByteStateV1,
        _post_state: &ByteStateV1,
        _op_code: Code32,
        _op_args: &[u8],
    ) -> Result<(), String> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Replay result
// ---------------------------------------------------------------------------

/// Successful replay result, carrying the final state and step count.
#[derive(Debug, Clone)]
pub struct ReplayResult {
    /// The final `ByteState` after replaying all winning-path edges.
    pub final_state: ByteStateV1,
    /// Number of edges (apply steps) on the winning path.
    pub step_count: usize,
    /// The winning-path node IDs from root to goal (inclusive).
    pub path: Vec<u64>,
}

// ---------------------------------------------------------------------------
// Core replay function
// ---------------------------------------------------------------------------

/// Replay the winning path from a compiled root state.
///
/// Extracts the goal path from the tape, re-applies each operator in
/// sequence, verifies state fingerprints at every step, and invokes the
/// invariant checker for world-specific semantic checks.
///
/// Returns `Err(ReplayError::NoGoalReached)` if the tape has no
/// `GoalReached` termination — this is not a verification failure, just
/// an indication that replay is not applicable (caller should treat as Ok).
///
/// # Errors
///
/// Returns [`ReplayError`] if any replay step fails.
pub fn replay_winning_path(
    tape: &SearchTapeV1,
    root_state: &ByteStateV1,
    registry: &OperatorRegistryV1,
    checker: &mut dyn ReplayInvariantChecker,
) -> Result<ReplayResult, ReplayError> {
    // Step 1: Find the goal node ID from the termination record.
    let goal_node_id = find_goal_node_id(tape)?;

    // Step 2: Reconstruct the goal path (root → ... → goal).
    let path = reconstruct_path_from_tape(tape, goal_node_id)?;

    // Step 3: For each edge, extract the unique (op_code, op_args).
    let edges = extract_edges(tape, &path)?;

    // Step 4: Replay from root state.
    let mut current_state = root_state.clone();

    // Verify root fingerprint matches the tape's root NodeCreation.
    verify_node_fingerprint(tape, path[0], &current_state, 0)?;

    for (step_index, edge) in edges.iter().enumerate() {
        let pre_state = current_state.clone();

        // Apply the operator.
        let (new_state, _record) = apply(&current_state, edge.op_code, &edge.op_args, registry)
            .map_err(|e| ReplayError::ReplayApplyFailed {
                step_index,
                detail: format!("{e:?}"),
            })?;

        current_state = new_state;

        // Verify fingerprint of the resulting state matches the child node.
        let child_node_id = path[step_index + 1];
        verify_node_fingerprint(tape, child_node_id, &current_state, step_index)?;

        // Invoke the invariant checker.
        checker
            .check(
                step_index,
                &pre_state,
                &current_state,
                edge.op_code,
                &edge.op_args,
            )
            .map_err(|detail| ReplayError::InvariantViolation {
                step_index,
                detail,
            })?;
    }

    Ok(ReplayResult {
        final_state: current_state,
        step_count: edges.len(),
        path,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// An edge on the winning path: the operator that transitions parent → child.
struct ReplayEdge {
    op_code: Code32,
    op_args: Vec<u8>,
}

/// Find the goal node ID from the tape's termination record.
fn find_goal_node_id(tape: &SearchTapeV1) -> Result<u64, ReplayError> {
    use sterling_search::graph::TerminationReasonV1;

    for record in &tape.records {
        if let TapeRecordV1::Termination(term) = record {
            if let TerminationReasonV1::GoalReached { node_id } = term.reason {
                return Ok(node_id);
            }
        }
    }
    Err(ReplayError::NoGoalReached)
}

/// Reconstruct the path from root to goal using tape `NodeCreation` records.
///
/// Walks backwards from `goal_node_id` via `parent_id` links, then reverses.
fn reconstruct_path_from_tape(
    tape: &SearchTapeV1,
    goal_node_id: u64,
) -> Result<Vec<u64>, ReplayError> {
    // Build a map: node_id → parent_id
    let mut parent_map = std::collections::HashMap::new();
    for record in &tape.records {
        if let TapeRecordV1::NodeCreation(nc) = record {
            parent_map.insert(nc.node_id, nc.parent_id);
        }
    }

    let mut path = Vec::new();
    let mut current = Some(goal_node_id);

    while let Some(id) = current {
        if path.contains(&id) {
            // Cycle detected — should never happen in a valid tape.
            return Err(ReplayError::PathNodeMissing { node_id: id });
        }
        path.push(id);
        current = *parent_map
            .get(&id)
            .ok_or(ReplayError::PathNodeMissing { node_id: id })?;
    }

    path.reverse();
    Ok(path)
}

/// Extract the unique `(op_code, op_args)` for each edge on the winning path.
///
/// For each parent→child pair, finds the expansion record for the parent,
/// then finds the unique `Applied { to_node: child }` candidate.
fn extract_edges(
    tape: &SearchTapeV1,
    path: &[u64],
) -> Result<Vec<ReplayEdge>, ReplayError> {
    if path.len() < 2 {
        return Ok(Vec::new());
    }

    let mut edges = Vec::with_capacity(path.len() - 1);

    for window in path.windows(2) {
        let parent = window[0];
        let child = window[1];

        // Find expansion record for this parent.
        let expansion = tape
            .records
            .iter()
            .find_map(|r| match r {
                TapeRecordV1::Expansion(exp) if exp.node_id == parent => Some(exp),
                _ => None,
            })
            .ok_or(ReplayError::ExpansionMissing { node_id: parent })?;

        // Find candidates with Applied { to_node: child }.
        let matching: Vec<_> = expansion
            .candidates
            .iter()
            .filter(|c| matches!(c.outcome, TapeCandidateOutcomeV1::Applied { to_node } if to_node == child))
            .collect();

        match matching.len() {
            0 => {
                return Err(ReplayError::ReplayEdgeMissing {
                    parent_node: parent,
                    child_node: child,
                });
            }
            1 => {
                let candidate = matching[0];
                edges.push(ReplayEdge {
                    op_code: Code32::from_le_bytes(candidate.op_code_bytes),
                    op_args: candidate.op_args.clone(),
                });
            }
            n => {
                return Err(ReplayError::ReplayEdgeAmbiguous {
                    parent_node: parent,
                    child_node: child,
                    count: n,
                });
            }
        }
    }

    Ok(edges)
}

/// Compute the state fingerprint (same as search: `canonical_hash(SearchNode, identity_bytes)`).
fn compute_fingerprint(state: &ByteStateV1) -> [u8; 32] {
    let hash = canonical_hash(HashDomain::SearchNode, &state.identity_bytes());
    // Convert hex digest to raw bytes.
    let hex = hash.hex_digest();
    let mut bytes = [0u8; 32];
    // hex::decode_to_slice is available via the hex crate.
    hex::decode_to_slice(hex, &mut bytes)
        .expect("canonical_hash produces valid hex; this is a bug if it fails");
    bytes
}

/// Verify that a node's fingerprint in the tape matches the computed state fingerprint.
fn verify_node_fingerprint(
    tape: &SearchTapeV1,
    node_id: u64,
    state: &ByteStateV1,
    step_index: usize,
) -> Result<(), ReplayError> {
    // Find the NodeCreation record for this node.
    let expected_fp = tape
        .records
        .iter()
        .find_map(|r| match r {
            TapeRecordV1::NodeCreation(nc) if nc.node_id == node_id => {
                Some(nc.state_fingerprint)
            }
            _ => None,
        })
        .ok_or(ReplayError::PathNodeMissing { node_id })?;

    let actual_fp = compute_fingerprint(state);

    if expected_fp != actual_fp {
        return Err(ReplayError::ReplayFingerprintMismatch {
            step_index,
            expected: hex::encode(expected_fp),
            actual: hex::encode(actual_fp),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sterling_kernel::carrier::bytestate::ByteStateV1;
    use sterling_kernel::carrier::code32::Code32;
    use sterling_kernel::operators::apply::{
        set_slot_args, OP_SET_SLOT,
    };
    use sterling_kernel::operators::operator_registry::kernel_operator_registry;
    use sterling_search::graph::TerminationReasonV1;
    use sterling_search::tape::{
        SearchTapeFooterV1, SearchTapeHeaderV1, SearchTapeV1,
        TapeCandidateOutcomeV1, TapeCandidateV1, TapeExpansionV1,
        TapeNodeCreationV1, TapeRecordV1, TapeScoreSourceV1, TapeTerminationV1,
    };

    /// Build a minimal `ByteState`: 1 layer, `n_slots` slots, all Hole.
    fn make_state(n_slots: usize) -> ByteStateV1 {
        ByteStateV1::new(1, n_slots)
    }

    fn fp_bytes(state: &ByteStateV1) -> [u8; 32] {
        compute_fingerprint(state)
    }

    fn make_candidate(
        index: u64,
        op_code: Code32,
        op_args: Vec<u8>,
        outcome: TapeCandidateOutcomeV1,
    ) -> TapeCandidateV1 {
        TapeCandidateV1 {
            index,
            op_code_bytes: op_code.to_le_bytes(),
            op_args,
            canonical_hash: [0u8; 32],
            score_bonus: 0,
            score_source: TapeScoreSourceV1::Uniform,
            outcome,
        }
    }

    fn tape_header() -> SearchTapeHeaderV1 {
        SearchTapeHeaderV1 {
            json_bytes: b"{}".to_vec(),
            json: serde_json::json!({}),
        }
    }

    fn tape_footer() -> SearchTapeFooterV1 {
        SearchTapeFooterV1 {
            record_count: 0,
            final_chain_hash: [0u8; 32],
        }
    }

    /// Build a minimal tape with a single `SET_SLOT` edge: root(0) → child(1).
    fn build_single_edge_tape() -> (ByteStateV1, SearchTapeV1) {
        let registry = kernel_operator_registry();
        let state0 = make_state(4);
        let fp0 = fp_bytes(&state0);

        let value = Code32::new(5, 0, 1);
        let args = set_slot_args(0, 0, value);
        let (state1, _) = apply(&state0, OP_SET_SLOT, &args, &registry).unwrap();
        let fp1 = fp_bytes(&state1);

        let tape = SearchTapeV1 {
            header: tape_header(),
            records: vec![
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 0,
                    parent_id: None,
                    state_fingerprint: fp0,
                    depth: 0,
                    f_cost: 0,
                    creation_order: 0,
                }),
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 1,
                    parent_id: Some(0),
                    state_fingerprint: fp1,
                    depth: 1,
                    f_cost: 0,
                    creation_order: 1,
                }),
                TapeRecordV1::Expansion(TapeExpansionV1 {
                    expansion_order: 0,
                    node_id: 0,
                    state_fingerprint: fp0,
                    pop_f_cost: 0,
                    pop_depth: 0,
                    pop_creation_order: 0,
                    candidates_truncated: false,
                    dead_end_reason: None,
                    candidates: vec![make_candidate(
                        0,
                        OP_SET_SLOT,
                        args,
                        TapeCandidateOutcomeV1::Applied { to_node: 1 },
                    )],
                    notes: vec![],
                }),
                TapeRecordV1::Termination(TapeTerminationV1 {
                    reason: TerminationReasonV1::GoalReached { node_id: 1 },
                    frontier_high_water: 1,
                }),
            ],
            footer: tape_footer(),
        };

        (state0, tape)
    }

    #[test]
    fn replay_single_edge_succeeds() {
        let registry = kernel_operator_registry();
        let (root, tape) = build_single_edge_tape();
        let mut checker = NoopInvariantChecker;

        let result =
            replay_winning_path(&tape, &root, &registry, &mut checker).unwrap();
        assert_eq!(result.step_count, 1);
        assert_eq!(result.path, vec![0, 1]);
    }

    #[test]
    fn replay_no_goal_returns_no_goal_reached() {
        let tape = SearchTapeV1 {
            header: tape_header(),
            records: vec![TapeRecordV1::Termination(TapeTerminationV1 {
                reason: TerminationReasonV1::FrontierExhausted,
                frontier_high_water: 0,
            })],
            footer: tape_footer(),
        };
        let state = make_state(4);
        let registry = kernel_operator_registry();
        let mut checker = NoopInvariantChecker;

        let err = replay_winning_path(&tape, &state, &registry, &mut checker).unwrap_err();
        assert_eq!(err, ReplayError::NoGoalReached);
    }

    #[test]
    fn replay_edge_missing_detected() {
        let state0 = make_state(4);
        let fp0 = fp_bytes(&state0);

        // Node 1 exists but the expansion of node 0 has no Applied → 1.
        let tape = SearchTapeV1 {
            header: tape_header(),
            records: vec![
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 0,
                    parent_id: None,
                    state_fingerprint: fp0,
                    depth: 0,
                    f_cost: 0,
                    creation_order: 0,
                }),
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 1,
                    parent_id: Some(0),
                    state_fingerprint: [0u8; 32],
                    depth: 1,
                    f_cost: 0,
                    creation_order: 1,
                }),
                TapeRecordV1::Expansion(TapeExpansionV1 {
                    expansion_order: 0,
                    node_id: 0,
                    state_fingerprint: fp0,
                    pop_f_cost: 0,
                    pop_depth: 0,
                    pop_creation_order: 0,
                    candidates_truncated: false,
                    dead_end_reason: None,
                    candidates: vec![], // No candidates!
                    notes: vec![],
                }),
                TapeRecordV1::Termination(TapeTerminationV1 {
                    reason: TerminationReasonV1::GoalReached { node_id: 1 },
                    frontier_high_water: 1,
                }),
            ],
            footer: tape_footer(),
        };
        let registry = kernel_operator_registry();
        let mut checker = NoopInvariantChecker;

        let err = replay_winning_path(&tape, &state0, &registry, &mut checker).unwrap_err();
        assert!(matches!(
            err,
            ReplayError::ReplayEdgeMissing {
                parent_node: 0,
                child_node: 1
            }
        ));
    }

    #[test]
    fn replay_edge_ambiguous_detected() {
        let registry = kernel_operator_registry();
        let state0 = make_state(4);
        let fp0 = fp_bytes(&state0);

        let value = Code32::new(5, 0, 1);
        let args = set_slot_args(0, 0, value);
        let (state1, _) = apply(&state0, OP_SET_SLOT, &args, &registry).unwrap();
        let fp1 = fp_bytes(&state1);

        // Two candidates both claim Applied { to_node: 1 }.
        let tape = SearchTapeV1 {
            header: tape_header(),
            records: vec![
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 0,
                    parent_id: None,
                    state_fingerprint: fp0,
                    depth: 0,
                    f_cost: 0,
                    creation_order: 0,
                }),
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 1,
                    parent_id: Some(0),
                    state_fingerprint: fp1,
                    depth: 1,
                    f_cost: 0,
                    creation_order: 1,
                }),
                TapeRecordV1::Expansion(TapeExpansionV1 {
                    expansion_order: 0,
                    node_id: 0,
                    state_fingerprint: fp0,
                    pop_f_cost: 0,
                    pop_depth: 0,
                    pop_creation_order: 0,
                    candidates_truncated: false,
                    dead_end_reason: None,
                    candidates: vec![
                        make_candidate(
                            0,
                            OP_SET_SLOT,
                            args.clone(),
                            TapeCandidateOutcomeV1::Applied { to_node: 1 },
                        ),
                        make_candidate(
                            1,
                            OP_SET_SLOT,
                            args,
                            TapeCandidateOutcomeV1::Applied { to_node: 1 },
                        ),
                    ],
                    notes: vec![],
                }),
                TapeRecordV1::Termination(TapeTerminationV1 {
                    reason: TerminationReasonV1::GoalReached { node_id: 1 },
                    frontier_high_water: 1,
                }),
            ],
            footer: tape_footer(),
        };
        let mut checker = NoopInvariantChecker;

        let err = replay_winning_path(&tape, &state0, &registry, &mut checker).unwrap_err();
        assert!(matches!(
            err,
            ReplayError::ReplayEdgeAmbiguous {
                parent_node: 0,
                child_node: 1,
                count: 2
            }
        ));
    }

    #[test]
    fn replay_fingerprint_mismatch_detected() {
        let registry = kernel_operator_registry();
        let state0 = make_state(4);
        let fp0 = fp_bytes(&state0);

        let value = Code32::new(5, 0, 1);
        let args = set_slot_args(0, 0, value);

        // Record a wrong fingerprint for node 1.
        let tape = SearchTapeV1 {
            header: tape_header(),
            records: vec![
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 0,
                    parent_id: None,
                    state_fingerprint: fp0,
                    depth: 0,
                    f_cost: 0,
                    creation_order: 0,
                }),
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 1,
                    parent_id: Some(0),
                    state_fingerprint: [0xFFu8; 32], // Wrong fingerprint!
                    depth: 1,
                    f_cost: 0,
                    creation_order: 1,
                }),
                TapeRecordV1::Expansion(TapeExpansionV1 {
                    expansion_order: 0,
                    node_id: 0,
                    state_fingerprint: fp0,
                    pop_f_cost: 0,
                    pop_depth: 0,
                    pop_creation_order: 0,
                    candidates_truncated: false,
                    dead_end_reason: None,
                    candidates: vec![make_candidate(
                        0,
                        OP_SET_SLOT,
                        args,
                        TapeCandidateOutcomeV1::Applied { to_node: 1 },
                    )],
                    notes: vec![],
                }),
                TapeRecordV1::Termination(TapeTerminationV1 {
                    reason: TerminationReasonV1::GoalReached { node_id: 1 },
                    frontier_high_water: 1,
                }),
            ],
            footer: tape_footer(),
        };
        let mut checker = NoopInvariantChecker;

        let err = replay_winning_path(&tape, &state0, &registry, &mut checker).unwrap_err();
        assert!(matches!(
            err,
            ReplayError::ReplayFingerprintMismatch { step_index: 0, .. }
        ));
    }

    struct CountingChecker {
        count: usize,
    }

    impl ReplayInvariantChecker for CountingChecker {
        fn check(
            &mut self,
            _step_index: usize,
            _pre_state: &ByteStateV1,
            _post_state: &ByteStateV1,
            _op_code: Code32,
            _op_args: &[u8],
        ) -> Result<(), String> {
            self.count += 1;
            Ok(())
        }
    }

    struct FailingChecker;

    impl ReplayInvariantChecker for FailingChecker {
        fn check(
            &mut self,
            _step_index: usize,
            _pre_state: &ByteStateV1,
            _post_state: &ByteStateV1,
            _op_code: Code32,
            _op_args: &[u8],
        ) -> Result<(), String> {
            Err("test invariant violation".into())
        }
    }

    #[test]
    fn replay_invariant_checker_called() {
        let registry = kernel_operator_registry();
        let (root, tape) = build_single_edge_tape();

        let mut checker = CountingChecker { count: 0 };
        let result =
            replay_winning_path(&tape, &root, &registry, &mut checker).unwrap();
        assert_eq!(result.step_count, 1);
        assert_eq!(checker.count, 1);
    }

    #[test]
    fn replay_invariant_checker_failure_reported() {
        let registry = kernel_operator_registry();
        let (root, tape) = build_single_edge_tape();

        let mut checker = FailingChecker;
        let err = replay_winning_path(&tape, &root, &registry, &mut checker).unwrap_err();
        assert!(matches!(
            err,
            ReplayError::InvariantViolation {
                step_index: 0,
                ..
            }
        ));
    }

    #[test]
    fn replay_multi_edge_path() {
        let registry = kernel_operator_registry();
        let state0 = make_state(4);
        let fp0 = fp_bytes(&state0);

        // Edge 0→1: set slot 0
        let v1 = Code32::new(5, 0, 1);
        let args1 = set_slot_args(0, 0, v1);
        let (state1, _) = apply(&state0, OP_SET_SLOT, &args1, &registry).unwrap();
        let fp1 = fp_bytes(&state1);

        // Edge 1→2: set slot 1
        let v2 = Code32::new(5, 0, 2);
        let args2 = set_slot_args(0, 1, v2);
        let (state2, _) = apply(&state1, OP_SET_SLOT, &args2, &registry).unwrap();
        let fp2 = fp_bytes(&state2);

        let tape = SearchTapeV1 {
            header: tape_header(),
            records: vec![
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 0,
                    parent_id: None,
                    state_fingerprint: fp0,
                    depth: 0,
                    f_cost: 0,
                    creation_order: 0,
                }),
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 1,
                    parent_id: Some(0),
                    state_fingerprint: fp1,
                    depth: 1,
                    f_cost: 0,
                    creation_order: 1,
                }),
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 2,
                    parent_id: Some(1),
                    state_fingerprint: fp2,
                    depth: 2,
                    f_cost: 0,
                    creation_order: 2,
                }),
                TapeRecordV1::Expansion(TapeExpansionV1 {
                    expansion_order: 0,
                    node_id: 0,
                    state_fingerprint: fp0,
                    pop_f_cost: 0,
                    pop_depth: 0,
                    pop_creation_order: 0,
                    candidates_truncated: false,
                    dead_end_reason: None,
                    candidates: vec![make_candidate(
                        0,
                        OP_SET_SLOT,
                        args1,
                        TapeCandidateOutcomeV1::Applied { to_node: 1 },
                    )],
                    notes: vec![],
                }),
                TapeRecordV1::Expansion(TapeExpansionV1 {
                    expansion_order: 1,
                    node_id: 1,
                    state_fingerprint: fp1,
                    pop_f_cost: 0,
                    pop_depth: 0,
                    pop_creation_order: 1,
                    candidates_truncated: false,
                    dead_end_reason: None,
                    candidates: vec![make_candidate(
                        0,
                        OP_SET_SLOT,
                        args2,
                        TapeCandidateOutcomeV1::Applied { to_node: 2 },
                    )],
                    notes: vec![],
                }),
                TapeRecordV1::Termination(TapeTerminationV1 {
                    reason: TerminationReasonV1::GoalReached { node_id: 2 },
                    frontier_high_water: 2,
                }),
            ],
            footer: tape_footer(),
        };
        let mut checker = NoopInvariantChecker;

        let result =
            replay_winning_path(&tape, &state0, &registry, &mut checker).unwrap();
        assert_eq!(result.step_count, 2);
        assert_eq!(result.path, vec![0, 1, 2]);
    }

    #[test]
    fn replay_expansion_missing_detected() {
        let state0 = make_state(4);
        let fp0 = fp_bytes(&state0);

        // Node 1 exists but there's no expansion for node 0.
        let tape = SearchTapeV1 {
            header: tape_header(),
            records: vec![
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 0,
                    parent_id: None,
                    state_fingerprint: fp0,
                    depth: 0,
                    f_cost: 0,
                    creation_order: 0,
                }),
                TapeRecordV1::NodeCreation(TapeNodeCreationV1 {
                    node_id: 1,
                    parent_id: Some(0),
                    state_fingerprint: [0u8; 32],
                    depth: 1,
                    f_cost: 0,
                    creation_order: 1,
                }),
                // No expansion record!
                TapeRecordV1::Termination(TapeTerminationV1 {
                    reason: TerminationReasonV1::GoalReached { node_id: 1 },
                    frontier_high_water: 1,
                }),
            ],
            footer: tape_footer(),
        };
        let registry = kernel_operator_registry();
        let mut checker = NoopInvariantChecker;

        let err = replay_winning_path(&tape, &state0, &registry, &mut checker).unwrap_err();
        assert!(matches!(
            err,
            ReplayError::ExpansionMissing { node_id: 0 }
        ));
    }
}
