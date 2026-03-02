//! Harness runner: orchestrates kernel APIs to produce an artifact bundle.
//!
//! The runner uses ONLY kernel APIs: `compile`, `apply`, `trace_to_bytes`,
//! `replay_verify`, `payload_hash`, `step_chain`. It does not implement
//! any proof logic itself.
//!
//! # Pipeline
//!
//! ```text
//! build_policy() → enforce_pre_execution()
//!   → encode_payload() → compile() → [apply() × N] → build trace
//!   → trace_to_bytes() → enforce_trace_bytes()
//!   → replay_verify() → hash → build bundle (includes policy_snapshot.json)
//! ```

use crate::bundle::{
    build_bundle, ArtifactBundleV1, ArtifactInput, BundleBuildError, DOMAIN_BUNDLE_ARTIFACT,
    DOMAIN_CODEBOOK_HASH, DOMAIN_HARNESS_FIXTURE,
};
use crate::contract::{WorldHarnessError, WorldHarnessV1};
use crate::policy::{
    build_policy, enforce_artifact_bytes, enforce_pre_execution, enforce_trace_bytes, PolicyConfig,
    PolicyViolation,
};

use std::collections::BTreeMap;

use sterling_search::contract::SearchWorldV1;
use sterling_search::policy::SearchPolicyV1;
use sterling_search::scorer::{TableScorer, ValueScorer};
use sterling_search::search::MetadataBindings;
use sterling_search::tape_reader::read_tape;

use sterling_kernel::carrier::bytetrace::{
    ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceFrameV1, ByteTraceHeaderV1, ByteTraceV1,
    ReplayVerdict, TraceBundleV1,
};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::carrier::trace_writer::trace_to_bytes;
use sterling_kernel::operators::apply::apply;
use sterling_kernel::operators::operator_registry::{kernel_operator_registry, OperatorRegistryV1};
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, ContentHash, HashDomain};
use sterling_kernel::proof::replay::replay_verify;
use sterling_kernel::proof::trace_hash::{payload_hash, step_chain};

/// Error during a harness run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// World harness method failed.
    WorldError(WorldHarnessError),
    /// Kernel compilation failed.
    CompilationFailed { detail: String },
    /// Kernel `apply()` failed.
    ApplyFailed { frame_index: usize, detail: String },
    /// Trace serialization failed.
    TraceWriteFailed { detail: String },
    /// Replay verification structural error.
    ReplayFailed { detail: String },
    /// Replay returned Divergence verdict (runner bug or world bug).
    ReplayDivergence { frame_index: usize, detail: String },
    /// Bundle assembly failed.
    BundleFailed(BundleBuildError),
    /// Trace hashing failed.
    HashFailed { detail: String },
    /// Canonical JSON serialization failed.
    CanonFailed { detail: String },
    /// Policy construction failed.
    PolicyBuildFailed { detail: String },
    /// Policy violation (fail-closed).
    PolicyViolation(PolicyViolation),
}

/// Error during a search harness run.
#[derive(Debug)]
pub enum SearchRunError {
    /// Linear run error (from the underlying harness pipeline).
    RunError(RunError),
    /// Search error (from the search loop).
    SearchError(sterling_search::error::SearchError),
    /// Canonical JSON serialization failed.
    CanonFailed { detail: String },
    /// Bundle assembly failed.
    BundleFailed(BundleBuildError),
    /// Policy construction failed.
    PolicyBuildFailed { detail: String },
    /// `SearchWorldV1::world_id()` and `WorldHarnessV1::world_id()` disagree.
    WorldIdMismatch {
        search_world_id: String,
        harness_world_id: String,
    },
    /// A scorer table key is not a valid `ContentHash` or uses an unsupported algorithm.
    InvalidScorerTableKey { key: String },
}

/// A scorer artifact ready for inclusion in a bundle.
///
/// Built by `build_table_scorer_input()` from a score table.
/// Contains the canonical JSON bytes and their content hash.
#[derive(Debug, Clone)]
pub struct ScorerArtifactV1 {
    /// Canonical JSON bytes of the scorer table.
    pub bytes: Vec<u8>,
    /// `canonical_hash(DOMAIN_BUNDLE_ARTIFACT, bytes)`.
    pub content_hash: ContentHash,
    /// Hex digest portion of `content_hash`.
    pub hex_digest: String,
}

/// Atomic scorer input: scorer behavior + artifact are inseparable.
///
/// Invalid states (e.g., `TableScorer` without artifact) are unrepresentable.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ScorerInputV1 {
    /// Uniform scoring (bonus=0 for all candidates). No scorer artifact.
    Uniform,
    /// Table scoring with a bundled artifact.
    Table {
        /// The scorer implementation.
        scorer: TableScorer,
        /// The artifact for bundle inclusion.
        artifact: ScorerArtifactV1,
    },
}

