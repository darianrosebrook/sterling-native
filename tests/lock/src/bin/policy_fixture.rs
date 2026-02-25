//! Binary that runs `RomeMini` through the harness with policy enforcement
//! and prints deterministic output lines for cross-process verification.
//!
//! Used by S1-M5-DETERMINISM-CROSSPROC to verify that policy-bearing bundle
//! production is identical across different process environments.
//!
//! Usage: `policy_fixture`
//! Output: seven lines, each `key=value`:
//!   `bundle_digest`=sha256:...
//!   `policy_digest`=sha256:...
//!   `policy_snapshot_hash`=sha256:...
//!   `verification_verdict`=Match
//!   `artifact_count`=5
//!   `policy_normative`=true
//!   `roundtrip`=ok

use sterling_harness::bundle::DOMAIN_BUNDLE_ARTIFACT;
use sterling_harness::bundle_dir::{read_bundle_dir, verify_bundle_dir, write_bundle_dir};
use sterling_harness::runner::run;
use sterling_harness::worlds::rome_mini::RomeMini;
use sterling_kernel::proof::hash::canonical_hash;

fn main() {
    let bundle = run(&RomeMini).expect("harness run failed");

    // Write to temp directory.
    let dir = std::env::temp_dir().join(format!("sterling_policy_fixture_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_bundle_dir(&bundle, &dir).expect("write_bundle_dir failed");

    // Read back.
    let loaded = read_bundle_dir(&dir).expect("read_bundle_dir failed");

    // Verify from disk.
    verify_bundle_dir(&dir).expect("verify_bundle_dir failed");

    // Clean up.
    let _ = std::fs::remove_dir_all(&dir);

    // Extract verification report.
    let report = loaded
        .artifacts
        .get("verification_report.json")
        .expect("missing verification_report.json");
    let report_json: serde_json::Value =
        serde_json::from_slice(&report.content).expect("invalid report JSON");
    let verdict = report_json["replay_verdict"]
        .as_str()
        .expect("missing replay_verdict");
    let policy_digest = report_json["policy_digest"]
        .as_str()
        .expect("missing policy_digest");

    // Extract policy snapshot.
    let policy = loaded
        .artifacts
        .get("policy_snapshot.json")
        .expect("missing policy_snapshot.json");
    let policy_snapshot_hash = canonical_hash(DOMAIN_BUNDLE_ARTIFACT, &policy.content);

    // Verify round-trip.
    let roundtrip = if loaded.digest.as_str() == bundle.digest.as_str()
        && loaded.manifest == bundle.manifest
        && loaded.digest_basis == bundle.digest_basis
    {
        "ok"
    } else {
        "MISMATCH"
    };

    println!("bundle_digest={}", loaded.digest.as_str());
    println!("policy_digest={policy_digest}");
    println!("policy_snapshot_hash={}", policy_snapshot_hash.as_str());
    println!("verification_verdict={verdict}");
    println!("artifact_count={}", loaded.artifacts.len());
    println!("policy_normative={}", policy.normative);
    println!("roundtrip={roundtrip}");
}
