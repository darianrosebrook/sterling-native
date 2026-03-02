//! `PartialObsWorld`: Mastermind-style hidden-truth world with epistemic operators.
//!
//! Two-layer `ByteStateV1`:
//! - Layer 0 (truth): hidden code (K positions, V values). Slots are Provisional
//!   after compilation — no operator targets layer 0.
//! - Layer 1 (workspace): guess slots + feedback slots + `solved_marker`.
//!
//! Two-step probe cycle:
//! 1. Agent turn: `OP_GUESS` writes K guess values to the next guess slots.
//! 2. Environment turn: `OP_FEEDBACK` writes the feedback (exact match count).
//!
//! `OP_DECLARE` writes the `solved_marker` when the agent believes it knows the truth.
//!
//! Authority boundary: kernel dispatch handlers are bounded-write primitives.
//! They do NOT read layer 0. The world computes feedback in `enumerate_candidates`
//! (harness privilege: reads layer 0). The verifier independently certifies feedback
//! correctness via winning-path replay.

use sterling_kernel::carrier::bytestate::{ByteStateV1, SchemaDescriptor};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply::{
    declare_args, feedback_args, guess_args, OP_DECLARE, OP_FEEDBACK, OP_GUESS, SOLVED_MARKER,
};
use sterling_kernel::operators::operator_registry::OperatorRegistryV1;
use sterling_kernel::proof::canon::canonical_json_bytes;

use sterling_search::contract::SearchWorldV1;
use sterling_search::node::CandidateActionV1;

use crate::contract::{FixtureDimensions, ProgramStep, WorldHarnessError, WorldHarnessV1};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Code length (number of positions in the hidden code).
const K: usize = 2;

/// Number of possible values per position.
const V: usize = 3;

/// Maximum number of probes (sufficient to solve any K=2, V=3 code).
const MAX_PROBES: usize = 4;

/// Total workspace slots on layer 1: `MAX_PROBES * (K + 1) + 1`.
const WORKSPACE_SLOTS: usize = MAX_PROBES * (K + 1) + 1;

/// Index of the `solved_marker` slot on layer 1 (last slot).
const SOLVED_MARKER_SLOT: usize = WORKSPACE_SLOTS - 1;

/// Truth layer index.
const LAYER_TRUTH: usize = 0;

/// Workspace layer index.
const LAYER_WORKSPACE: usize = 1;

/// Layer indices as `u32` for arg builders.
const LAYER_WORKSPACE_U32: u32 = 1;

/// `SOLVED_MARKER_SLOT` as `u32` for arg builders.
#[allow(clippy::cast_possible_truncation)]
const SOLVED_MARKER_SLOT_U32: u32 = SOLVED_MARKER_SLOT as u32;

/// PADDING bytes for "unwritten slot" detection.
const PADDING_BYTES: [u8; 4] = [0, 0, 0, 0];

// --- Concept values (domain = 3) ---

/// Code values for positions (domain 3, kind 0, `local_id` 0..V-1).
const CODE_VALUES: [Code32; V] = [
    Code32::new(3, 0, 0), // code:c0
    Code32::new(3, 0, 1), // code:c1
    Code32::new(3, 0, 2), // code:c2
];

/// Feedback values (domain 3, kind 2, `local_id` = exact match count 0..K).
const FEEDBACK_VALUES: [Code32; K + 1] = [
    Code32::new(3, 2, 0), // feedback:0
    Code32::new(3, 2, 1), // feedback:1
    Code32::new(3, 2, 2), // feedback:2 (= K, all correct)
];

/// Schema basis bytes for the partial obs world.
const SCHEMA_BASIS_BYTES: &[u8] =
    br#"{"domain_id":"partial_obs","schema_version":"partial_obs.v1","version":"1.0"}"#;

/// Compute the stable schema hash.
fn partial_obs_schema_hash() -> String {
    let hash = sterling_kernel::proof::hash::canonical_hash(
        crate::bundle::DOMAIN_HARNESS_FIXTURE,
        SCHEMA_BASIS_BYTES,
    );
    hash.as_str().to_string()
}

// ---------------------------------------------------------------------------
// State helpers
// ---------------------------------------------------------------------------

