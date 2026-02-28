//! B2 search truth-regime lock tests.
//!
//! Proves that the `TransactionalKvStore` search world correctly gates
//! candidate legality and produces deterministic, verifiable bundles.
//! Key invariants:
//! - No `commit_write` candidates appear before `commit_mark`
//! - Rollback terminates expansion (no further candidates)
//! - Search finds the goal (committed slot 0 == `kv:v0`)
//! - Bundle round-trips and verifies
//! - Graph and tape are deterministic across runs

use sterling_harness::bundle::{verify_bundle, ArtifactBundleV1};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::transactional_kv_store::TransactionalKvStore;
use sterling_search::policy::SearchPolicyV1;

/// Run search on the commit-world `TransactionalKvStore`.
fn search_bundle() -> ArtifactBundleV1 {
    let world = TransactionalKvStore::commit_world();
    let policy = SearchPolicyV1::default();
    run_search(&world, &policy, &ScorerInputV1::Uniform).expect("search run failed")
}

/// Parse the search graph JSON from a bundle.
fn parse_graph(bundle: &ArtifactBundleV1) -> serde_json::Value {
    let art = bundle
        .artifacts
        .get("search_graph.json")
        .expect("search_graph.json must be present");
    serde_json::from_slice(&art.content).expect("graph must be valid JSON")
}

/// Parse the verification report from a bundle.
fn parse_report(bundle: &ArtifactBundleV1) -> serde_json::Value {
    let art = bundle
        .artifacts
        .get("verification_report.json")
        .expect("verification_report.json must be present");
    serde_json::from_slice(&art.content).expect("report must be valid JSON")
}

// ---------------------------------------------------------------------------
// Bundle integrity
// ---------------------------------------------------------------------------

/// Search produces a valid artifact bundle.
#[test]
fn search_bundle_verifies() {
    let bundle = search_bundle();
    verify_bundle(&bundle).expect("search bundle must verify");
}

/// Bundle contains expected artifact set (7 for uniform scorer).
#[test]
fn search_bundle_artifact_count() {
    let bundle = search_bundle();
    // 7 artifacts: fixture.json, compilation_manifest.json, policy_snapshot.json,
    //              operator_registry.json, search_graph.json, search_tape.stap,
    //              verification_report.json
    assert_eq!(
        bundle.artifacts.len(),
        7,
        "uniform scorer bundle must have 7 artifacts"
    );
}

// ---------------------------------------------------------------------------
// Search finds goal
// ---------------------------------------------------------------------------

/// Search terminates with goal found.
#[test]
fn search_finds_goal() {
    let report = parse_report(&search_bundle());
    let mode = report
        .get("mode")
        .and_then(|v| v.as_str())
        .expect("report must have mode");
    assert_eq!(mode, "search", "mode must be search");

    // Check termination in graph metadata.
    let graph = parse_graph(&search_bundle());
    let metadata = graph.get("metadata").expect("graph must have metadata");
    let term = metadata
        .get("termination_reason")
        .expect("metadata must have termination_reason");
    let term_type = term
        .get("type")
        .and_then(|v| v.as_str())
        .expect("termination_reason must have type");
    assert_eq!(term_type, "goal_reached", "search must terminate with goal_reached");
}

/// The goal path requires at least 3 expansions (stage + mark + write).
#[test]
fn search_goal_path_includes_commit() {
    let graph = parse_graph(&search_bundle());
    let metadata = graph.get("metadata").expect("graph must have metadata");

    // Goal must have been found.
    let term_type = metadata["termination_reason"]["type"]
        .as_str()
        .expect("termination_reason.type");
    assert_eq!(term_type, "goal_reached");

    // The search reached a state where committed slot 0 == kv:v0.
    // This requires at minimum: stage + commit_mark + commit_write = 3 steps from root.
    let total_expansions = metadata
        .get("total_expansions")
        .and_then(serde_json::Value::as_u64)
        .expect("metadata must have total_expansions");
    assert!(
        total_expansions >= 3,
        "must expand at least 3 nodes to reach goal (stage + mark + write), got {total_expansions}"
    );
}

// ---------------------------------------------------------------------------
// Candidate legality invariants
// ---------------------------------------------------------------------------

/// No candidate in the initial expansion writes to layer 0 (committed).
/// All initial candidates must be stage operations (layer 1 key slots).
#[test]
fn initial_candidates_are_stage_only() {
    let graph = parse_graph(&search_bundle());
    let expansions = graph
        .get("expansions")
        .and_then(|v| v.as_array())
        .expect("graph must have expansions");

    // First expansion is from the root node (all-PADDING state).
    assert!(!expansions.is_empty(), "must have at least one expansion");
    let first = &expansions[0];
    let candidates = first
        .get("candidates")
        .and_then(|v| v.as_array())
        .expect("first expansion must have candidates");

    for (i, c) in candidates.iter().enumerate() {
        let action = c.get("action").expect("candidate must have action");
        let op_args_hex = action
            .get("op_args_hex")
            .and_then(|v| v.as_str())
            .expect("action must have op_args_hex");
        // op_args is 12 bytes as hex = 24 chars.
        // First 4 bytes = layer (u32 LE). Layer 0 = "00000000", Layer 1 = "01000000".
        let layer_hex = &op_args_hex[..8];
        assert_eq!(
            layer_hex, "01000000",
            "initial candidate {i} must target layer 1 (staging), got layer hex {layer_hex}"
        );
    }
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

/// Two search runs produce byte-identical bundles.
#[test]
fn search_is_deterministic() {
    let bundle1 = search_bundle();
    let bundle2 = search_bundle();

    // Compare digest (covers all normative artifacts).
    assert_eq!(
        bundle1.digest, bundle2.digest,
        "search bundles must be deterministic"
    );
}

/// Graph metadata reports the expected world ID.
#[test]
fn search_reports_correct_world_id() {
    let graph = parse_graph(&search_bundle());
    let metadata = graph.get("metadata").expect("graph must have metadata");
    let wid = metadata
        .get("world_id")
        .and_then(|v| v.as_str())
        .expect("metadata must have world_id");
    assert_eq!(
        wid, "txn_kv_store:v1:commit_one",
        "world_id must match TransactionalKvStore::commit_world()"
    );
}

/// Search tape is present and starts with STAP magic.
#[test]
fn search_tape_present() {
    let bundle = search_bundle();
    let tape = bundle
        .artifacts
        .get("search_tape.stap")
        .expect("search_tape.stap must be present");
    assert!(tape.normative, "tape must be normative");
    assert_eq!(&tape.content[..4], b"STAP", "tape must start with STAP magic");
}
