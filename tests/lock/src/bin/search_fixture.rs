//! Binary that runs `RomeMiniSearch` through the search pipeline
//! and prints deterministic output lines for cross-process verification.
//!
//! Usage: `search_fixture`
//!
//! Output: key=value lines (see source for format).

use sterling_harness::bundle::DOMAIN_BUNDLE_ARTIFACT;
use sterling_harness::runner::run_search;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::proof::hash::canonical_hash;
use sterling_search::policy::SearchPolicyV1;
use sterling_search::scorer::UniformScorer;

fn main() {
    let policy = SearchPolicyV1::default();
    let scorer = UniformScorer;
    let bundle = run_search(&RomeMiniSearch, &policy, &scorer).expect("search run failed");

    // Extract verification report.
    let report = bundle
        .artifacts
        .get("verification_report.json")
        .expect("missing verification_report.json");
    let report_json: serde_json::Value =
        serde_json::from_slice(&report.content).expect("invalid report JSON");
    let policy_digest = report_json["policy_digest"]
        .as_str()
        .expect("missing policy_digest");
    let mode = report_json["mode"].as_str().expect("missing mode");

    // Extract search graph.
    let graph = bundle
        .artifacts
        .get("search_graph.json")
        .expect("missing search_graph.json");
    let graph_json: serde_json::Value =
        serde_json::from_slice(&graph.content).expect("invalid graph JSON");
    let term_type = graph_json["metadata"]["termination_reason"]["type"]
        .as_str()
        .expect("missing termination_reason.type");
    let total_expansions = graph_json["metadata"]["total_expansions"]
        .as_u64()
        .expect("missing total_expansions");

    let search_graph_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &graph.content);

    println!("bundle_digest={}", bundle.digest.as_str());
    println!("search_graph_digest={}", search_graph_hash.as_str());
    println!("policy_digest={policy_digest}");
    println!("termination_reason={term_type}");
    println!("artifact_count={}", bundle.artifacts.len());
    println!("search_graph_normative={}", graph.normative);
    println!("total_expansions={total_expansions}");
    println!("mode={mode}");
}
