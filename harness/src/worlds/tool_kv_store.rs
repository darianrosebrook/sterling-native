//! `ToolKvStore`: tool-safety world using explicit STAGE/COMMIT/ROLLBACK operators.
//!
//! Extends the `TransactionalKvStore` pattern (2-layer `ByteStateV1`, write-once
//! per slot) but uses kernel-level `OP_STAGE`, `OP_COMMIT`, and `OP_ROLLBACK`
//! instead of encoding all transaction semantics through `OP_SET_SLOT`.
//!
//! Key differences from `TransactionalKvStore`:
//! - `OP_STAGE` for staging writes (not `SET_SLOT`)
//! - `OP_COMMIT` / `OP_ROLLBACK` for transaction finalization (not `SET_SLOT`)
//! - `SET_SLOT` is still used for commit-writes to layer 0 (direct writes)
//! - Declares `evidence_obligations: ["tool_transcript_v1"]`
//!
//! Committed-write safety: no layer 0 writes before `OP_COMMIT`, no writes
//! after `OP_ROLLBACK`. Enforced at the proposer level (this module) and
//! independently verified by the Cert trace-order audit.

use sterling_kernel::carrier::bytestate::{ByteStateV1, SchemaDescriptor};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply::{
    commit_args, rollback_args, set_slot_args, stage_args, OP_COMMIT, OP_ROLLBACK, OP_SET_SLOT,
    OP_STAGE,
};
use sterling_kernel::operators::operator_registry::OperatorRegistryV1;
use sterling_kernel::proof::canon::canonical_json_bytes;

use sterling_search::contract::SearchWorldV1;
use sterling_search::node::CandidateActionV1;

use crate::contract::{FixtureDimensions, ProgramStep, WorldHarnessError, WorldHarnessV1};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of key slots per layer.
const KEY_SLOTS: usize = 3;

/// Total slots per layer: key slots + 1 `txn_marker`.
const SLOT_COUNT: usize = KEY_SLOTS + 1;

/// Index of the transaction marker slot on each layer.
const TXN_MARKER_SLOT: usize = KEY_SLOTS; // slot 3

/// Committed store layer index.
const LAYER_COMMITTED: usize = 0;

/// Staging area layer index.
const LAYER_STAGED: usize = 1;

/// Layer indices as `u32` for arg builders.
const LAYER_COMMITTED_U32: u32 = 0;
const LAYER_STAGED_U32: u32 = 1;

/// PADDING bytes for "unwritten slot" detection.
const PADDING_BYTES: [u8; 4] = [0, 0, 0, 0];

// --- Concept values (domain = 2, same as TransactionalKvStore) ---------------

/// Storable values for key slots.
const KV_VALUES: [Code32; 3] = [
    Code32::new(2, 1, 0), // kv:v0
    Code32::new(2, 1, 1), // kv:v1
    Code32::new(2, 1, 2), // kv:v2
];

/// Normative schema basis bytes for the tool KV store.
///
/// Distinct from `TransactionalKvStore` to avoid schema hash collision.
const SCHEMA_BASIS_BYTES: &[u8] =
    br#"{"domain_id":"tool_kv_store","schema_version":"tool_kv.v1","version":"1.0"}"#;

/// Compute the stable schema hash.
fn tool_kv_schema_hash() -> String {
    let hash = sterling_kernel::proof::hash::canonical_hash(
        crate::bundle::DOMAIN_HARNESS_FIXTURE,
        SCHEMA_BASIS_BYTES,
    );
    hash.as_str().to_string()
}

// ---------------------------------------------------------------------------
// State helpers (shared with TransactionalKvStore)
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

/// Read the `txn_marker` value from the staging layer.
fn read_txn_marker(state: &ByteStateV1, slot_count: usize) -> [u8; 4] {
    read_slot_identity(state, LAYER_STAGED, TXN_MARKER_SLOT, slot_count)
}

/// Check if any key slot on the staging layer has been staged.
fn has_any_staged_key(state: &ByteStateV1, slot_count: usize) -> bool {
    (0..KEY_SLOTS).any(|slot| !is_slot_unwritten(state, LAYER_STAGED, slot, slot_count))
}

// ---------------------------------------------------------------------------
// World configuration
// ---------------------------------------------------------------------------

/// Which linear program to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTxnProgram {
    /// Stage slot 0 with `kv:v0`, then commit, then commit-write.
    CommitOne,
    /// Stage slot 0 with `kv:v0`, then rollback.
    RollbackOne,
}