/// Build a `ScorerInputV1::Table` from a score table.
///
/// Performs canonical JSON serialization and domain hashing once,
/// then wires both the scorer and artifact from the same bytes/hash.
///
/// Every key must be a valid `ContentHash` with the `sha256` algorithm.
/// Invalid or non-`sha256` keys are rejected at build time with
/// [`SearchRunError::InvalidScorerTableKey`].
///
/// # Errors
///
/// Returns [`SearchRunError::InvalidScorerTableKey`] if any key is not a
/// valid `sha256:` content hash.
/// Returns [`SearchRunError::CanonFailed`] if serialization fails.
///
/// # Panics
///
/// Panics if the internal placeholder hash literal is malformed (compile-time invariant).
pub fn build_table_scorer_input(
    table: BTreeMap<String, i64>,
) -> Result<ScorerInputV1, SearchRunError> {
    // Validate all keys are valid sha256 ContentHash values with full-length digest.
    for key in table.keys() {
        match ContentHash::parse(key) {
            Some(hash) if hash.algorithm() == "sha256" && hash.hex_digest().len() == 64 => {}
            _ => {
                return Err(SearchRunError::InvalidScorerTableKey { key: key.clone() });
            }
        }
    }

    // Build a temporary scorer to generate canonical bytes.
    // Use a placeholder digest — we'll replace it after hashing.
    let placeholder = ContentHash::parse(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    )
    .expect("valid placeholder hash");
    let temp_scorer = TableScorer::new(table.clone(), placeholder);
    let bytes = temp_scorer
        .to_canonical_json_bytes()
        .map_err(|e| SearchRunError::CanonFailed {
            detail: format!("{e:?}"),
        })?;

    let content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &bytes);
    let hex_digest = content_hash.hex_digest().to_string();

    let scorer = TableScorer::new(table, content_hash.clone());
    let artifact = ScorerArtifactV1 {
        bytes,
        content_hash,
        hex_digest,
    };

    Ok(ScorerInputV1::Table { scorer, artifact })
}

