//! SEARCH-CORE-001 M2 hardening lock tests.
//!
//! Tests for evidence preservation (panic/scorer/world contract violations),
//! symmetric bundle verification, generic world wiring, candidate canonicalization,
//! and panic-profile enforcement.

#![allow(clippy::unnecessary_literal_bound)]

use sterling_harness::bundle::{
    build_bundle, verify_bundle, BundleVerifyError, DOMAIN_BUNDLE_ARTIFACT,
};
use sterling_harness::runner::run_search;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply::{set_slot_args, OP_SET_SLOT};
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;
use sterling_search::contract::SearchWorldV1;
use sterling_search::graph::{PanicStageV1, TerminationReasonV1};
use sterling_search::node::{candidate_canonical_hash, CandidateActionV1, DOMAIN_SEARCH_NODE};
use sterling_search::policy::SearchPolicyV1;
use sterling_search::scorer::{CandidateScoreV1, ScoreSourceV1, UniformScorer, ValueScorer};
use sterling_search::search::{search, MetadataBindings};

use sterling_harness::contract::WorldHarnessV1;

fn default_bindings() -> MetadataBindings {
    MetadataBindings {
        world_id: "rome_mini_search".into(),
        schema_descriptor: "rome:1.0:test".into(),
        registry_digest: "test_registry_digest".into(),
        policy_snapshot_digest: "test_policy_digest".into(),
        search_policy_digest: "test_search_policy_digest".into(),
    }
}

fn default_registry() -> RegistryV1 {
    RomeMiniSearch.registry().unwrap()
}

fn root_state() -> ByteStateV1 {
    ByteStateV1::new(1, 2)
}

// ---------------------------------------------------------------------------
// Panic worlds/scorers for testing
// ---------------------------------------------------------------------------

struct PanicScorer;
impl ValueScorer for PanicScorer {
    fn score_candidates(
        &self,
        _node: &sterling_search::node::SearchNodeV1,
        _candidates: &[CandidateActionV1],
    ) -> Vec<CandidateScoreV1> {
        panic!("test panic in scorer");
    }
}

struct WrongArityScorer;
impl ValueScorer for WrongArityScorer {
    fn score_candidates(
        &self,
        _node: &sterling_search::node::SearchNodeV1,
        _candidates: &[CandidateActionV1],
    ) -> Vec<CandidateScoreV1> {
        // Always return exactly 1 score regardless of input length.
        vec![CandidateScoreV1 {
            bonus: 0,
            source: ScoreSourceV1::Uniform,
        }]
    }
}

struct PanicEnumerateWorld;
impl SearchWorldV1 for PanicEnumerateWorld {
    fn world_id(&self) -> &str {
        "panic_enumerate_world"
    }
    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        _registry: &RegistryV1,
    ) -> Vec<CandidateActionV1> {
        panic!("test panic in enumerate_candidates");
    }
    fn is_goal(&self, _state: &ByteStateV1) -> bool {
        false
    }
}

struct PanicGoalWorld;
impl SearchWorldV1 for PanicGoalWorld {
    fn world_id(&self) -> &str {
        "panic_goal_world"
    }
    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        _registry: &RegistryV1,
    ) -> Vec<CandidateActionV1> {
        vec![]
    }
    fn is_goal(&self, _state: &ByteStateV1) -> bool {
        panic!("test panic in is_goal");
    }
}

/// A world that panics in `is_goal` only after a child is created.
struct PanicGoalExpansionWorld {
    call_count: std::cell::Cell<u32>,
}
impl PanicGoalExpansionWorld {
    fn new() -> Self {
        Self {
            call_count: std::cell::Cell::new(0),
        }
    }
}
impl SearchWorldV1 for PanicGoalExpansionWorld {
    fn world_id(&self) -> &str {
        "panic_goal_expansion_world"
    }
    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        _registry: &RegistryV1,
    ) -> Vec<CandidateActionV1> {
        let op_args = set_slot_args(0, 0, Code32::new(2, 0, 0));
        vec![CandidateActionV1::new(OP_SET_SLOT, op_args)]
    }
    fn is_goal(&self, _state: &ByteStateV1) -> bool {
        let count = self.call_count.get();
        self.call_count.set(count + 1);
        if count == 0 {
            false // root check passes
        } else {
            panic!("test panic in is_goal during expansion");
        }
    }
}