/// Goal profile for search mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTxnGoalProfile {
    /// Goal when committed slot 0 has value `kv:v0`.
    CommittedSlot0IsV0,
}

/// Tool KV Store world — transactional semantics with explicit operators.
pub struct ToolKvStore {
    txn_program: ToolTxnProgram,
    goal_profile: ToolTxnGoalProfile,
}

impl ToolKvStore {
    /// Construct a new tool KV store world.
    #[must_use]
    pub fn new(txn_program: ToolTxnProgram, goal_profile: ToolTxnGoalProfile) -> Self {
        Self {
            txn_program,
            goal_profile,
        }
    }

    /// Convenience: commit-path world.
    #[must_use]
    pub fn commit_world() -> Self {
        Self::new(ToolTxnProgram::CommitOne, ToolTxnGoalProfile::CommittedSlot0IsV0)
    }

    /// Convenience: rollback-path world.
    #[must_use]
    pub fn rollback_world() -> Self {
        Self::new(
            ToolTxnProgram::RollbackOne,
            ToolTxnGoalProfile::CommittedSlot0IsV0,
        )
    }
}

impl WorldHarnessV1 for ToolKvStore {
    fn world_id(&self) -> &str {
        match self.txn_program {
            ToolTxnProgram::CommitOne => "tool_kv_store:v1:commit_one",
            ToolTxnProgram::RollbackOne => "tool_kv_store:v1:rollback_one",
        }
    }

    fn dimensions(&self) -> FixtureDimensions {
        FixtureDimensions {
            layer_count: 2,
            slot_count: SLOT_COUNT,
            arg_slot_count: 3, // max arg slots across all operators (STAGE takes 3)
            evidence_obligations: vec!["tool_transcript_v1".into()],
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, WorldHarnessError> {
        let total = 2 * SLOT_COUNT;
        let zeros = vec![vec![0u32; 4]; total];
        let status_zeros = vec![0u8; total];

        let payload = serde_json::json!({
            "identity": zeros,
            "layer_count": 2,
            "slot_count": SLOT_COUNT,
            "status": status_zeros,
        });

        canonical_json_bytes(&payload).map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("canonical JSON error: {e:?}"),
        })
    }

    fn schema_descriptor(&self) -> SchemaDescriptor {
        SchemaDescriptor {
            id: "tool_kv_store".into(),
            version: "1.0".into(),
            hash: tool_kv_schema_hash(),
        }
    }

    fn registry(&self) -> Result<RegistryV1, WorldHarnessError> {
        RegistryV1::new(
            "epoch-0".into(),
            vec![
                (KV_VALUES[0], "kv:v0".into()),
                (KV_VALUES[1], "kv:v1".into()),
                (KV_VALUES[2], "kv:v2".into()),
                (OP_SET_SLOT, "kv:op:set_slot".into()),
                (OP_STAGE, "kv:op:stage".into()),
                (OP_COMMIT, "kv:op:commit".into()),
                (OP_ROLLBACK, "kv:op:rollback".into()),
            ],
        )
        .map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("registry construction error: {e:?}"),
        })
    }

    fn program(&self) -> Vec<ProgramStep> {
        match self.txn_program {
            ToolTxnProgram::CommitOne => {
                // 1. STAGE slot 0 with kv:v0
                // 2. COMMIT (marks txn_marker via kernel operator)
                // 3. SET_SLOT commit-write: copy staged value to committed layer
                vec![
                    ProgramStep {
                        op_code: OP_STAGE,
                        op_args: stage_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
                    },
                    ProgramStep {
                        op_code: OP_COMMIT,
                        op_args: commit_args(LAYER_STAGED_U32),
                    },
                    ProgramStep {
                        op_code: OP_SET_SLOT,
                        op_args: set_slot_args(LAYER_COMMITTED_U32, 0, KV_VALUES[0]),
                    },
                ]
            }
            ToolTxnProgram::RollbackOne => {
                // 1. STAGE slot 0 with kv:v0
                // 2. ROLLBACK (marks txn_marker via kernel operator)
                vec![
                    ProgramStep {
                        op_code: OP_STAGE,
                        op_args: stage_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
                    },
                    ProgramStep {
                        op_code: OP_ROLLBACK,
                        op_args: rollback_args(LAYER_STAGED_U32),
                    },
                ]
            }
        }
    }
}