/// Run a search world through the search pipeline, producing a bundle.
///
/// The world must implement both [`SearchWorldV1`] and [`WorldHarnessV1`].
/// A runtime check verifies that both trait implementations report the same
/// `world_id()`, preventing split-brain wiring.
///
/// Pipeline:
/// 1. `encode_payload()` → `compile()` → root `ByteStateV1`
/// 2. Build search policy + metadata bindings
/// 3. `search_with_tape(root_state, world, policy, scorer)` → `(SearchResult, TapeOutput)`
/// 4. Assemble bundle with `search_graph.json` + `search_tape.stap` (both normative)
///
/// For `ScorerInputV1::Table`, the bundle includes 9 artifacts (adding
/// `scorer.json` as normative). For `ScorerInputV1::Uniform`, the bundle
/// has 8 artifacts.
///
/// # Errors
///
/// Returns [`SearchRunError`] at any pipeline step.
#[allow(clippy::too_many_lines)]
pub fn run_search<W: SearchWorldV1 + WorldHarnessV1>(
    world: &W,
    search_policy: &SearchPolicyV1,
    scorer_input: &ScorerInputV1,
) -> Result<ArtifactBundleV1, SearchRunError> {
    // World ID coherence check: both traits must agree.
    let search_wid = SearchWorldV1::world_id(world);
    let harness_wid = WorldHarnessV1::world_id(world);
    if search_wid != harness_wid {
        return Err(SearchRunError::WorldIdMismatch {
            search_world_id: search_wid.to_string(),
            harness_world_id: harness_wid.to_string(),
        });
    }

    // Phase 1: compile root state.
    let payload_bytes = world
        .encode_payload()
        .map_err(|e| SearchRunError::RunError(RunError::WorldError(e)))?;
    let schema = world.schema_descriptor();
    let concept_registry = world
        .registry()
        .map_err(|e| SearchRunError::RunError(RunError::WorldError(e)))?;
    let concept_registry_bytes =
        concept_registry
            .canonical_bytes()
            .map_err(|e| SearchRunError::CanonFailed {
                detail: format!("concept_registry canonical: {e:?}"),
            })?;
    let concept_registry_content_hash =
        canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &concept_registry_bytes);
    let operator_registry = kernel_operator_registry();
    let operator_registry_bytes =
        operator_registry
            .canonical_bytes()
            .map_err(|e| SearchRunError::CanonFailed {
                detail: format!("operator_registry canonical: {e:?}"),
            })?;
    let operator_registry_content_hash =
        canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &operator_registry_bytes);

    let compilation = compile(&payload_bytes, &schema, &concept_registry).map_err(|e| {
        SearchRunError::RunError(RunError::CompilationFailed {
            detail: format!("{e:?}"),
        })
    })?;

    // Phase 2: build metadata bindings.
    let policy_config = PolicyConfig::default();
    let policy_snapshot =
        build_policy(world, &policy_config).map_err(|e| SearchRunError::PolicyBuildFailed {
            detail: format!("{e:?}"),
        })?;
    let policy_content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &policy_snapshot.bytes);

    // Build search policy canonical bytes for digest binding.
    let search_policy_json = search_policy_to_json(search_policy);
    let search_policy_bytes =
        canonical_json_bytes(&search_policy_json).map_err(|e| SearchRunError::CanonFailed {
            detail: format!("{e:?}"),
        })?;
    let search_policy_digest = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &search_policy_bytes);

    let registry_digest = concept_registry.digest().map_err(|e| {
        SearchRunError::RunError(RunError::CompilationFailed {
            detail: format!("registry digest: {e:?}"),
        })
    })?;

    let codebook_hash = build_codebook_hash(world).map_err(SearchRunError::RunError)?;

    // Compute fixture JSON + content hash before bindings (policy-independent).
    let fixture_json =
        build_fixture_json(world, &payload_bytes).map_err(SearchRunError::RunError)?;
    let fixture_content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &fixture_json);

    // Extract scorer digest for metadata bindings (Table mode only).
    let scorer_digest_hex = match scorer_input {
        ScorerInputV1::Uniform => None,
        ScorerInputV1::Table { artifact, .. } => Some(artifact.hex_digest.clone()),
    };

    // Binding format: raw hex (no algorithm prefix). See bundle.rs::binding_hex().
    // Recompute root state plane digests from compilation.state (independent surface
    // from compilation_manifest — two sources that must agree is the coherence argument).
    let root_identity_digest =
        canonical_hash(HashDomain::IdentityPlane, &compilation.state.identity_bytes());
    let root_evidence_digest =
        canonical_hash(HashDomain::EvidencePlane, &compilation.state.evidence_bytes());

    let bindings = MetadataBindings {
        world_id: WorldHarnessV1::world_id(world).to_string(),
        schema_descriptor: format!("{}:{}:{}", schema.id, schema.version, schema.hash),
        registry_digest: registry_digest.hex_digest().to_string(),
        policy_snapshot_digest: policy_content_hash.hex_digest().to_string(),
        search_policy_digest: search_policy_digest.hex_digest().to_string(),
        fixture_digest: fixture_content_hash.hex_digest().to_string(),
        scorer_digest: scorer_digest_hex.clone(),
        operator_set_digest: Some(
            operator_registry_content_hash.hex_digest().to_string(),
        ),
        root_identity_digest: Some(root_identity_digest.hex_digest().to_string()),
        root_evidence_digest: Some(root_evidence_digest.hex_digest().to_string()),
    };

    // Phase 3: run search (with tape).
    let scorer_ref: &dyn ValueScorer = match scorer_input {
        ScorerInputV1::Uniform => &sterling_search::scorer::UniformScorer,
        ScorerInputV1::Table { scorer, .. } => scorer,
    };

    let (search_result, tape_output) = sterling_search::search::search_with_tape(
        compilation.state.clone(),
        world,
        &operator_registry,
        search_policy,
        scorer_ref,
        &bindings,
    )
    .map_err(SearchRunError::SearchError)?;

    // Phase 4: serialize search graph + build bundle.
    let search_graph_bytes =
        search_result
            .graph
            .to_canonical_json_bytes()
            .map_err(|e| SearchRunError::CanonFailed {
                detail: format!("{e:?}"),
            })?;

    // Build verification report for search.
    let search_graph_content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &search_graph_bytes);

    let scorer_digest_for_report = match scorer_input {
        ScorerInputV1::Uniform => None,
        ScorerInputV1::Table { artifact, .. } => Some(artifact.content_hash.as_str().to_string()),
    };

    // Compute tape content hash once; reused for report binding and precomputed_hash.
    let tape_content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &tape_output.bytes);

    // Phase 4b: render tool transcript if evidence_obligations require it.
    let dims = world.dimensions();
    let has_transcript_obligation = dims
        .evidence_obligations
        .iter()
        .any(|o| o == "tool_transcript_v1");

    let transcript_result = if has_transcript_obligation {
        let tape = read_tape(&tape_output.bytes).map_err(|e| SearchRunError::CanonFailed {
            detail: format!("tape parse for transcript: {e:?}"),
        })?;
        let transcript_bytes = crate::transcript::render_tool_transcript(
            &tape,
            &operator_registry,
            WorldHarnessV1::world_id(world),
        )
        .map_err(|e| SearchRunError::CanonFailed {
            detail: format!("transcript render: {e}"),
        })?;
        let transcript_content_hash =
            canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &transcript_bytes);
        Some((transcript_bytes, transcript_content_hash))
    } else {
        None
    };

    let transcript_digest_for_report = transcript_result
        .as_ref()
        .map(|(_, hash)| hash.as_str().to_string());

    // Phase 4c: render epistemic transcript if evidence_obligations require it.
    let has_epistemic_obligation = dims
        .evidence_obligations
        .iter()
        .any(|o| o == "epistemic_transcript_v1");

    let epistemic_result = if has_epistemic_obligation {
        let tape = read_tape(&tape_output.bytes).map_err(|e| SearchRunError::CanonFailed {
            detail: format!("tape parse for epistemic transcript: {e:?}"),
        })?;
        let maybe_bytes = crate::worlds::partial_obs::render_epistemic_transcript(
            &tape,
            &compilation.state,
            &operator_registry,
            WorldHarnessV1::world_id(world),
        )
        .map_err(|e| SearchRunError::CanonFailed {
            detail: format!("epistemic transcript render: {e}"),
        })?;
        maybe_bytes.map(|bytes| {
            let hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &bytes);
            (bytes, hash)
        })
    } else {
        None
    };

    let epistemic_digest_for_report = epistemic_result
        .as_ref()
        .map(|(_, hash)| hash.as_str().to_string());

    let health_metrics = search_result.graph.compute_health_metrics();

    let verification_report = build_search_verification_report(
        WorldHarnessV1::world_id(world),
        &policy_content_hash,
        &search_graph_content_hash,
        &codebook_hash,
        scorer_digest_for_report.as_deref(),
        Some(operator_registry_content_hash.as_str()),
        &fixture_content_hash,
        &health_metrics,
        &tape_content_hash,
        transcript_digest_for_report.as_deref(),
        epistemic_digest_for_report.as_deref(),
    )
    .map_err(SearchRunError::RunError)?;

    let mut artifacts: Vec<ArtifactInput> = vec![
        ArtifactInput {
            name: "fixture.json".into(),
            content: fixture_json,
            normative: true,
            precomputed_hash: Some(fixture_content_hash),
        },
        (
            "compilation_manifest.json".to_string(),
            compilation.compilation_manifest,
            true,
        )
            .into(),
        ("policy_snapshot.json".into(), policy_snapshot.bytes, true).into(),
        ArtifactInput {
            name: "search_graph.json".into(),
            content: search_graph_bytes,
            normative: true,
            precomputed_hash: Some(search_graph_content_hash),
        },
        ArtifactInput {
            name: "search_tape.stap".into(),
            content: tape_output.bytes,
            normative: true,
            precomputed_hash: Some(tape_content_hash),
        },
        ("verification_report.json".into(), verification_report, true).into(),
    ];

    // Include operator registry artifact (normative, always present).
    artifacts.push(ArtifactInput {
        name: "operator_registry.json".into(),
        content: operator_registry_bytes,
        normative: true,
        precomputed_hash: Some(operator_registry_content_hash),
    });

    // Include concept registry artifact (normative, always present).
    // Bytes are exactly RegistryV1::canonical_bytes() — the same bytes
    // that RegistryV1::digest() hashes with DOMAIN_REGISTRY_SNAPSHOT.
    artifacts.push(ArtifactInput {
        name: "concept_registry.json".into(),
        content: concept_registry_bytes,
        normative: true,
        precomputed_hash: Some(concept_registry_content_hash),
    });

    // Include scorer artifact for Table mode (normative).
    if let ScorerInputV1::Table { artifact, .. } = scorer_input {
        artifacts.push(("scorer.json".into(), artifact.bytes.clone(), true).into());
    }

    // Include tool transcript artifact (normative, tool worlds only).
    if let Some((transcript_bytes, transcript_hash)) = transcript_result {
        artifacts.push(ArtifactInput {
            name: "tool_transcript.json".into(),
            content: transcript_bytes,
            normative: true,
            precomputed_hash: Some(transcript_hash),
        });
    }

    // Include epistemic transcript artifact (normative, epistemic worlds only).
    if let Some((epistemic_bytes, epistemic_hash)) = epistemic_result {
        artifacts.push(ArtifactInput {
            name: "epistemic_transcript.json".into(),
            content: epistemic_bytes,
            normative: true,
            precomputed_hash: Some(epistemic_hash),
        });
    }

    build_bundle(artifacts).map_err(SearchRunError::BundleFailed)
}

