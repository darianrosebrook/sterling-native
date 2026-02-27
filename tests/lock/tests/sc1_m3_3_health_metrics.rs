//! SEARCH-CORE-001 M3.3 lock tests: search health metrics.
//!
//! Each test targets a specific acceptance criterion from the M3.3 milestone.
//! Health metrics are DIAGNOSTIC (INV-SC-M33-02): they do not participate in
//! any binding digest and verification must not fail due to their content.

use sterling_harness::bundle::verify_bundle;
use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};
use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::slot_lattice_regimes::{regime_duplicates, Regime};
use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_search::contract::SearchWorldV1;
use sterling_search::graph::{
    ApplyFailureKindV1, CandidateOutcomeV1, CandidateRecordV1, ExpandEventV1, FrontierPopKeyV1,
    SearchGraphMetadata, SearchGraphNodeSummaryV1, SearchGraphV1, TerminationReasonV1,
};
use sterling_search::node::CandidateActionV1;
use sterling_search::policy::{DedupKeyV1, PruneVisitedPolicyV1};
use sterling_search::scorer::{CandidateScoreV1, ScoreSourceV1, UniformScorer};
use sterling_search::search::{search, MetadataBindings};

/// Build `MetadataBindings` from a regime, matching what `run_search` does internally.
fn bindings_for(regime: &Regime) -> MetadataBindings {
    let world = &regime.world;
    let sd = world.schema_descriptor();
    MetadataBindings {
        world_id: SearchWorldV1::world_id(world).to_string(),
        schema_descriptor: format!("{}:{}:{}", sd.id, sd.version, sd.hash),
        registry_digest: "test_registry_digest".into(),
        policy_snapshot_digest: "test_policy_digest".into(),
        search_policy_digest: "test_search_policy_digest".into(),
        scorer_digest: None,
    }
}

/// Run search directly for `regime_duplicates`.
fn run_duplicates_search() -> sterling_search::search::SearchResult {
    let regime = regime_duplicates();
    let registry = regime.world.registry().unwrap();
    let root = ByteStateV1::new(1, 10);
    let bindings = bindings_for(&regime);
    search(
        root,
        &regime.world,
        &registry,
        &regime.policy,
        &UniformScorer,
        &bindings,
    )
    .unwrap()
}

/// Build a hand-crafted `SearchGraphV1` that exercises all 7 `CandidateOutcomeV1`
/// variants in a single graph. Used for the OUTCOME-EXHAUSTIVE test.
#[allow(clippy::too_many_lines)]
fn make_outcome_exhaustive_fixture() -> SearchGraphV1 {
    let dummy_action = CandidateActionV1::new(
        sterling_kernel::carrier::code32::Code32::new(0, 0, 1),
        vec![0u8; 4],
    );
    let uniform_score = CandidateScoreV1 {
        bonus: 0,
        source: ScoreSourceV1::Uniform,
    };

    let candidates = vec![
        CandidateRecordV1 {
            index: 0,
            action: dummy_action.clone(),
            score: uniform_score.clone(),
            outcome: CandidateOutcomeV1::Applied { to_node: 1 },
        },
        CandidateRecordV1 {
            index: 1,
            action: dummy_action.clone(),
            score: uniform_score.clone(),
            outcome: CandidateOutcomeV1::DuplicateSuppressed {
                existing_fingerprint: "abc123".into(),
            },
        },
        CandidateRecordV1 {
            index: 2,
            action: dummy_action.clone(),
            score: uniform_score.clone(),
            outcome: CandidateOutcomeV1::SkippedByDepthLimit,
        },
        CandidateRecordV1 {
            index: 3,
            action: dummy_action.clone(),
            score: uniform_score.clone(),
            outcome: CandidateOutcomeV1::SkippedByPolicy,
        },
        CandidateRecordV1 {
            index: 4,
            action: dummy_action.clone(),
            score: uniform_score.clone(),
            outcome: CandidateOutcomeV1::ApplyFailed(ApplyFailureKindV1::PreconditionNotMet),
        },
        CandidateRecordV1 {
            index: 5,
            action: dummy_action.clone(),
            score: uniform_score.clone(),
            outcome: CandidateOutcomeV1::IllegalOperator,
        },
        CandidateRecordV1 {
            index: 6,
            action: dummy_action,
            score: CandidateScoreV1 {
                bonus: 0,
                source: ScoreSourceV1::Unavailable,
            },
            outcome: CandidateOutcomeV1::NotEvaluated,
        },
    ];

    let expansion = ExpandEventV1 {
        expansion_order: 0,
        node_id: 0,
        state_fingerprint: "root_fp".into(),
        frontier_pop_key: FrontierPopKeyV1 {
            f_cost: 0,
            depth: 0,
            creation_order: 0,
        },
        candidates,
        candidates_truncated: false,
        dead_end_reason: None,
        notes: Vec::new(),
    };

    let node_summaries = vec![
        SearchGraphNodeSummaryV1 {
            node_id: 0,
            parent_id: None,
            state_fingerprint: "root_fp".into(),
            depth: 0,
            f_cost: 0,
            is_goal: false,
            dead_end_reason: None,
            expansion_order: Some(0),
        },
        SearchGraphNodeSummaryV1 {
            node_id: 1,
            parent_id: Some(0),
            state_fingerprint: "child_fp".into(),
            depth: 1,
            f_cost: 1,
            is_goal: false,
            dead_end_reason: None,
            expansion_order: None,
        },
    ];

    SearchGraphV1 {
        expansions: vec![expansion],
        node_summaries,
        metadata: SearchGraphMetadata {
            world_id: "test_exhaustive".into(),
            schema_descriptor: "test:v1".into(),
            registry_digest: "abc123".into(),
            policy_snapshot_digest: "def456".into(),
            search_policy_digest: "ghi789".into(),
            root_state_fingerprint: "root_fp".into(),
            scorer_digest: None,
            total_expansions: 1,
            total_candidates_generated: 7,
            total_duplicates_suppressed: 1,
            total_dead_ends_exhaustive: 0,
            total_dead_ends_budget_limited: 0,
            termination_reason: TerminationReasonV1::WorldContractViolation,
            frontier_high_water: 1,
            dedup_key: DedupKeyV1::IdentityOnly,
            prune_visited_policy: PruneVisitedPolicyV1::KeepVisited,
        },
    }
}