impl SearchWorldV1 for ToolKvStore {
    fn world_id(&self) -> &str {
        WorldHarnessV1::world_id(self)
    }

    fn enumerate_candidates(
        &self,
        state: &ByteStateV1,
        operator_registry: &OperatorRegistryV1,
    ) -> Vec<CandidateActionV1> {
        // All candidate op_codes must be in the operator registry.
        if !operator_registry.contains(&OP_STAGE)
            || !operator_registry.contains(&OP_COMMIT)
            || !operator_registry.contains(&OP_ROLLBACK)
            || !operator_registry.contains(&OP_SET_SLOT)
        {
            return Vec::new();
        }

        let marker = read_txn_marker(state, SLOT_COUNT);
        let marker_is_commit =
            marker == sterling_kernel::operators::apply::COMMIT_MARKER.to_le_bytes();
        let marker_is_rollback =
            marker == sterling_kernel::operators::apply::ROLLBACK_MARKER.to_le_bytes();
        let marker_is_unset = marker == PADDING_BYTES;

        // After rollback, no further actions are possible (INV-TOOL-09).
        if marker_is_rollback {
            return Vec::new();
        }

        let mut candidates = Vec::new();

        // Phase 1: Stage candidates (only if marker is unset).
        if marker_is_unset {
            for slot in 0..KEY_SLOTS {
                if is_slot_unwritten(state, LAYER_STAGED, slot, SLOT_COUNT) {
                    for &value in &KV_VALUES {
                        #[allow(clippy::cast_possible_truncation)]
                        candidates.push(CandidateActionV1::new(
                            OP_STAGE,
                            stage_args(LAYER_STAGED_U32, slot as u32, value),
                        ));
                    }
                }
            }
        }

        // Phase 2: Transaction finalization candidates (only if marker is unset
        // and at least one key slot is staged).
        if marker_is_unset && has_any_staged_key(state, SLOT_COUNT) {
            // COMMIT.
            candidates.push(CandidateActionV1::new(
                OP_COMMIT,
                commit_args(LAYER_STAGED_U32),
            ));
            // ROLLBACK.
            candidates.push(CandidateActionV1::new(
                OP_ROLLBACK,
                rollback_args(LAYER_STAGED_U32),
            ));
        }

        // Phase 3: Commit-write candidates (only after COMMIT, INV-TOOL-09).
        if marker_is_commit {
            for slot in 0..KEY_SLOTS {
                if !is_slot_unwritten(state, LAYER_STAGED, slot, SLOT_COUNT)
                    && is_slot_unwritten(state, LAYER_COMMITTED, slot, SLOT_COUNT)
                {
                    let staged_bytes = read_slot_identity(state, LAYER_STAGED, slot, SLOT_COUNT);
                    let value = Code32::from_le_bytes(staged_bytes);
                    #[allow(clippy::cast_possible_truncation)]
                    candidates.push(CandidateActionV1::new(
                        OP_SET_SLOT,
                        set_slot_args(LAYER_COMMITTED_U32, slot as u32, value),
                    ));
                }
            }
        }

        candidates
    }

