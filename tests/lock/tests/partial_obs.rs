//! Lock tests for partial observability world (POBS-001 M5).
//!
//! Tests cover:
//! - Bundle generation + Base/Cert verification
//! - Epistemic transcript artifact presence and structural integrity
//! - Winning-path replay (Cert) with invariant checker
//! - In-process determinism (N=10)
//! - Negative: forged transcript fails Cert equivalence
//! - Negative: belief monotonicity and feedback correctness visible in transcript
//! - Backward compatibility: existing worlds unaffected

use sterling_harness::bundle::{verify_bundle, verify_bundle_with_profile, VerificationProfile};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::partial_obs::PartialObsWorld;
use sterling_kernel::carrier::code32::Code32;
use sterling_search::policy::SearchPolicyV1;

/// Run search on the default partial obs world and return the bundle.
fn partial_obs_bundle() -> sterling_harness::bundle::ArtifactBundleV1 {
    let world = PartialObsWorld::default_world();
    let policy = SearchPolicyV1::default();
    run_search(&world, &policy, &ScorerInputV1::Uniform).expect("partial obs search failed")
}

#[test]
fn partial_obs_bundle_verifies_base() {
    let bundle = partial_obs_bundle();
    verify_bundle(&bundle).expect("partial obs bundle must pass Base verification");
}

#[test]
fn partial_obs_bundle_verifies_cert() {
    let bundle = partial_obs_bundle();
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert)
        .expect("partial obs bundle must pass Cert verification");
}

#[test]
fn partial_obs_bundle_has_epistemic_transcript() {
    let bundle = partial_obs_bundle();
    assert!(
        bundle.artifacts.contains_key("epistemic_transcript.json"),
        "partial obs bundle must contain epistemic_transcript.json"
    );
}

#[test]
fn epistemic_transcript_digest_matches_artifact() {
    let bundle = partial_obs_bundle();
    let report_art = bundle
        .artifacts
        .get("verification_report.json")
        .expect("report must exist");
    let report: serde_json::Value =
        serde_json::from_slice(&report_art.content).expect("report is JSON");
    let digest = report
        .get("epistemic_transcript_digest")
        .and_then(|v| v.as_str())
        .expect("report must have epistemic_transcript_digest");

    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    assert_eq!(
        transcript_art.content_hash.as_str(),
        digest,
        "transcript digest in report must match artifact content_hash"
    );
}

#[test]
fn epistemic_transcript_entry_count_matches() {
    let bundle = partial_obs_bundle();
    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    let transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).expect("transcript is JSON");

    let declared = transcript["entry_count"].as_u64().expect("entry_count");
    let actual = transcript["entries"]
        .as_array()
        .expect("entries")
        .len() as u64;
    assert_eq!(declared, actual, "entry_count must match entries array length");
}

#[test]
fn epistemic_transcript_schema_version() {
    let bundle = partial_obs_bundle();
    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    let transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).expect("transcript is JSON");
    assert_eq!(
        transcript["schema_version"].as_str(),
        Some("epistemic_transcript.v1")
    );
}

#[test]
fn epistemic_transcript_world_id_matches() {
    let bundle = partial_obs_bundle();
    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    let transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).expect("transcript is JSON");

    let report_art = bundle
        .artifacts
        .get("verification_report.json")
        .expect("report must exist");
    let report: serde_json::Value =
        serde_json::from_slice(&report_art.content).expect("report is JSON");

    assert_eq!(
        transcript["world_id"].as_str(),
        report["world_id"].as_str(),
        "transcript world_id must match report world_id"
    );
}

#[test]
fn epistemic_transcript_has_solved_and_belief() {
    let bundle = partial_obs_bundle();
    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    let transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).expect("transcript is JSON");

    assert_eq!(transcript["solved"].as_bool(), Some(true), "must be solved");
    assert_eq!(transcript["final_belief_size"].as_u64(), Some(1), "belief must converge to 1");
    assert!(transcript["probe_count"].as_u64().unwrap() >= 1, "must have at least 1 probe");
}

#[test]
fn epistemic_transcript_belief_monotonicity() {
    let bundle = partial_obs_bundle();
    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    let transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).expect("transcript is JSON");

    let entries = transcript["entries"].as_array().expect("entries");

    // Track belief sizes through FEEDBACK entries.
    let mut prev_belief = 9usize; // V^K = 9 initial candidates
    let mut had_strict_decrease = false;

    #[allow(clippy::cast_possible_truncation)]
    for entry in entries {
        if entry["operator"].as_str() == Some("FEEDBACK") {
            let before = entry["belief_size_before"]
                .as_u64()
                .expect("belief_size_before") as usize;
            let after = entry["belief_size_after"]
                .as_u64()
                .expect("belief_size_after") as usize;

            assert!(
                after <= before,
                "belief must be non-increasing: {before} -> {after}"
            );
            assert!(
                after <= prev_belief,
                "belief must be non-increasing across steps: {prev_belief} -> {after}"
            );
            if after < prev_belief {
                had_strict_decrease = true;
            }
            prev_belief = after;
        }
    }

    assert!(had_strict_decrease, "at least one feedback must strictly decrease belief");
}

#[test]
fn epistemic_transcript_has_all_operator_types() {
    let bundle = partial_obs_bundle();
    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    let transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).expect("transcript is JSON");

    let entries = transcript["entries"].as_array().expect("entries");
    let operators: Vec<&str> = entries
        .iter()
        .filter_map(|e| e["operator"].as_str())
        .collect();

    assert!(operators.contains(&"GUESS"), "must have GUESS entries");
    assert!(operators.contains(&"FEEDBACK"), "must have FEEDBACK entries");
    assert!(operators.contains(&"DECLARE"), "must have DECLARE entries");
}

