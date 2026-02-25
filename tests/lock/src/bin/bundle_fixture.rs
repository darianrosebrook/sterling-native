//! Binary that runs `RomeMini` through the harness, writes the bundle to a
//! temp directory, reads it back, verifies it, and prints deterministic
//! output lines for cross-process verification.
//!
//! Used by S1-M4-CROSSPROC-DETERMINISM to verify that persisted bundle
//! production and round-trip is identical across different process environments.
//!
//! Usage: `bundle_fixture`
//! Output: six lines, each `key=value`:
//!   `bundle_digest`=sha256:...
//!   `manifest_hash`=sha256:...
//!   `digest_basis_hash`=sha256:...
//!   `verification_verdict`=Match
//!   `artifact_count`=5
//!   `roundtrip`=ok

use sterling_harness::bundle::DOMAIN_BUNDLE_ARTIFACT;
use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};
use sterling_harness::runner::run;
use sterling_harness::worlds::rome_mini::RomeMini;
use sterling_kernel::proof::hash::canonical_hash;

fn main() {
    let bundle = run(&RomeMini).expect("harness run failed");

    // Write to temp directory.
    let dir = std::env::temp_dir().join(format!("sterling_bundle_fixture_{}", std::process::id()));
    // Clean up any previous run.
    let _ = std::fs::remove_dir_all(&dir);
    write_bundle_dir(&bundle, &dir).expect("write_bundle_dir failed");

    // Read back.
    let loaded = read_bundle_dir(&dir).expect("read_bundle_dir failed");

    // Verify from disk.
    verify_bundle_dir(&dir).expect("verify_bundle_dir failed");

    // Clean up.
    let _ = std::fs::remove_dir_all(&dir);

    // Extract verification report verdict.
    let report = loaded
        .artifacts
        .get("verification_report.json")
        .expect("missing verification_report.json");
    let report_json: serde_json::Value =
        serde_json::from_slice(&report.content).expect("invalid report JSON");
    let verdict = report_json["replay_verdict"]
        .as_str()
        .expect("missing replay_verdict");

    // Compute content hashes for manifest and digest_basis (using the same
    // domain as artifacts for consistency in the output contract).
    let manifest_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &loaded.manifest);
    let digest_basis_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &loaded.digest_basis);

    // Verify round-trip equivalence.
    let roundtrip = if loaded.digest.as_str() == bundle.digest.as_str()
        && loaded.manifest == bundle.manifest
        && loaded.digest_basis == bundle.digest_basis
    {
        "ok"
    } else {
        "MISMATCH"
    };

    println!("bundle_digest={}", loaded.digest.as_str());
    println!("manifest_hash={}", manifest_hash.as_str());
    println!("digest_basis_hash={}", digest_basis_hash.as_str());
    println!("verification_verdict={verdict}");
    println!("artifact_count={}", loaded.artifacts.len());
    println!("roundtrip={roundtrip}");
}
