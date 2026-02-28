//! B1b compile parity lock tests.
//!
//! Proves:
//! 1. Committed fixture inputs are valid canonical JSON
//! 2. `compile()` from fixture inputs produces manifest matching golden
//! 3. `compile()` from fixture inputs produces digests matching golden
//! 4. Round-trip: load inputs from disk, compile, compare to in-memory compile
//! 5. Manifest has no extra or missing keys (fail-closed schema)
//! 6. Expected hashes has exactly the expected keys (fail-closed)
//! 7. Returned descriptors echo fixture inputs (struct contract)

use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, HashDomain};

/// Path to the committed golden compile fixture directory.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/b1b_compile_rome_mini_search"
);

/// Load a fixture file as bytes.
fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = format!("{FIXTURE_DIR}/{name}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// Parse the expected hashes JSON fixture.
fn expected_hashes() -> serde_json::Value {
    let bytes = fixture_bytes("expected_hashes.json");
    serde_json::from_slice(&bytes).expect("invalid expected_hashes.json")
}

// ---------------------------------------------------------------------------
// 1. Fixture inputs are valid canonical JSON
// ---------------------------------------------------------------------------

#[test]
fn fixture_inputs_are_canonical() {
    for name in &[
        "payload.json",
        "schema_descriptor.json",
        "concept_registry.json",
    ] {
        let bytes = fixture_bytes(name);
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
        let re_canon =
            canonical_json_bytes(&value).unwrap_or_else(|e| panic!("{name} re-canon failed: {e}"));
        assert_eq!(
            bytes, re_canon,
            "{name} is not in canonical form (re-canonicalization changed bytes)"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Compile from fixture inputs matches golden manifest
// ---------------------------------------------------------------------------

#[test]
fn compile_from_fixture_matches_golden_manifest() {
    let world = RomeMiniSearch;
    let payload_bytes = fixture_bytes("payload.json");
    let schema = world.schema_descriptor();
    let concept_registry = world.registry().expect("registry failed");

    let result = compile(&payload_bytes, &schema, &concept_registry).expect("compile failed");

    let expected_manifest = fixture_bytes("expected_compilation_manifest.json");
    assert_eq!(
        result.compilation_manifest, expected_manifest,
        "compilation manifest bytes differ from golden"
    );
}

// ---------------------------------------------------------------------------
// 3. Compile digests match golden expected hashes
// ---------------------------------------------------------------------------

#[test]
fn compile_digests_match_golden_hashes() {
    let world = RomeMiniSearch;
    let payload_bytes = fixture_bytes("payload.json");
    let schema = world.schema_descriptor();
    let concept_registry = world.registry().expect("registry failed");

    let result = compile(&payload_bytes, &schema, &concept_registry).expect("compile failed");
    let hashes = expected_hashes();

    assert_eq!(
        result.identity_digest.as_str(),
        hashes["identity_digest"].as_str().unwrap(),
        "identity_digest mismatch"
    );
    assert_eq!(
        result.evidence_digest.as_str(),
        hashes["evidence_digest"].as_str().unwrap(),
        "evidence_digest mismatch"
    );

    let payload_hash = canonical_hash(HashDomain::CompilationPayload, &payload_bytes);
    assert_eq!(
        payload_hash.as_str(),
        hashes["payload_hash"].as_str().unwrap(),
        "payload_hash mismatch"
    );

    let registry_bytes = concept_registry
        .canonical_bytes()
        .expect("registry canonical_bytes failed");
    let registry_hash = canonical_hash(HashDomain::RegistrySnapshot, &registry_bytes);
    assert_eq!(
        registry_hash.as_str(),
        hashes["registry_hash"].as_str().unwrap(),
        "registry_hash mismatch"
    );
}

// ---------------------------------------------------------------------------
// 4. Round-trip: disk-loaded registry matches in-memory registry
// ---------------------------------------------------------------------------

#[test]
fn fixture_registry_matches_world_registry() {
    let world = RomeMiniSearch;
    let concept_registry = world.registry().expect("registry failed");
    let in_memory_bytes = concept_registry
        .canonical_bytes()
        .expect("canonical_bytes failed");

    let fixture_registry_bytes = fixture_bytes("concept_registry.json");
    assert_eq!(
        in_memory_bytes, fixture_registry_bytes,
        "fixture concept_registry.json differs from in-memory registry canonical bytes"
    );
}

// ---------------------------------------------------------------------------
// 5. Manifest has exactly the expected keys (fail-closed schema)
// ---------------------------------------------------------------------------

#[test]
fn manifest_has_exact_expected_keys() {
    let manifest_bytes = fixture_bytes("expected_compilation_manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).expect("invalid manifest JSON");

    let expected_keys = [
        "evidence_digest",
        "identity_digest",
        "payload_hash",
        "registry_epoch",
        "registry_hash",
        "schema_hash",
        "schema_id",
        "schema_version",
    ];

    let obj = manifest.as_object().expect("manifest must be a JSON object");

    // No extra keys.
    for key in obj.keys() {
        assert!(
            expected_keys.contains(&key.as_str()),
            "unexpected key in manifest: {key}"
        );
    }

    // No missing keys.
    for key in &expected_keys {
        assert!(
            obj.contains_key(*key),
            "missing expected key in manifest: {key}"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Expected hashes has exactly the expected keys (fail-closed)
// ---------------------------------------------------------------------------

#[test]
fn expected_hashes_has_exact_keys() {
    let hashes = expected_hashes();
    let expected_keys = [
        "evidence_digest",
        "identity_digest",
        "payload_hash",
        "registry_epoch",
        "registry_hash",
        "schema_hash",
    ];

    let obj = hashes.as_object().expect("expected_hashes must be a JSON object");

    for key in obj.keys() {
        assert!(
            expected_keys.contains(&key.as_str()),
            "unexpected key in expected_hashes: {key}"
        );
    }
    for key in &expected_keys {
        assert!(
            obj.contains_key(*key),
            "missing expected key in expected_hashes: {key}"
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Returned descriptors echo fixture inputs (struct contract)
// ---------------------------------------------------------------------------

/// Assert that `compile()` echoes the input descriptors on its result struct,
/// and that those echoed values are consistent with the manifest fields.
/// Catches regressions where the struct fields diverge from the manifest.
#[test]
fn compile_echoes_descriptors() {
    let world = RomeMiniSearch;
    let payload_bytes = fixture_bytes("payload.json");
    let schema = world.schema_descriptor();
    let concept_registry = world.registry().expect("registry failed");

    let result = compile(&payload_bytes, &schema, &concept_registry).expect("compile failed");

    // Schema descriptor echoed on struct matches fixture input.
    let fixture_schema: serde_json::Value = {
        let bytes = fixture_bytes("schema_descriptor.json");
        serde_json::from_slice(&bytes).expect("invalid schema_descriptor.json")
    };
    assert_eq!(
        result.schema_descriptor.id,
        fixture_schema["id"].as_str().unwrap(),
        "echoed schema_descriptor.id differs from fixture"
    );
    assert_eq!(
        result.schema_descriptor.version,
        fixture_schema["version"].as_str().unwrap(),
        "echoed schema_descriptor.version differs from fixture"
    );
    assert_eq!(
        result.schema_descriptor.hash,
        fixture_schema["hash"].as_str().unwrap(),
        "echoed schema_descriptor.hash differs from fixture"
    );

    // Registry descriptor echoed on struct matches expected hashes.
    let hashes = expected_hashes();
    assert_eq!(
        result.registry_descriptor.epoch,
        hashes["registry_epoch"].as_str().unwrap(),
        "echoed registry_descriptor.epoch differs from expected_hashes"
    );

    // Registry hash on struct matches externally computed hash.
    let registry_bytes = concept_registry
        .canonical_bytes()
        .expect("registry canonical_bytes failed");
    let registry_hash = canonical_hash(HashDomain::RegistrySnapshot, &registry_bytes);
    assert_eq!(
        result.registry_descriptor.hash,
        registry_hash.as_str(),
        "echoed registry_descriptor.hash differs from recomputed hash"
    );

    // Cross-check: manifest fields match echoed struct fields.
    let manifest: serde_json::Value = {
        let bytes = fixture_bytes("expected_compilation_manifest.json");
        serde_json::from_slice(&bytes).expect("invalid manifest JSON")
    };
    assert_eq!(
        result.schema_descriptor.id,
        manifest["schema_id"].as_str().unwrap(),
        "echoed schema_descriptor.id differs from manifest schema_id"
    );
    assert_eq!(
        result.schema_descriptor.version,
        manifest["schema_version"].as_str().unwrap(),
        "echoed schema_descriptor.version differs from manifest schema_version"
    );
    assert_eq!(
        result.registry_descriptor.epoch,
        manifest["registry_epoch"].as_str().unwrap(),
        "echoed registry_descriptor.epoch differs from manifest registry_epoch"
    );
    assert_eq!(
        result.registry_descriptor.hash,
        manifest["registry_hash"].as_str().unwrap(),
        "echoed registry_descriptor.hash differs from manifest registry_hash"
    );
}
