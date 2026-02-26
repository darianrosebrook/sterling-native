//! Auditable benchmark report harness.
//!
//! Follows Sterling v1's `BoundInputsV1 + RunTelemetryV1` pattern:
//! - **`InputSnapshotV1`**: canonical JSON, content-addressed. Records *what* was measured
//!   (regime, policy, scorer, registry digest, bench profile). Hashable and stable.
//! - **`MeasurementV1`**: observational JSON. Records *the measurements* (integer nanoseconds).
//!   References `input_snapshot_digest`. Not canonicalized (timing is not deterministic).
//!
//! Timing values are integer nanoseconds internally; presentation code derives μs/ms.
//! No floats in normative or observational surfaces.
//!
//! Run via `cargo bench --bench auditable_report`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

use serde::Serialize;

use sterling_benchmarks::{build_table_scorer_for_regime, prepare_search_setup, run_search_only};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::slot_lattice_regimes::{
    regime_budget_limited, regime_duplicates, regime_exhaustive_dead_end, regime_frontier_pressure,
    regime_truncation, Regime,
};
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;
use sterling_search::scorer::{UniformScorer, ValueScorer};

// ---------------------------------------------------------------------------
// Domain constant for input snapshot hashing
// ---------------------------------------------------------------------------

const DOMAIN_BENCH_INPUT: &[u8] = b"STERLING::BENCH_INPUT::V1\0";

// ---------------------------------------------------------------------------
// Input Snapshot (canonical, hashable)
// ---------------------------------------------------------------------------

/// Records *what* was measured. Content-addressed via `canonical_hash`.
fn build_input_snapshot(
    regime_name: &str,
    regime: &Regime,
    scorer_name: &str,
    scorer_input: &ScorerInputV1,
    setup: &sterling_benchmarks::SearchSetup,
) -> (serde_json::Value, String) {
    let scorer_digest = match scorer_input {
        ScorerInputV1::Uniform => None,
        ScorerInputV1::Table { artifact, .. } => Some(artifact.content_hash.as_str().to_string()),
    };

    // Policy as canonical JSON for hashing
    let policy_json = search_policy_to_json(&regime.policy);

    let snapshot = serde_json::json!({
        "schema_id": "sterling.bench_input.v1",
        "regime_name": regime_name,
        "world_id": setup.bindings.world_id,
        "policy": policy_json,
        "registry_digest": setup.bindings.registry_digest,
        "scorer_type": scorer_name,
        "scorer_digest": scorer_digest,
        "bench_profile": {
            "codegen_units": 1,
            "lto": "thin",
            "panic": "unwind"
        },
        "warmup_iterations": WARMUP_ITERATIONS,
        "timed_iterations": TIMED_ITERATIONS,
    });

    let bytes = canonical_json_bytes(&snapshot).expect("canonical_json_bytes");
    let hash = canonical_hash(DOMAIN_BENCH_INPUT, &bytes);
    (snapshot, hash.as_str().to_string())
}

// ---------------------------------------------------------------------------
// Measurement (observational, references input snapshot)
// ---------------------------------------------------------------------------

/// Timing statistics — all integer nanoseconds. Derive μs/ms in presentation only.
#[derive(Serialize)]
struct TimingStats {
    count: usize,
    sum_ns: u128,
    min_ns: u128,
    max_ns: u128,
    p50_ns: u128,
    p95_ns: u128,
}

#[derive(Serialize)]
struct MeasurementV1 {
    schema_id: &'static str,
    input_snapshot_digest: String,
    measurement_kind: String,
    timing: TimingStats,
    graph_metadata: Option<GraphMetadata>,
}

