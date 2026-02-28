//! Tiny binary that builds the M2 canonical test trace, serializes it,
//! and prints deterministic output lines for cross-process verification.
//!
//! Used by S1-M2-DETERMINISM-CROSSPROC to verify that trace write, read,
//! payload hash, and step chain are identical across different process
//! environments (cwd, locale, env).
//!
//! Usage: `trace_fixture`
//! Output: five lines, each `key=value`:
//!   `bst1_hex`=...
//!   `payload_hash`=sha256:...
//!   `step_chain_0`=sha256:...
//!   `step_chain_final`=sha256:...
//!   `replay_verdict`=Match

use lock_tests::m2_canonical_trace::canonical_test_trace;
use sterling_kernel::carrier::bytetrace::{ReplayVerdict, TraceBundleV1};
use sterling_kernel::carrier::trace_reader::bytes_to_trace;
use sterling_kernel::carrier::trace_writer::trace_to_bytes;
use sterling_kernel::operators::operator_registry::kernel_operator_registry;
use sterling_kernel::proof::replay::replay_verify;
use sterling_kernel::proof::trace_hash::{payload_hash, step_chain};

fn main() {
    let trace = canonical_test_trace();

    // Serialize
    let bst1_bytes = trace_to_bytes(&trace).unwrap();

    // Round-trip through reader
    let parsed = bytes_to_trace(&bst1_bytes).unwrap();
    let rebytes = trace_to_bytes(&parsed).unwrap();
    assert_eq!(
        bst1_bytes, rebytes,
        "round-trip produced different bytes — non-determinism detected"
    );

    // Payload hash
    let ph = payload_hash(&trace).unwrap();

    // Step chain
    let sc = step_chain(&trace).unwrap();

    // Replay
    let bundle = TraceBundleV1 {
        trace,
        compilation_manifest: vec![],
        input_payload: vec![],
    };
    let verdict = replay_verify(&bundle, &kernel_operator_registry()).unwrap();
    let verdict_str = match verdict {
        ReplayVerdict::Match => "Match",
        _ => "FAIL",
    };

    println!("bst1_hex={}", hex::encode(&bst1_bytes));
    println!("payload_hash={}", ph.as_str());
    println!("step_chain_0={}", sc.chain[0].as_str());
    println!("step_chain_final={}", sc.digest.as_str());
    println!("replay_verdict={verdict_str}");
}