/// Serialize a `SearchPolicyV1` to canonical JSON for digest binding.
fn search_policy_to_json(policy: &SearchPolicyV1) -> serde_json::Value {
    serde_json::json!({
        "dedup_key": match policy.dedup_key {
            sterling_search::policy::DedupKeyV1::IdentityOnly => "identity_only",
            sterling_search::policy::DedupKeyV1::FullState => "full_state",
        },
        "max_candidates_per_node": policy.max_candidates_per_node,
        "max_depth": policy.max_depth,
        "max_expansions": policy.max_expansions,
        "max_frontier_size": policy.max_frontier_size,
        "prune_visited_policy": match policy.prune_visited_policy {
            sterling_search::policy::PruneVisitedPolicyV1::KeepVisited => "keep_visited",
            sterling_search::policy::PruneVisitedPolicyV1::ReleaseVisited => "release_visited",
        },
        "schema_version": "search_policy.v1",
    })
}

/// Build a search-mode verification report.
#[allow(clippy::too_many_arguments)]
fn build_search_verification_report(
    world_id: &str,
    policy_content_hash: &ContentHash,
    search_graph_content_hash: &ContentHash,
    codebook_hash: &ContentHash,
    scorer_digest: Option<&str>,
    operator_set_digest: Option<&str>,
    fixture_content_hash: &ContentHash,
    health_metrics: &sterling_search::graph::SearchHealthMetricsV1,
    tape_content_hash: &ContentHash,
    tool_transcript_digest: Option<&str>,
    epistemic_transcript_digest: Option<&str>,
) -> Result<Vec<u8>, RunError> {
    let mut report = serde_json::json!({
        // DIAGNOSTIC: not verified by verify_bundle(); present for observability.
        "codebook_hash": codebook_hash.as_str(),
        // DIAGNOSTIC: health metrics derived from SearchGraphV1 (INV-SC-M33-02).
        "diagnostics": {
            "health_metrics": health_metrics.to_json_value(),
        },
        // BINDING: verified against fixture.json content_hash.
        "fixture_digest": fixture_content_hash.as_str(),
        "mode": "search",
        // BINDING: verified against policy_snapshot.json content_hash.
        "policy_digest": policy_content_hash.as_str(),
        "schema_version": "verification_report.v1",
        // BINDING: verified against search_graph.json content_hash.
        "search_graph_digest": search_graph_content_hash.as_str(),
        // BINDING: verified against search_tape.stap content_hash.
        "tape_digest": tape_content_hash.as_str(),
        // BINDING: cross-verified against search_graph.json metadata.world_id.
        "world_id": world_id,
    });

    // BINDING: verified against operator_registry.json content_hash.
    if let Some(digest) = operator_set_digest {
        report["operator_set_digest"] = serde_json::json!(digest);
    }

    // BINDING: verified against scorer.json content_hash (Table mode only).
    if let Some(digest) = scorer_digest {
        report["scorer_digest"] = serde_json::json!(digest);
    }

    // BINDING: verified against tool_transcript.json content_hash (tool worlds only).
    if let Some(digest) = tool_transcript_digest {
        report["tool_transcript_digest"] = serde_json::json!(digest);
    }

    // BINDING: verified against epistemic_transcript.json content_hash (epistemic worlds only).
    if let Some(digest) = epistemic_transcript_digest {
        report["epistemic_transcript_digest"] = serde_json::json!(digest);
    }

    canonical_json_bytes(&report).map_err(|e| RunError::CanonFailed {
        detail: format!("{e:?}"),
    })
}

