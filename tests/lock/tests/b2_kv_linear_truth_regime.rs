//! B2 linear truth-regime lock tests.
//!
//! Proves transactional KV store falsifiers using the harness `run()` pipeline:
//! - Commit path: stage→`commit_mark`→`commit_write` produces expected committed state
//! - Rollback path: stage→`rollback_mark` leaves committed layer unchanged
//! - Both paths produce verifiable traces (replay succeeds)
//! - Both paths produce valid artifact bundles (`verify_bundle` succeeds)
//!
//! These tests exercise the full harness pipeline (compile→execute→trace→verify→bundle)
//! and assert truth-regime invariants on the resulting state and artifacts.

use sterling_harness::bundle::verify_bundle;
use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::runner::run;
use sterling_harness::worlds::transactional_kv_store::TransactionalKvStore;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::operators::apply::apply;
use sterling_kernel::operators::operator_registry::kernel_operator_registry;
use sterling_search::contract::SearchWorldV1;

/// Helper: compile and execute a world's program step-by-step, returning
/// the final state and all step records.
fn execute_program(
    world: &TransactionalKvStore,
) -> (
    sterling_kernel::carrier::bytestate::ByteStateV1,
    Vec<sterling_kernel::operators::apply::StepRecord>,
) {
    let payload = world.encode_payload().expect("encode_payload");
    let schema = world.schema_descriptor();
    let registry = world.registry().expect("registry");
    let compiled = compile(&payload, &schema, &registry).expect("compile");
    let op_reg = kernel_operator_registry();

    let mut state = compiled.state;
    let mut records = Vec::new();

    for step in world.program() {
        let (new_state, record) =
            apply(&state, step.op_code, &step.op_args, &op_reg).expect("apply");
        state = new_state;
        records.push(record);
    }

    (state, records)
}

/// Helper: read a 4-byte identity slot from raw identity bytes.
fn read_slot(identity: &[u8], layer: usize, slot: usize, slot_count: usize) -> [u8; 4] {
    let offset = (layer * slot_count + slot) * 4;
    [
        identity[offset],
        identity[offset + 1],
        identity[offset + 2],
        identity[offset + 3],
    ]
}

const SLOT_COUNT: usize = 4; // 3 key slots + 1 txn_marker
const PADDING: [u8; 4] = [0, 0, 0, 0];
const KV_V0: [u8; 4] = [2, 1, 0, 0]; // Code32::new(2, 1, 0).to_le_bytes()
const KV_COMMIT: [u8; 4] = [2, 0, 1, 0]; // Code32::new(2, 0, 1).to_le_bytes()
const KV_ROLLBACK: [u8; 4] = [2, 0, 2, 0]; // Code32::new(2, 0, 2).to_le_bytes()
const TXN_MARKER_SLOT: usize = 3;

// ---------------------------------------------------------------------------
// Commit path tests
// ---------------------------------------------------------------------------

/// Falsifier: committed layer must reflect staged values after commit.
#[test]
fn commit_path_committed_layer_has_expected_values() {
    let world = TransactionalKvStore::commit_world();
    let (state, _records) = execute_program(&world);
    let identity = state.identity_bytes();

    // Committed slot 0 should have kv:v0.
    assert_eq!(
        read_slot(&identity, 0, 0, SLOT_COUNT),
        KV_V0,
        "committed slot 0 must be kv:v0 after commit"
    );

    // Committed slots 1, 2 should be untouched (PADDING).
    assert_eq!(
        read_slot(&identity, 0, 1, SLOT_COUNT),
        PADDING,
        "committed slot 1 must be unchanged"
    );
    assert_eq!(
        read_slot(&identity, 0, 2, SLOT_COUNT),
        PADDING,
        "committed slot 2 must be unchanged"
    );

    // Committed txn_marker slot should be untouched.
    assert_eq!(
        read_slot(&identity, 0, TXN_MARKER_SLOT, SLOT_COUNT),
        PADDING,
        "committed txn_marker must be unchanged"
    );
}

