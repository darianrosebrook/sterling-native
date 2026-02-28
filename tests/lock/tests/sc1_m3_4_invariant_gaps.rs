//! SC-001 M3.4 — Explicit invariant gap tests.
//!
//! These tests close the two invariant coverage gaps identified in the
//! post-M3.3 audit. Both invariants were previously covered only implicitly
//! (they held because the code works, but no test would fail if the invariant
//! were violated). These tests make the invariants fail-closed.
//!
//! - INV-SC-05: `state_fingerprint` is computed from the policy-declared dedup
//!   key (`DedupKeyV1::IdentityOnly` → `identity_bytes` only).
//! - INV-SC-08: `enumerate_candidates` uses the runner-supplied `RegistryV1`
//!   snapshot only; no split-brain registry reads.

use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_harness::worlds::slot_lattice_regimes::regime_duplicates;
use sterling_kernel::carrier::bytestate::{ByteStateV1, SlotStatus};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::operators::operator_registry::{kernel_operator_registry, OperatorRegistryV1};
use sterling_kernel::proof::hash::canonical_hash;
use sterling_search::contract::SearchWorldV1;
use sterling_search::node::DOMAIN_SEARCH_NODE;
use sterling_search::policy::SearchPolicyV1;
use sterling_search::scorer::UniformScorer;
use sterling_search::search::{search, MetadataBindings};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_bindings() -> MetadataBindings {
    MetadataBindings {
        world_id: "rome_mini_search".into(),
        schema_descriptor: "rome:1.0:test".into(),
        registry_digest: "test_registry_digest".into(),
        policy_snapshot_digest: "test_policy_digest".into(),
        search_policy_digest: "test_search_policy_digest".into(),
        scorer_digest: None,
    }
}

fn default_operator_registry() -> OperatorRegistryV1 {
    kernel_operator_registry()
}

fn root_state() -> ByteStateV1 {
    ByteStateV1::new(1, 2)
}

// ---------------------------------------------------------------------------
// INV-SC-05: Fingerprint derived from identity-only dedup key
// ---------------------------------------------------------------------------

/// Two states with identical identity planes but different status planes
/// must produce the same `state_fingerprint` under `DedupKeyV1::IdentityOnly`.
///
/// This is the fundamental invariant that prevents governance status changes
/// from inflating the search frontier.
///
/// ACCEPTANCE: SC1-M3.4-INV-SC-05-IDENTITY-ONLY
#[test]
fn inv_sc_05_identity_only_fingerprint_ignores_status() {
    let state_a = root_state();
    let mut state_b = root_state();

    // Verify they start with the same identity
    assert_eq!(
        state_a.identity_bytes(),
        state_b.identity_bytes(),
        "precondition: identity planes must match"
    );

    // Mutate status plane of state_b
    state_b.set_status(0, 0, SlotStatus::Certified);
    state_b.set_status(0, 1, SlotStatus::Promoted);

    // Status planes are now different
    assert_ne!(
        state_a.status_bytes(),
        state_b.status_bytes(),
        "precondition: status planes must differ"
    );

    // But identity planes are still the same
    assert_eq!(
        state_a.identity_bytes(),
        state_b.identity_bytes(),
        "precondition: identity planes must still match"
    );

    // The fingerprint that the search engine computes under IdentityOnly
    // uses canonical_hash(DOMAIN_SEARCH_NODE, &state.identity_bytes()).
    let fp_a = canonical_hash(DOMAIN_SEARCH_NODE, &state_a.identity_bytes());
    let fp_b = canonical_hash(DOMAIN_SEARCH_NODE, &state_b.identity_bytes());

    assert_eq!(
        fp_a, fp_b,
        "INV-SC-05 violated: same identity must produce same fingerprint"
    );
}

/// Two states with different identity planes must produce different
/// fingerprints, even if their status planes are identical.
///
/// This is the contrapositive of INV-SC-05: the dedup key must actually
/// discriminate states that differ in identity.
///
/// ACCEPTANCE: SC1-M3.4-INV-SC-05-DIFFERENT-IDENTITY
#[test]
fn inv_sc_05_different_identity_produces_different_fingerprint() {
    let state_a = root_state();
    let mut state_b = root_state();

    // Mutate identity plane of state_b
    state_b.set_identity(0, 0, Code32::new(7, 7, 7));

    assert_ne!(
        state_a.identity_bytes(),
        state_b.identity_bytes(),
        "precondition: identity planes must differ"
    );

    let fp_a = canonical_hash(DOMAIN_SEARCH_NODE, &state_a.identity_bytes());
    let fp_b = canonical_hash(DOMAIN_SEARCH_NODE, &state_b.identity_bytes());

    assert_ne!(
        fp_a, fp_b,
        "INV-SC-05 contrapositive violated: different identity must produce different fingerprint"
    );
}

