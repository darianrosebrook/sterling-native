use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use sterling_benchmarks::{build_table_scorer_for_regime, prepare_search_setup, run_search_only};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::slot_lattice_regimes::{
    regime_budget_limited, regime_duplicates, regime_exhaustive_dead_end, regime_frontier_pressure,
    regime_truncation, Regime,
};
use sterling_search::scorer::UniformScorer;

// ---------------------------------------------------------------------------
// Engine throughput: search() only (no compilation/bundling)
// ---------------------------------------------------------------------------

fn bench_search_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_engine");
    // Increase sample size for fast regimes, decrease for slow ones.
    group.sample_size(50);

    let regimes: Vec<(&str, Regime)> = vec![
        ("budget_limited", regime_budget_limited()),
        ("truncation", regime_truncation()),
        ("duplicates", regime_duplicates()),
        ("exhaustive_dead_end", regime_exhaustive_dead_end()),
        ("frontier_pressure", regime_frontier_pressure()),
    ];

    for (name, regime) in &regimes {
        // Uniform scorer
        let setup_u = prepare_search_setup(&regime.world, &regime.policy, &ScorerInputV1::Uniform);
        group.bench_with_input(
            BenchmarkId::new(format!("{name}/uniform"), ""),
            &(),
            |b, ()| {
                b.iter(|| run_search_only(&setup_u, &regime.world, &regime.policy, &UniformScorer));
            },
        );

        // Table scorer
        let table_input = build_table_scorer_for_regime(regime);
        let setup_t = prepare_search_setup(&regime.world, &regime.policy, &table_input);
        let scorer_ref: &dyn sterling_search::scorer::ValueScorer = match &table_input {
            ScorerInputV1::Uniform => &UniformScorer as &dyn sterling_search::scorer::ValueScorer,
            ScorerInputV1::Table { scorer, .. } => scorer,
        };
        group.bench_with_input(
            BenchmarkId::new(format!("{name}/table"), ""),
            &(),
            |b, ()| {
                b.iter(|| run_search_only(&setup_t, &regime.world, &regime.policy, scorer_ref));
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Artifact throughput: run_search() end-to-end (compile + search + bundle)
// ---------------------------------------------------------------------------

fn bench_run_search_artifact(c: &mut Criterion) {
    let mut group = c.benchmark_group("artifact_throughput");
    group.sample_size(20);

    let regimes: Vec<(&str, Regime)> = vec![
        ("budget_limited", regime_budget_limited()),
        ("truncation", regime_truncation()),
        ("duplicates", regime_duplicates()),
        ("exhaustive_dead_end", regime_exhaustive_dead_end()),
        ("frontier_pressure", regime_frontier_pressure()),
    ];

    for (name, regime) in &regimes {
        // Uniform scorer: full pipeline
        group.bench_with_input(
            BenchmarkId::new(format!("{name}/uniform"), ""),
            &(),
            |b, ()| {
                b.iter(|| {
                    run_search(&regime.world, &regime.policy, &ScorerInputV1::Uniform)
                        .expect("run_search");
                });
            },
        );

        // Table scorer: full pipeline
        let table_input = build_table_scorer_for_regime(regime);
        group.bench_with_input(
            BenchmarkId::new(format!("{name}/table"), ""),
            &(),
            |b, ()| {
                b.iter(|| {
                    run_search(&regime.world, &regime.policy, &table_input).expect("run_search");
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_search_engine, bench_run_search_artifact);
criterion_main!(benches);