#[test]
fn partial_obs_search_is_deterministic() {
    let first = partial_obs_bundle();
    for i in 1..=10 {
        let bundle = partial_obs_bundle();
        assert_eq!(
            first.digest.as_str(),
            bundle.digest.as_str(),
            "run {i}: bundle digest must be identical"
        );
    }
}

#[test]
fn partial_obs_bundle_artifact_count() {
    let bundle = partial_obs_bundle();
    // Standard 8 + epistemic_transcript.json = 9 artifacts.
    assert_eq!(
        bundle.artifacts.len(),
        9,
        "partial obs bundle should have 9 artifacts (standard 8 + epistemic transcript)"
    );
}

#[test]
fn different_truth_produces_different_bundle() {
    let world1 = PartialObsWorld::default_world(); // [c0, c1]
    let world2 = PartialObsWorld::new([Code32::new(3, 0, 2), Code32::new(3, 0, 0)]); // [c2, c0]
    let policy = SearchPolicyV1::default();

    let b1 = run_search(&world1, &policy, &ScorerInputV1::Uniform).expect("search 1");
    let b2 = run_search(&world2, &policy, &ScorerInputV1::Uniform).expect("search 2");

    assert_ne!(
        b1.digest.as_str(),
        b2.digest.as_str(),
        "different truths must produce different bundles"
    );

    // Both must verify.
    verify_bundle_with_profile(&b1, VerificationProfile::Cert).expect("b1 cert");
    verify_bundle_with_profile(&b2, VerificationProfile::Cert).expect("b2 cert");
}

#[test]
fn forged_transcript_fails_cert_equivalence() {
    use sterling_harness::bundle::{build_bundle, ArtifactInput, BundleVerifyError};

    let bundle = partial_obs_bundle();

    // Tamper with the transcript: change a field.
    let transcript_art = bundle
        .artifacts
        .get("epistemic_transcript.json")
        .expect("transcript must exist");
    let mut transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).expect("transcript is JSON");
    transcript["probe_count"] = serde_json::json!(999);
    let forged_bytes =
        sterling_kernel::proof::canon::canonical_json_bytes(&transcript).expect("canon");

    // Rebuild bundle with forged transcript.
    let forged_hash = sterling_kernel::proof::hash::canonical_hash(
        sterling_harness::bundle::DOMAIN_BUNDLE_ARTIFACT,
        &forged_bytes,
    );

    let mut artifacts: Vec<ArtifactInput> = Vec::new();
    for (name, art) in &bundle.artifacts {
        if name == "epistemic_transcript.json" {
            artifacts.push(ArtifactInput {
                name: name.clone(),
                content: forged_bytes.clone(),
                normative: true,
                precomputed_hash: Some(forged_hash.clone()),
            });
        } else if name == "verification_report.json" {
            // Update the report's epistemic_transcript_digest to match forged hash.
            let mut report: serde_json::Value =
                serde_json::from_slice(&art.content).expect("report JSON");
            report["epistemic_transcript_digest"] =
                serde_json::json!(forged_hash.as_str());
            let report_bytes =
                sterling_kernel::proof::canon::canonical_json_bytes(&report).expect("canon");
            artifacts.push(ArtifactInput {
                name: name.clone(),
                content: report_bytes,
                normative: true,
                precomputed_hash: None,
            });
        } else {
            artifacts.push(ArtifactInput {
                name: name.clone(),
                content: art.content.clone(),
                normative: art.normative,
                precomputed_hash: None,
            });
        }
    }

    let forged_bundle = build_bundle(artifacts).expect("rebuild");

    // Base should still pass (no equivalence check).
    verify_bundle(&forged_bundle).expect("forged bundle should pass Base");

    // Cert should fail with equivalence mismatch.
    let err = verify_bundle_with_profile(&forged_bundle, VerificationProfile::Cert).unwrap_err();
    assert!(
        matches!(
            err,
            BundleVerifyError::EpistemicTranscriptEquivalenceMismatch { .. }
        ),
        "expected EpistemicTranscriptEquivalenceMismatch, got: {err:?}"
    );
}

#[test]
fn backward_compat_rome_mini_search_still_verifies() {
    use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;

    let policy = SearchPolicyV1::default();
    let bundle =
        run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("rome search");

    // No epistemic transcript expected.
    assert!(
        !bundle.artifacts.contains_key("epistemic_transcript.json"),
        "RomeMiniSearch should not have epistemic transcript"
    );

    verify_bundle(&bundle).expect("Base");
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert).expect("Cert");
}

#[test]
fn backward_compat_tool_kv_store_still_verifies() {
    use sterling_harness::worlds::tool_kv_store::ToolKvStore;

    let world = ToolKvStore::commit_world();
    let policy = SearchPolicyV1::default();
    let bundle = run_search(&world, &policy, &ScorerInputV1::Uniform).expect("kv search");

    // Tool transcript yes, epistemic no.
    assert!(
        bundle.artifacts.contains_key("tool_transcript.json"),
        "ToolKvStore should have tool transcript"
    );
    assert!(
        !bundle.artifacts.contains_key("epistemic_transcript.json"),
        "ToolKvStore should not have epistemic transcript"
    );

    verify_bundle(&bundle).expect("Base");
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert).expect("Cert");
}
