//! SC-001 T1 lock tests: tape equivalence proofs.
//!
//! These tests prove that `search_with_tape()` produces a binary tape whose
//! rendered `SearchGraphV1` is byte-identical to the graph built by the
//! existing `build_graph()` path within the same search execution.
//!
//! The tape is also tested for:
//! - Tamper detection (mutated bytes → reader rejects with typed error)
//! - Determinism (N=10 runs produce identical tape bytes)
//! - Chain verification (reader recomputes chain, matches footer hash)
//! - Tape round-trip via `read_tape` parse verification

use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_harness::worlds::slot_lattice_regimes::{
    regime_budget_limited, regime_duplicates, regime_exhaustive_dead_end, regime_frontier_pressure,
    regime_scale_1000, regime_truncation, Regime,
};
use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::operators::operator_registry::kernel_operator_registry;
use sterling_search::contract::SearchWorldV1;
use sterling_search::scorer::UniformScorer;
use sterling_search::search::{search_with_tape, MetadataBindings};
use sterling_search::tape::TapeOutput;
use sterling_search::tape_reader::read_tape;
use sterling_search::tape_render::render_graph;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build `MetadataBindings` from a regime's world + policy.
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
        operator_set_digest: None,
    }
}

/// Run `search_with_tape()` for a regime, returning (`SearchResult`, `TapeOutput`).
fn run_with_tape(regime: &Regime) -> (sterling_search::search::SearchResult, TapeOutput) {
    let operator_registry = kernel_operator_registry();
    let root = ByteStateV1::new(1, 10); // MAX_SLOTS = 10
    let bindings = bindings_for(regime);
    search_with_tape(
        root,
        &regime.world,
        &operator_registry,
        &regime.policy,
        &UniformScorer,
        &bindings,
    )
    .expect("search_with_tape should succeed")
}

/// Assert byte-identical equivalence between the direct graph and the
/// tape-rendered graph for a given regime.
fn assert_tape_equivalence(regime: &Regime, label: &str) {
    let (result, tape_output) = run_with_tape(regime);

    // Parse the binary tape
    let tape = read_tape(&tape_output.bytes)
        .unwrap_or_else(|e| panic!("[{label}] tape parse failed: {e:?}"));

    // Render tape → SearchGraphV1
    let graph_from_tape =
        render_graph(&tape).unwrap_or_else(|e| panic!("[{label}] tape render failed: {e:?}"));

    // EQUIVALENCE: both graphs must produce identical canonical JSON
    let bytes_direct = result
        .graph
        .to_canonical_json_bytes()
        .unwrap_or_else(|e| panic!("[{label}] direct graph JSON failed: {e:?}"));
    let bytes_from_tape = graph_from_tape
        .to_canonical_json_bytes()
        .unwrap_or_else(|e| panic!("[{label}] tape-rendered graph JSON failed: {e:?}"));

    assert_eq!(
        bytes_direct, bytes_from_tape,
        "[{label}] SearchGraphV1 from direct search must be byte-identical to tape-rendered graph"
    );
}

// ---------------------------------------------------------------------------
// T1.1: Byte-identical equivalence — all regimes
// ---------------------------------------------------------------------------

#[test]
fn tape_equiv_rome_mini_search() {
    let world = RomeMiniSearch;
    let policy = sterling_search::policy::SearchPolicyV1::default();
    let operator_registry = kernel_operator_registry();
    let sd = world.schema_descriptor();
    let bindings = MetadataBindings {
        world_id: SearchWorldV1::world_id(&world).to_string(),
        schema_descriptor: format!("{}:{}:{}", sd.id, sd.version, sd.hash),
        registry_digest: "test_registry_digest".into(),
        policy_snapshot_digest: "test_policy_digest".into(),
        search_policy_digest: "test_search_policy_digest".into(),
        scorer_digest: None,
        operator_set_digest: None,
    };
    let root = ByteStateV1::new(1, 2);

    let (result, tape_output) =
        search_with_tape(root, &world, &operator_registry, &policy, &UniformScorer, &bindings)
            .expect("search_with_tape should succeed for RomeMiniSearch");

    let tape = read_tape(&tape_output.bytes).expect("tape parse failed");
    let graph_from_tape = render_graph(&tape).expect("tape render failed");

    let bytes_direct = result.graph.to_canonical_json_bytes().unwrap();
    let bytes_from_tape = graph_from_tape.to_canonical_json_bytes().unwrap();

    assert_eq!(
        bytes_direct, bytes_from_tape,
        "RomeMiniSearch: graph must be byte-identical via tape"
    );
}