// ---------------------------------------------------------------------------
// SC1-M3.3-METRICS-PURE
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_metrics_pure() {
    let result = run_duplicates_search();
    let a = result.graph.compute_health_metrics();
    let b = result.graph.compute_health_metrics();
    assert_eq!(a, b, "compute_health_metrics must be deterministic (struct equality)");
}

// ---------------------------------------------------------------------------
// SC1-M3.3-METRICS-GOLDEN
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_metrics_golden() {
    let result = run_duplicates_search();
    let metrics = result.graph.compute_health_metrics();
    let json_value = metrics.to_json_value();
    let bytes = canonical_json_bytes(&json_value).unwrap();

    // Verify determinism: 10 runs produce identical bytes.
    for _ in 0..9 {
        let r = run_duplicates_search();
        let m = r.graph.compute_health_metrics();
        let b = canonical_json_bytes(&m.to_json_value()).unwrap();
        assert_eq!(bytes, b, "metrics canonical JSON must be identical across runs");
    }

    // Golden snapshot: canonical JSON bytes of health metrics for regime_duplicates.
    // If the regime, search logic, or metrics computation changes, this snapshot
    // must be updated deliberately (not silently).
    let golden: &str = include_str!("goldens/sc1_m3_3_regime_duplicates_metrics.json");
    assert_eq!(
        std::str::from_utf8(&bytes).unwrap(),
        golden.trim(),
        "metrics golden snapshot mismatch — if the regime or metrics computation changed, \
         update the golden file deliberately"
    );
}

// ---------------------------------------------------------------------------
// SC1-M3.3-HISTOGRAMS-COVERAGE
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_histograms_coverage() {
    let result = run_duplicates_search();
    let m = result.graph.compute_health_metrics();

    // depth_histogram_pairs values must sum to node_summaries.len()
    let depth_sum: u64 = m.depth_histogram_pairs.iter().map(|p| p[1]).sum();
    assert_eq!(
        depth_sum,
        result.graph.node_summaries.len() as u64,
        "depth histogram sum must equal node count"
    );

    // candidate_count_histogram_pairs values must sum to expansions.len()
    let exp_sum: u64 = m.candidate_count_histogram_pairs.iter().map(|p| p[1]).sum();
    assert_eq!(
        exp_sum,
        result.graph.expansions.len() as u64,
        "candidate count histogram sum must equal expansion count"
    );

    // Histogram pairs must be sorted by key ascending.
    for w in m.depth_histogram_pairs.windows(2) {
        assert!(w[0][0] < w[1][0], "depth histogram must be sorted by depth ascending");
    }
    for w in m.candidate_count_histogram_pairs.windows(2) {
        assert!(
            w[0][0] < w[1][0],
            "candidate count histogram must be sorted by count ascending"
        );
    }
}

