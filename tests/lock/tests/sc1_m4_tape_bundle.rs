//! SC-001 M4 lock tests: tape as verified bundle artifact.
//!
//! Each test targets a specific acceptance criterion from the M4 milestone.
//! Tests prove that the tape verification pipeline is fail-closed: every
//! semantic mismatch is caught at the correct error variant.
//!
//! **Critical invariant**: tamper tests must *rebuild* the bundle after
//! modifying tape bytes. This recomputes content hash, manifest, and digest
//! basis so that `ContentHashMismatch` does not fire first. Only then do
//! tape-specific verification checks have an opportunity to trigger.

use lock_tests::bundle_test_helpers::{
    rebuild_with_modified_graph_and_report, rebuild_with_modified_tape,
    rebuild_without_artifact,
};
use sterling_harness::bundle::{
    verify_bundle, verify_bundle_with_profile, BundleVerifyError, VerificationProfile,
    ArtifactBundleV1,
};
use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_search::policy::SearchPolicyV1;

/// Produce a search bundle via `run_search(RomeMiniSearch)`.
fn search_bundle() -> ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("search run failed")
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-IN-BUNDLE
// ---------------------------------------------------------------------------

#[test]
fn tape_artifact_in_bundle() {
    let bundle = search_bundle();
    let tape = bundle
        .artifacts
        .get("search_tape.stap")
        .expect("search_tape.stap must be present after run_search");
    assert!(tape.normative, "search_tape.stap must be normative");
    assert!(!tape.content.is_empty(), "tape content must not be empty");
    // Tape starts with STAP magic.
    assert_eq!(&tape.content[..4], b"STAP", "tape must start with STAP magic");
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-DIGEST-IN-REPORT
// ---------------------------------------------------------------------------

#[test]
fn tape_digest_in_report() {
    let bundle = search_bundle();
    let report_art = bundle.artifacts.get("verification_report.json").unwrap();
    let report: serde_json::Value = serde_json::from_slice(&report_art.content).unwrap();

    let tape_digest = report
        .get("tape_digest")
        .and_then(|v| v.as_str())
        .expect("report must contain tape_digest field");

    let tape_art = bundle.artifacts.get("search_tape.stap").unwrap();
    assert_eq!(
        tape_digest,
        tape_art.content_hash.as_str(),
        "tape_digest in report must equal tape artifact content_hash"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-CHAIN-VERIFIED
// ---------------------------------------------------------------------------

#[test]
fn tape_chain_verified_on_clean_bundle() {
    let bundle = search_bundle();
    // Base profile with tape present: parses tape, verifies chain hash internally.
    verify_bundle(&bundle).unwrap();
    // Cert profile: same + graph equivalence.
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-HEADER-BINDING
// ---------------------------------------------------------------------------

#[test]
fn tape_header_binding_coherence() {
    let bundle = search_bundle();

    // Extract tape header fields to verify they match authoritative artifacts.
    let tape_art = bundle.artifacts.get("search_tape.stap").unwrap();
    let tape = sterling_search::tape_reader::read_tape(&tape_art.content).unwrap();
    let header = &tape.header.json;

    // Graph metadata is the authoritative source for several fields.
    let graph_art = bundle.artifacts.get("search_graph.json").unwrap();
    let graph: serde_json::Value = serde_json::from_slice(&graph_art.content).unwrap();
    let metadata = &graph["metadata"];

    // world_id
    assert_eq!(
        header["world_id"].as_str().unwrap(),
        metadata["world_id"].as_str().unwrap(),
        "tape world_id must match graph metadata"
    );

    // registry_digest
    assert_eq!(
        header["registry_digest"].as_str().unwrap(),
        metadata["registry_digest"].as_str().unwrap(),
        "tape registry_digest must match graph metadata"
    );

    // search_policy_digest
    assert_eq!(
        header["search_policy_digest"].as_str().unwrap(),
        metadata["search_policy_digest"].as_str().unwrap(),
        "tape search_policy_digest must match graph metadata"
    );

    // root_state_fingerprint
    assert_eq!(
        header["root_state_fingerprint"].as_str().unwrap(),
        metadata["root_state_fingerprint"].as_str().unwrap(),
        "tape root_state_fingerprint must match graph metadata"
    );

    // policy_snapshot_digest: authoritative source is policy_snapshot.json content_hash (raw hex).
    let policy_art = bundle.artifacts.get("policy_snapshot.json").unwrap();
    assert_eq!(
        header["policy_snapshot_digest"].as_str().unwrap(),
        policy_art.content_hash.hex_digest(),
        "tape policy_snapshot_digest must match policy artifact content_hash hex"
    );

    // Uniform mode: no scorer_digest in header, no scorer.json in bundle.
    assert!(
        header.get("scorer_digest").is_none()
            || header["scorer_digest"].is_null(),
        "uniform-mode tape must not have scorer_digest"
    );
    assert!(
        bundle.artifacts.get("scorer.json").is_none(),
        "uniform-mode bundle must not have scorer.json"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-GRAPH-EQUIVALENCE
// ---------------------------------------------------------------------------

#[test]
fn tape_graph_equivalence_cert_pass() {
    let bundle = search_bundle();
    // Cert profile verifies tape→graph canonical equivalence.
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-TAMPER-BODY-REJECTED
// ---------------------------------------------------------------------------

#[test]
fn tape_tamper_body_rejected() {
    let bundle = search_bundle();

    // Tamper: flip a byte in a record body (after the header).
    // The header ends at offset 10 + header_len. We flip a byte well past that.
    let tampered = rebuild_with_modified_tape(&bundle, |bytes| {
        let mut b = bytes.to_vec();
        // Header length is at bytes 6..10 (u32le).
        let header_len =
            u32::from_le_bytes([b[6], b[7], b[8], b[9]]) as usize;
        let first_record_offset = 10 + header_len;
        // Flip a byte in the first record area (if long enough).
        if b.len() > first_record_offset + 10 {
            b[first_record_offset + 5] ^= 0xFF;
        }
        b
    });

    // Base profile: tape present → parsed → chain hash fails → TapeParseFailed.
    let err = verify_bundle(&tampered).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::TapeParseFailed { .. }),
        "expected TapeParseFailed from tampered record body, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-TAMPER-HEADER-REJECTED
// ---------------------------------------------------------------------------

#[test]
fn tape_tamper_header_world_id_rejected() {
    let bundle = search_bundle();

    // Patch BOTH graph metadata and report world_id together so the
    // graph↔report binding (Step 12) passes. The tape header still has the
    // original world_id, creating a mismatch at Step 16d.
    let tampered = rebuild_with_modified_graph_and_report(
        &bundle,
        |graph| {
            graph["metadata"]["world_id"] = serde_json::json!("tampered_world_id");
        },
        |report| {
            report["world_id"] = serde_json::json!("tampered_world_id");
        },
    );

    let err = verify_bundle(&tampered).unwrap_err();
    assert!(
        matches!(
            err,
            BundleVerifyError::TapeHeaderBindingMismatch { field: "world_id", .. }
        ),
        "expected TapeHeaderBindingMismatch for world_id, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-TAMPER-HEADER-POLICY-BINDING
// ---------------------------------------------------------------------------

#[test]
fn tape_tamper_header_policy_snapshot_rejected() {
    use sterling_harness::bundle::{build_bundle, DOMAIN_BUNDLE_ARTIFACT};
    use sterling_kernel::proof::canon::canonical_json_bytes;
    use sterling_kernel::proof::hash::canonical_hash;

    let bundle = search_bundle();

    // Strategy: modify policy_snapshot.json content (semantically valid but
    // different bytes), update report's policy_digest to match, then rebuild.
    // The tape header still has the original policy_snapshot_digest (raw hex
    // of the old content_hash), creating a mismatch at Step 16d.
    let policy_art = bundle.artifacts.get("policy_snapshot.json").unwrap();
    let mut policy_json: serde_json::Value =
        serde_json::from_slice(&policy_art.content).unwrap();
    policy_json["_tamper"] = serde_json::json!(true);
    let modified_policy_bytes = canonical_json_bytes(&policy_json).unwrap();
    let new_policy_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_policy_bytes);

    // Update report's policy_digest so Step 8 (report ↔ policy artifact) passes.
    let report_art = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_art.content).unwrap();
    report_json["policy_digest"] = serde_json::json!(new_policy_hash.as_str());
    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    // Also update graph metadata policy_snapshot_digest so Step 12 passes.
    let graph_art = bundle.artifacts.get("search_graph.json").unwrap();
    let mut graph_json: serde_json::Value =
        serde_json::from_slice(&graph_art.content).unwrap();
    graph_json["metadata"]["policy_snapshot_digest"] =
        serde_json::json!(new_policy_hash.hex_digest());

    let modified_graph_bytes = canonical_json_bytes(&graph_json).unwrap();
    let new_graph_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_graph_bytes);

    // Patch report's search_graph_digest to match modified graph.
    let mut report_json2: serde_json::Value =
        serde_json::from_slice(&modified_report_bytes).unwrap();
    report_json2["search_graph_digest"] = serde_json::json!(new_graph_hash.as_str());
    let final_report_bytes = canonical_json_bytes(&report_json2).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "policy_snapshot.json" {
                (a.name.clone(), modified_policy_bytes.clone(), a.normative)
            } else if a.name == "verification_report.json" {
                (a.name.clone(), final_report_bytes.clone(), a.normative)
            } else if a.name == "search_graph.json" {
                (a.name.clone(), modified_graph_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let tampered = build_bundle(artifacts).unwrap();

    // Tape header policy_snapshot_digest (old hex) != new policy artifact hex.
    let err = verify_bundle(&tampered).unwrap_err();
    assert!(
        matches!(
            err,
            BundleVerifyError::TapeHeaderBindingMismatch {
                field: "policy_snapshot_digest",
                ..
            }
        ),
        "expected TapeHeaderBindingMismatch for policy_snapshot_digest, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-CERT-REQUIRES-TAPE
// ---------------------------------------------------------------------------

#[test]
fn cert_requires_tape() {
    let bundle = search_bundle();

    // Remove tape from bundle.
    let without_tape = rebuild_without_artifact(&bundle, "search_tape.stap");

    // Base: tape optional → passes (tape absent is fine).
    verify_bundle_with_profile(&without_tape, VerificationProfile::Base).unwrap();

    // Cert: tape required → fails.
    let err =
        verify_bundle_with_profile(&without_tape, VerificationProfile::Cert).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::TapeMissing),
        "expected TapeMissing under Cert profile, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-BASE-ACCEPTS-NO-TAPE
// ---------------------------------------------------------------------------

#[test]
fn base_accepts_no_tape() {
    let bundle = search_bundle();

    // Remove tape from bundle.
    let without_tape = rebuild_without_artifact(&bundle, "search_tape.stap");

    // Base profile: tape is optional, no error.
    verify_bundle_with_profile(&without_tape, VerificationProfile::Base).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M4-BUNDLE-PERSISTENCE-WITH-TAPE
// ---------------------------------------------------------------------------

#[test]
fn bundle_persistence_with_tape() {
    use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};

    let bundle = search_bundle();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // Tape artifact must survive round-trip.
    let tape = loaded
        .artifacts
        .get("search_tape.stap")
        .expect("tape must survive persistence round-trip");
    assert_eq!(
        tape.content,
        bundle.artifacts.get("search_tape.stap").unwrap().content,
        "tape bytes must be identical after round-trip"
    );

    // Full verification passes on loaded bundle.
    verify_bundle(&loaded).unwrap();
    verify_bundle_with_profile(&loaded, VerificationProfile::Cert).unwrap();
    verify_bundle_dir(dir.path()).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-DETERMINISM-N10
// ---------------------------------------------------------------------------

#[test]
fn tape_determinism_n10() {
    let policy = SearchPolicyV1::default();
    let mut tape_bytes: Option<Vec<u8>> = None;

    for i in 0..10 {
        let bundle =
            run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();
        let tape = bundle.artifacts.get("search_tape.stap").unwrap();

        if let Some(ref expected) = tape_bytes {
            assert_eq!(
                &tape.content, expected,
                "tape bytes diverged on run {i}"
            );
        } else {
            tape_bytes = Some(tape.content.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// SC1-M4-ARTIFACT-COUNT-UNIFORM
// ---------------------------------------------------------------------------

#[test]
fn artifact_count_uniform() {
    let bundle = search_bundle();
    assert_eq!(
        bundle.artifacts.len(),
        6,
        "uniform search bundle must have 6 artifacts: fixture, compilation_manifest, \
         policy_snapshot, search_graph, verification_report, search_tape"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-ARTIFACT-COUNT-TABLE
// ---------------------------------------------------------------------------

#[test]
fn artifact_count_table() {
    use std::collections::BTreeMap;
    use sterling_harness::runner::build_table_scorer_input;
    use sterling_harness::worlds::slot_lattice_regimes::regime_truncation;
    use sterling_kernel::carrier::bytestate::ByteStateV1;
    use sterling_search::contract::SearchWorldV1;

    let regime = regime_truncation();
    let root = ByteStateV1::new(1, 10);
    let registry = regime.world.registry().unwrap();
    let mut candidates = regime.world.enumerate_candidates(&root, &registry);
    candidates.sort();

    #[allow(clippy::cast_possible_truncation)]
    if candidates.len() as u64 > regime.policy.max_candidates_per_node {
        candidates.truncate(regime.policy.max_candidates_per_node as usize);
    }

    let last = candidates.last().unwrap();
    let mut table = BTreeMap::new();
    table.insert(last.canonical_hash().as_str().to_string(), 100_i64);
    let scorer_input = build_table_scorer_input(table).unwrap();

    let bundle =
        run_search(&regime.world, &regime.policy, &scorer_input).unwrap();
    assert_eq!(
        bundle.artifacts.len(),
        7,
        "table scorer bundle must have 7 artifacts: fixture, compilation_manifest, \
         policy_snapshot, search_graph, verification_report, search_tape, scorer"
    );
}

// ---------------------------------------------------------------------------
// SC1-M4-TAPE-DIGEST-MISMATCH
// ---------------------------------------------------------------------------

#[test]
fn tape_digest_mismatch_rejected() {
    let bundle = search_bundle();

    // Modify the report's tape_digest to a bogus value, then rebuild the
    // bundle with consistent content hashes for everything except the
    // tape_digest field being wrong.
    let report_art = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_art.content).unwrap();
    report_json["tape_digest"] = serde_json::json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    let modified_report =
        sterling_kernel::proof::canon::canonical_json_bytes(&report_json).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "verification_report.json" {
                (a.name.clone(), modified_report.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    let tampered = sterling_harness::bundle::build_bundle(artifacts).unwrap();

    let err = verify_bundle(&tampered).unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::TapeDigestMismatch { .. }),
        "expected TapeDigestMismatch, got {err:?}"
    );
}

// ===========================================================================
// Table-mode tape verification tests
// ===========================================================================

/// Build a table-scorer bundle using `regime_truncation`.
fn table_scorer_bundle() -> ArtifactBundleV1 {
    use std::collections::BTreeMap;
    use sterling_harness::runner::build_table_scorer_input;
    use sterling_harness::worlds::slot_lattice_regimes::regime_truncation;
    use sterling_kernel::carrier::bytestate::ByteStateV1;
    use sterling_search::contract::SearchWorldV1;

    let regime = regime_truncation();
    let root = ByteStateV1::new(1, 10);
    let registry = regime.world.registry().unwrap();
    let mut candidates = regime.world.enumerate_candidates(&root, &registry);
    candidates.sort();

    #[allow(clippy::cast_possible_truncation)]
    if candidates.len() as u64 > regime.policy.max_candidates_per_node {
        candidates.truncate(regime.policy.max_candidates_per_node as usize);
    }

    let last = candidates.last().unwrap();
    let mut table = BTreeMap::new();
    table.insert(last.canonical_hash().as_str().to_string(), 100_i64);
    let scorer_input = build_table_scorer_input(table).unwrap();

    run_search(&regime.world, &regime.policy, &scorer_input).unwrap()
}

// ---------------------------------------------------------------------------
// SC1-M4-TABLE-SCORER-DIGEST-COHERENCE
// ---------------------------------------------------------------------------

#[test]
fn table_mode_scorer_digest_coherence_base_pass() {
    let bundle = table_scorer_bundle();

    // Tape header must have scorer_digest matching scorer.json content_hash.
    let tape_art = bundle.artifacts.get("search_tape.stap").unwrap();
    let tape = sterling_search::tape_reader::read_tape(&tape_art.content).unwrap();
    let header = &tape.header.json;

    let scorer_art = bundle.artifacts.get("scorer.json").unwrap();
    let tape_scorer = header
        .get("scorer_digest")
        .and_then(|v| v.as_str())
        .expect("table-mode tape must have scorer_digest");
    assert_eq!(
        tape_scorer,
        scorer_art.content_hash.hex_digest(),
        "tape scorer_digest must match scorer.json content_hash hex"
    );

    // Base verification passes.
    verify_bundle(&bundle).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M4-TABLE-CERT-EQUIVALENCE
// ---------------------------------------------------------------------------

#[test]
fn table_mode_cert_equivalence_pass() {
    let bundle = table_scorer_bundle();
    // Cert profile: full tape→graph equivalence on table-scorer bundle.
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert).unwrap();
}

// ---------------------------------------------------------------------------
// SC1-M4-TABLE-BUNDLE-PERSISTENCE-WITH-TAPE
// ---------------------------------------------------------------------------

#[test]
fn table_mode_bundle_persistence_with_tape() {
    use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};

    let bundle = table_scorer_bundle();
    let dir = tempfile::tempdir().unwrap();

    write_bundle_dir(&bundle, dir.path()).unwrap();
    let loaded = read_bundle_dir(dir.path()).unwrap();

    // Tape + scorer survive round-trip.
    assert!(loaded.artifacts.contains_key("search_tape.stap"));
    assert!(loaded.artifacts.contains_key("scorer.json"));
    assert_eq!(loaded.artifacts.len(), 7);

    // Full verification passes including tape on loaded bundle.
    verify_bundle(&loaded).unwrap();
    verify_bundle_with_profile(&loaded, VerificationProfile::Cert).unwrap();
    verify_bundle_dir(dir.path()).unwrap();
}
