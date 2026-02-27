//! SEARCH-CORE-001 M3.2 lock tests: table scorer as auditable bundle artifact.
//!
//! Each test targets a specific acceptance criterion from the M3.2 milestone.
//! Tests prove that scorer behavior is bound to a normative in-bundle artifact,
//! with fail-closed coherence invariants.

use std::collections::BTreeMap;

use lock_tests::bundle_test_helpers::rebuild_with_modified_graph;
use sterling_harness::bundle::{build_bundle, verify_bundle, BundleVerifyError};
use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};
use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::runner::{build_table_scorer_input, run_search, ScorerInputV1};
use sterling_harness::worlds::slot_lattice_regimes::{regime_truncation, Regime};
use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_search::contract::SearchWorldV1;

/// Build a scorer table that boosts the *last* candidate (by `canonical_hash` sort)
/// from the root expansion to `bonus=100`, forcing a real permutation in the
/// post-score sort order.
///
/// This mirrors the search pre-score pipeline exactly:
///
/// 1. enumerate root candidates
/// 2. sort by `canonical_hash`
/// 3. apply `max_candidates_per_node` cap
///
/// Then assigns `bonus=100` to the last candidate in that capped list.
fn build_scorer_for_regime(regime: &Regime) -> ScorerInputV1 {
    let root = ByteStateV1::new(1, 10); // MAX_SLOTS = 10
    let registry = regime.world.registry().unwrap();
    let mut candidates = regime.world.enumerate_candidates(&root, &registry);
    candidates.sort();

    // Apply cap.
    #[allow(clippy::cast_possible_truncation)]
    if candidates.len() as u64 > regime.policy.max_candidates_per_node {
        candidates.truncate(regime.policy.max_candidates_per_node as usize);
    }

    assert!(
        !candidates.is_empty(),
        "regime must produce at least one candidate"
    );

    // Boost the last candidate (by canonical_hash order) to force reordering.
    let last = candidates.last().unwrap();
    let mut table = BTreeMap::new();
    table.insert(last.canonical_hash().as_str().to_string(), 100_i64);

    build_table_scorer_input(table).expect("build_table_scorer_input should succeed")
}

// ---------------------------------------------------------------------------
// AC-M3.2-01: table_scorer_reorders_candidates_concretely
// ---------------------------------------------------------------------------

#[test]
fn table_scorer_reorders_candidates_concretely() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);

    // Extract the boosted hash for assertion.
    let boosted_hash = match &scorer_input {
        ScorerInputV1::Table(t) => t.scorer.table().keys().next().unwrap().clone(),
        ScorerInputV1::Uniform => panic!("expected Table scorer"),
    };

    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();
    let graph_artifact = bundle.artifacts.get("search_graph.json").unwrap();
    let graph_json: serde_json::Value = serde_json::from_slice(&graph_artifact.content).unwrap();

    // First expansion, first candidate (post-score sort) should be the boosted one.
    let first_expansion = &graph_json["expansions"][0];
    let first_candidate = &first_expansion["candidates"][0];
    let first_hash = first_candidate["action"]["canonical_hash"]
        .as_str()
        .expect("missing canonical_hash");
    assert_eq!(
        first_hash, boosted_hash,
        "boosted candidate should be first after score sort"
    );

    // Verify the bonus is 100.
    let bonus = first_candidate["score"]["bonus"].as_i64().unwrap();
    assert_eq!(bonus, 100, "boosted candidate should have bonus=100");

    // Verify score source is model_digest.
    assert!(
        first_candidate["score"]["source"]["model_digest"].is_string(),
        "score source should be model_digest"
    );
}

// ---------------------------------------------------------------------------
// AC-M3.2-02: table_scorer_deterministic_n10
// ---------------------------------------------------------------------------

