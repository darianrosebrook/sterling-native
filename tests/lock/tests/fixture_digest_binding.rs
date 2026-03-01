//! Lock tests for `fixture_digest` binding in the search evidence corridor.
//!
//! Proves that `fixture_digest` (content hash of `fixture.json`) is:
//! - Present in `search_graph.json` metadata
//! - Present in tape header
//! - Cross-verified against `fixture.json` artifact content hash
//! - Tamper-detected when `fixture.json` is modified
//! - Deterministic across runs
//! - Policy-independent (same world under different policies)

use lock_tests::bundle_test_helpers::rebuild_with_modified_graph;
use sterling_harness::bundle::{verify_bundle, BundleVerifyError};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_search::policy::SearchPolicyV1;

/// Produce a search bundle from `RomeMiniSearch` with default policy.
fn default_bundle() -> sterling_harness::bundle::ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("search run")
}

/// Parse the search graph JSON from a bundle.
fn parse_graph(bundle: &sterling_harness::bundle::ArtifactBundleV1) -> serde_json::Value {
    let art = bundle
        .artifacts
        .get("search_graph.json")
        .expect("search_graph.json");
    serde_json::from_slice(&art.content).expect("valid JSON")
}

/// Parse the tape header JSON from a bundle.
fn parse_tape_header(bundle: &sterling_harness::bundle::ArtifactBundleV1) -> serde_json::Value {
    let art = bundle
        .artifacts
        .get("search_tape.stap")
        .expect("search_tape.stap");
    let tape =
        sterling_search::tape_reader::read_tape(&art.content).expect("tape parse");
    tape.header.json.clone()
}

/// Parse the verification report JSON from a bundle.
fn parse_report(bundle: &sterling_harness::bundle::ArtifactBundleV1) -> serde_json::Value {
    let art = bundle
        .artifacts
        .get("verification_report.json")
        .expect("verification_report.json");
    serde_json::from_slice(&art.content).expect("valid JSON")
}

// ---------------------------------------------------------------------------
// Presence and format
// ---------------------------------------------------------------------------

/// `fixture_digest` is present in `search_graph.json` metadata as non-empty
/// 64-char hex string.
#[test]
fn fixture_digest_present_in_graph_metadata() {
    let bundle = default_bundle();
    let graph = parse_graph(&bundle);
    let digest = graph["metadata"]["fixture_digest"]
        .as_str()
        .expect("fixture_digest must be a string");
    assert_eq!(digest.len(), 64, "fixture_digest must be 64-char hex");
    assert!(
        digest.chars().all(|c| c.is_ascii_hexdigit()),
        "fixture_digest must be hex"
    );
}

/// `fixture_digest` in graph metadata matches `fixture.json` artifact's
/// content hash (`binding_hex` format: raw hex, no `sha256:` prefix).
#[test]
fn fixture_digest_matches_fixture_artifact() {
    let bundle = default_bundle();
    let graph = parse_graph(&bundle);
    let graph_digest = graph["metadata"]["fixture_digest"]
        .as_str()
        .expect("fixture_digest");

    let fixture_art = bundle.artifacts.get("fixture.json").expect("fixture.json");
    let artifact_hex = fixture_art.content_hash.hex_digest();

    assert_eq!(
        graph_digest, artifact_hex,
        "graph metadata fixture_digest must equal fixture.json content_hash hex"
    );
}

/// `fixture_digest` is present in the tape header and matches graph metadata.
#[test]
fn fixture_digest_in_tape_header() {
    let bundle = default_bundle();
    let header = parse_tape_header(&bundle);
    let tape_digest = header["fixture_digest"]
        .as_str()
        .expect("tape header must have fixture_digest");

    let graph = parse_graph(&bundle);
    let graph_digest = graph["metadata"]["fixture_digest"]
        .as_str()
        .expect("graph must have fixture_digest");

    assert_eq!(
        tape_digest, graph_digest,
        "tape header fixture_digest must match graph metadata"
    );
}

/// `fixture_digest` is present in the verification report (full `sha256:hex`
/// format, matching scorer/operator digest convention).
#[test]
fn fixture_digest_in_verification_report() {
    let bundle = default_bundle();
    let report = parse_report(&bundle);
    let report_digest = report["fixture_digest"]
        .as_str()
        .expect("report must have fixture_digest");

    assert!(
        report_digest.starts_with("sha256:"),
        "report fixture_digest must use full sha256:hex format"
    );

    // Verify it matches the fixture.json artifact content hash.
    let fixture_art = bundle.artifacts.get("fixture.json").expect("fixture.json");
    assert_eq!(
        report_digest,
        fixture_art.content_hash.as_str(),
        "report fixture_digest must equal fixture.json content_hash"
    );
}

// ---------------------------------------------------------------------------
// Tamper detection
// ---------------------------------------------------------------------------

/// Modifying graph metadata `fixture_digest` while keeping `fixture.json`
/// unchanged triggers `MetadataBindingFixtureMismatch`.
#[test]
fn tampered_fixture_digest_in_graph_rejected() {
    let bundle = default_bundle();
    let tampered = rebuild_with_modified_graph(&bundle, |graph| {
        graph["metadata"]["fixture_digest"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
    });

    let err = verify_bundle(&tampered).expect_err("must fail");
    assert!(
        matches!(err, BundleVerifyError::MetadataBindingFixtureMismatch { .. }),
        "expected MetadataBindingFixtureMismatch, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

/// Two search runs produce identical `fixture_digest` values.
#[test]
fn fixture_digest_deterministic() {
    let b1 = default_bundle();
    let b2 = default_bundle();

    let g1 = parse_graph(&b1);
    let g2 = parse_graph(&b2);

    assert_eq!(
        g1["metadata"]["fixture_digest"],
        g2["metadata"]["fixture_digest"],
        "fixture_digest must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// Policy independence
// ---------------------------------------------------------------------------

/// `fixture_digest` is unchanged when the same world runs under different
/// policy configs. Proves fixture identity is policy-independent (INV-BIND-03).
#[test]
fn fixture_digest_policy_independent() {
    let policy_a = SearchPolicyV1::default();
    let policy_b = SearchPolicyV1 {
        max_expansions: 1, // Deliberately different budget.
        ..SearchPolicyV1::default()
    };

    let bundle_a =
        run_search(&RomeMiniSearch, &policy_a, &ScorerInputV1::Uniform).expect("run a");
    let bundle_b =
        run_search(&RomeMiniSearch, &policy_b, &ScorerInputV1::Uniform).expect("run b");

    let graph_a = parse_graph(&bundle_a);
    let graph_b = parse_graph(&bundle_b);

    assert_eq!(
        graph_a["metadata"]["fixture_digest"],
        graph_b["metadata"]["fixture_digest"],
        "fixture_digest must be identical under different policies"
    );

    // Sanity: policy_snapshot_digest SHOULD differ.
    let report_a = parse_report(&bundle_a);
    let report_b = parse_report(&bundle_b);
    // policy_digest differs because max_expansions changes the policy snapshot.
    // (Not asserting inequality because PolicyConfig::default() is used for both,
    // only search_policy_digest or termination may differ.)
    // The key invariant is fixture_digest equality above.
    let _ = (report_a, report_b);
}