#[test]
fn tape_equiv_truncation() {
    assert_tape_equivalence(&regime_truncation(), "truncation");
}

#[test]
fn tape_equiv_duplicates() {
    assert_tape_equivalence(&regime_duplicates(), "duplicates");
}

#[test]
fn tape_equiv_exhaustive_dead_end() {
    assert_tape_equivalence(&regime_exhaustive_dead_end(), "exhaustive_dead_end");
}

#[test]
fn tape_equiv_budget_limited() {
    assert_tape_equivalence(&regime_budget_limited(), "budget_limited");
}

#[test]
fn tape_equiv_frontier_pressure() {
    assert_tape_equivalence(&regime_frontier_pressure(), "frontier_pressure");
}

#[test]
fn tape_equiv_scale_1000() {
    assert_tape_equivalence(&regime_scale_1000(), "scale_1000");
}

// ---------------------------------------------------------------------------
// T1.2: Tape round-trip (parse succeeds for all regimes)
// ---------------------------------------------------------------------------
//
// Detailed write → read → re-write byte-identity is tested in
// `tape_reader::tests::write_read_rewrite_identical` (unit test).
// The lock-level round-trip verifies that search-generated tapes parse
// cleanly and that two identical runs produce identical tape bytes.

#[test]
fn tape_roundtrip_parse_all_regimes() {
    let regimes: Vec<(&str, Regime)> = vec![
        ("truncation", regime_truncation()),
        ("duplicates", regime_duplicates()),
        ("exhaustive_dead_end", regime_exhaustive_dead_end()),
        ("budget_limited", regime_budget_limited()),
        ("frontier_pressure", regime_frontier_pressure()),
        ("scale_1000", regime_scale_1000()),
    ];

    for (label, regime) in &regimes {
        let (_, tape_output) = run_with_tape(regime);
        let tape = read_tape(&tape_output.bytes)
            .unwrap_or_else(|e| panic!("[{label}] tape round-trip parse failed: {e:?}"));

        // Structural sanity: at minimum root NodeCreation + Termination
        assert!(
            tape.records.len() >= 2,
            "[{label}] tape must have at least root node + termination"
        );

        // Footer record count matches
        assert_eq!(
            tape.footer.record_count,
            tape.records.len() as u64,
            "[{label}] footer record count must match parsed records"
        );
    }
}

// ---------------------------------------------------------------------------
// T1.3: Tamper detection
// ---------------------------------------------------------------------------

#[test]
fn tape_tamper_header_byte_rejected() {
    let regime = regime_truncation();
    let (_, tape_output) = run_with_tape(&regime);
    let mut tampered = tape_output.bytes.clone();

    // Mutate a byte in the header region (after magic + version + header_len = 10 bytes)
    if tampered.len() > 15 {
        tampered[15] ^= 0xFF;
    }

    let err = read_tape(&tampered);
    assert!(
        err.is_err(),
        "tampered header byte should cause parse failure"
    );
}

#[test]
fn tape_tamper_record_byte_rejected() {
    let regime = regime_truncation();
    let (_, tape_output) = run_with_tape(&regime);
    let mut tampered = tape_output.bytes.clone();

    // Find the first record region (after header)
    // magic(4) + version(2) + header_len(4) + header_bytes
    let header_len =
        u32::from_le_bytes([tampered[6], tampered[7], tampered[8], tampered[9]]) as usize;
    let records_start = 10 + header_len;

    if tampered.len() > records_start + 10 {
        tampered[records_start + 10] ^= 0xFF;
    }

    let err = read_tape(&tampered);
    assert!(
        err.is_err(),
        "tampered record byte should cause parse failure"
    );
}

#[test]
fn tape_tamper_footer_byte_rejected() {
    let regime = regime_truncation();
    let (_, tape_output) = run_with_tape(&regime);
    let mut tampered = tape_output.bytes.clone();

    // Mutate a byte in the footer region (last 48 bytes)
    let footer_start = tampered.len() - sterling_search::tape::FOOTER_SIZE;
    // Mutate the record count field
    tampered[footer_start] ^= 0xFF;

    let err = read_tape(&tampered);
    assert!(
        err.is_err(),
        "tampered footer byte should cause parse failure"
    );
}

