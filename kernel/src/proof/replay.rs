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
    use crate::carrier::bytetrace::{ByteTraceHeaderV1, ByteTraceV1, TraceBundleV1};

    #[test]
    #[should_panic(expected = "M2")]
    fn replay_verify_stub_panics() {
        let bundle = TraceBundleV1 {
            trace: ByteTraceV1 {
                header: ByteTraceHeaderV1 {
                    schema_id: "test".into(),
                    schema_hash: "sha256:000".into(),
                    registry_epoch: "epoch-0".into(),
                    registry_hash: "sha256:000".into(),
                },
                frames: vec![],
            },
            compilation_manifest: vec![],
            input_payload: vec![],
        };
        let _ = replay_verify(&bundle);
    }
}
