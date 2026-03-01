//! Shared test helpers for mutating and rebuilding artifact bundles.
//!
//! These helpers maintain digest consistency when modifying bundle artifacts,
//! preventing tests from accidentally testing digest mismatch instead of the
//! semantic mismatch they intend to exercise.

use sterling_harness::bundle::{build_bundle, ArtifactBundleV1, DOMAIN_BUNDLE_ARTIFACT};
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::canonical_hash;
use sterling_search::tape::{
    raw_hash, raw_hash2, DOMAIN_SEARCH_TAPE, DOMAIN_SEARCH_TAPE_CHAIN, FOOTER_SIZE,
    SEARCH_TAPE_MAGIC, SEARCH_TAPE_VERSION,
};

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

/// Modify both `search_graph.json` and `verification_report.json` in a bundle
/// and rebuild with consistent content hashes.
///
/// Unlike [`rebuild_with_modified_graph`], which patches the report's
/// `search_graph_digest` automatically, this gives the caller full control
/// over both JSON values. The caller is responsible for keeping
/// `search_graph_digest` and other cross-referenced fields consistent.
///
/// Primary use case: patching graph metadata + report fields together so
/// earlier verification steps pass, isolating tape-header-specific checks.
///
/// # Panics
///
/// Panics if the bundle is missing `search_graph.json` or `verification_report.json`.
pub fn rebuild_with_modified_graph_and_report(
    bundle: &ArtifactBundleV1,
    modify_graph: impl FnOnce(&mut serde_json::Value),
    modify_report: impl FnOnce(&mut serde_json::Value),
) -> ArtifactBundleV1 {
    let graph_artifact = bundle.artifacts.get("search_graph.json").unwrap();
    let mut graph_json: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).unwrap();
    modify_graph(&mut graph_json);
    let modified_graph_bytes = canonical_json_bytes(&graph_json).unwrap();
    let new_graph_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_graph_bytes);

    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();
    // Keep search_graph_digest consistent with the modified graph.
    report_json["search_graph_digest"] = serde_json::json!(new_graph_hash.as_str());
    modify_report(&mut report_json);
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