// ---------------------------------------------------------------------------
// SC1-M3.3-OUTCOME-EXHAUSTIVE
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_outcome_exhaustive() {
    let graph = make_outcome_exhaustive_fixture();
    let m = graph.compute_health_metrics();

    assert!(m.candidates_applied > 0, "Applied must be non-zero");
    assert!(
        m.candidates_duplicate_suppressed > 0,
        "DuplicateSuppressed must be non-zero"
    );
    assert!(m.candidates_skipped_depth > 0, "SkippedByDepthLimit must be non-zero");
    assert!(m.candidates_skipped_policy > 0, "SkippedByPolicy must be non-zero");
    assert!(m.candidates_apply_failed > 0, "ApplyFailed must be non-zero");
    assert!(m.candidates_illegal > 0, "IllegalOperator must be non-zero");
    assert!(m.candidates_not_evaluated > 0, "NotEvaluated must be non-zero");

    let sum = m.candidates_applied
        + m.candidates_duplicate_suppressed
        + m.candidates_skipped_depth
        + m.candidates_skipped_policy
        + m.candidates_apply_failed
        + m.candidates_illegal
        + m.candidates_not_evaluated;
    assert_eq!(sum, m.total_candidates, "outcome counts must sum to total_candidates");
}

// ---------------------------------------------------------------------------
// SC1-M3.3-NONBINDING
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_nonbinding() {
    let regime = regime_duplicates();
    let bundle = run_search(&regime.world, &regime.policy, &ScorerInputV1::Uniform).unwrap();

    // Verify the bundle passes cleanly first.
    verify_bundle(&bundle).unwrap();

    // Tamper with diagnostics.health_metrics in the verification report.
    // Rebuild with modified report to prove verify_bundle ignores diagnostics.
    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();

    // Replace health_metrics with garbage.
    report_json["diagnostics"]["health_metrics"] = serde_json::json!({"tampered": true});

    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    // Rebuild the bundle with the tampered report.
    let artifacts: Vec<sterling_harness::bundle::ArtifactInput> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "verification_report.json" {
                sterling_harness::bundle::ArtifactInput {
                    name: a.name.clone(),
                    content: modified_report_bytes.clone(),
                    normative: a.normative,
                    precomputed_hash: None,
                }
            } else {
                sterling_harness::bundle::ArtifactInput {
                    name: a.name.clone(),
                    content: a.content.clone(),
                    normative: a.normative,
                    precomputed_hash: Some(a.content_hash.clone()),
                }
            }
        })
        .collect();

    let tampered_bundle = sterling_harness::bundle::build_bundle(artifacts).unwrap();

    // This must pass: verify_bundle does not check diagnostics content.
    // The report content hash changed, but the bundle is self-consistent
    // (content_hash is recomputed from actual bytes).
    verify_bundle(&tampered_bundle).unwrap();
}

// ---------------------------------------------------------------------------
// Additional: cross-check metrics against metadata counters
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_cross_check_metadata_counters() {
    let result = run_duplicates_search();
    let m = result.graph.compute_health_metrics();
    let md = &result.graph.metadata;

    // These should agree (same data, different computation paths).
    assert_eq!(
        m.total_expansions, md.total_expansions,
        "metrics total_expansions must match metadata"
    );
    assert_eq!(
        m.total_candidates, md.total_candidates_generated,
        "metrics total_candidates must match metadata"
    );
    assert_eq!(
        m.candidates_duplicate_suppressed, md.total_duplicates_suppressed,
        "metrics duplicates must match metadata"
    );
    assert_eq!(
        m.dead_ends_exhaustive, md.total_dead_ends_exhaustive,
        "metrics exhaustive dead ends must match metadata"
    );
    assert_eq!(
        m.dead_ends_budget_limited, md.total_dead_ends_budget_limited,
        "metrics budget-limited dead ends must match metadata"
    );
}

// ---------------------------------------------------------------------------
// Additional: report contains diagnostics.health_metrics
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_report_contains_health_metrics() {
    let regime = regime_duplicates();
    let bundle = run_search(&regime.world, &regime.policy, &ScorerInputV1::Uniform).unwrap();

    let report = bundle.artifacts.get("verification_report.json").unwrap();
    let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();

    let health = &json["diagnostics"]["health_metrics"];
    assert!(health.is_object(), "health_metrics must be present as object");
    assert!(health["total_expansions"].is_u64());
    assert!(health["total_candidates"].is_u64());
    assert!(health["unique_nodes"].is_u64());
    assert!(health["candidates_applied"].is_u64());
    assert!(health["depth_histogram_pairs"].is_array());
    assert!(health["candidate_count_histogram_pairs"].is_array());
    assert!(health["max_depth"].is_u64());
}

// ---------------------------------------------------------------------------
// Additional: persistence round-trip preserves diagnostics
// ---------------------------------------------------------------------------

#[test]
fn sc1_m3_3_persistence_roundtrip() {
    let regime = regime_duplicates();
    let bundle = run_search(&regime.world, &regime.policy, &ScorerInputV1::Uniform).unwrap();

    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // Report bytes should be identical.
    let original_report = &bundle.artifacts["verification_report.json"].content;
    let loaded_report = &loaded.artifacts["verification_report.json"].content;
    assert_eq!(original_report, loaded_report, "report bytes must survive round-trip");

    // Verify loaded bundle passes.
    verify_bundle(&loaded).unwrap();
    verify_bundle_dir(dir.path()).unwrap();
}
