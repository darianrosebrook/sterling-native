//! Binary that runs `RomeMini` through the harness and prints deterministic
//! output lines for cross-process verification.
//!
//! Used by S1-M3-DETERMINISM-CROSSPROC to verify that bundle production
//! is identical across different process environments.
//!
//! Usage: `harness_fixture`
//! Output: five lines, each `key=value`:
//!   `bundle_digest`=sha256:...
//!   `fixture_hash`=sha256:...
//!   `trace_payload_hash`=sha256:...
//!   `verification_verdict`=Match|...
//!   `artifact_count`=5

use sterling_harness::runner::run;
use sterling_harness::worlds::rome_mini::RomeMini;

fn main() {
    let bundle = run(&RomeMini).expect("harness run failed");

    // Extract the verification report to get payload_hash and verdict.
    let report = bundle
        .artifacts
        .get("verification_report.json")
        .expect("missing verification_report.json");
    let report_json: serde_json::Value =
        serde_json::from_slice(&report.content).expect("invalid report JSON");

    let payload_hash = report_json["payload_hash"]
        .as_str()
        .expect("missing payload_hash");
    let verdict = report_json["replay_verdict"]
        .as_str()
        .expect("missing replay_verdict");

    // Extract fixture content hash.
    let fixture = bundle
        .artifacts
        .get("fixture.json")
        .expect("missing fixture.json");

    println!("bundle_digest={}", bundle.digest.as_str());
    println!("fixture_hash={}", fixture.content_hash.as_str());
    println!("trace_payload_hash={payload_hash}");
    println!("verification_verdict={verdict}");
    println!("artifact_count={}", bundle.artifacts.len());
}