#[derive(Serialize)]
struct GraphMetadata {
    total_expansions: u64,
    total_candidates_generated: u64,
    total_duplicates_suppressed: u64,
    frontier_high_water: u64,
    total_nodes: usize,
    termination_reason: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Top-level report
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct BenchReportV1 {
    schema_id: &'static str,
    timestamp_utc: String,
    machine: MachineInfo,
    definitions: Definitions,
    input_snapshots: BTreeMap<String, serde_json::Value>,
    measurements: Vec<MeasurementV1>,
}

#[derive(Serialize)]
struct MachineInfo {
    /// `_telemetry_only` sentinel: machine info is observational, not normative.
    _telemetry_only: bool,
    os: &'static str,
    arch: &'static str,
}

#[derive(Serialize)]
struct Definitions {
    node_definition: &'static str,
    p95_method: &'static str,
    timing_unit: &'static str,
}

// ---------------------------------------------------------------------------
// Timing helpers
// ---------------------------------------------------------------------------

const WARMUP_ITERATIONS: usize = 5;
const TIMED_ITERATIONS: usize = 50;

fn percentile_ns(sorted: &[u128], pct: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (pct / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn compute_timing_stats(durations_ns: &mut [u128]) -> TimingStats {
    durations_ns.sort_unstable();
    let sum_ns: u128 = durations_ns.iter().sum();
    TimingStats {
        count: durations_ns.len(),
        sum_ns,
        min_ns: durations_ns.first().copied().unwrap_or(0),
        max_ns: durations_ns.last().copied().unwrap_or(0),
        p50_ns: percentile_ns(durations_ns, 50.0),
        p95_ns: percentile_ns(durations_ns, 95.0),
    }
}

// ---------------------------------------------------------------------------
// Stable termination reason serialization (matches search_graph.json shape)
// ---------------------------------------------------------------------------

fn termination_reason_to_stable_json(
    r: &sterling_search::graph::TerminationReasonV1,
) -> serde_json::Value {
    use sterling_search::graph::{FrontierInvariantStageV1, PanicStageV1, TerminationReasonV1};
    match r {
        TerminationReasonV1::GoalReached { node_id } => {
            serde_json::json!({"node_id": node_id, "type": "goal_reached"})
        }
        TerminationReasonV1::FrontierExhausted => {
            serde_json::json!({"type": "frontier_exhausted"})
        }
        TerminationReasonV1::ExpansionBudgetExceeded => {
            serde_json::json!({"type": "expansion_budget_exceeded"})
        }
        TerminationReasonV1::DepthBudgetExceeded => {
            serde_json::json!({"type": "depth_budget_exceeded"})
        }
        TerminationReasonV1::WorldContractViolation => {
            serde_json::json!({"type": "world_contract_violation"})
        }
        TerminationReasonV1::ScorerContractViolation { expected, actual } => {
            serde_json::json!({"actual": actual, "expected": expected, "type": "scorer_contract_violation"})
        }
        TerminationReasonV1::InternalPanic { stage } => {
            let stage_str = match stage {
                PanicStageV1::EnumerateCandidates => "enumerate_candidates",
                PanicStageV1::ScoreCandidates => "score_candidates",
                PanicStageV1::IsGoalRoot => "is_goal_root",
                PanicStageV1::IsGoalExpansion => "is_goal_expansion",
            };
            serde_json::json!({"stage": stage_str, "type": "internal_panic"})
        }
        TerminationReasonV1::FrontierInvariantViolation { stage } => {
            let stage_str = match stage {
                FrontierInvariantStageV1::PopFromNonEmptyFrontier => "pop_from_non_empty_frontier",
            };
            serde_json::json!({"stage": stage_str, "type": "frontier_invariant_violation"})
        }
    }
}

// ---------------------------------------------------------------------------
// Regime runner
// ---------------------------------------------------------------------------

struct RegimeSpec {
    name: &'static str,
    regime: Regime,
}

fn run_regime_benchmarks(
    spec: &RegimeSpec,
    input_snapshots: &mut BTreeMap<String, serde_json::Value>,
) -> Vec<MeasurementV1> {
    let mut measurements = Vec::new();

    for scorer_name in &["uniform", "table"] {
        let scorer_input = if *scorer_name == "uniform" {
            ScorerInputV1::Uniform
        } else {
            build_table_scorer_for_regime(&spec.regime)
        };

        let setup = prepare_search_setup(&spec.regime.world, &spec.regime.policy, &scorer_input);

        // Build and register canonical input snapshot
        let (snapshot_json, snapshot_digest) =
            build_input_snapshot(spec.name, &spec.regime, scorer_name, &scorer_input, &setup);
        input_snapshots.insert(snapshot_digest.clone(), snapshot_json);

        let scorer_ref: Box<dyn ValueScorer> = match &scorer_input {
            ScorerInputV1::Uniform => Box::new(UniformScorer),
            ScorerInputV1::Table { scorer, .. } => Box::new(scorer.clone()),
        };

        // -- search() total (includes post-loop build_graph) --
        for _ in 0..WARMUP_ITERATIONS {
            let _ = run_search_only(
                &setup,
                &spec.regime.world,
                &spec.regime.policy,
                &*scorer_ref,
            );
        }

        let mut durations_ns = Vec::with_capacity(TIMED_ITERATIONS);
        let mut last_result = None;
        // Determinism guard: hash graph metadata across iterations.
        let mut prev_meta_digest: Option<String> = None;
        for _ in 0..TIMED_ITERATIONS {
            let start = Instant::now();
            let result = run_search_only(
                &setup,
                &spec.regime.world,
                &spec.regime.policy,
                &*scorer_ref,
            );
            let elapsed = start.elapsed();
            durations_ns.push(elapsed.as_nanos());

            // Determinism guard: verify graph metadata is identical across iterations.
            let meta_json = serde_json::json!({
                "total_expansions": result.graph.metadata.total_expansions,
                "total_candidates_generated": result.graph.metadata.total_candidates_generated,
                "termination_reason": termination_reason_to_stable_json(&result.graph.metadata.termination_reason),
            });
            let meta_bytes = serde_json::to_vec(&meta_json).expect("json");
            let meta_digest = sha2_digest(&meta_bytes);
            if let Some(ref prev) = prev_meta_digest {
                assert_eq!(
                    prev, &meta_digest,
                    "graph metadata changed between iterations — search is not deterministic"
                );
            }
            prev_meta_digest = Some(meta_digest);
            last_result = Some(result);
        }

        let timing = compute_timing_stats(&mut durations_ns);
        let graph_meta = last_result.as_ref().map(|r| {
            let m = &r.graph.metadata;
            GraphMetadata {
                total_expansions: m.total_expansions,
                total_candidates_generated: m.total_candidates_generated,
                total_duplicates_suppressed: m.total_duplicates_suppressed,
                frontier_high_water: m.frontier_high_water,
                total_nodes: r.nodes.len(),
                termination_reason: termination_reason_to_stable_json(&m.termination_reason),
            }
        });

        eprintln!(
            "  {}/{scorer_name}/search_fn_total: p50={}ns p95={}ns",
            spec.name, timing.p50_ns, timing.p95_ns,
        );

        measurements.push(MeasurementV1 {
            schema_id: "sterling.bench_measurement.v1",
            input_snapshot_digest: snapshot_digest.clone(),
            measurement_kind: "search_fn_total".to_string(),
            timing,
            graph_metadata: graph_meta,
        });

        // -- run_search() end-to-end --
        for _ in 0..WARMUP_ITERATIONS {
            let _ = run_search(&spec.regime.world, &spec.regime.policy, &scorer_input);
        }

        let mut durations_ns = Vec::with_capacity(TIMED_ITERATIONS);
        for _ in 0..TIMED_ITERATIONS {
            let start = Instant::now();
            let _bundle = run_search(&spec.regime.world, &spec.regime.policy, &scorer_input)
                .expect("run_search");
            let elapsed = start.elapsed();
            durations_ns.push(elapsed.as_nanos());
        }

        let timing = compute_timing_stats(&mut durations_ns);

        eprintln!(
            "  {}/{scorer_name}/run_search_e2e: p50={}ns p95={}ns",
            spec.name, timing.p50_ns, timing.p95_ns,
        );

        measurements.push(MeasurementV1 {
            schema_id: "sterling.bench_measurement.v1",
            input_snapshot_digest: snapshot_digest,
            measurement_kind: "run_search_e2e".to_string(),
            timing,
            graph_metadata: None,
        });
    }

    measurements
}

/// Quick SHA-256 hex string for determinism guard.
fn sha2_digest(data: &[u8]) -> String {
    canonical_hash(b"STERLING::BENCH_GUARD\0", data)
        .hex_digest()
        .to_string()
}

// ---------------------------------------------------------------------------
// Policy serialization (matches runner.rs)
// ---------------------------------------------------------------------------

fn search_policy_to_json(policy: &sterling_search::policy::SearchPolicyV1) -> serde_json::Value {
    let dedup_key = match policy.dedup_key {
        sterling_search::policy::DedupKeyV1::IdentityOnly => "identity_only",
        sterling_search::policy::DedupKeyV1::FullState => "full_state",
    };
    let prune_visited = match policy.prune_visited_policy {
        sterling_search::policy::PruneVisitedPolicyV1::KeepVisited => "keep_visited",
        sterling_search::policy::PruneVisitedPolicyV1::ReleaseVisited => "release_visited",
    };
    serde_json::json!({
        "dedup_key": dedup_key,
        "max_candidates_per_node": policy.max_candidates_per_node,
        "max_depth": policy.max_depth,
        "max_expansions": policy.max_expansions,
        "max_frontier_size": policy.max_frontier_size,
        "prune_visited_policy": prune_visited,
    })
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

    let mut input_snapshots = BTreeMap::new();
    let mut all_measurements = Vec::new();
    for spec in &specs {
        eprintln!("Benchmarking regime: {} ...", spec.name);
        let measurements = run_regime_benchmarks(spec, &mut input_snapshots);
        all_measurements.extend(measurements);
    }

    let report = BenchReportV1 {
        schema_id: "sterling.bench_report.v1",
        timestamp_utc: {
            let since_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            format!("epoch:{}", since_epoch.as_secs())
        },
        machine: MachineInfo {
            _telemetry_only: true,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        definitions: Definitions {
            node_definition: "A node is a unique (state, fingerprint) pair created during search. \
                total_nodes counts all SearchNodeV1 instances (root + children). \
                total_expansions counts frontier pops that produced candidate enumeration.",
            p95_method: "Sort all iteration durations ascending, take value at index \
                round(0.95 * (N-1)) where N = timed_iterations.",
            timing_unit: "All timing values are integer nanoseconds. Derive microseconds/milliseconds only in presentation code.",
        },
        input_snapshots,
        measurements: all_measurements,
    };

    // Write to target/bench_reports/
    let report_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/bench_reports");
    fs::create_dir_all(report_dir).expect("create bench_reports dir");

    let report_path = format!("{report_dir}/bench_report_v1_latest.json");
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    fs::write(&report_path, &json).expect("write report");

    eprintln!("\nReport written to: {report_path}");
    eprintln!(
        "({} measurements, {} input snapshots)",
        report.measurements.len(),
        report.input_snapshots.len()
    );
}