/// Replace the raw bytes of `search_tape.stap` in a bundle and rebuild with
/// consistent content hashes, manifest, and digest basis.
///
/// The report's `tape_digest` is updated to match the new tape content hash,
/// so tests bypass `TapeDigestMismatch` and reach deeper tape-specific checks.
///
/// # Panics
///
/// Panics if the bundle is missing `search_tape.stap` or `verification_report.json`.
pub fn rebuild_with_modified_tape(
    bundle: &ArtifactBundleV1,
    modify_bytes: impl FnOnce(&[u8]) -> Vec<u8>,
) -> ArtifactBundleV1 {
    let tape_artifact = bundle.artifacts.get("search_tape.stap").unwrap();
    let modified_tape_bytes = modify_bytes(&tape_artifact.content);
    let new_tape_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_tape_bytes);

    // Update the report's tape_digest to match the modified tape.
    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();
    report_json["tape_digest"] = serde_json::json!(new_tape_hash.as_str());
    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "search_tape.stap" {
                (a.name.clone(), modified_tape_bytes.clone(), a.normative)
            } else if a.name == "verification_report.json" {
                (a.name.clone(), modified_report_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    build_bundle(artifacts).unwrap()
}

/// Rebuild a bundle with one artifact removed entirely.
///
/// Useful for testing "missing artifact" error paths after the read boundary.
/// The resulting bundle has consistent content hashes and digest basis for the
/// remaining artifacts.
///
/// # Panics
///
/// Panics if the named artifact does not exist in the bundle.
#[must_use]
pub fn rebuild_without_artifact(bundle: &ArtifactBundleV1, remove_name: &str) -> ArtifactBundleV1 {
    assert!(
        bundle.artifacts.contains_key(remove_name),
        "cannot remove non-existent artifact: {remove_name}"
    );
    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .filter(|a| a.name != remove_name)
        .map(|a| (a.name.clone(), a.content.clone(), a.normative))
        .collect();
    build_bundle(artifacts).unwrap()
}

/// Modify `compilation_manifest.json` in a bundle and rebuild with consistent
/// content hashes, manifest, and digest basis.
///
/// No upstream artifacts reference the compilation manifest's content hash,
/// so no report patching is needed — only `build_bundle()` re-signing.
///
/// Artifact ordering is stabilized by sorting names before collection.
///
/// # Panics
///
/// Panics if the bundle is missing `compilation_manifest.json`.
pub fn resign_bundle_with_modified_compilation_manifest(
    bundle: &ArtifactBundleV1,
    modify: impl FnOnce(&mut serde_json::Value),
) -> ArtifactBundleV1 {
    let cm_artifact = bundle
        .artifacts
        .get("compilation_manifest.json")
        .unwrap();
    let mut cm_json: serde_json::Value =
        serde_json::from_slice(&cm_artifact.content).unwrap();
    modify(&mut cm_json);
    let modified_cm_bytes = canonical_json_bytes(&cm_json).unwrap();

    // Stable ordering: sort by artifact name to avoid HashMap iteration nondeterminism.
    let mut names: Vec<&str> = bundle.artifacts.keys().map(String::as_str).collect();
    names.sort_unstable();

    let artifacts: Vec<(String, Vec<u8>, bool)> = names
        .into_iter()
        .map(|name| {
            let a = &bundle.artifacts[name];
            if name == "compilation_manifest.json" {
                (a.name.clone(), modified_cm_bytes.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    build_bundle(artifacts).unwrap()
}

/// Modify `search_graph.json` metadata AND `search_tape.stap` header in a bundle,
/// keeping chain integrity and all upstream digest bindings consistent.
///
/// This is the sanctioned way to test "legacy bundle" scenarios where fields
/// are absent from both graph metadata and tape header simultaneously.
///
/// The tape is rebuilt by binary surgery: the header JSON is replaced and the
/// chain hash is recomputed through unchanged record frames, preserving the
/// footer's record count and chain integrity.
///
/// # Panics
///
/// Panics if the bundle is missing required artifacts or if the tape binary
/// structure is malformed.
pub fn rebuild_with_modified_graph_and_tape_header(
    bundle: &ArtifactBundleV1,
    modify_graph: impl FnOnce(&mut serde_json::Value),
    modify_tape_header: impl FnOnce(&mut serde_json::Value),
) -> ArtifactBundleV1 {
    // --- Modify graph metadata ---
    let graph_artifact = bundle.artifacts.get("search_graph.json").unwrap();
    let mut graph_json: serde_json::Value =
        serde_json::from_slice(&graph_artifact.content).unwrap();
    modify_graph(&mut graph_json);
    let modified_graph_bytes = canonical_json_bytes(&graph_json).unwrap();
    let new_graph_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_graph_bytes);

    // --- Modify tape header with chain-hash rebuild ---
    let tape_artifact = bundle.artifacts.get("search_tape.stap").unwrap();
    let modified_tape_bytes = rebuild_tape_with_header(&tape_artifact.content, modify_tape_header);
    let new_tape_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &modified_tape_bytes);

    // --- Patch report digests ---
    let report_artifact = bundle.artifacts.get("verification_report.json").unwrap();
    let mut report_json: serde_json::Value =
        serde_json::from_slice(&report_artifact.content).unwrap();
    report_json["search_graph_digest"] = serde_json::json!(new_graph_hash.as_str());
    report_json["tape_digest"] = serde_json::json!(new_tape_hash.as_str());
    let modified_report_bytes = canonical_json_bytes(&report_json).unwrap();

    let artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| match a.name.as_str() {
            "search_graph.json" => (a.name.clone(), modified_graph_bytes.clone(), a.normative),
            "search_tape.stap" => (a.name.clone(), modified_tape_bytes.clone(), a.normative),
            "verification_report.json" => {
                (a.name.clone(), modified_report_bytes.clone(), a.normative)
            }
            _ => (a.name.clone(), a.content.clone(), a.normative),
        })
        .collect();
    build_bundle(artifacts).unwrap()
}

/// Rebuild tape binary with modified header JSON, recomputing the chain hash.
///
/// Binary layout: `[magic:4][version:2][header_len:4][header:N][frames...][footer:44]`
/// Footer: `[record_count:8][chain_hash:32][footer_magic:4]`
fn rebuild_tape_with_header(
    tape_bytes: &[u8],
    modify: impl FnOnce(&mut serde_json::Value),
) -> Vec<u8> {
    // Parse original header to get the record frames region.
    assert!(tape_bytes.len() >= 10 + FOOTER_SIZE, "tape too short");
    assert_eq!(&tape_bytes[0..4], &SEARCH_TAPE_MAGIC, "bad magic");
    let version = u16::from_le_bytes([tape_bytes[4], tape_bytes[5]]);
    assert_eq!(version, SEARCH_TAPE_VERSION, "bad version");
    let old_header_len =
        u32::from_le_bytes([tape_bytes[6], tape_bytes[7], tape_bytes[8], tape_bytes[9]]) as usize;
    let old_header_bytes = &tape_bytes[10..10 + old_header_len];

    // Parse and modify header JSON.
    let mut header_json: serde_json::Value = serde_json::from_slice(old_header_bytes).unwrap();
    modify(&mut header_json);
    let new_header_bytes = canonical_json_bytes(&header_json).unwrap();

    // Extract record frames region (between header and footer).
    let frames_start = 10 + old_header_len;
    let footer_start = tape_bytes.len() - FOOTER_SIZE;
    let frames_region = &tape_bytes[frames_start..footer_start];

    // Parse footer to get record count.
    let record_count = u64::from_le_bytes(
        tape_bytes[footer_start..footer_start + 8]
            .try_into()
            .unwrap(),
    );

    // Reseed chain hash with new header and replay through frames.
    let mut chain_hash = raw_hash(DOMAIN_SEARCH_TAPE, &new_header_bytes);
    let mut pos = 0;
    let mut replayed_records: u64 = 0;
    while pos < frames_region.len() {
        let frame_len = u32::from_le_bytes(
            frames_region[pos..pos + 4].try_into().unwrap(),
        ) as usize;
        let full_frame = &frames_region[pos..pos + 4 + frame_len];
        chain_hash = raw_hash2(DOMAIN_SEARCH_TAPE_CHAIN, &chain_hash, full_frame);
        pos += 4 + frame_len;
        replayed_records += 1;
    }
    assert_eq!(replayed_records, record_count, "record count mismatch");

    // Assemble new tape.
    #[allow(clippy::cast_possible_truncation)]
    let new_header_len = new_header_bytes.len() as u32;
    let mut buf = Vec::with_capacity(10 + new_header_bytes.len() + frames_region.len() + FOOTER_SIZE);
    buf.extend_from_slice(&SEARCH_TAPE_MAGIC);
    buf.extend_from_slice(&SEARCH_TAPE_VERSION.to_le_bytes());
    buf.extend_from_slice(&new_header_len.to_le_bytes());
    buf.extend_from_slice(&new_header_bytes);
    buf.extend_from_slice(frames_region);
    // Footer: [record_count:8][chain_hash:32][footer_magic:4]
    buf.extend_from_slice(&record_count.to_le_bytes());
    buf.extend_from_slice(&chain_hash);
    buf.extend_from_slice(&u32::from_le_bytes(*b"PATS").to_le_bytes());
    buf
}

