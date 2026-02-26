//! Auditable benchmark report harness.
//!
//! Uses `std::time::Instant` for wall-clock timing, NOT Criterion.
//! Emits a versioned `bench_report_v1` JSON artifact to `target/bench_reports/`.
//!
//! Measures both "engine throughput" (`search()` only) and "artifact throughput"
//! (`run_search()` end-to-end) for each regime × scorer combination.
//!
//! Run via `cargo bench --bench auditable_report`.

// Numeric casts in timing harness are intentional and benign:
// - u64→usize for Vec capacity (iterations fit in usize)
// - u128→f64 for microseconds (precision loss negligible at μs scale)
// - usize→f64 for percentile indexing
// - f64→usize for percentile index (always non-negative, bounded)
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::fs;
use std::time::Instant;

use serde::Serialize;

use sterling_benchmarks::{build_table_scorer_for_regime, prepare_search_setup, run_search_only};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::slot_lattice_regimes::{
    regime_budget_limited, regime_duplicates, regime_exhaustive_dead_end, regime_frontier_pressure,
    regime_truncation, Regime,
};
use sterling_search::scorer::{UniformScorer, ValueScorer};

// ---------------------------------------------------------------------------
// Report schema
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct BenchReport {
    version: &'static str,
    timestamp_utc: String,
    machine: MachineInfo,
    definitions: Definitions,
    results: Vec<BenchResult>,
}

#[derive(Serialize)]
struct MachineInfo {
    os: &'static str,
    arch: &'static str,
    rust_version: String,
}

/// Pin definitions so future readers know what the numbers mean.
#[derive(Serialize)]
struct Definitions {
    /// What "node" means in the 1000-node target.
    node_definition: &'static str,
    /// How p95 is computed.
    p95_method: &'static str,
    /// Number of warmup iterations before measurement.
    warmup_iterations: usize,
    /// Number of timed iterations.
    timed_iterations: usize,
}

#[derive(Serialize)]
struct BenchResult {
    name: String,
    regime: String,
    scorer: String,
    measurement: String,
    iterations: usize,
    mean_us: f64,
    p50_us: f64,
    p95_us: f64,
    min_us: f64,
    max_us: f64,
    graph_metadata: Option<GraphMetadata>,
}

#[derive(Serialize)]
struct GraphMetadata {
    total_expansions: u64,
    total_candidates_generated: u64,
    total_duplicates_suppressed: u64,
    frontier_high_water: u64,
    total_nodes: usize,
    termination_type: String,
}

// ---------------------------------------------------------------------------
// Timing helpers
// ---------------------------------------------------------------------------

