//! SC-001 lock tests: search determinism, graph completeness,
//! loop detection, dead-end semantics, budget enforcement, candidate legality,
//! score provenance, frontier pruning, goal path reconstruction, and metadata binding.

use sterling_harness::bundle::verify_bundle;
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::operators::operator_registry::{kernel_operator_registry, OperatorRegistryV1};
use sterling_kernel::operators::apply::{set_slot_args, OP_SET_SLOT};
use sterling_search::contract::SearchWorldV1;
use sterling_search::node::CandidateActionV1;
use sterling_search::policy::SearchPolicyV1;
use sterling_search::scorer::UniformScorer;
use sterling_search::search::{reconstruct_path, search, MetadataBindings};

fn default_bindings() -> MetadataBindings {
    MetadataBindings {
        world_id: "rome_mini_search".into(),
        schema_descriptor: "rome:1.0:test".into(),
        registry_digest: "test_registry_digest".into(),
        policy_snapshot_digest: "test_policy_digest".into(),
        search_policy_digest: "test_search_policy_digest".into(),
        scorer_digest: None,
        operator_set_digest: None,
    }
}

fn default_operator_registry() -> OperatorRegistryV1 {
    kernel_operator_registry()
}

fn root_state() -> ByteStateV1 {
    ByteStateV1::new(1, 2)
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-SEARCH-DETERMINISM-INPROC
// ---------------------------------------------------------------------------

#[test]
fn search_determinism_inproc_n10() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let operator_registry = default_operator_registry();
    let bindings = default_bindings();

    let first = search(
        root_state(),
        &RomeMiniSearch,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();
    let first_bytes = first.graph.to_canonical_json_bytes().unwrap();

    for _ in 1..10 {
        let other = search(
            root_state(),
            &RomeMiniSearch,
            &operator_registry,
            &policy,
            &scorer,
            &bindings,
        )
        .unwrap();
        let other_bytes = other.graph.to_canonical_json_bytes().unwrap();
        assert_eq!(
            first_bytes, other_bytes,
            "SearchGraphV1 bytes differ across runs"
        );
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-GRAPH-IN-BUNDLE
// ---------------------------------------------------------------------------

#[test]
fn graph_in_bundle_normative_and_verifiable() {
    let policy = SearchPolicyV1::default();
    let bundle = run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();

    let graph = bundle
        .artifacts
        .get("search_graph.json")
        .expect("missing search_graph.json");
    assert!(graph.normative, "search_graph.json must be normative");

    verify_bundle(&bundle).unwrap();
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-GRAPH-METADATA-BINDING
// ---------------------------------------------------------------------------

#[test]
fn graph_metadata_has_snapshot_bindings() {
    let policy = SearchPolicyV1::default();
    let bundle = run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();

    let graph = bundle.artifacts.get("search_graph.json").unwrap();
    let json: serde_json::Value = serde_json::from_slice(&graph.content).unwrap();
    let meta = &json["metadata"];

    assert!(meta["world_id"].is_string());
    assert!(meta["schema_descriptor"].is_string());
    assert!(meta["registry_digest"].is_string());
    assert!(meta["policy_snapshot_digest"].is_string());
    assert!(meta["search_policy_digest"].is_string());
    assert!(meta["root_state_fingerprint"].is_string());
    assert!(meta["dedup_key"].is_string());
    assert!(meta["prune_visited_policy"].is_string());
    assert!(meta["termination_reason"].is_object());
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-GRAPH-COMPLETENESS
// ---------------------------------------------------------------------------

#[test]
fn graph_completeness_every_pop_recorded() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let operator_registry = default_operator_registry();
    let bindings = default_bindings();

    let result = search(
        root_state(),
        &RomeMiniSearch,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    let graph = &result.graph;

    // Every expansion has expansion_order and candidates
    for (i, exp) in graph.expansions.iter().enumerate() {
        assert_eq!(
            exp.expansion_order, i as u64,
            "expansion_order should be sequential"
        );
        // Every candidate has an outcome (this is structural — no empty candidates lists
        // unless there truly were none)
    }

    // Total expansions in metadata matches expansion count
    assert_eq!(
        graph.metadata.total_expansions,
        graph.expansions.len() as u64
    );

    // Node summaries sorted by node_id ascending (INV-SC-09)
    for window in graph.node_summaries.windows(2) {
        assert!(
            window[0].node_id < window[1].node_id,
            "node_summaries must be sorted by node_id ascending"
        );
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-LOOP-DETECTION
// ---------------------------------------------------------------------------

/// A world that creates a cycle: from any state, enumerate a candidate
/// that returns to the initial state.
struct CycleWorld;

impl SearchWorldV1 for CycleWorld {
    #[allow(clippy::unnecessary_literal_bound)]
    fn world_id(&self) -> &str {
        "cycle_world"
    }

    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        _operator_registry: &OperatorRegistryV1,
    ) -> Vec<CandidateActionV1> {
        // Always propose SET_SLOT(0, 0, Code32::new(1,0,0)) — same value as initial padding
        // This creates a state that's already visited after the first expansion.
        let op_args = set_slot_args(0, 0, Code32::new(1, 0, 0));
        vec![CandidateActionV1::new(OP_SET_SLOT, op_args)]
    }

    fn is_goal(&self, _state: &ByteStateV1) -> bool {
        false // Never a goal — forces exhaustion
    }
}

#[test]
fn loop_detection_terminates_without_infinite_expansion() {
    let operator_registry = kernel_operator_registry();
    let policy = SearchPolicyV1 {
        max_expansions: 100,
        ..SearchPolicyV1::default()
    };
    let scorer = UniformScorer;
    let bindings = MetadataBindings {
        world_id: "cycle_world".into(),
        schema_descriptor: "test".into(),
        registry_digest: "test".into(),
        policy_snapshot_digest: "test".into(),
        search_policy_digest: "test".into(),
        scorer_digest: None,
        operator_set_digest: None,
    };

    let result = search(
        root_state(),
        &CycleWorld,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    // Should terminate with FrontierExhausted (not hit max_expansions=100)
    let bytes = result.graph.to_canonical_json_bytes().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let term = &json["metadata"]["termination_reason"];
    assert_eq!(term["type"], "frontier_exhausted");

    // Should have recorded duplicate suppressions in the graph
    assert!(
        result.graph.metadata.total_duplicates_suppressed > 0
            || result.graph.metadata.total_dead_ends_exhaustive > 0,
        "cycle detection should produce suppressions or dead ends"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-DEAD-END-SEMANTICS
// ---------------------------------------------------------------------------

/// A world that produces no candidates from any state.
struct NoMovesWorld;

impl SearchWorldV1 for NoMovesWorld {
    #[allow(clippy::unnecessary_literal_bound)]
    fn world_id(&self) -> &str {
        "no_moves_world"
    }

    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        _operator_registry: &OperatorRegistryV1,
    ) -> Vec<CandidateActionV1> {
        vec![] // No moves possible
    }

    fn is_goal(&self, _state: &ByteStateV1) -> bool {
        false
    }
}

#[test]
fn exhaustive_dead_end_tagged_correctly() {
    let operator_registry = kernel_operator_registry();
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bindings = MetadataBindings {
        world_id: "no_moves_world".into(),
        schema_descriptor: "test".into(),
        registry_digest: "test".into(),
        policy_snapshot_digest: "test".into(),
        search_policy_digest: "test".into(),
        scorer_digest: None,
        operator_set_digest: None,
    };

    let result = search(
        root_state(),
        &NoMovesWorld,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    // Should have 1 expansion with dead_end_reason = exhaustive
    assert_eq!(result.graph.expansions.len(), 1);
    let bytes = result.graph.to_canonical_json_bytes().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let reason = &json["expansions"][0]["dead_end_reason"];
    assert_eq!(reason, "exhaustive");
    assert_eq!(result.graph.metadata.total_dead_ends_exhaustive, 1);
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-BUDGET-EXPANSION-OVERFLOW
// ---------------------------------------------------------------------------

#[test]
fn expansion_budget_overflow() {
    // Use max_expansions=1 with the RomeMiniSearch world.
    // The first expansion finds the goal among its candidates, so we
    // need a world where the goal is not reachable in 1 expansion.
    // Use CycleWorld which never reaches a goal.
    let operator_registry = kernel_operator_registry();
    let policy = SearchPolicyV1 {
        max_expansions: 1,
        ..SearchPolicyV1::default()
    };
    let scorer = UniformScorer;
    let bindings = MetadataBindings {
        world_id: "cycle_world".into(),
        schema_descriptor: "test".into(),
        registry_digest: "test".into(),
        policy_snapshot_digest: "test".into(),
        search_policy_digest: "test".into(),
        scorer_digest: None,
        operator_set_digest: None,
    };

    let result = search(
        root_state(),
        &CycleWorld,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    let bytes = result.graph.to_canonical_json_bytes().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let term = &json["metadata"]["termination_reason"];
    assert_eq!(term["type"], "expansion_budget_exceeded");

    // Graph should be complete up to the cutoff point
    assert_eq!(result.graph.metadata.total_expansions, 1);
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-CANDIDATE-LEGALITY
// ---------------------------------------------------------------------------

/// A world that returns an illegal candidate (`op_code` not in registry).
struct IllegalCandidateWorld;

impl SearchWorldV1 for IllegalCandidateWorld {
    #[allow(clippy::unnecessary_literal_bound)]
    fn world_id(&self) -> &str {
        "illegal_candidate_world"
    }

    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        _operator_registry: &OperatorRegistryV1,
    ) -> Vec<CandidateActionV1> {
        let illegal_code = Code32::new(99, 99, 99);
        let op_args = vec![0u8; 12];
        vec![CandidateActionV1::new(illegal_code, op_args)]
    }

    fn is_goal(&self, _state: &ByteStateV1) -> bool {
        false
    }
}

#[test]
fn illegal_candidate_triggers_world_contract_violation() {
    let operator_registry = kernel_operator_registry();
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bindings = MetadataBindings {
        world_id: "illegal_candidate_world".into(),
        schema_descriptor: "test".into(),
        registry_digest: "test".into(),
        policy_snapshot_digest: "test".into(),
        search_policy_digest: "test".into(),
        scorer_digest: None,
        operator_set_digest: None,
    };

    // WorldContractViolation is now a soft termination — returns Ok with graph evidence
    let result = search(
        root_state(),
        &IllegalCandidateWorld,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .expect("search should return Ok even for contract violations");

    assert!(
        !result.is_goal_reached(),
        "illegal candidate world should not reach goal"
    );
    assert_eq!(
        result.graph.metadata.termination_reason,
        sterling_search::graph::TerminationReasonV1::WorldContractViolation,
        "expected WorldContractViolation termination reason"
    );
    // Graph must contain the partial expansion with the illegal candidate recorded
    assert!(
        !result.graph.expansions.is_empty(),
        "evidence graph must contain at least one expansion"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-SCORE-PROVENANCE
// ---------------------------------------------------------------------------

#[test]
fn score_provenance_recorded_for_all_candidates() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let operator_registry = default_operator_registry();
    let bindings = default_bindings();

    let result = search(
        root_state(),
        &RomeMiniSearch,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    let bytes = result.graph.to_canonical_json_bytes().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    // Every candidate record must have a score with source
    for expansion in json["expansions"].as_array().unwrap() {
        for candidate in expansion["candidates"].as_array().unwrap() {
            assert!(candidate["score"].is_object(), "missing score object");
            assert!(
                candidate["score"]["source"].is_string()
                    || candidate["score"]["source"].is_object(),
                "missing score source"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-FRONTIER-PRUNING
// ---------------------------------------------------------------------------

#[test]
fn frontier_pruning_records_pruned_ids() {
    // Use a very small frontier size to force pruning
    let policy = SearchPolicyV1 {
        max_frontier_size: 2,
        max_expansions: 10,
        ..SearchPolicyV1::default()
    };
    let scorer = UniformScorer;
    let operator_registry = default_operator_registry();
    let bindings = default_bindings();

    let result = search(
        root_state(),
        &RomeMiniSearch,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    let bytes = result.graph.to_canonical_json_bytes().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    // Check if frontier pruning occurred (there should be pruning notes)
    let mut found_prune = false;
    for expansion in json["expansions"].as_array().unwrap() {
        for note in expansion["notes"].as_array().unwrap() {
            if note["type"] == "frontier_pruned" {
                found_prune = true;
                assert!(
                    note["pruned_node_ids"].is_array(),
                    "pruning note should list pruned_node_ids"
                );
            }
        }
    }
    assert!(
        found_prune,
        "frontier pruning should have occurred with max_frontier_size=2"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-GOAL-PATH-RECONSTRUCTION
// ---------------------------------------------------------------------------

#[test]
fn goal_path_reconstruction() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let operator_registry = default_operator_registry();
    let bindings = default_bindings();

    let result = search(
        root_state(),
        &RomeMiniSearch,
        &operator_registry,
        &policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    let goal = result.goal_node.expect("should find goal");
    let path = reconstruct_path(&result.nodes, goal.node_id);

    // Path starts at root (node 0) and ends at goal
    assert_eq!(path[0], 0, "path should start at root");
    assert_eq!(
        *path.last().unwrap(),
        goal.node_id,
        "path should end at goal"
    );

    // Each step in the path has correct parent linkage
    for window in path.windows(2) {
        let child = result
            .nodes
            .iter()
            .find(|n| n.node_id == window[1])
            .unwrap();
        assert_eq!(
            child.parent_id,
            Some(window[0]),
            "parent linkage broken in path"
        );
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M1-GOV-LINT-MULTISPEC
// ---------------------------------------------------------------------------

#[test]
fn governance_lint_finds_search_core_spec() {
    // Verify that the SC-001.yaml spec file exists and contains
    // SC1-M1 acceptance IDs.
    let spec_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(".caws/specs/SC-001.yaml");
    assert!(spec_path.exists(), "SC-001.yaml should exist");

    let content = std::fs::read_to_string(&spec_path).unwrap();
    assert!(
        content.contains("SC1-M1-SEARCH-DETERMINISM-INPROC"),
        "spec should contain SC1-M1-SEARCH-DETERMINISM-INPROC"
    );
    assert!(
        content.contains("SC1-M1-GRAPH-IN-BUNDLE"),
        "spec should contain SC1-M1-GRAPH-IN-BUNDLE"
    );
}