/// No-moves world for testing non-goal terminations.
struct NoMovesWorld;
impl SearchWorldV1 for NoMovesWorld {
    fn world_id(&self) -> &str {
        "no_moves_world"
    }
    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        _registry: &RegistryV1,
    ) -> Vec<CandidateActionV1> {
        vec![]
    }
    fn is_goal(&self, _state: &ByteStateV1) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-SCORER-PANIC-PRESERVES-GRAPH
// ---------------------------------------------------------------------------

#[test]
fn scorer_panic_preserves_graph() {
    let registry = default_registry();
    let policy = SearchPolicyV1::default();
    let bindings = default_bindings();

    let result = search(
        root_state(),
        &RomeMiniSearch,
        &registry,
        &policy,
        &PanicScorer,
        &bindings,
    )
    .expect("search should return Ok even when scorer panics");

    assert!(!result.is_goal_reached());
    assert_eq!(
        result.graph.metadata.termination_reason,
        TerminationReasonV1::InternalPanic {
            stage: PanicStageV1::ScoreCandidates,
        }
    );
    // At least root node + one expansion event for the panic
    assert!(
        !result.graph.expansions.is_empty(),
        "graph must contain expansion events"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-ENUMERATE-PANIC-PRESERVES-GRAPH
// ---------------------------------------------------------------------------

#[test]
fn enumerate_panic_preserves_graph() {
    let registry = default_registry();
    let policy = SearchPolicyV1::default();
    let mut bindings = default_bindings();
    bindings.world_id = "panic_enumerate_world".into();

    let result = search(
        root_state(),
        &PanicEnumerateWorld,
        &registry,
        &policy,
        &UniformScorer,
        &bindings,
    )
    .expect("search should return Ok even when enumerate_candidates panics");

    assert!(!result.is_goal_reached());
    assert_eq!(
        result.graph.metadata.termination_reason,
        TerminationReasonV1::InternalPanic {
            stage: PanicStageV1::EnumerateCandidates,
        }
    );
    assert!(
        !result.graph.expansions.is_empty(),
        "graph must record partial expansion event"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-IS-GOAL-PANIC-PRESERVES-GRAPH
// ---------------------------------------------------------------------------

#[test]
fn is_goal_root_panic_preserves_graph() {
    let registry = default_registry();
    let policy = SearchPolicyV1::default();
    let mut bindings = default_bindings();
    bindings.world_id = "panic_goal_world".into();

    let result = search(
        root_state(),
        &PanicGoalWorld,
        &registry,
        &policy,
        &UniformScorer,
        &bindings,
    )
    .expect("search should return Ok even when is_goal panics on root");

    assert!(!result.is_goal_reached());
    assert_eq!(
        result.graph.metadata.termination_reason,
        TerminationReasonV1::InternalPanic {
            stage: PanicStageV1::IsGoalRoot,
        }
    );
}

#[test]
fn is_goal_expansion_panic_preserves_graph() {
    let registry = default_registry();
    let policy = SearchPolicyV1::default();
    let mut bindings = default_bindings();
    bindings.world_id = "panic_goal_expansion_world".into();

    let world = PanicGoalExpansionWorld::new();
    let result = search(
        root_state(),
        &world,
        &registry,
        &policy,
        &UniformScorer,
        &bindings,
    )
    .expect("search should return Ok even when is_goal panics during expansion");

    assert!(!result.is_goal_reached());
    assert_eq!(
        result.graph.metadata.termination_reason,
        TerminationReasonV1::InternalPanic {
            stage: PanicStageV1::IsGoalExpansion,
        }
    );
    // Should have recorded at least one expansion
    assert!(
        !result.graph.expansions.is_empty(),
        "graph must record expansion events before panic"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-SCORER-CONTRACT-VIOLATION-PRESERVES-GRAPH
// ---------------------------------------------------------------------------

#[test]
fn scorer_wrong_arity_preserves_graph() {
    let registry = default_registry();
    let policy = SearchPolicyV1::default();
    let bindings = default_bindings();

    let result = search(
        root_state(),
        &RomeMiniSearch,
        &registry,
        &policy,
        &WrongArityScorer,
        &bindings,
    )
    .expect("search should return Ok for scorer contract violation");

    assert!(!result.is_goal_reached());
    match &result.graph.metadata.termination_reason {
        TerminationReasonV1::ScorerContractViolation { expected, actual } => {
            assert!(*expected > 1, "RomeMiniSearch has >1 candidates");
            assert_eq!(*actual, 1, "WrongArityScorer always returns 1");
        }
        other => panic!("expected ScorerContractViolation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-IS-GOAL-REACHED-HELPER
// ---------------------------------------------------------------------------

#[test]
fn is_goal_reached_helper() {
    let registry = default_registry();
    let policy = SearchPolicyV1::default();
    let bindings = default_bindings();

    // Goal found → true
    let goal_result = search(
        root_state(),
        &RomeMiniSearch,
        &registry,
        &policy,
        &UniformScorer,
        &bindings,
    )
    .unwrap();
    assert!(goal_result.is_goal_reached());

    // Frontier exhausted → false
    let mut no_moves_bindings = default_bindings();
    no_moves_bindings.world_id = "no_moves_world".into();
    let no_goal = search(
        root_state(),
        &NoMovesWorld,
        &registry,
        &policy,
        &UniformScorer,
        &no_moves_bindings,
    )
    .unwrap();
    assert!(!no_goal.is_goal_reached());
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-PREFLIGHT-UNSUPPORTED-MODE-ERR-NO-GRAPH
// ---------------------------------------------------------------------------

#[test]
fn preflight_unsupported_mode_returns_err_no_graph() {
    let registry = default_registry();
    let mut policy = SearchPolicyV1::default();
    policy.dedup_key = sterling_search::policy::DedupKeyV1::FullState; // reserved in M1
    let bindings = default_bindings();

    let err = search(
        root_state(),
        &RomeMiniSearch,
        &registry,
        &policy,
        &UniformScorer,
        &bindings,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            sterling_search::error::SearchError::UnsupportedPolicyMode { .. }
        ),
        "expected UnsupportedPolicyMode, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-TERMINATION-DETAILS-DETERMINISTIC
// ---------------------------------------------------------------------------

#[test]
fn termination_details_are_deterministic() {
    let registry = default_registry();
    let policy = SearchPolicyV1::default();
    let bindings = default_bindings();

    // Run same scorer-panic scenario twice, check JSON is identical
    let r1 = search(
        root_state(),
        &RomeMiniSearch,
        &registry,
        &policy,
        &PanicScorer,
        &bindings,
    )
    .unwrap();
    let r2 = search(
        root_state(),
        &RomeMiniSearch,
        &registry,
        &policy,
        &PanicScorer,
        &bindings,
    )
    .unwrap();

    let b1 = r1.graph.to_canonical_json_bytes().unwrap();
    let b2 = r2.graph.to_canonical_json_bytes().unwrap();
    assert_eq!(
        b1, b2,
        "termination payloads must be byte-identical across runs"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-SEARCH-GRAPH-DIGEST-MANDATORY
// ---------------------------------------------------------------------------

#[test]
fn search_graph_digest_mandatory_in_search_bundle() {
    // Build a valid search bundle, then mutate the report to remove search_graph_digest
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bundle = run_search(&RomeMiniSearch, &policy, &scorer).unwrap();

    // Reconstruct bundle with modified report (search_graph_digest removed)
    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();
    report_json
        .as_object_mut()
        .unwrap()
        .remove("search_graph_digest");
    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    // Rebuild bundle with modified report
    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "verification_report.json" {
                (a.name.clone(), modified_report_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let modified_bundle = build_bundle(artifacts).unwrap();

    let err = verify_bundle(&modified_bundle).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::SearchGraphDigestMissing),
        "expected SearchGraphDigestMissing, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-MODE-COHERENCE-SEARCH-NO-GRAPH
// ---------------------------------------------------------------------------

#[test]
fn mode_search_requires_search_graph_artifact() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bundle = run_search(&RomeMiniSearch, &policy, &scorer).unwrap();

    // Rebuild bundle without search_graph.json
    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .filter(|a| a.name != "search_graph.json")
        .map(|a| (a.name.clone(), a.content.clone(), a.normative))
        .collect();
    let modified_bundle = build_bundle(artifacts).unwrap();

    let err = verify_bundle(&modified_bundle).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::SearchGraphArtifactMissing),
        "expected SearchGraphArtifactMissing, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-MODE-COHERENCE-GRAPH-NO-SEARCH
// ---------------------------------------------------------------------------

#[test]
fn search_graph_requires_mode_search() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bundle = run_search(&RomeMiniSearch, &policy, &scorer).unwrap();

    // Modify report to say mode="linear" instead of "search"
    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();
    report_json["mode"] = serde_json::json!("linear");
    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "verification_report.json" {
                (a.name.clone(), modified_report_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let modified_bundle = build_bundle(artifacts).unwrap();

    let err = verify_bundle(&modified_bundle).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::ModeSearchExpected { .. }),
        "expected ModeSearchExpected, got {err:?}"
    );
}

/// Modify the `search_graph.json` in a bundle and rebuild with consistent
/// `search_graph_digest` in the report, so that digest-binding checks pass
/// and only the metadata-binding check under test fires.
fn rebuild_with_modified_graph(
    bundle: &sterling_harness::bundle::ArtifactBundleV1,
    modify: impl FnOnce(&mut serde_json::Value),
) -> sterling_harness::bundle::ArtifactBundleV1 {
    let graph_artifact = bundle.artifacts.get("search_graph.json").unwrap();
    let mut graph_json: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).unwrap();
    modify(&mut graph_json);
    let modified_graph_bytes = canonical_json_bytes(&graph_json).unwrap();

    // Recompute the content hash for the modified graph.
    let new_graph_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_graph_bytes);

    // Update the report's search_graph_digest to match the modified graph.
    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();
    report_json["search_graph_digest"] = serde_json::json!(new_graph_hash.as_str());
    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "search_graph.json" {
                (a.name.clone(), modified_graph_bytes.clone(), a.normative)
            } else if a.name == "verification_report.json" {
                (a.name.clone(), modified_report_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    build_bundle(artifacts).unwrap()
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-METADATA-BINDING-POLICY-DIGEST
// ---------------------------------------------------------------------------

#[test]
fn metadata_binding_policy_digest_mismatch_detected() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bundle = run_search(&RomeMiniSearch, &policy, &scorer).unwrap();

    let modified_bundle = rebuild_with_modified_graph(&bundle, |graph_json| {
        graph_json["metadata"]["policy_snapshot_digest"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
    });

    let err = verify_bundle(&modified_bundle).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::MetadataBindingPolicyMismatch { .. }),
        "expected MetadataBindingPolicyMismatch, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-METADATA-BINDING-WORLD-ID
// ---------------------------------------------------------------------------

#[test]
fn metadata_binding_world_id_mismatch_detected() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bundle = run_search(&RomeMiniSearch, &policy, &scorer).unwrap();

    let modified_bundle = rebuild_with_modified_graph(&bundle, |graph_json| {
        graph_json["metadata"]["world_id"] = serde_json::json!("wrong_world");
    });

    let err = verify_bundle(&modified_bundle).unwrap_err();
    assert!(
        matches!(
            err,
            BundleVerifyError::MetadataBindingWorldIdMismatch { .. }
        ),
        "expected MetadataBindingWorldIdMismatch, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-RUN-SEARCH-GENERIC-SINGLE-WORLD
// ---------------------------------------------------------------------------

#[test]
fn run_search_generic_single_world_compiles_and_runs() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    // This test verifies the generic signature compiles with a single world arg.
    let bundle = run_search(&RomeMiniSearch, &policy, &scorer).unwrap();
    assert_eq!(bundle.artifacts.len(), 5);
    verify_bundle(&bundle).unwrap();
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-CANDIDATE-DOMAIN-SEPARATION
// ---------------------------------------------------------------------------

#[test]
fn candidate_domain_separation() {
    let code = Code32::new(1, 1, 1);
    let args = vec![0u8; 12];
    let cand_hash = candidate_canonical_hash(code, &args);

    // Same input bytes through the node domain must differ
    let mut data = Vec::with_capacity(4 + args.len());
    data.extend_from_slice(&code.to_le_bytes());
    data.extend_from_slice(&args);
    let node_hash = canonical_hash(DOMAIN_SEARCH_NODE, &data);

    assert_ne!(
        cand_hash.as_str(),
        node_hash.as_str(),
        "DOMAIN_SEARCH_CANDIDATE and DOMAIN_SEARCH_NODE must produce different hashes"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-CANDIDATE-CONSTRUCTOR-DETERMINISTIC
// ---------------------------------------------------------------------------

#[test]
fn candidate_constructor_deterministic() {
    let a = CandidateActionV1::new(Code32::new(1, 0, 0), vec![0u8; 12]);
    let b = CandidateActionV1::new(Code32::new(1, 0, 0), vec![0u8; 12]);
    assert_eq!(a, b, "CandidateActionV1::new must be deterministic");
    assert_eq!(
        a.canonical_hash(),
        b.canonical_hash(),
        "canonical_hash must match for identical inputs"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M2-PANIC-PROFILE-UNWIND-ENFORCED
// ---------------------------------------------------------------------------

#[test]
fn panic_profile_unwind_enforced() {
    // Verify that catch_unwind actually works (would fail silently with panic=abort)
    let result = std::panic::catch_unwind(|| {
        panic!("test panic for unwind verification");
    });
    assert!(
        result.is_err(),
        "catch_unwind must catch panics (panic=unwind required)"
    );
}
