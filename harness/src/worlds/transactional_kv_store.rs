//! `TransactionalKvStore`: truth-regime world for transactional semantics.
//!
//! Models a 2-layer `ByteStateV1` where layer 0 is the committed store and
//! layer 1 is the staging area. All mutations use `OP_SET_SLOT` with
//! write-once-per-slot semantics (Hole→Provisional only; no rewrites).
//!
//! Transaction outcomes are signaled by writing a marker value (`kv:commit`
//! or `kv:rollback`) to a dedicated `txn_marker` slot on layer 1.
//! Committed writes to layer 0 are only legal after `txn_marker == kv:commit`.
//! Rollback is simply the marker — staged residue is ignored.
//!
//! # Kernel boundary
//!
//! This world is entirely a consumer of existing kernel primitives
//! (`ByteStateV1`, `Code32`, `SET_SLOT` via `apply()`). No kernel changes.
//! The write-once constraint arises from `apply_set_slot()` always setting
//! `SlotStatus::Provisional` and `validate_effect_kind` requiring exactly
//! 1 status diff per step.

use sterling_kernel::carrier::bytestate::{ByteStateV1, SchemaDescriptor};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply::{set_slot_args, OP_SET_SLOT};
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

/// `set_slot_args` layer/slot parameters as `u32` for direct use.
const LAYER_COMMITTED_U32: u32 = 0;
const LAYER_STAGED_U32: u32 = 1;
const TXN_MARKER_SLOT_U32: u32 = 3; // KEY_SLOTS

/// PADDING bytes for "unwritten slot" detection.
const PADDING_BYTES: [u8; 4] = [0, 0, 0, 0];

// --- Concept values (domain = 2 to distinguish from existing worlds) -------

/// Transaction marker: commit.
const KV_COMMIT: Code32 = Code32::new(2, 0, 1);

/// Transaction marker: rollback.
const KV_ROLLBACK: Code32 = Code32::new(2, 0, 2);

/// Storable values for key slots.
const KV_VALUES: [Code32; 3] = [
    Code32::new(2, 1, 0), // kv:v0
    Code32::new(2, 1, 1), // kv:v1
    Code32::new(2, 1, 2), // kv:v2
];

/// Normative schema basis bytes for the transactional KV store.
///
/// Changing this constant is a schema version bump.
const SCHEMA_BASIS_BYTES: &[u8] =
    br#"{"domain_id":"transactional_kv_store","schema_version":"txn_kv.v1","version":"1.0"}"#;