/// Run a world through the full harness pipeline with default policy.
///
/// Equivalent to `run_with_policy(world, &PolicyConfig::default())`.
///
/// # Errors
///
/// Returns [`RunError`] at any pipeline step.
pub fn run(world: &dyn WorldHarnessV1) -> Result<ArtifactBundleV1, RunError> {
    run_with_policy(world, &PolicyConfig::default())
}

/// Run a world through the full harness pipeline with explicit policy config.
///
/// Produces an [`ArtifactBundleV1`] containing:
/// - `fixture.json` (normative) — canonical JSON fixture description
/// - `compilation_manifest.json` (normative) — from `CompilationResultV1`
/// - `policy_snapshot.json` (normative) — auditable policy declaration
/// - `trace.bst1` (observational) — full trace binary including envelope
/// - `verification_report.json` (normative) — replay verdict + hashes + policy digest
///
/// # Errors
///
/// Returns [`RunError`] at any step. Fail-closed: partial bundles are not
/// produced.
pub fn run_with_policy(
    world: &dyn WorldHarnessV1,
    config: &PolicyConfig,
) -> Result<ArtifactBundleV1, RunError> {
    let world_id = world.world_id();

    // Phase 0: build and enforce policy.
    let policy_snapshot = build_policy(world, config).map_err(|e| RunError::PolicyBuildFailed {
        detail: format!("{e:?}"),
    })?;

    enforce_pre_execution(world, config).map_err(RunError::PolicyViolation)?;

    // Phase 1: compile + execute program.
    let payload_bytes = world.encode_payload().map_err(RunError::WorldError)?;
    let schema = world.schema_descriptor();
    let concept_registry = world.registry().map_err(RunError::WorldError)?;
    let operator_registry = kernel_operator_registry();

    let compilation = compile(&payload_bytes, &schema, &concept_registry).map_err(|e| {
        RunError::CompilationFailed {
            detail: format!("{e:?}"),
        }
    })?;

    let frames = execute_program(world, &compilation.state, &operator_registry)?;

    // Phase 2: build trace.
    let fixture_json = build_fixture_json(world, &payload_bytes)?;
    let trace = assemble_trace(world, &schema, &compilation, &fixture_json, frames)?;

    // Phase 3: serialize trace + enforce byte budget.
    let trace_bytes = trace_to_bytes(&trace).map_err(|e| RunError::TraceWriteFailed {
        detail: format!("{e:?}"),
    })?;

    enforce_trace_bytes(&trace_bytes, config).map_err(RunError::PolicyViolation)?;

    // Phase 4: verify + hash + bundle.
    verify_trace(&trace, &compilation, &payload_bytes, &operator_registry)?;

    let policy_content_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &policy_snapshot.bytes);
    let verification_report =
        build_verification_report_from_trace(world_id, &trace, &policy_content_hash)?;

    let artifacts = vec![
        ("fixture.json".into(), fixture_json, true),
        (
            "compilation_manifest.json".into(),
            compilation.compilation_manifest,
            true,
        ),
        ("policy_snapshot.json".into(), policy_snapshot.bytes, true),
        ("trace.bst1".into(), trace_bytes, false),
        ("verification_report.json".into(), verification_report, true),
    ];

    // Enforce total artifact byte budget.
    let total_bytes: usize = artifacts.iter().map(|(_, content, _)| content.len()).sum();
    enforce_artifact_bytes(total_bytes, config).map_err(RunError::PolicyViolation)?;

    build_bundle(artifacts).map_err(RunError::BundleFailed)
}