/// Read a single slot's 4-byte identity from a `ByteStateV1`.
fn read_slot_identity(
    state: &ByteStateV1,
    layer: usize,
    slot: usize,
    slot_count: usize,
) -> [u8; 4] {
    let identity = state.identity_bytes();
    let offset = (layer * slot_count + slot) * 4;
    if offset + 4 > identity.len() {
        return PADDING_BYTES;
    }
    [
        identity[offset],
        identity[offset + 1],
        identity[offset + 2],
        identity[offset + 3],
    ]
}

/// Check if a slot is unwritten (identity == PADDING).
fn is_slot_unwritten(state: &ByteStateV1, layer: usize, slot: usize, slot_count: usize) -> bool {
    read_slot_identity(state, layer, slot, slot_count) == PADDING_BYTES
}

/// Compute the starting guess slot index for probe `p`.
const fn guess_start(p: usize) -> usize {
    p * (K + 1)
}

/// Compute the feedback slot index for probe `p`.
const fn feedback_slot(p: usize) -> usize {
    p * (K + 1) + K
}

/// Feedback slot index as `u32`.
#[allow(clippy::cast_possible_truncation)]
const fn feedback_slot_u32(p: usize) -> u32 {
    feedback_slot(p) as u32
}

/// Guess start index as `u32`.
#[allow(clippy::cast_possible_truncation)]
const fn guess_start_u32(p: usize) -> u32 {
    guess_start(p) as u32
}

/// Determine the current probe index and phase from workspace state.
///
/// Returns `(probe_index, needs_feedback)`:
/// - `needs_feedback = true`: guess slots written but feedback slot is empty.
/// - `needs_feedback = false`: either no guess written yet or both present.
fn current_probe_phase(state: &ByteStateV1) -> (usize, bool) {
    for p in 0..MAX_PROBES {
        let gs = guess_start(p);
        let fs = feedback_slot(p);

        let guess_written = !is_slot_unwritten(state, LAYER_WORKSPACE, gs, WORKSPACE_SLOTS);
        let feedback_written = !is_slot_unwritten(state, LAYER_WORKSPACE, fs, WORKSPACE_SLOTS);

        if guess_written && !feedback_written {
            return (p, true);
        }
        if !guess_written {
            return (p, false);
        }
    }
    // All probes used.
    (MAX_PROBES, false)
}

/// Compute exact match count between guess and truth.
fn exact_matches(state: &ByteStateV1, guess_values: [Code32; K]) -> usize {
    let mut count = 0;
    for (i, gv) in guess_values.iter().enumerate() {
        let truth_bytes = read_slot_identity(state, LAYER_TRUTH, i, WORKSPACE_SLOTS);
        if truth_bytes == gv.to_le_bytes() {
            count += 1;
        }
    }
    count
}

/// Read K guess values from the workspace for probe `p`.
fn read_guess(state: &ByteStateV1, p: usize) -> [Code32; K] {
    let gs = guess_start(p);
    let mut values = [Code32::PADDING; K];
    for (i, val) in values.iter_mut().enumerate() {
        let bytes = read_slot_identity(state, LAYER_WORKSPACE, gs + i, WORKSPACE_SLOTS);
        *val = Code32::from_le_bytes(bytes);
    }
    values
}

/// Compute the implicit belief set from all completed probes.
/// Returns the set of candidate codes consistent with all (guess, feedback) pairs.
fn compute_belief(state: &ByteStateV1) -> Vec<[Code32; K]> {
    // Collect completed probes.
    let mut probes: Vec<([Code32; K], usize)> = Vec::new();
    for p in 0..MAX_PROBES {
        let gs = guess_start(p);
        let fs = feedback_slot(p);

        if is_slot_unwritten(state, LAYER_WORKSPACE, gs, WORKSPACE_SLOTS) {
            break;
        }
        if is_slot_unwritten(state, LAYER_WORKSPACE, fs, WORKSPACE_SLOTS) {
            break;
        }

        let guess = read_guess(state, p);
        let fb_bytes = read_slot_identity(state, LAYER_WORKSPACE, fs, WORKSPACE_SLOTS);
        let fb = Code32::from_le_bytes(fb_bytes);

        // Extract exact_match count from feedback Code32 (domain 3, kind 2, local_id = count).
        let fb_count = fb.local_id() as usize;
        probes.push((guess, fb_count));
    }

    // Enumerate all V^K candidates and filter.
    let mut belief = Vec::new();
    enumerate_all_codes(&mut belief, &probes, &mut [Code32::PADDING; K], 0);
    belief
}

