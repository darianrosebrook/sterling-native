//! TOOLSCRIPT-001 lock tests: tool transcript system.
//!
//! Covers acceptance criteria:
//! - TOOL-001-TRANSCRIPT-DERIVED: transcript rendered from tape
//! - TOOL-001-TRANSCRIPT-EQUIVALENCE: Cert equivalence render
//! - TOOL-001-CORRIDOR-BINDING: digest in report
//! - TOOL-001-OBLIGATION-GATING: non-tool worlds skip transcript
//! - TOOL-001-COMMITTED-WRITE-SAFETY: Cert trace-order audit
//! - TOOL-001-BACKWARD-COMPAT: existing worlds unaffected
//! - TOOL-001-COHERENT-FORGE: Cert catches forged transcript
//! - TOOL-001-DOUBLE-FINALIZATION: kernel rejects
//! - TOOL-001-ORDERING-PERMUTATIONS: expected behaviors

use sterling_harness::bundle::{
    verify_bundle, verify_bundle_with_profile, ArtifactBundleV1, BundleVerifyError,
    VerificationProfile,
};
use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_harness::worlds::tool_kv_store::ToolKvStore;
use sterling_search::policy::SearchPolicyV1;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn commit_bundle() -> ArtifactBundleV1 {
    let world = ToolKvStore::commit_world();
    let policy = SearchPolicyV1::default();
    run_search(&world, &policy, &ScorerInputV1::Uniform).expect("commit search failed")
}

fn rollback_bundle() -> ArtifactBundleV1 {
    let world = ToolKvStore::rollback_world();
    let policy = SearchPolicyV1::default();
    run_search(&world, &policy, &ScorerInputV1::Uniform).expect("rollback search failed")
}

fn rome_bundle() -> ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("rome search failed")
}

fn parse_json(bundle: &ArtifactBundleV1, name: &str) -> serde_json::Value {
    let art = bundle.artifacts.get(name).unwrap_or_else(|| {
        panic!("{name} must be present in bundle");
    });
    serde_json::from_slice(&art.content).unwrap_or_else(|_| {
        panic!("{name} must be valid JSON");
    })
}

// ---------------------------------------------------------------------------
// TOOL-001-TRANSCRIPT-DERIVED: transcript exists and has correct structure
// ---------------------------------------------------------------------------

#[test]
fn commit_bundle_has_tool_transcript() {
    let bundle = commit_bundle();
    assert!(
        bundle.artifacts.contains_key("tool_transcript.json"),
        "commit bundle must contain tool_transcript.json"
    );
}

#[test]
fn rollback_bundle_has_tool_transcript() {
    let bundle = rollback_bundle();
    assert!(
        bundle.artifacts.contains_key("tool_transcript.json"),
        "rollback bundle must contain tool_transcript.json"
    );
}

#[test]
fn transcript_schema_version() {
    let bundle = commit_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    assert_eq!(transcript["schema_version"], "tool_transcript.v1");
}

#[test]
fn transcript_txn_epoch_is_zero() {
    let bundle = commit_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    assert_eq!(transcript["txn_epoch"], 0);
}

#[test]
fn transcript_world_id_matches_report() {
    let bundle = commit_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    let report = parse_json(&bundle, "verification_report.json");
    assert_eq!(transcript["world_id"], report["world_id"]);
}

#[test]
fn transcript_entry_count_matches_entries() {
    let bundle = commit_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    let entries = transcript["entries"].as_array().expect("entries array");
    let count = transcript["entry_count"].as_u64().expect("entry_count");
    assert_eq!(
        count,
        entries.len() as u64,
        "entry_count must match entries array length"
    );
}

#[test]
fn commit_transcript_has_stage_and_commit_entries() {
    let bundle = commit_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    let entries = transcript["entries"].as_array().expect("entries array");

    // Commit path should have at least STAGE and COMMIT entries.
    assert!(entries.len() >= 2, "commit path needs at least 2 tool entries");

    let operators: Vec<&str> = entries
        .iter()
        .map(|e| e["operator"].as_str().unwrap())
        .collect();

    assert!(operators.contains(&"STAGE"), "must have STAGE entry");
    assert!(operators.contains(&"COMMIT"), "must have COMMIT entry");
}