    fn is_goal(&self, state: &ByteStateV1) -> bool {
        match self.goal_profile {
            ToolTxnGoalProfile::CommittedSlot0IsV0 => {
                let slot0 = read_slot_identity(state, LAYER_COMMITTED, 0, SLOT_COUNT);
                slot0 == KV_VALUES[0].to_le_bytes()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sterling_kernel::carrier::compile::compile;
    use sterling_kernel::operators::apply::apply;
    use sterling_kernel::operators::operator_registry::kernel_operator_registry;

    fn op_reg() -> OperatorRegistryV1 {
        kernel_operator_registry()
    }

    fn compile_world(world: &ToolKvStore) -> ByteStateV1 {
        let payload = world.encode_payload().expect("encode_payload");
        let schema = world.schema_descriptor();
        let registry = world.registry().expect("registry");
        let result = compile(&payload, &schema, &registry).expect("compile");
        result.state
    }

    #[test]
    fn initial_state_is_all_hole() {
        let world = ToolKvStore::commit_world();
        let state = compile_world(&world);
        let identity = state.identity_bytes();
        assert_eq!(identity.len(), 2 * SLOT_COUNT * 4);
        for chunk in identity.chunks(4) {
            assert_eq!(chunk, &PADDING_BYTES);
        }
    }

    #[test]
    fn commit_program_runs_to_completion() {
        let world = ToolKvStore::commit_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        for step in world.program() {
            let (new_state, _) = apply(&state, step.op_code, &step.op_args, &reg).expect("apply");
            state = new_state;
        }

        // Committed slot 0 should have kv:v0.
        let committed = read_slot_identity(&state, LAYER_COMMITTED, 0, SLOT_COUNT);
        assert_eq!(committed, KV_VALUES[0].to_le_bytes());
        assert!(world.is_goal(&state));
    }

    #[test]
    fn rollback_program_runs_to_completion() {
        let world = ToolKvStore::rollback_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        for step in world.program() {
            let (new_state, _) = apply(&state, step.op_code, &step.op_args, &reg).expect("apply");
            state = new_state;
        }

        // Committed slot 0 unchanged.
        let committed = read_slot_identity(&state, LAYER_COMMITTED, 0, SLOT_COUNT);
        assert_eq!(committed, PADDING_BYTES);
        assert!(!world.is_goal(&state));
    }

    #[test]
    fn initial_state_is_not_goal() {
        let world = ToolKvStore::commit_world();
        let state = compile_world(&world);
        assert!(!world.is_goal(&state));
    }

    #[test]
    fn enumerate_candidates_from_initial_state() {
        let world = ToolKvStore::commit_world();
        let state = compile_world(&world);
        let reg = op_reg();

        let candidates = world.enumerate_candidates(&state, &reg);
        // 3 key slots × 3 values = 9 STAGE candidates.
        // No commit/rollback (nothing staged yet).
        assert_eq!(candidates.len(), 9);
        for c in &candidates {
            assert_eq!(c.op_code, OP_STAGE);
        }
    }

    #[test]
    fn enumerate_after_staging_includes_commit_rollback() {
        let world = ToolKvStore::commit_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        // Stage slot 0.
        let (s1, _) = apply(
            &state,
            OP_STAGE,
            &stage_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
            &reg,
        )
        .expect("stage");
        state = s1;

        let candidates = world.enumerate_candidates(&state, &reg);
        // 2 remaining key slots × 3 values = 6 STAGE candidates
        // + 1 COMMIT + 1 ROLLBACK = 8 total
        assert_eq!(candidates.len(), 8);

        let commit_count = candidates.iter().filter(|c| c.op_code == OP_COMMIT).count();
        let rollback_count = candidates
            .iter()
            .filter(|c| c.op_code == OP_ROLLBACK)
            .count();
        assert_eq!(commit_count, 1);
        assert_eq!(rollback_count, 1);
    }

    #[test]
    fn enumerate_after_commit_has_set_slot_writes() {
        let world = ToolKvStore::commit_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        // Stage + commit.
        let (s1, _) = apply(
            &state,
            OP_STAGE,
            &stage_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
            &reg,
        )
        .expect("stage");
        state = s1;
        let (s2, _) = apply(&state, OP_COMMIT, &commit_args(LAYER_STAGED_U32), &reg)
            .expect("commit");
        state = s2;

        let candidates = world.enumerate_candidates(&state, &reg);
        // Only commit-writes: 1 staged slot → 1 SET_SLOT candidate.
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].op_code, OP_SET_SLOT);
    }

    #[test]
    fn enumerate_after_rollback_is_empty() {
        let world = ToolKvStore::commit_world();
        let mut state = compile_world(&world);
        let reg = op_reg();

        let (s1, _) = apply(
            &state,
            OP_STAGE,
            &stage_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
            &reg,
        )
        .expect("stage");
        state = s1;
        let (s2, _) = apply(&state, OP_ROLLBACK, &rollback_args(LAYER_STAGED_U32), &reg)
            .expect("rollback");
        state = s2;

        let candidates = world.enumerate_candidates(&state, &reg);
        assert!(candidates.is_empty(), "no actions after rollback");
    }

    #[test]
    fn evidence_obligations_declared() {
        let world = ToolKvStore::commit_world();
        let dims = world.dimensions();
        assert_eq!(dims.evidence_obligations, vec!["tool_transcript_v1"]);
    }

    #[test]
    fn enumeration_is_deterministic() {
        let world = ToolKvStore::commit_world();
        let state = compile_world(&world);
        let reg = op_reg();
        let c1 = world.enumerate_candidates(&state, &reg);
        let c2 = world.enumerate_candidates(&state, &reg);
        assert_eq!(c1, c2);
    }
}