#[test]
fn table_scorer_deterministic_n10() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);

    let first_bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();
    let first_graph = first_bundle
        .artifacts
        .get("search_graph.json")
        .unwrap()
        .content
        .clone();

    for _ in 1..10 {
        let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();
        let graph = &bundle.artifacts.get("search_graph.json").unwrap().content;
        assert_eq!(
            &first_graph, graph,
            "search_graph.json bytes differ across runs with same scorer"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-M3.2-03: advisory_only_candidate_set_preserved
// ---------------------------------------------------------------------------

#[test]
fn advisory_only_candidate_set_preserved() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);

    // Uniform run.
    let uniform_bundle =
        run_search(&regime.world, &regime.policy, &ScorerInputV1::Uniform).unwrap();
    let uniform_graph: serde_json::Value = serde_json::from_slice(
        &uniform_bundle
            .artifacts
            .get("search_graph.json")
            .unwrap()
            .content,
    )
    .unwrap();

    // Table run.
    let table_bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();
    let table_graph: serde_json::Value = serde_json::from_slice(
        &table_bundle
            .artifacts
            .get("search_graph.json")
            .unwrap()
            .content,
    )
    .unwrap();

    // First expansion: candidate *sets* (by canonical_hash) must be identical.
    let uniform_hashes: std::collections::BTreeSet<String> = uniform_graph["expansions"][0]
        ["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["action"]["canonical_hash"].as_str().unwrap().to_string())
        .collect();

    let table_hashes: std::collections::BTreeSet<String> = table_graph["expansions"][0]
        ["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["action"]["canonical_hash"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(
        uniform_hashes, table_hashes,
        "scorer must not change the candidate set (advisory-only)"
    );
}

// ---------------------------------------------------------------------------
// AC-M3.2-04: scorer_artifact_in_bundle
// ---------------------------------------------------------------------------

#[test]
fn scorer_artifact_in_bundle() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    let scorer_artifact = bundle
        .artifacts
        .get("scorer.json")
        .expect("scorer.json must be present in table scorer bundle");
    assert!(scorer_artifact.normative, "scorer.json must be normative");
    assert_eq!(
        bundle.artifacts.len(),
        6,
        "table scorer bundle has 6 artifacts"
    );

    // Verify bytes are preserved (content matches what the input provided).
    if let ScorerInputV1::Table(t) = &scorer_input {
        assert_eq!(
            scorer_artifact.content, t.artifact.bytes,
            "scorer.json bytes must match input artifact"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-M3.2-05: scorer_digest_in_report
// ---------------------------------------------------------------------------

#[test]
fn scorer_digest_in_report() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    let report = bundle.artifacts.get("verification_report.json").unwrap();
    let report_json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();
    let scorer_digest = report_json["scorer_digest"]
        .as_str()
        .expect("report must have scorer_digest");
    assert!(scorer_digest.starts_with("sha256:"));

    let scorer_artifact = bundle.artifacts.get("scorer.json").unwrap();
    assert_eq!(
        scorer_digest,
        scorer_artifact.content_hash.as_str(),
        "report scorer_digest must match scorer.json content_hash"
    );
}

// ---------------------------------------------------------------------------
// AC-M3.2-06: scorer_digest_in_graph_metadata
// ---------------------------------------------------------------------------

#[test]
fn scorer_digest_in_graph_metadata() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    let graph = bundle.artifacts.get("search_graph.json").unwrap();
    let graph_json: serde_json::Value = serde_json::from_slice(&graph.content).unwrap();
    let metadata_digest = graph_json["metadata"]["scorer_digest"]
        .as_str()
        .expect("graph metadata must have scorer_digest");

    let scorer_artifact = bundle.artifacts.get("scorer.json").unwrap();
    assert_eq!(
        metadata_digest,
        scorer_artifact.content_hash.hex_digest(),
        "metadata scorer_digest must match scorer.json content_hash hex"
    );
}

// ---------------------------------------------------------------------------
// AC-M3.2-07: verify_bundle_passes_with_scorer
// ---------------------------------------------------------------------------

#[test]
fn verify_bundle_passes_with_scorer() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();
    verify_bundle(&bundle).unwrap();
}

// ---------------------------------------------------------------------------
// AC-M3.2-08: scorer_tamper_detected
// ---------------------------------------------------------------------------

