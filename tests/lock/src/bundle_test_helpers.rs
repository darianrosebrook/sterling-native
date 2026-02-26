//! Shared test helpers for mutating and rebuilding artifact bundles.
//!
//! These helpers maintain digest consistency when modifying bundle artifacts,
//! preventing tests from accidentally testing digest mismatch instead of the
//! semantic mismatch they intend to exercise.

use sterling_harness::bundle::{build_bundle, ArtifactBundleV1, DOMAIN_BUNDLE_ARTIFACT};
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;

/// Modify the `search_graph.json` in a bundle and rebuild with consistent
/// `search_graph_digest` in the report, so that digest-binding checks pass
/// and only the metadata-binding check under test fires.
///
/// This is the **only** sanctioned way to mutate a bundle for negative tests.
/// Call sites must NOT manually patch report digests.
///
/// # Panics
///
/// Panics if the bundle is missing `search_graph.json` or `verification_report.json`,
/// or if their contents are not valid JSON. These are test-only invariants.
pub fn rebuild_with_modified_graph(
    bundle: &ArtifactBundleV1,
    modify: impl FnOnce(&mut serde_json::Value),
) -> ArtifactBundleV1 {
    let graph_artifact = bundle.artifacts.get("search_graph.json").unwrap();
    let mut graph_json: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).unwrap();
    modify(&mut graph_json);
    let modified_graph_bytes = canonical_json_bytes(&graph_json).unwrap();

    // Recompute the content hash for the modified graph.
    let new_graph_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_graph_bytes);

    // Update the report's search_graph_digest to match the modified graph.
    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();
    report_json["search_graph_digest"] = serde_json::json!(new_graph_hash.as_str());
    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "search_graph.json" {
                (a.name.clone(), modified_graph_bytes.clone(), a.normative)
            } else if a.name == "verification_report.json" {
                (a.name.clone(), modified_report_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    build_bundle(artifacts).unwrap()
}