#[test]
fn rollback_transcript_has_stage_and_rollback_entries() {
    let bundle = rollback_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    let entries = transcript["entries"].as_array().expect("entries array");

    assert!(
        entries.len() >= 2,
        "rollback path needs at least 2 tool entries"
    );

    let operators: Vec<&str> = entries
        .iter()
        .map(|e| e["operator"].as_str().unwrap())
        .collect();

    assert!(operators.contains(&"STAGE"), "must have STAGE entry");
    assert!(operators.contains(&"ROLLBACK"), "must have ROLLBACK entry");
}

#[test]
fn transcript_step_indices_monotonic() {
    let bundle = commit_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    let entries = transcript["entries"].as_array().expect("entries array");

    let mut prev = None;
    for entry in entries {
        let step = entry["step_index"].as_u64().expect("step_index");
        if let Some(p) = prev {
            assert!(step >= p, "step_index {step} must be >= prev {p}");
        }
        prev = Some(step);
    }
}

#[test]
fn transcript_entries_have_all_fields() {
    let bundle = commit_bundle();
    let transcript = parse_json(&bundle, "tool_transcript.json");
    let entries = transcript["entries"].as_array().expect("entries array");

    for entry in entries {
        assert!(entry.get("args").is_some(), "entry missing args");
        assert!(entry.get("op_code").is_some(), "entry missing op_code");
        assert!(entry.get("operator").is_some(), "entry missing operator");
        assert_eq!(entry["outcome"], "applied");
        assert!(entry.get("step_index").is_some(), "entry missing step_index");
    }
}

// ---------------------------------------------------------------------------
// TOOL-001-CORRIDOR-BINDING: digest in report
// ---------------------------------------------------------------------------

#[test]
fn report_has_tool_transcript_digest() {
    let bundle = commit_bundle();
    let report = parse_json(&bundle, "verification_report.json");
    assert!(
        report.get("tool_transcript_digest").is_some(),
        "report must contain tool_transcript_digest for tool worlds"
    );
}

#[test]
fn tool_transcript_digest_matches_artifact() {
    let bundle = commit_bundle();
    let report = parse_json(&bundle, "verification_report.json");
    let digest = report["tool_transcript_digest"]
        .as_str()
        .expect("tool_transcript_digest");

    let artifact = bundle
        .artifacts
        .get("tool_transcript.json")
        .expect("transcript artifact");
    assert_eq!(
        digest,
        artifact.content_hash.as_str(),
        "report digest must match artifact content_hash"
    );
}

// ---------------------------------------------------------------------------
// TOOL-001-TRANSCRIPT-EQUIVALENCE: Cert verification passes
// ---------------------------------------------------------------------------

#[test]
fn commit_bundle_verifies_base() {
    let bundle = commit_bundle();
    verify_bundle(&bundle).expect("commit bundle must verify (Base)");
}

#[test]
fn commit_bundle_verifies_cert() {
    let bundle = commit_bundle();
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert)
        .expect("commit bundle must verify (Cert)");
}

#[test]
fn rollback_bundle_verifies_base() {
    let bundle = rollback_bundle();
    verify_bundle(&bundle).expect("rollback bundle must verify (Base)");
}

#[test]
fn rollback_bundle_verifies_cert() {
    let bundle = rollback_bundle();
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert)
        .expect("rollback bundle must verify (Cert)");
}

// ---------------------------------------------------------------------------
// TOOL-001-OBLIGATION-GATING: non-tool worlds skip transcript
// ---------------------------------------------------------------------------

#[test]
fn rome_bundle_has_no_transcript() {
    let bundle = rome_bundle();
    assert!(
        !bundle.artifacts.contains_key("tool_transcript.json"),
        "non-tool world must not have tool_transcript.json"
    );
}

#[test]
fn rome_report_has_no_transcript_digest() {
    let bundle = rome_bundle();
    let report = parse_json(&bundle, "verification_report.json");
    assert!(
        report.get("tool_transcript_digest").is_none(),
        "non-tool world report must not have tool_transcript_digest"
    );
}

// ---------------------------------------------------------------------------
// TOOL-001-BACKWARD-COMPAT: existing worlds unaffected
// ---------------------------------------------------------------------------

#[test]
fn rome_bundle_verifies_base() {
    let bundle = rome_bundle();
    verify_bundle(&bundle).expect("rome bundle must still verify (Base)");
}

#[test]
fn rome_bundle_verifies_cert() {
    let bundle = rome_bundle();
    verify_bundle_with_profile(&bundle, VerificationProfile::Cert)
        .expect("rome bundle must still verify (Cert)");
}