/// Compute the stable schema hash.
fn txn_kv_schema_hash() -> String {
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
fn read_slot_identity(state: &ByteStateV1, layer: usize, slot: usize, slot_count: usize) -> [u8; 4] {
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

/// Read the `txn_marker` value from layer 1.
fn read_txn_marker(state: &ByteStateV1, slot_count: usize) -> [u8; 4] {
    read_slot_identity(state, LAYER_STAGED, TXN_MARKER_SLOT, slot_count)
}

/// Check if a slot on layer 1 has a staged value (non-PADDING key slot).
fn has_any_staged_key(state: &ByteStateV1, slot_count: usize) -> bool {
    for slot in 0..KEY_SLOTS {
        if !is_slot_unwritten(state, LAYER_STAGED, slot, slot_count) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// World configuration
// ---------------------------------------------------------------------------

/// Which linear program to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxnProgram {
    /// Stage slot 0 with `kv:v0`, then commit (mark + write).
    CommitOne,
    /// Stage slot 0 with `kv:v0`, then rollback (mark only).
    RollbackOne,
}

/// Goal profile for search mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxnGoalProfile {
    /// Goal when committed slot 0 has value `kv:v0`.
    CommittedSlot0IsV0,
}

/// Transactional KV Store world.
pub struct TransactionalKvStore {
    /// Which linear program to use (for `program()`).
    txn_program: TxnProgram,
    /// Goal profile for search mode.
    goal_profile: TxnGoalProfile,
}

impl TransactionalKvStore {
    /// Construct a new transactional KV store world.
    #[must_use]
    pub fn new(txn_program: TxnProgram, goal_profile: TxnGoalProfile) -> Self {
        Self {
            txn_program,
            goal_profile,
        }
    }

    /// Convenience: commit-path world (linear + search).
    #[must_use]
    pub fn commit_world() -> Self {
        Self::new(TxnProgram::CommitOne, TxnGoalProfile::CommittedSlot0IsV0)
    }

    /// Convenience: rollback-path world (linear only; search uses commit goal).
    #[must_use]
    pub fn rollback_world() -> Self {
        Self::new(TxnProgram::RollbackOne, TxnGoalProfile::CommittedSlot0IsV0)
    }
}

impl WorldHarnessV1 for TransactionalKvStore {
    fn world_id(&self) -> &str {
        match self.txn_program {
            TxnProgram::CommitOne => "txn_kv_store:v1:commit_one",
            TxnProgram::RollbackOne => "txn_kv_store:v1:rollback_one",
        }
    }

    fn dimensions(&self) -> FixtureDimensions {
        FixtureDimensions {
            layer_count: 2,
            slot_count: SLOT_COUNT,
            arg_slot_count: 3, // SET_SLOT takes 3 arg slots (layer, slot, value)
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, WorldHarnessError> {
        // 2 layers × SLOT_COUNT slots = 2*SLOT_COUNT identity entries + status entries.
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
            id: "transactional_kv_store".into(),
            version: "1.0".into(),
            hash: txn_kv_schema_hash(),
        }
    }

    fn registry(&self) -> Result<RegistryV1, WorldHarnessError> {
        RegistryV1::new(
            "epoch-0".into(),
            vec![
                (KV_COMMIT, "kv:commit".into()),
                (KV_ROLLBACK, "kv:rollback".into()),
                (KV_VALUES[0], "kv:v0".into()),
                (KV_VALUES[1], "kv:v1".into()),
                (KV_VALUES[2], "kv:v2".into()),
                (OP_SET_SLOT, "kv:op:set_slot".into()),
            ],
        )
        .map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("registry construction error: {e:?}"),
        })
    }

    fn program(&self) -> Vec<ProgramStep> {
        match self.txn_program {
            TxnProgram::CommitOne => {
                // 1. Stage slot 0 with kv:v0
                // 2. Mark commit
                // 3. Write committed slot 0 with kv:v0
                vec![
                    ProgramStep {
                        op_code: OP_SET_SLOT,
                        op_args: set_slot_args(
                            LAYER_STAGED_U32,
                            0,
                            KV_VALUES[0],
                        ),
                    },
                    ProgramStep {
                        op_code: OP_SET_SLOT,
                        op_args: set_slot_args(
                            LAYER_STAGED_U32,
                            TXN_MARKER_SLOT_U32,
                            KV_COMMIT,
                        ),
                    },
                    ProgramStep {
                        op_code: OP_SET_SLOT,
                        op_args: set_slot_args(
                            LAYER_COMMITTED_U32,
                            0,
                            KV_VALUES[0],
                        ),
                    },
                ]
            }
            TxnProgram::RollbackOne => {
                // 1. Stage slot 0 with kv:v0
                // 2. Mark rollback (no committed writes)
                vec![
                    ProgramStep {
                        op_code: OP_SET_SLOT,
                        op_args: set_slot_args(
                            LAYER_STAGED_U32,
                            0,
                            KV_VALUES[0],
                        ),
                    },
                    ProgramStep {
                        op_code: OP_SET_SLOT,
                        op_args: set_slot_args(
                            LAYER_STAGED_U32,
                            TXN_MARKER_SLOT_U32,
                            KV_ROLLBACK,
                        ),
                    },
                ]
            }
        }
    }
}

impl SearchWorldV1 for TransactionalKvStore {
    fn world_id(&self) -> &str {
        WorldHarnessV1::world_id(self)
    }

    fn enumerate_candidates(
        &self,
        state: &ByteStateV1,
        operator_registry: &OperatorRegistryV1,
    ) -> Vec<CandidateActionV1> {
        // INV-SC-02: all candidate op_codes must be in the operator registry.
        if !operator_registry.contains(&OP_SET_SLOT) {
            return Vec::new();
        }

        let marker = read_txn_marker(state, SLOT_COUNT);
        let marker_is_commit = marker == KV_COMMIT.to_le_bytes();
        let marker_is_rollback = marker == KV_ROLLBACK.to_le_bytes();
        let marker_is_unset = marker == PADDING_BYTES;

        // After rollback, no further actions are possible.
        if marker_is_rollback {
            return Vec::new();
        }

        let mut candidates = Vec::new();

        // Phase 1: Stage candidates (only if marker is unset — staging happens
        // before commit/rollback decision).
        if marker_is_unset {
            for slot in 0..KEY_SLOTS {
                if is_slot_unwritten(state, LAYER_STAGED, slot, SLOT_COUNT) {
                    for &value in &KV_VALUES {
                        #[allow(clippy::cast_possible_truncation)]
                        candidates.push(CandidateActionV1::new(
                            OP_SET_SLOT,
                            set_slot_args(LAYER_STAGED_U32, slot as u32, value),
                        ));
                    }
                }
            }
        }

        // Phase 2: Marker candidates (only if marker is unset and at least
        // one key slot is staged).
        if marker_is_unset && has_any_staged_key(state, SLOT_COUNT) {
            // Commit marker.
            candidates.push(CandidateActionV1::new(
                OP_SET_SLOT,
                set_slot_args(
                    LAYER_STAGED_U32,
                    TXN_MARKER_SLOT_U32,
                    KV_COMMIT,
                ),
            ));
            // Rollback marker.
            candidates.push(CandidateActionV1::new(
                OP_SET_SLOT,
                set_slot_args(
                    LAYER_STAGED_U32,
                    TXN_MARKER_SLOT_U32,
                    KV_ROLLBACK,
                ),
            ));
        }

        // Phase 3: Commit-write candidates (only if marker == kv:commit).
        if marker_is_commit {
            for slot in 0..KEY_SLOTS {
                // Only write to committed layer if:
                // (a) staged slot has a value, and
                // (b) committed slot is still unwritten (write-once).
                if !is_slot_unwritten(state, LAYER_STAGED, slot, SLOT_COUNT)
                    && is_slot_unwritten(state, LAYER_COMMITTED, slot, SLOT_COUNT)
                {
                    // Commit-write must carry the exact staged value.
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
            TxnGoalProfile::CommittedSlot0IsV0 => {
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

    fn compile_world(world: &TransactionalKvStore) -> ByteStateV1 {
        let payload = world.encode_payload().expect("encode_payload");
        let schema = world.schema_descriptor();
        let registry = world.registry().expect("registry");
        let result = compile(&payload, &schema, &registry).expect("compile");
        result.state
    }

    #[test]
    fn initial_state_is_all_hole() {
        let world = TransactionalKvStore::commit_world();
        let state = compile_world(&world);
        // 2 layers × 4 slots = 8 identity entries, all PADDING.
        let identity = state.identity_bytes();
        assert_eq!(identity.len(), 2 * SLOT_COUNT * 4);
        for chunk in identity.chunks(4) {
            assert_eq!(chunk, &PADDING_BYTES);
        }
    }

    #[test]
    fn commit_program_runs_to_completion() {
        let world = TransactionalKvStore::commit_world();
        let mut state = compile_world(&world);
        let op_reg = op_reg();

        for step in world.program() {
            let (new_state, _record) =
                apply(&state, step.op_code, &step.op_args, &op_reg).expect("apply");
            state = new_state;
        }

        // Committed slot 0 should have kv:v0.
        let committed_slot0 = read_slot_identity(&state, LAYER_COMMITTED, 0, SLOT_COUNT);
        assert_eq!(committed_slot0, KV_VALUES[0].to_le_bytes());

        // Marker should be kv:commit.
        let marker = read_txn_marker(&state, SLOT_COUNT);
        assert_eq!(marker, KV_COMMIT.to_le_bytes());

        // Goal should be reached.
        assert!(world.is_goal(&state));
    }

    #[test]
    fn rollback_program_runs_to_completion() {
        let world = TransactionalKvStore::rollback_world();
        let mut state = compile_world(&world);
        let op_reg = op_reg();

        for step in world.program() {
            let (new_state, _record) =
                apply(&state, step.op_code, &step.op_args, &op_reg).expect("apply");
            state = new_state;
        }

        // Committed slot 0 should still be PADDING (untouched).
        let committed_slot0 = read_slot_identity(&state, LAYER_COMMITTED, 0, SLOT_COUNT);
        assert_eq!(committed_slot0, PADDING_BYTES);

        // Marker should be kv:rollback.
        let marker = read_txn_marker(&state, SLOT_COUNT);
        assert_eq!(marker, KV_ROLLBACK.to_le_bytes());

        // Goal should NOT be reached (committed layer unchanged).
        assert!(!world.is_goal(&state));
    }

    #[test]
    fn initial_state_is_not_goal() {
        let world = TransactionalKvStore::commit_world();
        let state = compile_world(&world);
        assert!(!world.is_goal(&state));
    }

    #[test]
    fn enumerate_candidates_from_initial_state() {
        let world = TransactionalKvStore::commit_world();
        let state = compile_world(&world);
        let op_reg = op_reg();

        let candidates = world.enumerate_candidates(&state, &op_reg);
        // 3 key slots × 3 values = 9 stage candidates.
        // No marker candidates (no staged keys yet).
        // No commit-write candidates (no marker).
        assert_eq!(candidates.len(), 9);
        for c in &candidates {
            assert_eq!(c.op_code, OP_SET_SLOT);
        }
    }

    #[test]
    fn enumerate_after_staging_includes_markers() {
        let world = TransactionalKvStore::commit_world();
        let mut state = compile_world(&world);
        let op_reg = op_reg();

        // Stage slot 0 with kv:v0.
        let (new_state, _) = apply(
            &state,
            OP_SET_SLOT,
            &set_slot_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
            &op_reg,
        )
        .expect("apply stage");
        state = new_state;

        let candidates = world.enumerate_candidates(&state, &op_reg);
        // 2 remaining key slots × 3 values = 6 stage candidates.
        // + 2 marker candidates (commit + rollback).
        // = 8 total.
        assert_eq!(candidates.len(), 8);
    }

    #[test]
    fn enumerate_after_commit_mark_has_commit_writes() {
        let world = TransactionalKvStore::commit_world();
        let mut state = compile_world(&world);
        let op_reg = op_reg();

        // Stage slot 0 with kv:v0.
        let (s1, _) = apply(
            &state,
            OP_SET_SLOT,
            &set_slot_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
            &op_reg,
        )
        .expect("apply stage");
        state = s1;

        // Mark commit.
        let (s2, _) = apply(
            &state,
            OP_SET_SLOT,
            &set_slot_args(LAYER_STAGED_U32, TXN_MARKER_SLOT_U32, KV_COMMIT),
            &op_reg,
        )
        .expect("apply commit mark");
        state = s2;

        let candidates = world.enumerate_candidates(&state, &op_reg);
        // Only commit-write candidates: 1 staged slot = 1 candidate.
        // No staging candidates (marker is set, not unset).
        // No marker candidates (marker is set).
        assert_eq!(candidates.len(), 1);

        // The commit-write carries the staged value.
        let c = &candidates[0];
        assert_eq!(c.op_code, OP_SET_SLOT);
        // Args should be: layer=0, slot=0, value=kv:v0
        let expected_args = set_slot_args(LAYER_COMMITTED_U32, 0, KV_VALUES[0]);
        assert_eq!(c.op_args, expected_args);
    }

    #[test]
    fn enumerate_after_rollback_mark_is_empty() {
        let world = TransactionalKvStore::commit_world();
        let mut state = compile_world(&world);
        let op_reg = op_reg();

        // Stage slot 0.
        let (s1, _) = apply(
            &state,
            OP_SET_SLOT,
            &set_slot_args(LAYER_STAGED_U32, 0, KV_VALUES[0]),
            &op_reg,
        )
        .expect("apply stage");
        state = s1;

        // Mark rollback.
        let (s2, _) = apply(
            &state,
            OP_SET_SLOT,
            &set_slot_args(LAYER_STAGED_U32, TXN_MARKER_SLOT_U32, KV_ROLLBACK),
            &op_reg,
        )
        .expect("apply rollback mark");
        state = s2;

        let candidates = world.enumerate_candidates(&state, &op_reg);
        assert!(candidates.is_empty(), "no actions after rollback");
    }

    #[test]
    fn enumeration_is_deterministic() {
        let world = TransactionalKvStore::commit_world();
        let state = compile_world(&world);
        let op_reg = op_reg();
        let c1 = world.enumerate_candidates(&state, &op_reg);
        let c2 = world.enumerate_candidates(&state, &op_reg);
        assert_eq!(c1, c2);
    }
}
