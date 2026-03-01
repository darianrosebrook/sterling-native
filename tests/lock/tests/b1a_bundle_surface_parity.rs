//! B1a bundle surface parity lock tests.
//!
//! Proves:
//! 1. The committed golden bundle directory self-verifies (read + `verify_bundle`)
//! 2. The golden bundle passes `verify_bundle_dir` end-to-end
//! 3. The golden bundle contains exactly the expected 7 artifacts
//! 4. In-memory regeneration produces byte-identical artifacts
//! 5. Regenerated bundle digest matches golden bundle digest
//! 6. All normative artifact content hashes match between golden and regenerated

use sterling_harness::bundle::{verify_bundle, ArtifactBundleV1};
use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_search::policy::SearchPolicyV1;

/// Path to the committed golden fixture directory.
const GOLDEN_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/b1a_rome_mini_search"
);

/// The expected artifact names in a Uniform-scorer `RomeMiniSearch` bundle.
const EXPECTED_ARTIFACTS: &[&str] = &[
    "compilation_manifest.json",
    "concept_registry.json",
    "fixture.json",
    "operator_registry.json",
    "policy_snapshot.json",
    "search_graph.json",
    "search_tape.stap",
    "verification_report.json",
];

/// Produce the canonical in-memory bundle for comparison.
fn regen_bundle() -> ArtifactBundleV1 {
    let policy = SearchPolicyV1::default();
    run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("search run failed")
}

// ---------------------------------------------------------------------------
// 1. Golden self-verifies via verify_bundle
// ---------------------------------------------------------------------------

/// ACCEPTANCE: B1A-001-SELF-VERIFY
#[test]
fn golden_bundle_passes_verify_bundle() {
    let bundle = read_bundle_dir(GOLDEN_DIR.as_ref()).expect("failed to read golden bundle");
    verify_bundle(&bundle).expect("golden bundle must pass verify_bundle");
}

// ---------------------------------------------------------------------------
// 2. Golden passes verify_bundle_dir end-to-end
// ---------------------------------------------------------------------------

/// ACCEPTANCE: B1A-001-SELF-VERIFY
#[test]
fn golden_bundle_passes_verify_bundle_dir() {
    verify_bundle_dir(GOLDEN_DIR.as_ref()).expect("golden bundle must pass verify_bundle_dir");
}

// ---------------------------------------------------------------------------
// 3. Golden contains exactly the expected artifacts
// ---------------------------------------------------------------------------

/// ACCEPTANCE: B1A-001-GOLDEN
#[test]
fn golden_bundle_has_expected_artifacts() {
    let bundle = read_bundle_dir(GOLDEN_DIR.as_ref()).expect("failed to read golden bundle");

    assert_eq!(
        bundle.artifacts.len(),
        EXPECTED_ARTIFACTS.len(),
        "golden bundle artifact count mismatch: expected {}, got {}",
        EXPECTED_ARTIFACTS.len(),
        bundle.artifacts.len()
    );

    for name in EXPECTED_ARTIFACTS {
        assert!(
            bundle.artifacts.contains_key(*name),
            "golden bundle missing expected artifact: {name}"
        );
    }

    // All artifacts must be normative.
    for (name, art) in &bundle.artifacts {
        assert!(art.normative, "artifact {name} must be normative");
    }
}

// ---------------------------------------------------------------------------
// 4. Regeneration produces byte-identical artifacts
// ---------------------------------------------------------------------------

/// ACCEPTANCE: B1A-001-REGEN
#[test]
fn regen_matches_golden_byte_for_byte() {
    let golden = read_bundle_dir(GOLDEN_DIR.as_ref()).expect("failed to read golden bundle");
    let regen = regen_bundle();

    // Same artifact set.
    assert_eq!(
        golden.artifacts.len(),
        regen.artifacts.len(),
        "artifact count mismatch: golden={}, regen={}",
        golden.artifacts.len(),
        regen.artifacts.len()
    );

    for (name, golden_art) in &golden.artifacts {
        let regen_art = regen
            .artifacts
            .get(name)
            .unwrap_or_else(|| panic!("regenerated bundle missing artifact: {name}"));

        assert_eq!(
            golden_art.content, regen_art.content,
            "byte mismatch in artifact {name}: golden {} bytes, regen {} bytes",
            golden_art.content.len(),
            regen_art.content.len()
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Regenerated bundle digest matches golden
// ---------------------------------------------------------------------------

/// ACCEPTANCE: B1A-001-REGEN
#[test]
fn regen_bundle_digest_matches_golden() {
    let golden = read_bundle_dir(GOLDEN_DIR.as_ref()).expect("failed to read golden bundle");
    let regen = regen_bundle();

    assert_eq!(
        golden.digest.as_str(),
        regen.digest.as_str(),
        "bundle digest mismatch: golden={}, regen={}",
        golden.digest.as_str(),
        regen.digest.as_str()
    );
}

// ---------------------------------------------------------------------------
// 6. All content hashes match
// ---------------------------------------------------------------------------

/// ACCEPTANCE: B1A-001-REGEN
#[test]
fn regen_content_hashes_match_golden() {
    let golden = read_bundle_dir(GOLDEN_DIR.as_ref()).expect("failed to read golden bundle");
    let regen = regen_bundle();

    for (name, golden_art) in &golden.artifacts {
        let regen_art = regen
            .artifacts
            .get(name)
            .unwrap_or_else(|| panic!("regenerated bundle missing artifact: {name}"));

        assert_eq!(
            golden_art.content_hash.as_str(),
            regen_art.content_hash.as_str(),
            "content_hash mismatch for {name}: golden={}, regen={}",
            golden_art.content_hash.as_str(),
            regen_art.content_hash.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Regenerated bundle also self-verifies
// ---------------------------------------------------------------------------

#[test]
fn regen_bundle_passes_verify_bundle() {
    let regen = regen_bundle();
    verify_bundle(&regen).expect("regenerated bundle must pass verify_bundle");
}

// ---------------------------------------------------------------------------
// Fixture regeneration (run manually with --ignored)
// ---------------------------------------------------------------------------

/// Regenerate the golden fixture directory from a fresh search run.
///
/// Run with: `cargo test -p lock-tests --test b1a_bundle_surface_parity regen_golden_fixtures -- --ignored`
#[test]
#[ignore = "manual fixture regeneration"]
fn regen_golden_fixtures() {
    let dir = std::path::Path::new(GOLDEN_DIR);

    // Remove and recreate the directory.
    if dir.exists() {
        std::fs::remove_dir_all(dir).expect("remove old fixtures");
    }
    std::fs::create_dir_all(dir).expect("create fixture dir");

    let bundle = regen_bundle();
    write_bundle_dir(&bundle, dir).expect("write bundle dir failed");

    // Verify the written fixtures round-trip.
    let readback = read_bundle_dir(dir).expect("read back failed");
    verify_bundle(&readback).expect("readback must verify");

    eprintln!("Regenerated golden fixtures at {}", dir.display());
}