/// Recursively enumerate all V^K codes and keep those consistent with probes.
fn enumerate_all_codes(
    belief: &mut Vec<[Code32; K]>,
    probes: &[([Code32; K], usize)],
    current: &mut [Code32; K],
    pos: usize,
) {
    if pos == K {
        // Check consistency with all probes.
        let consistent = probes.iter().all(|(guess, expected_matches)| {
            let matches = (0..K).filter(|&i| current[i] == guess[i]).count();
            matches == *expected_matches
        });
        if consistent {
            belief.push(*current);
        }
        return;
    }
    for &val in &CODE_VALUES {
        current[pos] = val;
        enumerate_all_codes(belief, probes, current, pos + 1);
    }
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

/// Partial observability world (Mastermind-style).
pub struct PartialObsWorld {
    /// The hidden truth code.
    truth: [Code32; K],
}

impl PartialObsWorld {
    /// Construct a new partial obs world with a specific truth code.
    #[must_use]
    pub fn new(truth: [Code32; K]) -> Self {
        Self { truth }
    }

    /// Default truth for testing: `[code:c0, code:c1]`.
    #[must_use]
    pub fn default_world() -> Self {
        Self::new([CODE_VALUES[0], CODE_VALUES[1]])
    }
}

impl WorldHarnessV1 for PartialObsWorld {
    fn world_id(&self) -> &'static str {
        "partial_obs:v1:k2_v3"
    }

    fn dimensions(&self) -> FixtureDimensions {
        FixtureDimensions {
            layer_count: 2,
            slot_count: WORKSPACE_SLOTS,
            arg_slot_count: 4, // max arg slots: OP_GUESS/OP_DECLARE = 16 bytes = 4 slots
            evidence_obligations: vec![
                "epistemic_transcript_v1".into(),
                "winning_path_replay_v1".into(),
            ],
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, WorldHarnessError> {
        let total = 2 * WORKSPACE_SLOTS;

        // Build identity: truth on layer 0, zeros on layer 1.
        let mut identity: Vec<Vec<u64>> = Vec::with_capacity(total);
        for slot in 0..WORKSPACE_SLOTS {
            if slot < K {
                let bytes = self.truth[slot].to_le_bytes();
                identity.push(vec![
                    u64::from(bytes[0]),
                    u64::from(bytes[1]),
                    u64::from(bytes[2]),
                    u64::from(bytes[3]),
                ]);
            } else {
                identity.push(vec![0, 0, 0, 0]);
            }
        }
        // Layer 1: all zeros.
        for _ in 0..WORKSPACE_SLOTS {
            identity.push(vec![0, 0, 0, 0]);
        }

        // Status: truth slots are Provisional (128), workspace slots are Hole (0).
        let mut status = vec![0u8; total];
        for s in status.iter_mut().take(K) {
            *s = 128; // SlotStatus::Provisional
        }

        let payload = serde_json::json!({
            "identity": identity,
            "layer_count": 2,
            "slot_count": WORKSPACE_SLOTS,
            "status": status,
        });

        canonical_json_bytes(&payload).map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("canonical JSON error: {e:?}"),
        })
    }

    fn schema_descriptor(&self) -> SchemaDescriptor {
        SchemaDescriptor {
            id: "partial_obs".into(),
            version: "1.0".into(),
            hash: partial_obs_schema_hash(),
        }
    }

    fn registry(&self) -> Result<RegistryV1, WorldHarnessError> {
        let mut entries = Vec::new();

        // Code values.
        for (i, &val) in CODE_VALUES.iter().enumerate() {
            entries.push((val, format!("code:c{i}")));
        }

        // Feedback values.
        for (i, &fv) in FEEDBACK_VALUES.iter().enumerate() {
            entries.push((fv, format!("feedback:{i}")));
        }

        // Solved marker.
        entries.push((SOLVED_MARKER, "solved:yes".into()));

        // Operator codes.
        entries.push((OP_GUESS, "op:guess".into()));
        entries.push((OP_FEEDBACK, "op:feedback".into()));
        entries.push((OP_DECLARE, "op:declare".into()));

        RegistryV1::new("epoch-0".into(), entries).map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("registry construction error: {e:?}"),
        })
    }

    fn program(&self) -> Vec<ProgramStep> {
        // Linear program: guess truth directly, get perfect feedback, declare.
        let g = guess_args(LAYER_WORKSPACE_U32, guess_start_u32(0), &self.truth);
        let fb = feedback_args(
            LAYER_WORKSPACE_U32,
            feedback_slot_u32(0),
            FEEDBACK_VALUES[K], // K exact matches = perfect
        );
        let d = declare_args(LAYER_WORKSPACE_U32, SOLVED_MARKER_SLOT_U32, &self.truth);

        vec![
            ProgramStep {
                op_code: OP_GUESS,
                op_args: g,
            },
            ProgramStep {
                op_code: OP_FEEDBACK,
                op_args: fb,
            },
            ProgramStep {
                op_code: OP_DECLARE,
                op_args: d,
            },
        ]
    }
}