/// In a live search, states that differ only in status must be dedup-suppressed
/// as duplicates (they share the same identity-only fingerprint).
///
/// We use `regime_duplicates` (`SlotLatticeSearch` configured to produce many
/// duplicate states) and verify that every node's fingerprint matches the
/// identity-only computation, and that dedup-suppressed candidates reference
/// known node fingerprints.
///
/// ACCEPTANCE: SC1-M3.4-INV-SC-05-SEARCH-DEDUP
#[test]
fn inv_sc_05_search_dedup_uses_identity_fingerprint() {
    let regime = regime_duplicates();
    let scorer = UniformScorer;
    let operator_registry = kernel_operator_registry();
    let sd = regime.world.schema_descriptor();
    let bindings = MetadataBindings {
        world_id: SearchWorldV1::world_id(&regime.world).to_string(),
        schema_descriptor: format!("{}:{}:{}", sd.id, sd.version, sd.hash),
        registry_digest: "test_registry_digest".into(),
        policy_snapshot_digest: "test_policy_digest".into(),
        search_policy_digest: "test_search_policy_digest".into(),
        scorer_digest: None,
    };

    let result = search(
        ByteStateV1::new(1, 10),
        &regime.world,
        &operator_registry,
        &regime.policy,
        &scorer,
        &bindings,
    )
    .unwrap();

    // Collect all node fingerprints
    let node_fps: std::collections::BTreeSet<String> = result
        .graph
        .node_summaries
        .iter()
        .map(|ns| ns.state_fingerprint.as_str().to_string())
        .collect();

    // Verify that every node fingerprint matches what we'd compute from
    // the node's identity_bytes alone (not full evidence bytes)
    for node in &result.nodes {
        let expected_fp = canonical_hash(DOMAIN_SEARCH_NODE, &node.state.identity_bytes());
        assert_eq!(
            node.state_fingerprint, expected_fp,
            "INV-SC-05: node {} fingerprint doesn't match identity-only computation",
            node.node_id,
        );
    }

    // Verify dedup-suppressed candidates reference known fingerprints
    for expansion in &result.graph.expansions {
        for cr in &expansion.candidates {
            if let sterling_search::graph::CandidateOutcomeV1::DuplicateSuppressed {
                existing_fingerprint,
            } = &cr.outcome
            {
                assert!(
                    node_fps.contains(existing_fingerprint),
                    "INV-SC-05: dedup-suppressed candidate references unknown fingerprint {existing_fingerprint}",
                );
            }
        }
    }

    // Sanity: regime_duplicates is specifically designed to produce duplicates
    let total_suppressed: u64 = result
        .graph
        .expansions
        .iter()
        .flat_map(|e| &e.candidates)
        .filter(|cr| {
            matches!(
                cr.outcome,
                sterling_search::graph::CandidateOutcomeV1::DuplicateSuppressed { .. }
            )
        })
        .count() as u64;

    assert!(
        total_suppressed > 0,
        "precondition: regime_duplicates must produce dedup-suppressed candidates"
    );
}

// ---------------------------------------------------------------------------
// INV-SC-08: Candidate enumeration uses runner-supplied registry only
// ---------------------------------------------------------------------------

/// Supplying an operator registry that contains `OP_SET_SLOT` produces
/// candidates; an empty operator registry produces zero. The world's
/// internal concept registry is irrelevant — only the runner-supplied
/// operator registry governs which operators are legal (INV-SC-08).
///
/// ACCEPTANCE: SC1-M3.4-INV-SC-08-RESTRICTED-REGISTRY
#[test]
fn inv_sc_08_restricted_registry_reduces_candidates() {
    let full_operator_registry = default_operator_registry();
    let state = root_state();

    // Full operator registry: get all candidates
    let full_candidates = RomeMiniSearch.enumerate_candidates(&state, &full_operator_registry);
    assert!(
        full_candidates.len() > 1,
        "precondition: full operator registry must produce multiple candidates"
    );

    // Empty operator registry (no OP_SET_SLOT) → zero candidates
    let empty = OperatorRegistryV1::new("empty.v1".into(), vec![]).unwrap();
    let empty_candidates = RomeMiniSearch.enumerate_candidates(&state, &empty);

    assert!(
        empty_candidates.is_empty(),
        "INV-SC-08: empty operator registry must produce zero candidates, got {}",
        empty_candidates.len(),
    );
}

/// An empty operator registry must produce zero candidates from
/// `enumerate_candidates`, regardless of the world's internal knowledge.
///
/// ACCEPTANCE: SC1-M3.4-INV-SC-08-EMPTY-REGISTRY
#[test]
fn inv_sc_08_empty_registry_produces_zero_candidates() {
    let empty_registry = OperatorRegistryV1::new("empty.v1".into(), vec![]).unwrap();
    let state = root_state();

    let candidates = RomeMiniSearch.enumerate_candidates(&state, &empty_registry);

    assert!(
        candidates.is_empty(),
        "INV-SC-08: empty operator registry must produce zero candidates, got {}",
        candidates.len(),
    );
}

/// In a live search with the kernel operator registry, every candidate in
/// the graph must use `OP_SET_SLOT` (the only operator in the registry).
/// Operator legality is now checked via `OperatorRegistryV1`, not concept
/// `RegistryV1`.
///
/// ACCEPTANCE: SC1-M3.4-INV-SC-08-SEARCH-COHERENCE
#[test]
fn inv_sc_08_search_with_operator_registry() {
    let op_set_slot = sterling_kernel::operators::apply::OP_SET_SLOT;
    let operator_registry = default_operator_registry();

    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
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

    // Every candidate in the graph must use OP_SET_SLOT (the only op in the registry)
    for expansion in &result.graph.expansions {
        for cr in &expansion.candidates {
            assert_eq!(
                cr.action.op_code, op_set_slot,
                "INV-SC-08: candidate uses op_code not in runner-supplied operator registry",
            );
        }
    }

    // All expansions should have 4 values * 2 slots = 8 candidates (unless already set)
    let first_exp = &result.graph.expansions[0];
    assert_eq!(
        first_exp.candidates.len(),
        8,
        "INV-SC-08: first expansion should have 8 candidates (4 values × 2 slots)"
    );
}
