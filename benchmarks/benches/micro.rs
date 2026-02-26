use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::operators::apply::{apply, set_slot_args, OP_SET_SLOT};
use sterling_kernel::proof::hash::{canonical_hash, ContentHash};
use sterling_search::frontier::BestFirstFrontier;
use sterling_search::node::{CandidateActionV1, SearchNodeV1, DOMAIN_SEARCH_NODE};
use sterling_search::scorer::{UniformScorer, ValueScorer};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_node(id: u64, layer_count: usize, slot_count: usize) -> SearchNodeV1 {
    let state = ByteStateV1::new(layer_count, slot_count);
    let fp = canonical_hash(DOMAIN_SEARCH_NODE, &id.to_le_bytes());
    SearchNodeV1 {
        node_id: id,
        parent_id: if id == 0 { None } else { Some(0) },
        state,
        state_fingerprint: fp,
        depth: 0,
        g_cost: 0,
        h_cost: 0,
        creation_order: id,
        producing_action: None,
    }
}

fn make_candidates(n: usize) -> Vec<CandidateActionV1> {
    (0..n)
        .map(|i| {
            let code = Code32::new(
                u8::try_from(i % 256).unwrap_or(0),
                u8::try_from((i / 256) % 256).unwrap_or(0),
                1,
            );
            let args = set_slot_args(0, 0, code);
            CandidateActionV1::new(code, args)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Frontier push/pop
// ---------------------------------------------------------------------------

fn bench_frontier(c: &mut Criterion) {
    let mut group = c.benchmark_group("frontier_push_pop");
    for &size in &[10u64, 100, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            b.iter_batched(
                || {
                    // Setup: build N nodes with unique fingerprints.
                    (0..n).map(|i| make_node(i, 1, 2)).collect::<Vec<_>>()
                },
                |nodes| {
                    let mut frontier = BestFirstFrontier::new();
                    for node in nodes {
                        black_box(frontier.push(node));
                    }
                    while let Some(node) = frontier.pop() {
                        black_box(node);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scorer: Uniform
// ---------------------------------------------------------------------------

fn bench_scorer_uniform(c: &mut Criterion) {
    let mut group = c.benchmark_group("scorer_uniform");
    let scorer = UniformScorer;
    let node = make_node(0, 1, 2);

    for &n in &[1usize, 10, 32] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || make_candidates(n),
                |candidates| black_box(scorer.score_candidates(&node, &candidates)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scorer: Table
// ---------------------------------------------------------------------------

fn bench_scorer_table(c: &mut Criterion) {
    let mut group = c.benchmark_group("scorer_table");

    for &n in &[1usize, 10, 32] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            // Build candidates and a table that maps all of them.
            let candidates = make_candidates(n);
            let table: std::collections::BTreeMap<String, i64> = candidates
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    #[allow(clippy::cast_possible_wrap)]
                    let bonus = i as i64;
                    (c.canonical_hash().as_str().to_string(), bonus)
                })
                .collect();
            let digest = ContentHash::parse(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .expect("valid hash");
            let scorer = sterling_search::scorer::TableScorer::new(table, digest);
            let node = make_node(0, 1, 2);

            b.iter_batched(
                || candidates.clone(),
                |cands| black_box(scorer.score_candidates(&node, &cands)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Apply + fingerprint (one candidate)
// ---------------------------------------------------------------------------

fn bench_apply_fingerprint(c: &mut Criterion) {
    let state = ByteStateV1::new(1, 2);
    let op_code = OP_SET_SLOT;
    let op_args = set_slot_args(0, 0, Code32::new(1, 0, 0));

    c.bench_function("apply_fingerprint", |b| {
        b.iter(|| {
            let (new_state, _record) =
                apply(black_box(&state), black_box(op_code), black_box(&op_args))
                    .expect("apply should succeed");
            black_box(canonical_hash(
                DOMAIN_SEARCH_NODE,
                &new_state.identity_bytes(),
            ))
        });
    });
}

// ---------------------------------------------------------------------------
// Graph serialization
// ---------------------------------------------------------------------------

fn bench_graph_serialization(c: &mut Criterion) {
    use sterling_benchmarks::{prepare_search_setup, run_search_only};
    use sterling_harness::runner::ScorerInputV1;
    use sterling_harness::worlds::slot_lattice_regimes::{
        regime_budget_limited, regime_duplicates, regime_truncation,
    };

    let mut group = c.benchmark_group("graph_serialization");

    let regimes: Vec<(&str, _)> = vec![
        ("budget_limited", regime_budget_limited()),
        ("truncation", regime_truncation()),
        ("duplicates", regime_duplicates()),
    ];

    for (name, regime) in &regimes {
        let setup = prepare_search_setup(&regime.world, &regime.policy, &ScorerInputV1::Uniform);
        let result = run_search_only(&setup, &regime.world, &regime.policy, &UniformScorer);
        let expansions = result.graph.metadata.total_expansions;

        group.bench_with_input(
            BenchmarkId::new(*name, expansions),
            &result.graph,
            |b, graph| {
                b.iter(|| black_box(graph.to_canonical_json_bytes().expect("serialization")));
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_frontier,
    bench_scorer_uniform,
    bench_scorer_table,
    bench_apply_fingerprint,
    bench_graph_serialization,
);
criterion_main!(benches);
