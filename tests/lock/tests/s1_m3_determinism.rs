//! S1-M3 Determinism Tests: prove bundle production is deterministic.
//!
//! N=10 in-process runs must produce identical bundle digests, artifact
//! bytes, and manifest bytes.

use sterling_harness::runner::run;
use sterling_harness::worlds::rome_mini::RomeMini;

// ---------------------------------------------------------------------------
// S1-M3-DETERMINISM-INPROC: N=10
// ---------------------------------------------------------------------------

#[test]
fn bundle_digest_deterministic_n10() {
    let first = run(&RomeMini).unwrap();
    for i in 1..10 {
        let result = run(&RomeMini).unwrap();
        assert_eq!(
            first.digest.as_str(),
            result.digest.as_str(),
            "bundle digest differed on run {i}"
        );
    }
}

#[test]
fn all_artifact_bytes_deterministic_n10() {
    let first = run(&RomeMini).unwrap();
    for i in 1..10 {
        let result = run(&RomeMini).unwrap();
        for (name, artifact) in &first.artifacts {
            let other = result
                .artifacts
                .get(name)
                .unwrap_or_else(|| panic!("missing artifact {name} on run {i}"));
            assert_eq!(
                artifact.content, other.content,
                "artifact {name} bytes differed on run {i}"
            );
        }
    }
}

#[test]
fn manifest_bytes_deterministic_n10() {
    let first = run(&RomeMini).unwrap();
    for i in 1..10 {
        let result = run(&RomeMini).unwrap();
        assert_eq!(
            first.manifest, result.manifest,
            "manifest bytes differed on run {i}"
        );
        assert_eq!(
            first.digest_basis, result.digest_basis,
            "digest_basis bytes differed on run {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// Observational envelope mutation does not affect bundle digest
// ---------------------------------------------------------------------------

#[test]
fn bundle_digest_ignores_observational_envelope_mutation() {
    let bundle = run(&RomeMini).unwrap();
    let original_digest = bundle.digest.as_str().to_string();

    // Mutate the trace.bst1 envelope bytes (first few bytes after length prefix).
    // The envelope is the first section of .bst1 format:
    // [envelope_len:u16le][envelope:JSON]
    let trace_artifact = bundle.artifacts.get("trace.bst1").unwrap();
    let mut mutated_bst1 = trace_artifact.content.clone();

    // The envelope JSON starts at offset 2 (after u16le length prefix).
    // Flip a byte in the envelope.
    if mutated_bst1.len() > 4 {
        mutated_bst1[3] ^= 0x01;
    }

    // Rebuild the bundle with mutated trace.bst1.
    let mut artifacts: Vec<(String, Vec<u8>, bool)> = bundle
        .artifacts
        .values()
        .map(|a| {
            if a.name == "trace.bst1" {
                (a.name.clone(), mutated_bst1.clone(), a.normative)
            } else {
                (a.name.clone(), a.content.clone(), a.normative)
            }
        })
        .collect();
    artifacts.sort_by(|a, b| a.0.cmp(&b.0));

    let mutated_bundle = sterling_harness::bundle::build_bundle(artifacts).unwrap();

    // trace.bst1 is observational → bundle digest should be unchanged.
    assert_eq!(
        original_digest,
        mutated_bundle.digest.as_str(),
        "bundle digest changed when only observational trace.bst1 was mutated"
    );

    // But the manifest should differ (it includes all artifact hashes).
    assert_ne!(
        bundle.manifest, mutated_bundle.manifest,
        "manifest should differ when trace.bst1 content changes"
    );
}
