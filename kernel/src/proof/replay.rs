//! `replay_verify()`: verify a trace bundle by deterministic replay.
//!
//! # M0 scope
//!
//! Types and signature only. Logic is M2 scope.

use crate::carrier::bytetrace::{ReplayVerdict, TraceBundleV1};

/// Error during replay (distinct from a divergence verdict).
///
/// A `ReplayError` means the trace could not be replayed at all.
/// A `ReplayVerdict::Divergence` means it was replayed but did not match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// The trace bundle is structurally malformed.
    MalformedBundle { detail: String },
    /// Schema or registry referenced by the trace is unavailable.
    MissingDependency { detail: String },
}

/// Result type for replay verification.
pub type ReplayResult = Result<ReplayVerdict, ReplayError>;

/// Verify a trace bundle by deterministic replay.
///
/// Re-executes the trace from the initial state, comparing each frame's
/// result against the recorded result. If all frames match, returns
/// [`ReplayVerdict::Match`]. Otherwise returns [`ReplayVerdict::Divergence`]
/// pointing to the first differing frame.
///
/// # Errors
///
/// Returns [`ReplayError`] if the bundle is malformed or dependencies are missing.
///
/// # Panics
///
/// M0 stub. Will panic until M2 implementation.
pub fn replay_verify(_trace_bundle: &TraceBundleV1) -> ReplayResult {
    todo!("M2: implement replay verification")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carrier::bytetrace::{
        ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceHeaderV1, ByteTraceV1, TraceBundleV1,
    };

    #[test]
    #[should_panic(expected = "M2")]
    fn replay_verify_stub_panics() {
        let bundle = TraceBundleV1 {
            trace: ByteTraceV1 {
                envelope: ByteTraceEnvelopeV1 {
                    timestamp: "2026-01-01T00:00:00Z".into(),
                    trace_id: "test-trace".into(),
                    runner_version: "0.0.1".into(),
                    wall_time_ms: 0,
                },
                header: ByteTraceHeaderV1 {
                    schema_version: "1.0".into(),
                    domain_id: "test".into(),
                    registry_epoch_hash: "sha256:000".into(),
                    codebook_hash: "sha256:000".into(),
                    fixture_hash: "sha256:000".into(),
                    step_count: 0,
                    layer_count: 1,
                    slot_count: 1,
                    arg_slot_count: 0,
                },
                frames: vec![],
                footer: ByteTraceFooterV1 {
                    suite_identity: "sha256:000".into(),
                    witness_store_digest: None,
                },
            },
            compilation_manifest: vec![],
            input_payload: vec![],
        };
        let _ = replay_verify(&bundle);
    }
}