/// Execute the world's program, producing trace frames.
fn execute_program(
    world: &dyn WorldHarnessV1,
    initial_state: &sterling_kernel::carrier::bytestate::ByteStateV1,
    operator_registry: &OperatorRegistryV1,
) -> Result<Vec<ByteTraceFrameV1>, RunError> {
    let dims = world.dimensions();
    let program = world.program();

    let frame_0 = ByteTraceFrameV1 {
        op_code: Code32::INITIAL_STATE.to_le_bytes(),
        op_args: vec![0; dims.arg_slot_count * 4],
        result_identity: initial_state.identity_bytes(),
        result_status: initial_state.status_bytes(),
    };

    let mut frames = vec![frame_0];
    let mut current_state = initial_state.clone();

    for (i, step) in program.iter().enumerate() {
        let (new_state, record) =
            apply(&current_state, step.op_code, &step.op_args, operator_registry).map_err(|e| {
                RunError::ApplyFailed {
                    frame_index: i + 1,
                    detail: format!("{e:?}"),
                }
            })?;

        frames.push(ByteTraceFrameV1 {
            op_code: record.op_code,
            op_args: record.op_args,
            result_identity: record.result_identity,
            result_status: record.result_status,
        });

        current_state = new_state;
    }

    Ok(frames)
}

/// Assemble a `ByteTraceV1` from frames and world metadata.
fn assemble_trace(
    world: &dyn WorldHarnessV1,
    schema: &sterling_kernel::carrier::bytestate::SchemaDescriptor,
    compilation: &sterling_kernel::carrier::compile::CompilationResultV1,
    fixture_json: &[u8],
    frames: Vec<ByteTraceFrameV1>,
) -> Result<ByteTraceV1, RunError> {
    let world_id = world.world_id();
    let dims = world.dimensions();
    let fixture_hash = canonical_hash(DOMAIN_HARNESS_FIXTURE, fixture_json);
    let codebook_hash = build_codebook_hash(world)?;

    let header = ByteTraceHeaderV1 {
        schema_version: schema.version.clone(),
        domain_id: schema.id.clone(),
        registry_epoch_hash: compilation.registry_descriptor.hash.clone(),
        codebook_hash: codebook_hash.as_str().to_string(),
        fixture_hash: fixture_hash.as_str().to_string(),
        step_count: frames.len(),
        layer_count: dims.layer_count,
        slot_count: dims.slot_count,
        arg_slot_count: dims.arg_slot_count,
    };

    let footer = ByteTraceFooterV1 {
        suite_identity: build_suite_identity(world_id).as_str().to_string(),
        witness_store_digest: None,
    };

    let envelope = ByteTraceEnvelopeV1 {
        timestamp: "1970-01-01T00:00:00Z".into(),
        trace_id: format!("harness-{world_id}"),
        runner_version: env!("CARGO_PKG_VERSION").to_string(),
        wall_time_ms: 0,
    };

    Ok(ByteTraceV1 {
        envelope,
        header,
        frames,
        footer,
    })
}

/// Replay-verify the trace. Returns error on divergence or structural failure.
fn verify_trace(
    trace: &ByteTraceV1,
    compilation: &sterling_kernel::carrier::compile::CompilationResultV1,
    payload_bytes: &[u8],
    operator_registry: &OperatorRegistryV1,
) -> Result<(), RunError> {
    let trace_bundle = TraceBundleV1 {
        trace: trace.clone(),
        compilation_manifest: compilation.compilation_manifest.clone(),
        input_payload: payload_bytes.to_vec(),
    };

    let verdict =
        replay_verify(&trace_bundle, operator_registry).map_err(|e| RunError::ReplayFailed {
            detail: format!("{e:?}"),
        })?;

    match &verdict {
        ReplayVerdict::Match => Ok(()),
        ReplayVerdict::Divergence {
            frame_index,
            detail,
            ..
        } => Err(RunError::ReplayDivergence {
            frame_index: *frame_index,
            detail: detail.clone(),
        }),
        ReplayVerdict::Invalid { detail } => Err(RunError::ReplayFailed {
            detail: detail.clone(),
        }),
    }
}

/// Build verification report from a verified trace (hash + format).
fn build_verification_report_from_trace(
    world_id: &str,
    trace: &ByteTraceV1,
    policy_content_hash: &ContentHash,
) -> Result<Vec<u8>, RunError> {
    let p_hash = payload_hash(trace).map_err(|e| RunError::HashFailed {
        detail: format!("{e:?}"),
    })?;

    let s_chain = step_chain(trace).map_err(|e| RunError::HashFailed {
        detail: format!("{e:?}"),
    })?;

    build_verification_report(world_id, &p_hash, &s_chain, policy_content_hash)
}

