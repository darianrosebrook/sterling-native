//! SEARCH-CORE-001 M3.1 lock tests: slot lattice world stress regimes.
//!
//! Each test targets a specific acceptance criterion from the M3.1 milestone.
//! Regime constructors return matched `(world, policy, expectations)` triples
//! so that tests assert failure modes were *exercised*, not just that search
//! returned `Ok`.

use sterling_harness::bundle::{verify_bundle, ArtifactBundleV1};
use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};
use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::slot_lattice_regimes::{
    regime_budget_limited, regime_duplicates, regime_exhaustive_dead_end, regime_frontier_pressure,
    regime_truncation, Regime,
};
use sterling_harness::worlds::slot_lattice_search::{GoalProfile, SlotLatticeSearch};
use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_search::contract::SearchWorldV1;
use sterling_search::graph::{CandidateOutcomeV1, TerminationReasonV1};
use sterling_search::policy::DedupKeyV1;
use sterling_search::scorer::UniformScorer;
use sterling_search::search::{search, MetadataBindings};

/// Build `MetadataBindings` from a regime's world + policy, matching what `run_search` does.
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

/// Run search directly for a regime (low-level — no bundle).
fn run_regime_search(regime: &Regime) -> sterling_search::search::SearchResult {
    let registry = regime.world.registry().unwrap();
    let root = ByteStateV1::new(1, 10); // MAX_SLOTS = 10
    let bindings = bindings_for(regime);
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

/// Run search through the full bundle pipeline for a regime.
fn run_regime_bundle(regime: &Regime) -> ArtifactBundleV1 {
    run_search(&regime.world, &regime.policy, &ScorerInputV1::Uniform).unwrap()
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-TRUNCATION-REACHABLE
// ---------------------------------------------------------------------------

#[test]
fn truncation_is_reachable() {
    let regime = regime_truncation();
    let result = run_regime_search(&regime);

    assert!(regime.expectations.expects_truncation);

    // First expansion must have candidates_truncated == true.
    let first = &result.graph.expansions[0];
    assert!(
        first.candidates_truncated,
        "first expansion should be truncated (root has 32 candidates, cap is 5)"
    );
    assert_eq!(
        first.candidates.len(),
        usize::try_from(regime.policy.max_candidates_per_node).unwrap(),
        "truncated candidate count should equal cap"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-DUPLICATES-REACHABLE
// ---------------------------------------------------------------------------

#[test]
fn duplicate_suppression_is_reachable() {
    let regime = regime_duplicates();

    // Guard: dedup must use IdentityOnly so order-independent slot assignment
    // collapses to the same fingerprint. If this changes, the test's semantic
    // premise (commutativity → duplicates) no longer holds.
    assert_eq!(
        regime.policy.dedup_key,
        DedupKeyV1::IdentityOnly,
        "duplicates regime must use IdentityOnly dedup"
    );
    assert_eq!(
        regime.world.config().goal_profile,
        GoalProfile::Never,
        "duplicates regime must use goal=Never to force exhaustive exploration"
    );

    let result = run_regime_search(&regime);

    assert!(
        result.graph.metadata.total_duplicates_suppressed
            >= regime.expectations.min_duplicates_suppressed,
        "expected at least {} duplicates suppressed, got {}",
        regime.expectations.min_duplicates_suppressed,
        result.graph.metadata.total_duplicates_suppressed,
    );

    // Verify that at least one DuplicateSuppressed outcome exists.
    let has_dup = result.graph.expansions.iter().any(|exp| {
        exp.candidates
            .iter()
            .any(|c| matches!(c.outcome, CandidateOutcomeV1::DuplicateSuppressed { .. }))
    });
    assert!(
        has_dup,
        "should see at least one DuplicateSuppressed outcome"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-EXHAUSTIVE-DEAD-END-REACHABLE
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_dead_end_is_reachable() {
    let regime = regime_exhaustive_dead_end();
    let result = run_regime_search(&regime);

    assert!(regime.expectations.expects_exhaustive_dead_end);

    assert!(
        result.graph.metadata.total_dead_ends_exhaustive > 0,
        "expected at least one exhaustive dead end, got 0"
    );

    // Verify the dead-end reason appears in at least one expansion.
    let has_exhaustive = result.graph.expansions.iter().any(|exp| {
        exp.dead_end_reason == Some(sterling_search::graph::DeadEndReasonV1::Exhaustive)
    });
    assert!(
        has_exhaustive,
        "should see at least one Exhaustive dead-end reason in expansions"
    );

    // Non-trap branches should still reach the goal.
    if regime.expectations.expects_goal_reached {
        assert!(
            result.is_goal_reached(),
            "non-trap branches should reach the goal"
        );
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-BUDGET-TERMINATION
// ---------------------------------------------------------------------------

#[test]
fn budget_termination_is_reachable() {
    let regime = regime_budget_limited();
    let result = run_regime_search(&regime);

    assert_eq!(
        result.graph.metadata.termination_reason,
        TerminationReasonV1::ExpansionBudgetExceeded,
        "expected ExpansionBudgetExceeded termination"
    );

    assert_eq!(
        result.graph.metadata.total_expansions, regime.policy.max_expansions,
        "total expansions should equal max_expansions budget"
    );

    assert!(
        !regime.expectations.expects_goal_reached,
        "budget regime should not expect goal"
    );
    assert!(!result.is_goal_reached());
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-FRONTIER-PRESSURE
// ---------------------------------------------------------------------------

#[test]
fn frontier_pressure_is_reachable() {
    let regime = regime_frontier_pressure();
    let result = run_regime_search(&regime);

    assert!(
        result.graph.metadata.frontier_high_water >= regime.expectations.min_frontier_high_water,
        "expected frontier_high_water >= {}, got {}",
        regime.expectations.min_frontier_high_water,
        result.graph.metadata.frontier_high_water,
    );

    // Stable pressure invariant: frontier_high_water must reach the cap.
    assert!(
        result.graph.metadata.frontier_high_water >= regime.policy.max_frontier_size,
        "frontier_high_water ({}) must reach max_frontier_size ({})",
        result.graph.metadata.frontier_high_water,
        regime.policy.max_frontier_size,
    );

    // Typed prune notes: assert if the graph surface exposes them.
    // This is a diagnostic signal that may be refactored; the stable metric
    // above (high_water >= cap) is the primary pressure proof.
    let has_any_notes = result
        .graph
        .expansions
        .iter()
        .any(|exp| !exp.notes.is_empty());
    if has_any_notes {
        let has_prune_note = result.graph.expansions.iter().any(|exp| {
            exp.notes.iter().any(|n| {
                matches!(
                    n,
                    sterling_search::graph::ExpansionNoteV1::FrontierPruned { .. }
                )
            })
        });
        assert!(
            has_prune_note,
            "expansion notes exist but none are FrontierPruned — expected pruning with cap=8"
        );
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-DETERMINISM-N10
// ---------------------------------------------------------------------------

#[test]
fn determinism_n10() {
    let regime = regime_duplicates(); // Use duplicates regime for broad exploration.
    let registry = regime.world.registry().unwrap();
    let bindings = bindings_for(&regime);

    let first = search(
        ByteStateV1::new(1, 10),
        &regime.world,
        &registry,
        &regime.policy,
        &UniformScorer,
        &bindings,
    )
    .unwrap();
    let first_bytes = first.graph.to_canonical_json_bytes().unwrap();

    for run in 1..10 {
        let other = search(
            ByteStateV1::new(1, 10),
            &regime.world,
            &registry,
            &regime.policy,
            &UniformScorer,
            &bindings,
        )
        .unwrap();
        let other_bytes = other.graph.to_canonical_json_bytes().unwrap();
        assert_eq!(
            first_bytes, other_bytes,
            "SearchGraphV1 bytes differ on run {run}"
        );
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-BUNDLE-VERIFY
// ---------------------------------------------------------------------------

#[test]
fn bundle_verify_passes() {
    let regime = regime_exhaustive_dead_end();
    let bundle = run_regime_bundle(&regime);
    verify_bundle(&bundle).expect("verify_bundle should pass for slot lattice bundle");
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-BUNDLE-PERSISTENCE
// ---------------------------------------------------------------------------

#[test]
fn bundle_persistence_roundtrip() {
    let regime = regime_budget_limited();
    let bundle = run_regime_bundle(&regime);

    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();
    let reloaded = read_bundle_dir(dir.path()).unwrap();

    // Verify the reloaded bundle passes integrity checks.
    verify_bundle(&reloaded).expect("reloaded bundle should verify");
    verify_bundle_dir(dir.path()).expect("verify_bundle_dir should pass");

    // Normative artifact content must be identical.
    for (name, original) in &bundle.artifacts {
        let round_tripped = reloaded
            .artifacts
            .get(name)
            .unwrap_or_else(|| panic!("missing artifact {name} after round-trip"));
        assert_eq!(
            original.content, round_tripped.content,
            "artifact {name} content differs after round-trip"
        );
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-WORLD-ID-BINDS-CONFIG
// ---------------------------------------------------------------------------

#[test]
fn world_id_binds_config() {
    let regime = regime_truncation();
    let world_id = <SlotLatticeSearch as SearchWorldV1>::world_id(&regime.world);

    // Exact equality against the expected world_id for this config.
    assert_eq!(
        world_id, "slot_lattice:v1:n8:v4:trap_none:goal_all_nonzero",
        "world_id must encode config params"
    );

    // Graph metadata world_id matches.
    let result = run_regime_search(&regime);
    assert_eq!(
        result.graph.metadata.world_id, world_id,
        "graph metadata world_id must match world's world_id()"
    );
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-SCHEMA-DESCRIPTOR-STABLE
// ---------------------------------------------------------------------------

#[test]
fn schema_descriptor_is_real_and_stable() {
    // Schema descriptor must be the same across all regimes.
    let regimes: Vec<Regime> = vec![
        regime_truncation(),
        regime_duplicates(),
        regime_exhaustive_dead_end(),
        regime_budget_limited(),
        regime_frontier_pressure(),
    ];

    let first_sd = regimes[0].world.schema_descriptor();

    // Must not be a placeholder.
    assert_ne!(
        first_sd.hash, "placeholder",
        "schema hash must not be placeholder"
    );
    assert!(!first_sd.hash.is_empty(), "schema hash must not be empty");
    assert!(
        first_sd.hash.starts_with("sha256:"),
        "schema hash must be a sha256 content hash, got: {}",
        first_sd.hash,
    );

    // Must be stable across all regimes.
    for (i, regime) in regimes.iter().enumerate().skip(1) {
        let sd = regime.world.schema_descriptor();
        assert_eq!(first_sd.id, sd.id, "schema id differs for regime {i}");
        assert_eq!(
            first_sd.version, sd.version,
            "schema version differs for regime {i}"
        );
        assert_eq!(first_sd.hash, sd.hash, "schema hash differs for regime {i}");
    }
}

// ---------------------------------------------------------------------------
// ACCEPTANCE: SC1-M3.1-ENUMERATION-DETERMINISTIC
// ---------------------------------------------------------------------------

#[test]
fn enumeration_is_deterministic() {
    let regime = regime_exhaustive_dead_end();
    let registry = regime.world.registry().unwrap();
    let state = ByteStateV1::new(1, 10);

    let c1 = regime.world.enumerate_candidates(&state, &registry);
    let c2 = regime.world.enumerate_candidates(&state, &registry);

    assert_eq!(c1, c2, "enumerate_candidates must be deterministic");
    assert!(!c1.is_empty(), "initial state should have candidates");
}