const WARMUP_ITERATIONS: usize = 5;
const TIMED_ITERATIONS: usize = 50;

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (pct / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn statistics(durations_us: &mut [f64]) -> (f64, f64, f64, f64, f64) {
    durations_us.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let sum: f64 = durations_us.iter().sum();
    let mean = sum / durations_us.len() as f64;
    let p50 = percentile(durations_us, 50.0);
    let p95 = percentile(durations_us, 95.0);
    let min = durations_us.first().copied().unwrap_or(0.0);
    let max = durations_us.last().copied().unwrap_or(0.0);
    (mean, p50, p95, min, max)
}

// ---------------------------------------------------------------------------
// Regime runner
// ---------------------------------------------------------------------------

struct RegimeSpec {
    name: &'static str,
    regime: Regime,
}

fn run_regime_benchmarks(spec: &RegimeSpec) -> Vec<BenchResult> {
    let mut results = Vec::new();

    for scorer_name in &["uniform", "table"] {
        let scorer_input = if *scorer_name == "uniform" {
            ScorerInputV1::Uniform
        } else {
            build_table_scorer_for_regime(&spec.regime)
        };

        // -- Engine throughput (search() only) --
        let setup = prepare_search_setup(&spec.regime.world, &spec.regime.policy, &scorer_input);
        let scorer_ref: Box<dyn ValueScorer> = match &scorer_input {
            ScorerInputV1::Uniform => Box::new(UniformScorer),
            ScorerInputV1::Table { scorer, .. } => Box::new(scorer.clone()),
        };

        // Warmup
        for _ in 0..WARMUP_ITERATIONS {
            let _ = run_search_only(
                &setup,
                &spec.regime.world,
                &spec.regime.policy,
                &*scorer_ref,
            );
        }

        // Timed runs
        let mut durations_us = Vec::with_capacity(TIMED_ITERATIONS);
        let mut last_result = None;
        for _ in 0..TIMED_ITERATIONS {
            let start = Instant::now();
            let result = run_search_only(
                &setup,
                &spec.regime.world,
                &spec.regime.policy,
                &*scorer_ref,
            );
            let elapsed = start.elapsed();
            durations_us.push(elapsed.as_micros() as f64);
            last_result = Some(result);
        }

        let (mean, p50, p95, min, max) = statistics(&mut durations_us);
        let graph_meta = last_result.as_ref().map(|r| {
            let m = &r.graph.metadata;
            let term_type = format!("{:?}", m.termination_reason);
            GraphMetadata {
                total_expansions: m.total_expansions,
                total_candidates_generated: m.total_candidates_generated,
                total_duplicates_suppressed: m.total_duplicates_suppressed,
                frontier_high_water: m.frontier_high_water,
                total_nodes: r.nodes.len(),
                termination_type: term_type,
            }
        });

        results.push(BenchResult {
            name: format!("{}/{scorer_name}/engine", spec.name),
            regime: spec.name.to_string(),
            scorer: (*scorer_name).to_string(),
            measurement: "search_only".to_string(),
            iterations: TIMED_ITERATIONS,
            mean_us: mean,
            p50_us: p50,
            p95_us: p95,
            min_us: min,
            max_us: max,
            graph_metadata: graph_meta,
        });

        // -- Artifact throughput (run_search() end-to-end) --
        for _ in 0..WARMUP_ITERATIONS {
            let _ = run_search(&spec.regime.world, &spec.regime.policy, &scorer_input);
        }

        let mut durations_us = Vec::with_capacity(TIMED_ITERATIONS);
        for _ in 0..TIMED_ITERATIONS {
            let start = Instant::now();
            let _bundle = run_search(&spec.regime.world, &spec.regime.policy, &scorer_input)
                .expect("run_search");
            let elapsed = start.elapsed();
            durations_us.push(elapsed.as_micros() as f64);
        }

        let (mean, p50, p95, min, max) = statistics(&mut durations_us);

        results.push(BenchResult {
            name: format!("{}/{scorer_name}/artifact", spec.name),
            regime: spec.name.to_string(),
            scorer: (*scorer_name).to_string(),
            measurement: "run_search_e2e".to_string(),
            iterations: TIMED_ITERATIONS,
            mean_us: mean,
            p50_us: p50,
            p95_us: p95,
            min_us: min,
            max_us: max,
            graph_metadata: None,
        });
    }

    results
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let specs = vec![
        RegimeSpec {
            name: "budget_limited",
            regime: regime_budget_limited(),
        },
        RegimeSpec {
            name: "truncation",
            regime: regime_truncation(),
        },
        RegimeSpec {
            name: "duplicates",
            regime: regime_duplicates(),
        },
        RegimeSpec {
            name: "exhaustive_dead_end",
            regime: regime_exhaustive_dead_end(),
        },
        RegimeSpec {
            name: "frontier_pressure",
            regime: regime_frontier_pressure(),
        },
    ];

    let mut all_results = Vec::new();
    for spec in &specs {
        eprintln!("Benchmarking regime: {} ...", spec.name);
        let results = run_regime_benchmarks(spec);
        for r in &results {
            eprintln!(
                "  {}: mean={:.0}us p50={:.0}us p95={:.0}us",
                r.name, r.mean_us, r.p50_us, r.p95_us,
            );
        }
        all_results.extend(results);
    }

    let rust_version = option_env!("RUSTC_VERSION")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string();

    let report = BenchReport {
        version: "bench_report_v1",
        timestamp_utc: {
            let since_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            format!("epoch:{}", since_epoch.as_secs())
        },
        machine: MachineInfo {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            rust_version,
        },
        definitions: Definitions {
            node_definition: "A node is a unique (state, fingerprint) pair created during search. \
                total_nodes counts all SearchNodeV1 instances (root + children). \
                total_expansions counts frontier pops that produced candidate enumeration.",
            p95_method: "Sort all iteration durations ascending, take value at index \
                round(0.95 * (N-1)) where N = timed_iterations.",
            warmup_iterations: WARMUP_ITERATIONS,
            timed_iterations: TIMED_ITERATIONS,
        },
        results: all_results,
    };

    // Write to target/bench_reports/
    let report_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/bench_reports");
    fs::create_dir_all(report_dir).expect("create bench_reports dir");

    let report_path = format!("{report_dir}/bench_report_v1_latest.json");
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    fs::write(&report_path, &json).expect("write report");

    eprintln!("\nReport written to: {report_path}");
    eprintln!(
        "({} results across {} regimes)",
        report.results.len(),
        specs.len()
    );
}