/// Build the fixture JSON artifact (canonical JSON bytes).
fn build_fixture_json(
    world: &dyn WorldHarnessV1,
    payload_bytes: &[u8],
) -> Result<Vec<u8>, RunError> {
    let dims = world.dimensions();
    let program = world.program();

    let program_steps: Vec<serde_json::Value> = program
        .iter()
        .map(|step| {
            serde_json::json!({
                "op_args_hex": hex::encode(&step.op_args),
                "op_code_hex": hex::encode(step.op_code.to_le_bytes()),
            })
        })
        .collect();

    let obligations: Vec<serde_json::Value> = dims
        .evidence_obligations
        .iter()
        .map(|s| serde_json::Value::String(s.clone()))
        .collect();

    let fixture_value = serde_json::json!({
        "dimensions": {
            "arg_slot_count": dims.arg_slot_count,
            "layer_count": dims.layer_count,
            "slot_count": dims.slot_count,
        },
        "evidence_obligations": obligations,
        "initial_payload_hex": hex::encode(payload_bytes),
        "program": program_steps,
        "schema_version": "fixture.v1",
        "world_id": world.world_id(),
    });

    canonical_json_bytes(&fixture_value).map_err(|e| RunError::CanonFailed {
        detail: format!("{e:?}"),
    })
}

/// Build the codebook hash from the world's program operator signatures.
///
/// Codebook hash basis includes only stable signature fields:
/// `op_code_hex` and `arg_slot_count`, sorted by `op_code_hex`.
fn build_codebook_hash(world: &dyn WorldHarnessV1) -> Result<ContentHash, RunError> {
    use std::collections::BTreeMap;

    let dims = world.dimensions();
    let program = world.program();

    // Deduplicate operators by op_code_hex (BTreeMap gives sorted order).
    let mut operators: BTreeMap<String, usize> = BTreeMap::new();
    for step in &program {
        let hex = hex::encode(step.op_code.to_le_bytes());
        operators.entry(hex).or_insert(dims.arg_slot_count);
    }

    let operator_entries: Vec<serde_json::Value> = operators
        .iter()
        .map(|(hex, arg_count)| {
            serde_json::json!({
                "arg_slot_count": arg_count,
                "op_code_hex": hex,
            })
        })
        .collect();

    let basis = serde_json::json!({
        "operators": operator_entries,
        "schema_version": "codebook_hash_basis.v1",
    });

    let bytes = canonical_json_bytes(&basis).map_err(|e| RunError::CanonFailed {
        detail: format!("{e:?}"),
    })?;

    Ok(canonical_hash(DOMAIN_CODEBOOK_HASH, &bytes))
}

/// Build a deterministic suite identity from the world ID.
fn build_suite_identity(world_id: &str) -> ContentHash {
    canonical_hash(HashDomain::SuiteIdentity, world_id.as_bytes())
}