impl SearchWorldV1 for PartialObsWorld {
    fn world_id(&self) -> &'static str {
        "partial_obs:v1:k2_v3"
    }

    fn enumerate_candidates(
        &self,
        state: &ByteStateV1,
        operator_registry: &OperatorRegistryV1,
    ) -> Vec<CandidateActionV1> {
        if !operator_registry.contains(&OP_GUESS)
            || !operator_registry.contains(&OP_FEEDBACK)
            || !operator_registry.contains(&OP_DECLARE)
        {
            return Vec::new();
        }

        // Check if already solved.
        if !is_slot_unwritten(state, LAYER_WORKSPACE, SOLVED_MARKER_SLOT, WORKSPACE_SLOTS) {
            return Vec::new();
        }

        let (probe_idx, needs_feedback) = current_probe_phase(state);

        // All probes exhausted.
        if probe_idx >= MAX_PROBES {
            return Vec::new();
        }

        let mut candidates = Vec::new();

        if needs_feedback {
            // Environment turn: emit a single OP_FEEDBACK with correct feedback.
            let guess = read_guess(state, probe_idx);
            let matches = exact_matches(state, guess);

            candidates.push(CandidateActionV1::new(
                OP_FEEDBACK,
                feedback_args(
                    LAYER_WORKSPACE_U32,
                    feedback_slot_u32(probe_idx),
                    FEEDBACK_VALUES[matches],
                ),
            ));
        } else {
            // Agent turn: emit OP_GUESS candidates for all V^K combinations.
            // Also check if belief has converged to 1 — if so, emit OP_DECLARE.
            let belief = compute_belief(state);

            if belief.len() == 1 {
                // Belief converged: declare the unique solution.
                candidates.push(CandidateActionV1::new(
                    OP_DECLARE,
                    declare_args(LAYER_WORKSPACE_U32, SOLVED_MARKER_SLOT_U32, &belief[0]),
                ));
            }

            // Emit all V^K guess combinations.
            let mut guess_buf = [Code32::PADDING; K];
            enumerate_guess_candidates(&mut candidates, &mut guess_buf, 0, probe_idx);
        }

        candidates
    }

    fn is_goal(&self, state: &ByteStateV1) -> bool {
        // solved_marker must be set.
        let marker_bytes =
            read_slot_identity(state, LAYER_WORKSPACE, SOLVED_MARKER_SLOT, WORKSPACE_SLOTS);
        if marker_bytes != SOLVED_MARKER.to_le_bytes() {
            return false;
        }

        // The declared solution is NOT stored in state (only the marker is).
        // For goal checking in search, we verify that the truth is fully determined
        // by the probe history — i.e., belief has converged to exactly one candidate
        // and that candidate matches truth.
        let belief = compute_belief(state);
        if belief.len() != 1 {
            return false;
        }

        // Verify the unique belief matches truth (layer 0).
        for i in 0..K {
            let truth_bytes = read_slot_identity(state, LAYER_TRUTH, i, WORKSPACE_SLOTS);
            if truth_bytes != belief[0][i].to_le_bytes() {
                return false;
            }
        }

        true
    }
}

