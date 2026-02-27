//! Shared helpers for sterling benchmark suites.

use std::collections::BTreeMap;

use sterling_harness::runner::{build_table_scorer_input, ScorerInputV1};
use sterling_harness::worlds::slot_lattice_regimes::Regime;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;
use sterling_search::contract::SearchWorldV1;
use sterling_search::policy::SearchPolicyV1;
use sterling_search::scorer::ValueScorer;
use sterling_search::search::{MetadataBindings, SearchResult};
use sterling_search::tape::TapeOutput;

use sterling_harness::bundle::DOMAIN_BUNDLE_ARTIFACT;
use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::policy::{build_policy, PolicyConfig};

/// Prepared inputs for calling `search()` directly, bypassing `run_search()` overhead.
pub struct SearchSetup {
    /// The compiled root state.
    pub root_state: sterling_kernel::carrier::bytestate::ByteStateV1,
    /// The world's operator registry.
    pub registry: sterling_kernel::carrier::registry::RegistryV1,
    /// Pre-computed metadata bindings for search graph construction.
    pub bindings: MetadataBindings,
}

/// Build the pre-search compilation and metadata once for a regime.
///
/// This isolates Phase 1+2 of `run_search()` so benchmarks can time only `search()`.
///
/// # Panics
///
/// Panics if any pipeline step fails (world encoding, compilation, policy build,
/// canonical serialization, or registry digest). Benchmark setup failures are fatal.
pub fn prepare_search_setup<W: SearchWorldV1 + WorldHarnessV1>(
    world: &W,
    search_policy: &SearchPolicyV1,
    scorer_input: &ScorerInputV1,
) -> SearchSetup {
    let payload_bytes = world.encode_payload().expect("encode_payload");
    let schema = world.schema_descriptor();
    let registry = world.registry().expect("registry");
    let compilation = compile(&payload_bytes, &schema, &registry).expect("compile");

    let policy_snapshot = build_policy(world, &PolicyConfig::default()).expect("build_policy");
    let policy_content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &policy_snapshot.bytes);

    let search_policy_json = search_policy_to_json(search_policy);
    let search_policy_bytes = canonical_json_bytes(&search_policy_json).expect("canon");
    let search_policy_digest = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &search_policy_bytes);

    let registry_digest = registry.digest().expect("registry digest");

    let scorer_digest_hex = match scorer_input {
        ScorerInputV1::Uniform => None,
        ScorerInputV1::Table { artifact, .. } => Some(artifact.hex_digest.clone()),
    };

    let bindings = MetadataBindings {
        world_id: WorldHarnessV1::world_id(world).to_string(),
        schema_descriptor: format!("{}:{}:{}", schema.id, schema.version, schema.hash),
        registry_digest: registry_digest.hex_digest().to_string(),
        policy_snapshot_digest: policy_content_hash.hex_digest().to_string(),
        search_policy_digest: search_policy_digest.hex_digest().to_string(),
        scorer_digest: scorer_digest_hex,
    };

    SearchSetup {
        root_state: compilation.state,
        registry,
        bindings,
    }
}

/// Run `search()` with prepared setup. Returns the full `SearchResult`.
///
/// # Panics
///
/// Panics if `search()` returns an error. Benchmark runs are expected to succeed.
pub fn run_search_only<W: SearchWorldV1>(
    setup: &SearchSetup,
    world: &W,
    policy: &SearchPolicyV1,
    scorer: &dyn ValueScorer,
) -> SearchResult {
    sterling_search::search::search(
        setup.root_state.clone(),
        world,
        &setup.registry,
        policy,
        scorer,
        &setup.bindings,
    )
    .expect("search should succeed in benchmarks")
}

/// Run `search_with_tape()` with prepared setup. Returns the full
/// `SearchResult` and `TapeOutput`.
///
/// # Panics
///
/// Panics if `search_with_tape()` returns an error.
pub fn run_search_with_tape_only<W: SearchWorldV1>(
    setup: &SearchSetup,
    world: &W,
    policy: &SearchPolicyV1,
    scorer: &dyn ValueScorer,
) -> (SearchResult, TapeOutput) {
    sterling_search::search::search_with_tape(
        setup.root_state.clone(),
        world,
        &setup.registry,
        policy,
        scorer,
        &setup.bindings,
    )
    .expect("search_with_tape should succeed in benchmarks")
}

/// Build a `ScorerInputV1::Table` from a regime's world.
///
/// Runs a quick uniform search to discover candidate canonical hashes,
/// then builds a table with descending bonus values. This produces a
/// realistically-sized table that exercises `BTreeMap` lookup behavior
/// in `TableScorer::score_candidates()`.
///
/// # Panics
///
/// Panics if the discovery search or table scorer construction fails.
#[must_use]
pub fn build_table_scorer_for_regime(regime: &Regime) -> ScorerInputV1 {
    let setup = prepare_search_setup(&regime.world, &regime.policy, &ScorerInputV1::Uniform);
    let result = run_search_only(
        &setup,
        &regime.world,
        &regime.policy,
        &sterling_search::scorer::UniformScorer,
    );

    // Collect unique candidate canonical hashes from expansion records.
    let mut table = BTreeMap::new();
    for expansion in &result.graph.expansions {
        for candidate in &expansion.candidates {
            let hash_str = candidate.action.canonical_hash().as_str().to_string();
            #[allow(clippy::cast_possible_wrap)]
            let len = table.len() as i64;
            table.entry(hash_str).or_insert(100 - len);
        }
    }

    if table.is_empty() {
        return ScorerInputV1::Uniform;
    }

    build_table_scorer_input(table).expect("build_table_scorer_input")
}

/// Convert search policy to JSON value (mirrors runner.rs internal function).
fn search_policy_to_json(policy: &SearchPolicyV1) -> serde_json::Value {
    let dedup_key = match policy.dedup_key {
        sterling_search::policy::DedupKeyV1::IdentityOnly => "identity_only",
        sterling_search::policy::DedupKeyV1::FullState => "full_state",
    };
    let prune_visited = match policy.prune_visited_policy {
        sterling_search::policy::PruneVisitedPolicyV1::KeepVisited => "keep_visited",
        sterling_search::policy::PruneVisitedPolicyV1::ReleaseVisited => "release_visited",
    };
    serde_json::json!({
        "max_expansions": policy.max_expansions,
        "max_depth": policy.max_depth,
        "max_candidates_per_node": policy.max_candidates_per_node,
        "max_frontier_size": policy.max_frontier_size,
        "dedup_key": dedup_key,
        "prune_visited_policy": prune_visited,
    })
}