/// Build the verification report JSON (canonical JSON bytes).
///
/// Called only after successful replay verification (verdict is always Match).
fn build_verification_report(
    world_id: &str,
    p_hash: &ContentHash,
    s_chain: &sterling_kernel::proof::trace_hash::StepChainResult,
    policy_content_hash: &ContentHash,
) -> Result<Vec<u8>, RunError> {
    let report = serde_json::json!({
        "payload_hash": p_hash.as_str(),
        "planes_verified": ["identity", "status"],
        "policy_digest": policy_content_hash.as_str(),
        "replay_verdict": "Match",
        "schema_version": "verification_report.v1",
        "step_chain_digest": s_chain.digest.as_str(),
        "step_chain_length": s_chain.chain.len(),
        "step_count": s_chain.chain.len(),
        "world_id": world_id,
    });

    canonical_json_bytes(&report).map_err(|e| RunError::CanonFailed {
        detail: format!("{e:?}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::rome_mini::RomeMini;
    use crate::worlds::rome_mini_search::RomeMiniSearch;

    #[test]
    fn rome_mini_produces_bundle() {
        let bundle = run(&RomeMini).unwrap();
        assert_eq!(bundle.artifacts.len(), 5);
        assert!(bundle.artifacts.contains_key("fixture.json"));
        assert!(bundle.artifacts.contains_key("compilation_manifest.json"));
        assert!(bundle.artifacts.contains_key("policy_snapshot.json"));
        assert!(bundle.artifacts.contains_key("trace.bst1"));
        assert!(bundle.artifacts.contains_key("verification_report.json"));
    }

    #[test]
    fn rome_mini_trace_bst1_is_observational() {
        let bundle = run(&RomeMini).unwrap();
        let trace = bundle.artifacts.get("trace.bst1").unwrap();
        assert!(!trace.normative);
    }

    #[test]
    fn rome_mini_normative_artifacts() {
        let bundle = run(&RomeMini).unwrap();
        for name in [
            "fixture.json",
            "compilation_manifest.json",
            "policy_snapshot.json",
            "verification_report.json",
        ] {
            let artifact = bundle.artifacts.get(name).unwrap();
            assert!(artifact.normative, "{name} should be normative");
        }
    }

    #[test]
    fn rome_mini_verification_report_contains_planes_verified() {
        let bundle = run(&RomeMini).unwrap();
        let report = bundle.artifacts.get("verification_report.json").unwrap();
        let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();
        let planes = json["planes_verified"].as_array().unwrap();
        assert_eq!(planes.len(), 2);
        assert_eq!(planes[0], "identity");
        assert_eq!(planes[1], "status");
    }

    #[test]
    fn rome_mini_verification_report_contains_policy_digest() {
        let bundle = run(&RomeMini).unwrap();
        let report = bundle.artifacts.get("verification_report.json").unwrap();
        let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();
        let policy_digest = json["policy_digest"].as_str().unwrap();
        assert!(policy_digest.starts_with("sha256:"));

        // Should match policy artifact's content_hash.
        let policy = bundle.artifacts.get("policy_snapshot.json").unwrap();
        assert_eq!(policy_digest, policy.content_hash.as_str());
    }

    #[test]
    fn rome_mini_policy_snapshot_is_normative() {
        let bundle = run(&RomeMini).unwrap();
        let policy = bundle.artifacts.get("policy_snapshot.json").unwrap();
        assert!(policy.normative);
    }

    #[test]
    fn run_with_step_budget_violation_fails() {
        let config = PolicyConfig {
            max_steps: Some(1),
            ..PolicyConfig::default()
        };
        let err = run_with_policy(&RomeMini, &config).unwrap_err();
        match err {
            RunError::PolicyViolation(PolicyViolation::StepBudgetExceeded { .. }) => {}
            other => panic!("expected PolicyViolation(StepBudgetExceeded), got {other:?}"),
        }
    }

    #[test]
    fn run_with_allowlist_violation_fails() {
        let config = PolicyConfig {
            allowed_ops: Some(vec![Code32::new(99, 99, 99)]),
            ..PolicyConfig::default()
        };
        let err = run_with_policy(&RomeMini, &config).unwrap_err();
        match err {
            RunError::PolicyViolation(PolicyViolation::AllowlistViolation { .. }) => {}
            other => panic!("expected PolicyViolation(AllowlistViolation), got {other:?}"),
        }
    }

    // --- Search runner tests ---

    #[test]
    fn run_search_produces_bundle() {
        let policy = SearchPolicyV1::default();
        let bundle = run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();
        assert!(bundle.artifacts.contains_key("fixture.json"));
        assert!(bundle.artifacts.contains_key("compilation_manifest.json"));
        assert!(bundle.artifacts.contains_key("concept_registry.json"));
        assert!(bundle.artifacts.contains_key("policy_snapshot.json"));
        assert!(bundle.artifacts.contains_key("search_graph.json"));
        assert!(bundle.artifacts.contains_key("search_tape.stap"));
        assert!(bundle.artifacts.contains_key("operator_registry.json"));
        assert!(bundle.artifacts.contains_key("verification_report.json"));
        assert_eq!(bundle.artifacts.len(), 8);
    }

    #[test]
    fn run_search_graph_is_normative() {
        let policy = SearchPolicyV1::default();
        let bundle = run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();
        let graph = bundle.artifacts.get("search_graph.json").unwrap();
        assert!(graph.normative, "search_graph.json should be normative");
    }

    #[test]
    fn run_search_verification_report_contains_search_graph_digest() {
        let policy = SearchPolicyV1::default();
        let bundle = run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();

        let report = bundle.artifacts.get("verification_report.json").unwrap();
        let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();
        let digest = json["search_graph_digest"].as_str().unwrap();
        assert!(digest.starts_with("sha256:"));

        // Should match search_graph.json's content_hash.
        let graph = bundle.artifacts.get("search_graph.json").unwrap();
        assert_eq!(digest, graph.content_hash.as_str());
    }

    #[test]
    fn run_search_verify_bundle_passes() {
        use crate::bundle::verify_bundle;
        let policy = SearchPolicyV1::default();
        let bundle = run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();
        verify_bundle(&bundle).unwrap();
    }

    #[test]
    fn run_search_finds_goal() {
        let policy = SearchPolicyV1::default();
        let bundle = run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).unwrap();

        let report = bundle.artifacts.get("verification_report.json").unwrap();
        let json: serde_json::Value = serde_json::from_slice(&report.content).unwrap();
        assert_eq!(json["mode"], "search");

        // Search graph should have goal_reached termination
        let graph = bundle.artifacts.get("search_graph.json").unwrap();
        let graph_json: serde_json::Value = serde_json::from_slice(&graph.content).unwrap();
        let term = &graph_json["metadata"]["termination_reason"];
        assert_eq!(term["type"], "goal_reached");
    }

    #[test]
    fn short_sha256_scorer_key_rejected() {
        let mut table = BTreeMap::new();
        // Valid algorithm, but only 4 hex chars instead of 64.
        table.insert("sha256:abcd".to_string(), 10);

        let err = build_table_scorer_input(table).unwrap_err();
        assert!(
            matches!(err, SearchRunError::InvalidScorerTableKey { .. }),
            "short sha256 digest must be rejected, got: {err:?}"
        );
    }

    #[test]
    fn full_length_sha256_scorer_key_accepted() {
        let mut table = BTreeMap::new();
        table.insert(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            10,
        );

        // Should succeed (full 64-char hex digest).
        let result = build_table_scorer_input(table);
        assert!(result.is_ok(), "full-length sha256 key must be accepted");
    }
}