/// Recursively enumerate all V^K guess candidates.
fn enumerate_guess_candidates(
    candidates: &mut Vec<CandidateActionV1>,
    current: &mut [Code32; K],
    pos: usize,
    probe_idx: usize,
) {
    if pos == K {
        candidates.push(CandidateActionV1::new(
            OP_GUESS,
            guess_args(LAYER_WORKSPACE_U32, guess_start_u32(probe_idx), current),
        ));
        return;
    }
    for &val in &CODE_VALUES {
        current[pos] = val;
        enumerate_guess_candidates(candidates, current, pos + 1, probe_idx);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sterling_kernel::carrier::compile::compile;
    use sterling_kernel::operators::apply::apply;
    use sterling_kernel::operators::operator_registry::kernel_operator_registry;

    fn op_reg() -> OperatorRegistryV1 {
        kernel_operator_registry()
    }

    fn compile_world(world: &PartialObsWorld) -> ByteStateV1 {
        let payload = world.encode_payload().expect("encode_payload");
        let schema = world.schema_descriptor();
        let registry = world.registry().expect("registry");
        let result = compile(&payload, &schema, &registry).expect("compile");
        result.state
    }

    #[test]
    fn initial_state_has_truth_on_layer_0() {
        let world = PartialObsWorld::default_world();
        let state = compile_world(&world);

        // Truth: [code:c0, code:c1] on layer 0.
        assert_eq!(
            read_slot_identity(&state, LAYER_TRUTH, 0, WORKSPACE_SLOTS),
            CODE_VALUES[0].to_le_bytes()
        );
        assert_eq!(
            read_slot_identity(&state, LAYER_TRUTH, 1, WORKSPACE_SLOTS),
            CODE_VALUES[1].to_le_bytes()
        );

        // Layer 1 all unwritten.
        for s in 0..WORKSPACE_SLOTS {
            assert!(
                is_slot_unwritten(&state, LAYER_WORKSPACE, s, WORKSPACE_SLOTS),
                "layer 1 slot {s} should be unwritten"
            );
        }
    }

    #[test]
    fn initial_state_is_not_goal() {
        let world = PartialObsWorld::default_world();
        let state = compile_world(&world);
        assert!(!world.is_goal(&state));
    }

    #[test]
    fn program_runs_to_goal() {
        let world = PartialObsWorld::default_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        for step in world.program() {
            let (new_state, _) = apply(&state, step.op_code, &step.op_args, &reg).expect("apply");
            state = new_state;
        }

        assert!(world.is_goal(&state));
    }

    #[test]
    fn enumerate_initial_has_guess_candidates() {
        let world = PartialObsWorld::default_world();
        let state = compile_world(&world);
        let reg = op_reg();

        let candidates = world.enumerate_candidates(&state, &reg);
        // V^K = 3^2 = 9 guess candidates.
        assert_eq!(candidates.len(), 9);
        for c in &candidates {
            assert_eq!(c.op_code, OP_GUESS);
        }
    }

    #[test]
    fn enumerate_after_guess_has_feedback() {
        let world = PartialObsWorld::default_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        // Apply a guess.
        let g = guess_args(LAYER_WORKSPACE_U32, 0, &[CODE_VALUES[0], CODE_VALUES[0]]);
        let (s1, _) = apply(&state, OP_GUESS, &g, &reg).expect("guess");
        state = s1;

        let candidates = world.enumerate_candidates(&state, &reg);
        // Should be exactly 1 feedback candidate.
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].op_code, OP_FEEDBACK);
    }

    #[test]
    fn enumerate_after_feedback_has_guesses_again() {
        let world = PartialObsWorld::default_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        // Guess [c0, c0] -> truth is [c0, c1] -> 1 match.
        let g = guess_args(LAYER_WORKSPACE_U32, 0, &[CODE_VALUES[0], CODE_VALUES[0]]);
        let (s1, _) = apply(&state, OP_GUESS, &g, &reg).expect("guess");
        state = s1;

        // Feedback: 1 match.
        let f = feedback_args(LAYER_WORKSPACE_U32, feedback_slot_u32(0), FEEDBACK_VALUES[1]);
        let (s2, _) = apply(&state, OP_FEEDBACK, &f, &reg).expect("feedback");
        state = s2;

        let candidates = world.enumerate_candidates(&state, &reg);
        // 9 guesses for probe 1 (no OP_DECLARE — belief hasn't converged).
        let guess_count = candidates.iter().filter(|c| c.op_code == OP_GUESS).count();
        assert_eq!(guess_count, 9);
    }

    #[test]
    fn declare_after_convergence() {
        let world = PartialObsWorld::default_world(); // truth = [c0, c1]
        let mut state = compile_world(&world);
        let reg = op_reg();

        // Guess exactly the truth -> K exact matches.
        let g = guess_args(LAYER_WORKSPACE_U32, 0, &[CODE_VALUES[0], CODE_VALUES[1]]);
        let (s1, _) = apply(&state, OP_GUESS, &g, &reg).expect("guess");
        state = s1;

        // Feedback: 2 matches (= K, all correct).
        let f = feedback_args(LAYER_WORKSPACE_U32, feedback_slot_u32(0), FEEDBACK_VALUES[2]);
        let (s2, _) = apply(&state, OP_FEEDBACK, &f, &reg).expect("feedback");
        state = s2;

        let candidates = world.enumerate_candidates(&state, &reg);
        // Belief converged to 1: should include OP_DECLARE + 9 guesses.
        let declare_count = candidates.iter().filter(|c| c.op_code == OP_DECLARE).count();
        assert_eq!(declare_count, 1, "should have exactly one OP_DECLARE candidate");
    }

    #[test]
    fn no_candidates_after_declare() {
        let world = PartialObsWorld::default_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        for step in world.program() {
            let (s, _) = apply(&state, step.op_code, &step.op_args, &reg).expect("apply");
            state = s;
        }

        let candidates = world.enumerate_candidates(&state, &reg);
        assert!(candidates.is_empty(), "no candidates after solve");
    }

    #[test]
    fn evidence_obligations_declared() {
        let world = PartialObsWorld::default_world();
        let dims = world.dimensions();
        assert_eq!(dims.evidence_obligations.len(), 2);
        assert!(dims
            .evidence_obligations
            .contains(&"epistemic_transcript_v1".to_string()));
        assert!(dims
            .evidence_obligations
            .contains(&"winning_path_replay_v1".to_string()));
    }

    #[test]
    fn enumeration_is_deterministic() {
        let world = PartialObsWorld::default_world();
        let state = compile_world(&world);
        let reg = op_reg();
        let c1 = world.enumerate_candidates(&state, &reg);
        let c2 = world.enumerate_candidates(&state, &reg);
        assert_eq!(c1, c2);
    }

    #[test]
    fn belief_shrinks_with_probes() {
        let world = PartialObsWorld::default_world(); // truth = [c0, c1]
        let mut state = compile_world(&world);
        let reg = op_reg();

        // Before any probes: belief = all V^K = 9.
        let b0 = compute_belief(&state);
        assert_eq!(b0.len(), 9);

        // Probe 0: guess [c0, c0] -> 1 match.
        let g0 = guess_args(LAYER_WORKSPACE_U32, 0, &[CODE_VALUES[0], CODE_VALUES[0]]);
        let (s1, _) = apply(&state, OP_GUESS, &g0, &reg).expect("guess 0");
        state = s1;
        let f0 = feedback_args(LAYER_WORKSPACE_U32, feedback_slot_u32(0), FEEDBACK_VALUES[1]);
        let (s2, _) = apply(&state, OP_FEEDBACK, &f0, &reg).expect("feedback 0");
        state = s2;

        let b1 = compute_belief(&state);
        assert!(
            b1.len() < b0.len(),
            "belief should shrink: {} < {}",
            b1.len(),
            b0.len()
        );
    }

    #[test]
    fn wrong_solution_is_not_goal() {
        // Truth is [c0, c1]. Declare [c1, c0] -- wrong.
        let world = PartialObsWorld::default_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        let g = guess_args(LAYER_WORKSPACE_U32, 0, &[CODE_VALUES[1], CODE_VALUES[0]]);
        let (s1, _) = apply(&state, OP_GUESS, &g, &reg).expect("guess");
        state = s1;

        // Feedback: 0 matches (neither position matches).
        let f = feedback_args(LAYER_WORKSPACE_U32, feedback_slot_u32(0), FEEDBACK_VALUES[0]);
        let (s2, _) = apply(&state, OP_FEEDBACK, &f, &reg).expect("feedback");
        state = s2;

        // Declare with wrong solution.
        let d = declare_args(
            LAYER_WORKSPACE_U32,
            SOLVED_MARKER_SLOT_U32,
            &[CODE_VALUES[1], CODE_VALUES[0]],
        );
        let (s3, _) = apply(&state, OP_DECLARE, &d, &reg).expect("declare");
        state = s3;

        // Marker is set, but belief hasn't converged to 1. is_goal should be false.
        assert!(!world.is_goal(&state));
    }
}