/// Falsifier: `txn_marker` must be `kv:commit` after commit path.
#[test]
fn commit_path_marker_is_commit() {
    let world = TransactionalKvStore::commit_world();
    let (state, _records) = execute_program(&world);
    let identity = state.identity_bytes();

    let marker = read_slot(&identity, 1, TXN_MARKER_SLOT, SLOT_COUNT);
    assert_eq!(marker, KV_COMMIT, "txn_marker must be kv:commit");
}

/// Falsifier: staged slot must retain its value (residue is allowed).
#[test]
fn commit_path_staged_residue_preserved() {
    let world = TransactionalKvStore::commit_world();
    let (state, _records) = execute_program(&world);
    let identity = state.identity_bytes();

    // Staged slot 0 was written during STAGE step; residue is allowed.
    let staged_slot0 = read_slot(&identity, 1, 0, SLOT_COUNT);
    assert_eq!(
        staged_slot0, KV_V0,
        "staged slot 0 retains kv:v0 (residue is documented)"
    );
}

/// The commit path program must have exactly 3 steps.
#[test]
fn commit_path_step_count() {
    let world = TransactionalKvStore::commit_world();
    let (_state, records) = execute_program(&world);
    assert_eq!(records.len(), 3, "commit path: stage + mark + write = 3 steps");
}

/// Commit-write carries the exact staged value (v in args == staged identity).
#[test]
fn commit_write_carries_staged_value() {
    let world = TransactionalKvStore::commit_world();
    let (_state, records) = execute_program(&world);

    // Step 0: stage slot 0 with kv:v0 on layer 1.
    // Step 1: mark commit on layer 1 txn_marker.
    // Step 2: commit-write slot 0 with kv:v0 on layer 0.

    // The value written in step 2 (commit-write) must equal the value in step 0 (stage).
    let stage_result_id = records[0].result_identity.as_slice();
    let commit_write_result_id = records[2].result_identity.as_slice();

    // After stage: layer 1 slot 0 has kv:v0.
    let staged_val = &stage_result_id[SLOT_COUNT * 4..SLOT_COUNT * 4 + 4];
    assert_eq!(staged_val, &KV_V0, "stage step wrote kv:v0 to layer 1 slot 0");

    // After commit-write: layer 0 slot 0 has kv:v0.
    let committed_val = &commit_write_result_id[0..4]; // layer 0, slot 0
    assert_eq!(committed_val, &KV_V0, "commit-write wrote kv:v0 to layer 0 slot 0");
}

/// Commit path produces a valid artifact bundle.
#[test]
fn commit_path_bundle_verifies() {
    let world = TransactionalKvStore::commit_world();
    let bundle = run(&world).expect("run commit world");
    verify_bundle(&bundle).expect("commit bundle must verify");
}

/// Commit path reaches the goal.
#[test]
fn commit_path_reaches_goal() {
    let world = TransactionalKvStore::commit_world();
    let (state, _records) = execute_program(&world);
    assert!(world.is_goal(&state), "commit path must reach goal");
}

// ---------------------------------------------------------------------------
// Rollback path tests
// ---------------------------------------------------------------------------

/// Falsifier: committed layer must be unchanged after rollback.
#[test]
fn rollback_path_committed_layer_unchanged() {
    let world = TransactionalKvStore::rollback_world();

    // Capture pre-transaction committed layer.
    let payload = world.encode_payload().expect("encode_payload");
    let schema = world.schema_descriptor();
    let registry = world.registry().expect("registry");
    let compiled = compile(&payload, &schema, &registry).expect("compile");
    let pre_committed_identity = compiled.state.identity_bytes();
    let pre_committed_layer: Vec<u8> = pre_committed_identity[..SLOT_COUNT * 4].to_vec();

    // Execute rollback program.
    let (state, _records) = execute_program(&world);
    let post_identity = state.identity_bytes();
    let post_committed_layer: Vec<u8> = post_identity[..SLOT_COUNT * 4].to_vec();

    assert_eq!(
        pre_committed_layer, post_committed_layer,
        "committed layer must be byte-identical after rollback"
    );
}