#[test]
fn rome_bundle_artifact_count_unchanged() {
    let bundle = rome_bundle();
    // 8 artifacts (no tool_transcript — unchanged relative to non-tool worlds)
    assert_eq!(bundle.artifacts.len(), 8);
}

// ---------------------------------------------------------------------------
// TOOL-001-COHERENT-FORGE: Cert catches forged transcript
// ---------------------------------------------------------------------------

#[test]
fn forged_transcript_fails_cert_equivalence() {
    use sterling_kernel::proof::hash::{canonical_hash, HashDomain};

    let mut bundle = commit_bundle();

    // Tamper the transcript content (add a field to make it different).
    let transcript_art = bundle
        .artifacts
        .get("tool_transcript.json")
        .expect("transcript")
        .clone();
    let mut transcript: serde_json::Value =
        serde_json::from_slice(&transcript_art.content).unwrap();
    transcript["tampered"] = serde_json::json!(true);
    let tampered_bytes =
        sterling_kernel::proof::canon::canonical_json_bytes(&transcript).unwrap();

    // Compute new content hash for the tampered transcript.
    let tampered_hash =
        canonical_hash(HashDomain::BundleArtifact, &tampered_bytes);

    // Update the artifact in the bundle.
    let art = bundle.artifacts.get_mut("tool_transcript.json").unwrap();
    art.content = tampered_bytes;
    art.content_hash = tampered_hash.clone();

    // Update report digest to match (coherent forgery).
    let report_art = bundle
        .artifacts
        .get("verification_report.json")
        .unwrap()
        .clone();
    let mut report: serde_json::Value =
        serde_json::from_slice(&report_art.content).unwrap();
    report["tool_transcript_digest"] = serde_json::json!(tampered_hash.as_str());
    let new_report_bytes =
        sterling_kernel::proof::canon::canonical_json_bytes(&report).unwrap();
    let new_report_hash =
        canonical_hash(HashDomain::BundleArtifact, &new_report_bytes);

    let report_art = bundle
        .artifacts
        .get_mut("verification_report.json")
        .unwrap();
    report_art.content = new_report_bytes;
    report_art.content_hash = new_report_hash;

    // Rebuild manifest + digest_basis + digest for the tampered bundle.
    // (Without this, Step 1-6 catches the tamper before we get to Step 19.)
    rebuild_bundle_integrity(&mut bundle);

    // Base may pass (digest binding is coherent).
    // Cert MUST fail with equivalence mismatch.
    let result = verify_bundle_with_profile(&bundle, VerificationProfile::Cert);
    assert!(
        result.is_err(),
        "coherently forged transcript must fail Cert"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, BundleVerifyError::ToolTranscriptEquivalenceMismatch { .. }),
        "expected ToolTranscriptEquivalenceMismatch, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// TOOL-001-DOUBLE-FINALIZATION
// ---------------------------------------------------------------------------

#[test]
fn double_commit_rejected_by_kernel() {
    use sterling_kernel::carrier::compile::compile;
    use sterling_kernel::operators::apply::{
        apply, commit_args, stage_args, OP_COMMIT, OP_STAGE,
    };
    use sterling_kernel::operators::operator_registry::kernel_operator_registry;

    let world = ToolKvStore::commit_world();
    let payload = world.encode_payload().unwrap();
    let schema = world.schema_descriptor();
    let registry = world.registry().unwrap();
    let op_reg = kernel_operator_registry();
    let compilation = compile(&payload, &schema, &registry).unwrap();
    let mut state = compilation.state;

    // Stage.
    let (s1, _) = apply(
        &state,
        OP_STAGE,
        &stage_args(1, 0, sterling_kernel::carrier::code32::Code32::new(2, 1, 0)),
        &op_reg,
    )
    .unwrap();
    state = s1;

    // First commit.
    let (s2, _) = apply(&state, OP_COMMIT, &commit_args(1), &op_reg).unwrap();
    state = s2;

    // Second commit: must fail.
    let result = apply(&state, OP_COMMIT, &commit_args(1), &op_reg);
    assert!(result.is_err(), "double commit must be rejected");
}

#[test]
fn commit_after_rollback_rejected_by_kernel() {
    use sterling_kernel::carrier::compile::compile;
    use sterling_kernel::operators::apply::{
        apply, commit_args, rollback_args, stage_args, OP_COMMIT, OP_ROLLBACK, OP_STAGE,
    };
    use sterling_kernel::operators::operator_registry::kernel_operator_registry;

    let world = ToolKvStore::commit_world();
    let payload = world.encode_payload().unwrap();
    let schema = world.schema_descriptor();
    let registry = world.registry().unwrap();
    let op_reg = kernel_operator_registry();
    let compilation = compile(&payload, &schema, &registry).unwrap();
    let mut state = compilation.state;

    // Stage.
    let (s1, _) = apply(
        &state,
        OP_STAGE,
        &stage_args(1, 0, sterling_kernel::carrier::code32::Code32::new(2, 1, 0)),
        &op_reg,
    )
    .unwrap();
    state = s1;

    // Rollback.
    let (s2, _) = apply(&state, OP_ROLLBACK, &rollback_args(1), &op_reg).unwrap();
    state = s2;

    // Commit after rollback: must fail (txn_marker already written).
    let result = apply(&state, OP_COMMIT, &commit_args(1), &op_reg);
    assert!(result.is_err(), "commit after rollback must be rejected");
}

// ---------------------------------------------------------------------------
// Search determinism
// ---------------------------------------------------------------------------

#[test]
fn commit_search_is_deterministic() {
    let b1 = commit_bundle();
    let b2 = commit_bundle();

    let g1 = &b1.artifacts["search_graph.json"].content;
    let g2 = &b2.artifacts["search_graph.json"].content;
    assert_eq!(g1, g2, "search graph must be deterministic");

    let t1 = &b1.artifacts["tool_transcript.json"].content;
    let t2 = &b2.artifacts["tool_transcript.json"].content;
    assert_eq!(t1, t2, "tool transcript must be deterministic");
}

#[test]
fn rollback_search_is_deterministic() {
    let b1 = rollback_bundle();
    let b2 = rollback_bundle();

    let g1 = &b1.artifacts["search_graph.json"].content;
    let g2 = &b2.artifacts["search_graph.json"].content;
    assert_eq!(g1, g2, "search graph must be deterministic");

    let t1 = &b1.artifacts["tool_transcript.json"].content;
    let t2 = &b2.artifacts["tool_transcript.json"].content;
    assert_eq!(t1, t2, "tool transcript must be deterministic");
}

// ---------------------------------------------------------------------------
// Artifact count for tool worlds
// ---------------------------------------------------------------------------

#[test]
fn commit_bundle_artifact_count() {
    let bundle = commit_bundle();
    // 9 artifacts: fixture, compilation_manifest, concept_registry,
    //              policy_snapshot, operator_registry, search_graph,
    //              search_tape, verification_report, tool_transcript
    assert_eq!(bundle.artifacts.len(), 9);
}

// ---------------------------------------------------------------------------
// Helper: rebuild bundle manifest/digest after tampering
// ---------------------------------------------------------------------------

fn rebuild_bundle_integrity(bundle: &mut ArtifactBundleV1) {
    use sterling_kernel::proof::canon::canonical_json_bytes;
    use sterling_kernel::proof::hash::{canonical_hash, HashDomain};

    // Recompute manifest (must match compute_manifest_bytes format exactly).
    let manifest_arts: Vec<serde_json::Value> = bundle
        .artifacts
        .values()
        .map(|a| {
            serde_json::json!({
                "content_hash": a.content_hash.as_str(),
                "name": a.name,
                "normative": a.normative,
            })
        })
        .collect();
    let manifest_json = serde_json::json!({
        "artifacts": manifest_arts,
        "schema_version": "bundle.v1",
    });
    bundle.manifest = canonical_json_bytes(&manifest_json).unwrap();

    // Recompute digest_basis (must match compute_digest_basis_bytes format).
    let normative_arts: Vec<serde_json::Value> = bundle
        .artifacts
        .values()
        .filter(|a| a.normative)
        .map(|a| {
            serde_json::json!({
                "content_hash": a.content_hash.as_str(),
                "name": a.name,
            })
        })
        .collect();
    let basis_json = serde_json::json!({
        "artifacts": normative_arts,
        "schema_version": "bundle_digest_basis.v1",
    });
    bundle.digest_basis = canonical_json_bytes(&basis_json).unwrap();

    // Recompute digest.
    bundle.digest = canonical_hash(HashDomain::BundleDigest, &bundle.digest_basis);
}