#[test]
fn scorer_tamper_detected() {
    use sterling_kernel::proof::canon::canonical_json_bytes;

    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    // Tamper with scorer.json by modifying the JSON semantically
    // (add a field) while keeping it canonical.
    let scorer_artifact = bundle.artifacts.get("scorer.json").unwrap();
    let mut scorer_json: serde_json::Value =
        serde_json::from_slice(&scorer_artifact.content).unwrap();
    scorer_json["tampered"] = serde_json::json!(true);
    let tampered_scorer_bytes = canonical_json_bytes(&scorer_json).unwrap();

    // Rebuild bundle with tampered scorer.json. build_bundle recomputes
    // content_hash from the new bytes, so the hash will change and
    // mismatch the report's scorer_digest.
    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "scorer.json" {
                (a.name.clone(), tampered_scorer_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let tampered_bundle = build_bundle(artifacts).unwrap();

    let err = verify_bundle(&tampered_bundle).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::ScorerDigestMismatch { .. }),
        "tampered scorer should trigger ScorerDigestMismatch, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-M3.2-09: score_source_digest_mismatch_detected
// ---------------------------------------------------------------------------

#[test]
fn score_source_digest_mismatch_detected() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    // Modify graph: change a candidate's model_digest to a wrong value.
    // We need to also update the report search_graph_digest (via rebuild helper)
    // and scorer digest to match, but change the candidate digest to be wrong.
    let modified_bundle = rebuild_with_modified_graph(&bundle, |graph_json| {
        // Find first candidate with model_digest source and change it.
        if let Some(expansions) = graph_json["expansions"].as_array_mut() {
            for expansion in expansions {
                if let Some(candidates) = expansion["candidates"].as_array_mut() {
                    for candidate in candidates {
                        if candidate["score"]["source"]["model_digest"].is_string() {
                            candidate["score"]["source"]["model_digest"] =
                                serde_json::json!("sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
                            return;
                        }
                    }
                }
            }
        }
    });

    let err = verify_bundle(&modified_bundle).unwrap_err();
    assert!(
        matches!(
            err,
            BundleVerifyError::CandidateScoreSourceScorerMismatch { .. }
        ),
        "mismatched candidate model_digest should trigger CandidateScoreSourceScorerMismatch, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-M3.2-10: uniform_scorer_no_scorer_artifact
// ---------------------------------------------------------------------------

#[test]
fn uniform_scorer_no_scorer_artifact() {
    let regime = regime_truncation();
    let bundle = run_search(&regime.world, &regime.policy, &ScorerInputV1::Uniform).unwrap();

    assert!(
        !bundle.artifacts.contains_key("scorer.json"),
        "uniform scorer bundle must not contain scorer.json"
    );
    assert_eq!(bundle.artifacts.len(), 5, "uniform bundle has 5 artifacts");
    verify_bundle(&bundle).unwrap();
}

// ---------------------------------------------------------------------------
// AC-M3.2-11: bundle_persistence_roundtrip_with_scorer
// ---------------------------------------------------------------------------

#[test]
fn bundle_persistence_roundtrip_with_scorer() {
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    let dir = tempfile::tempdir().unwrap();
    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();
    verify_bundle_dir(dir.path()).unwrap();
    verify_bundle(&loaded).unwrap();

    // All 6 artifacts must round-trip identically.
    assert_eq!(loaded.artifacts.len(), 6);
    for (name, orig) in &bundle.artifacts {
        let loaded_art = loaded
            .artifacts
            .get(name)
            .unwrap_or_else(|| panic!("missing artifact {name} after round-trip"));
        assert_eq!(
            orig.content, loaded_art.content,
            "artifact {name} content differs after round-trip"
        );
        assert_eq!(
            orig.content_hash.as_str(),
            loaded_art.content_hash.as_str(),
            "artifact {name} hash differs after round-trip"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-M3.2-12: scorer_outputs_deterministic_across_invocations
// ---------------------------------------------------------------------------

#[test]
fn scorer_outputs_deterministic_across_invocations() {
    // Build scorer bundle twice in-process and verify identical output.
    // This verifies determinism holds across independent pipeline invocations.
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);

    let bundle1 = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();
    let bundle2 = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    assert_eq!(
        bundle1.digest.as_str(),
        bundle2.digest.as_str(),
        "bundle digests must be identical across runs"
    );

    // Verify scorer presence and digest match.
    let scorer1 = bundle1.artifacts.get("scorer.json").unwrap();
    let scorer2 = bundle2.artifacts.get("scorer.json").unwrap();
    assert_eq!(
        scorer1.content_hash.as_str(),
        scorer2.content_hash.as_str(),
        "scorer artifact digests must match across runs"
    );
    assert_eq!(
        scorer1.content, scorer2.content,
        "scorer artifact bytes must match across runs"
    );
}

// ---------------------------------------------------------------------------
// AC-M3.2-13: table_scorer_root_goal_bundle_verifiable
// ---------------------------------------------------------------------------

#[test]
fn table_scorer_root_goal_bundle_verifiable() {
    // Verifier-surface test: simulate a root-is-goal search with a table scorer.
    // scorer.json exists, but total_expansions==0 and no candidate has ModelDigest.
    // This must verify successfully — the scorer artifact is legitimate evidence
    // even though scoring never ran.
    //
    // The synthetic graph is minimally coherent: root node summary present with
    // is_goal=true, expansion_order=null, correct depth/f_cost. This avoids
    // brittleness if future verifier tightening checks node summary coverage.
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    let modified_bundle = rebuild_with_modified_graph(&bundle, |graph_json| {
        // Extract root fingerprint from original node_summaries (node_id 0).
        let root_fp = graph_json["node_summaries"]
            .as_array()
            .and_then(|ns| {
                ns.iter()
                    .find(|n| n["node_id"].as_u64() == Some(0))
                    .and_then(|n| n["state_fingerprint"].as_str())
                    .map(std::string::ToString::to_string)
            })
            .expect("original graph must have root node summary");

        // Clear expansions and set total_expansions to 0.
        graph_json["expansions"] = serde_json::json!([]);
        graph_json["metadata"]["total_expansions"] = serde_json::json!(0);
        graph_json["metadata"]["total_candidates_generated"] = serde_json::json!(0);
        graph_json["metadata"]["total_duplicates_suppressed"] = serde_json::json!(0);
        graph_json["metadata"]["termination_reason"] =
            serde_json::json!({"node_id": 0, "type": "goal_reached"});

        // Keep root node summary, coherent with goal_reached at root.
        graph_json["node_summaries"] = serde_json::json!([{
            "dead_end_reason": null,
            "depth": 0,
            "expansion_order": null,
            "f_cost": 0,
            "is_goal": true,
            "node_id": 0,
            "parent_id": null,
            "state_fingerprint": root_fp,
        }]);
    });

    assert!(
        modified_bundle.artifacts.contains_key("scorer.json"),
        "scorer.json must still be present"
    );
    verify_bundle(&modified_bundle).unwrap();
}

// ---------------------------------------------------------------------------
// AC-M3.2-14: table_scorer_failure_shape_bundle_verifiable
// ---------------------------------------------------------------------------

#[test]
fn table_scorer_failure_shape_bundle_verifiable() {
    // Verifier-surface test: simulate the bundle shape M2 evidence preservation
    // produces when a TableScorer panics or returns wrong arity.
    // scorer.json exists, at least one expansion recorded, but all candidate
    // score sources are "unavailable" (no ModelDigest) and termination is
    // scorer_contract_violation.
    let regime = regime_truncation();
    let scorer_input = build_scorer_for_regime(&regime);
    let bundle = run_search(&regime.world, &regime.policy, &scorer_input).unwrap();

    let modified_bundle = rebuild_with_modified_graph(&bundle, |graph_json| {
        // Change termination to scorer failure.
        graph_json["metadata"]["termination_reason"] =
            serde_json::json!({"actual": 0, "expected": 1, "type": "scorer_contract_violation"});

        // Replace all candidate score sources with "unavailable" and outcomes
        // with "not_evaluated", matching the shapes from score_source_to_json
        // and outcome_to_json in search/src/graph.rs.
        if let Some(expansions) = graph_json["expansions"].as_array_mut() {
            for expansion in expansions {
                if let Some(candidates) = expansion["candidates"].as_array_mut() {
                    for candidate in candidates {
                        candidate["score"]["bonus"] = serde_json::json!(0);
                        candidate["score"]["source"] = serde_json::json!("unavailable");
                        candidate["outcome"] = serde_json::json!({"type": "not_evaluated"});
                    }
                }
            }
        }
    });

    // Bundle has scorer.json, has expansions, but no ModelDigest — must verify
    // because termination is scorer-failure.
    assert!(
        modified_bundle.artifacts.contains_key("scorer.json"),
        "scorer.json must still be present"
    );

    // Confirm there are expansions (this is not the zero-expansion case).
    let graph: serde_json::Value = serde_json::from_slice(
        &modified_bundle
            .artifacts
            .get("search_graph.json")
            .unwrap()
            .content,
    )
    .unwrap();
    let total_expansions = graph["metadata"]["total_expansions"].as_u64().unwrap();
    assert!(
        total_expansions > 0,
        "must have expansions to exercise the scorer-failure path"
    );

    verify_bundle(&modified_bundle).unwrap();
}