/// Falsifier: `txn_marker` must be `kv:rollback` after rollback path.
#[test]
fn rollback_path_marker_is_rollback() {
    let world = TransactionalKvStore::rollback_world();
    let (state, _records) = execute_program(&world);
    let identity = state.identity_bytes();

    let marker = read_slot(&identity, 1, TXN_MARKER_SLOT, SLOT_COUNT);
    assert_eq!(marker, KV_ROLLBACK, "txn_marker must be kv:rollback");
}

/// The rollback path program must have exactly 2 steps.
#[test]
fn rollback_path_step_count() {
    let world = TransactionalKvStore::rollback_world();
    let (_state, records) = execute_program(&world);
    assert_eq!(records.len(), 2, "rollback path: stage + mark = 2 steps");
}

/// Rollback path does NOT reach the goal.
#[test]
fn rollback_path_does_not_reach_goal() {
    let world = TransactionalKvStore::rollback_world();
    let (state, _records) = execute_program(&world);
    assert!(!world.is_goal(&state), "rollback path must not reach goal");
}

/// Rollback path produces a valid artifact bundle.
#[test]
fn rollback_path_bundle_verifies() {
    let world = TransactionalKvStore::rollback_world();
    let bundle = run(&world).expect("run rollback world");
    verify_bundle(&bundle).expect("rollback bundle must verify");
}

/// Staged slot retains its value after rollback (residue is allowed).
#[test]
fn rollback_path_staged_residue_preserved() {
    let world = TransactionalKvStore::rollback_world();
    let (state, _records) = execute_program(&world);
    let identity = state.identity_bytes();

    let staged_slot0 = read_slot(&identity, 1, 0, SLOT_COUNT);
    assert_eq!(
        staged_slot0, KV_V0,
        "staged slot 0 retains kv:v0 after rollback (residue is documented)"
    );
}

// ---------------------------------------------------------------------------
// Cross-path invariants
// ---------------------------------------------------------------------------

/// Both paths start from the same initial state.
#[test]
fn both_paths_share_initial_state() {
    let commit_world = TransactionalKvStore::commit_world();
    let rollback_world = TransactionalKvStore::rollback_world();

    let commit_payload = commit_world.encode_payload().expect("encode_payload");
    let rollback_payload = rollback_world.encode_payload().expect("encode_payload");
    assert_eq!(commit_payload, rollback_payload, "payloads must be identical");

    let commit_schema = commit_world.schema_descriptor();
    let rollback_schema = rollback_world.schema_descriptor();
    assert_eq!(commit_schema.id, rollback_schema.id);
    assert_eq!(commit_schema.hash, rollback_schema.hash);
}

/// Both worlds use the same registry.
#[test]
fn both_paths_share_registry() {
    let commit_world = TransactionalKvStore::commit_world();
    let rollback_world = TransactionalKvStore::rollback_world();

    let commit_reg = commit_world.registry().expect("registry");
    let rollback_reg = rollback_world.registry().expect("registry");

    let commit_snap = commit_reg.snapshot().expect("snapshot");
    let rollback_snap = rollback_reg.snapshot().expect("snapshot");
    assert_eq!(commit_snap.epoch, rollback_snap.epoch);
    assert_eq!(commit_snap.hash, rollback_snap.hash);
}

/// Dimensions are identical across both worlds.
#[test]
fn both_paths_share_dimensions() {
    let commit_world = TransactionalKvStore::commit_world();
    let rollback_world = TransactionalKvStore::rollback_world();

    let cd = commit_world.dimensions();
    let rd = rollback_world.dimensions();
    assert_eq!(cd.layer_count, rd.layer_count);
    assert_eq!(cd.slot_count, rd.slot_count);
    assert_eq!(cd.arg_slot_count, rd.arg_slot_count);
    assert_eq!(cd.layer_count, 2, "must be 2-layer");
    assert_eq!(cd.slot_count, 4, "must be 4 slots (3 key + 1 marker)");
}