// ---------------------------------------------------------------------------
// T1.4: Determinism (N=10 runs produce identical tape bytes)
// ---------------------------------------------------------------------------

#[test]
fn tape_determinism_n10() {
    let regime = regime_truncation();
    let (_, baseline) = run_with_tape(&regime);

    for i in 1..10 {
        let (_, tape_i) = run_with_tape(&regime);
        assert_eq!(
            baseline.bytes, tape_i.bytes,
            "tape bytes must be deterministic across runs (diverged at run {i})"
        );
    }
}

#[test]
fn tape_determinism_scale_1000_n10() {
    let regime = regime_scale_1000();
    let (_, baseline) = run_with_tape(&regime);

    for i in 1..10 {
        let (_, tape_i) = run_with_tape(&regime);
        assert_eq!(
            baseline.bytes, tape_i.bytes,
            "scale_1000 tape bytes must be deterministic (diverged at run {i})"
        );
    }
}

// ---------------------------------------------------------------------------
// T1.5: Chain verification (reader recomputes chain, matches footer)
// ---------------------------------------------------------------------------

#[test]
fn tape_chain_hash_verified_by_reader() {
    // The reader already verifies the chain hash — if it parses successfully,
    // the chain is correct. Run all regimes to ensure chain is valid.
    let regimes: Vec<(&str, Regime)> = vec![
        ("truncation", regime_truncation()),
        ("duplicates", regime_duplicates()),
        ("exhaustive_dead_end", regime_exhaustive_dead_end()),
        ("budget_limited", regime_budget_limited()),
        ("frontier_pressure", regime_frontier_pressure()),
        ("scale_1000", regime_scale_1000()),
    ];

    for (label, regime) in &regimes {
        let (_, tape_output) = run_with_tape(regime);
        read_tape(&tape_output.bytes)
            .unwrap_or_else(|e| panic!("[{label}] chain verification failed on parse: {e:?}"));
    }
}

// ---------------------------------------------------------------------------
// T1.6: Tape output metadata sanity
// ---------------------------------------------------------------------------

#[test]
fn tape_header_contains_expected_fields() {
    let regime = regime_truncation();
    let (_, tape_output) = run_with_tape(&regime);
    let tape = read_tape(&tape_output.bytes).expect("parse failed");

    let header = &tape.header.json;
    // Required fields per wire spec
    assert!(
        header.get("world_id").is_some(),
        "header must have world_id"
    );
    assert!(
        header.get("schema_descriptor").is_some(),
        "header must have schema_descriptor"
    );
    assert!(
        header.get("dedup_key").is_some(),
        "header must have dedup_key"
    );
    assert!(
        header.get("prune_visited_policy").is_some(),
        "header must have prune_visited_policy"
    );
    assert!(
        header.get("root_state_fingerprint").is_some(),
        "header must have root_state_fingerprint"
    );
}

#[test]
fn tape_graph_equivalence_preserves_metadata() {
    // Verify that metadata fields (counters, termination, bindings) survive
    // the tape→graph round-trip for a non-trivial regime.
    let regime = regime_scale_1000();
    let (result, tape_output) = run_with_tape(&regime);

    let tape = read_tape(&tape_output.bytes).expect("parse failed");
    let graph_from_tape = render_graph(&tape).expect("render failed");

    let m1 = &result.graph.metadata;
    let m2 = &graph_from_tape.metadata;

    assert_eq!(m1.world_id, m2.world_id);
    assert_eq!(m1.total_expansions, m2.total_expansions);
    assert_eq!(m1.total_candidates_generated, m2.total_candidates_generated);
    assert_eq!(
        m1.total_duplicates_suppressed,
        m2.total_duplicates_suppressed
    );
    assert_eq!(m1.termination_reason, m2.termination_reason);
    assert_eq!(m1.frontier_high_water, m2.frontier_high_water);
    assert_eq!(m1.dedup_key, m2.dedup_key);
    assert_eq!(m1.prune_visited_policy, m2.prune_visited_policy);
}
