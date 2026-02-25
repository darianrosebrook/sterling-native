//! Trace hashing: payload hash and step chain digest.
//!
//! Two independent claim surfaces, computed from the same trace but used
//! for different purposes:
//!
//! - **Payload hash** (V1-compatible): whole-trace digest over
//!   `magic || header_json || body || footer_json`. Matches v1's
//!   `compute_payload_hash()` exactly.
//!
//! - **Step chain** (Native-originated): per-frame hash chain for
//!   O(1) divergence localization. Not present in v1.
//!
//! These two digests are never mixed. They are separate columns in the
//! claim/certificate surface.

use crate::carrier::bytetrace::ByteTraceV1;
use crate::carrier::trace_writer::{extract_payload_bytes, TraceWriteError};
use crate::proof::hash::{
    canonical_hash, ContentHash, DOMAIN_BYTETRACE, DOMAIN_TRACE_STEP, DOMAIN_TRACE_STEP_CHAIN,
};

/// Error during trace hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceHashError {
    /// Payload extraction failed (dimension overflow, etc.).
    PayloadExtraction { detail: String },
    /// Trace has no frames (step chain requires at least one frame).
    EmptyTrace,
}

impl From<TraceWriteError> for TraceHashError {
    fn from(e: TraceWriteError) -> Self {
        TraceHashError::PayloadExtraction {
            detail: format!("{e:?}"),
        }
    }
}

/// Compute the V1-compatible payload hash of a trace.
///
/// Formula: `sha256(DOMAIN_BYTETRACE || magic || header_json || body || footer_json)`
///
/// The envelope is excluded. The magic bytes are included.
///
/// # Errors
///
/// Returns [`TraceHashError`] if payload extraction fails.
pub fn payload_hash(trace: &ByteTraceV1) -> Result<ContentHash, TraceHashError> {
    let payload_bytes = extract_payload_bytes(trace)?;
    Ok(canonical_hash(DOMAIN_BYTETRACE, &payload_bytes))
}

/// Compute the step hash chain digest of a trace.
///
/// Formula:
/// - `chain_0 = sha256(DOMAIN_TRACE_STEP || frame_0_bytes)`
/// - `chain_i = sha256(DOMAIN_TRACE_STEP_CHAIN || chain_{i-1} || frame_i_bytes)`
///
/// Returns the final chain value as a [`ContentHash`], plus the full
/// chain of intermediate digests (one per frame).
///
/// # Errors
///
/// Returns [`TraceHashError::EmptyTrace`] if the trace has no frames.
pub fn step_chain(trace: &ByteTraceV1) -> Result<StepChainResult, TraceHashError> {
    if trace.frames.is_empty() {
        return Err(TraceHashError::EmptyTrace);
    }

    let mut chain: Vec<ContentHash> = Vec::with_capacity(trace.frames.len());

    // chain_0 = sha256(DOMAIN_TRACE_STEP || frame_0_bytes)
    let frame_0_bytes = trace.frames[0].to_bytes();
    let mut prev = canonical_hash(DOMAIN_TRACE_STEP, &frame_0_bytes);
    chain.push(prev.clone());

    // chain_i = sha256(DOMAIN_TRACE_STEP_CHAIN || chain_{i-1} || frame_i_bytes)
    for frame in &trace.frames[1..] {
        let prev_digest_bytes = hex_digest_to_bytes(prev.hex_digest());
        let frame_bytes = frame.to_bytes();

        let mut input = Vec::with_capacity(prev_digest_bytes.len() + frame_bytes.len());
        input.extend_from_slice(&prev_digest_bytes);
        input.extend_from_slice(&frame_bytes);

        prev = canonical_hash(DOMAIN_TRACE_STEP_CHAIN, &input);
        chain.push(prev.clone());
    }

    Ok(StepChainResult {
        digest: prev,
        chain,
    })
}

/// Decode a hex digest string to raw bytes.
///
/// Internal helper. `canonical_hash` always returns valid lowercase hex,
/// so this cannot fail in practice. Uses a manual decode to avoid
/// `expect`/`unwrap` that would trigger clippy panics warnings.
fn hex_digest_to_bytes(hex_str: &str) -> Vec<u8> {
    // canonical_hash guarantees 64-char lowercase hex (SHA-256).
    // Fall back to empty vec if somehow invalid (fail-safe, not fail-open:
    // a wrong digest just means the hash won't match, not UB).
    hex::decode(hex_str).unwrap_or_default()
}

/// Result of computing the step hash chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepChainResult {
    /// The final chain digest (last element of `chain`).
    pub digest: ContentHash,
    /// One digest per frame, in order. `chain[0]` is the initial step
    /// commitment, `chain[n-1]` is the final digest.
    pub chain: Vec<ContentHash>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carrier::bytetrace::{
        ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceFrameV1, ByteTraceHeaderV1,
    };
    use crate::carrier::code32::Code32;

    fn test_envelope() -> ByteTraceEnvelopeV1 {
        ByteTraceEnvelopeV1 {
            timestamp: "2026-01-01T00:00:00Z".into(),
            trace_id: "test-trace-001".into(),
            runner_version: "0.0.1".into(),
            wall_time_ms: 42,
        }
    }

    fn test_header(step_count: usize) -> ByteTraceHeaderV1 {
        ByteTraceHeaderV1 {
            schema_version: "1.0".into(),
            domain_id: "rome".into(),
            registry_epoch_hash: "sha256:aaa".into(),
            codebook_hash: "sha256:bbb".into(),
            fixture_hash: "sha256:ccc".into(),
            step_count,
            layer_count: 1,
            slot_count: 2,
            arg_slot_count: 1,
        }
    }

    fn test_footer() -> ByteTraceFooterV1 {
        ByteTraceFooterV1 {
            suite_identity: "sha256:ddd".into(),
            witness_store_digest: None,
        }
    }

    fn initial_frame() -> ByteTraceFrameV1 {
        ByteTraceFrameV1 {
            op_code: Code32::INITIAL_STATE.to_le_bytes(),
            op_args: vec![0; 4],
            result_identity: vec![1, 0, 0, 0, 0, 0, 0, 0],
            result_status: vec![0, 0],
        }
    }

    fn second_frame() -> ByteTraceFrameV1 {
        ByteTraceFrameV1 {
            op_code: Code32::new(1, 1, 0).to_le_bytes(),
            op_args: vec![0; 4],
            result_identity: vec![2, 0, 0, 0, 0, 0, 0, 0],
            result_status: vec![0, 64], // Hole, Shadow
        }
    }

    fn make_trace(step_count: usize, frames: Vec<ByteTraceFrameV1>) -> ByteTraceV1 {
        ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(step_count),
            frames,
            footer: test_footer(),
        }
    }

    #[test]
    fn payload_hash_is_sha256() {
        let trace = make_trace(1, vec![initial_frame()]);
        let hash = payload_hash(&trace).unwrap();
        assert_eq!(hash.algorithm(), "sha256");
        assert_eq!(hash.hex_digest().len(), 64);
    }

    #[test]
    fn payload_hash_excludes_envelope() {
        let trace1 = make_trace(1, vec![initial_frame()]);
        let mut trace2 = make_trace(1, vec![initial_frame()]);
        trace2.envelope.trace_id = "completely-different-id".into();
        trace2.envelope.wall_time_ms = 999_999;

        let h1 = payload_hash(&trace1).unwrap();
        let h2 = payload_hash(&trace2).unwrap();
        assert_eq!(h1, h2, "payload hash must not depend on envelope");
    }

    #[test]
    fn payload_hash_deterministic_n10() {
        let trace = make_trace(1, vec![initial_frame()]);
        let first = payload_hash(&trace).unwrap();
        for _ in 0..10 {
            assert_eq!(payload_hash(&trace).unwrap(), first);
        }
    }

    #[test]
    fn step_chain_single_frame() {
        let trace = make_trace(1, vec![initial_frame()]);
        let result = step_chain(&trace).unwrap();
        assert_eq!(result.chain.len(), 1);
        assert_eq!(result.digest, result.chain[0]);
        assert_eq!(result.digest.algorithm(), "sha256");
    }

    #[test]
    fn step_chain_two_frames() {
        let trace = make_trace(2, vec![initial_frame(), second_frame()]);
        let result = step_chain(&trace).unwrap();
        assert_eq!(result.chain.len(), 2);
        assert_ne!(result.chain[0], result.chain[1]);
        assert_eq!(result.digest, result.chain[1]);
    }

    #[test]
    fn step_chain_empty_trace_errors() {
        let trace = make_trace(0, vec![]);
        let err = step_chain(&trace).unwrap_err();
        assert!(matches!(err, TraceHashError::EmptyTrace));
    }

    #[test]
    fn step_chain_deterministic_n10() {
        let trace = make_trace(2, vec![initial_frame(), second_frame()]);
        let first = step_chain(&trace).unwrap();
        for _ in 0..10 {
            assert_eq!(step_chain(&trace).unwrap(), first);
        }
    }

    #[test]
    fn payload_hash_and_step_chain_are_independent() {
        let trace = make_trace(1, vec![initial_frame()]);
        let ph = payload_hash(&trace).unwrap();
        let sc = step_chain(&trace).unwrap();
        assert_ne!(
            ph.hex_digest(),
            sc.digest.hex_digest(),
            "payload hash and step chain must be distinct claim surfaces"
        );
    }

    #[test]
    fn payload_hash_changes_with_footer() {
        let trace1 = make_trace(1, vec![initial_frame()]);
        let mut trace2 = make_trace(1, vec![initial_frame()]);
        trace2.footer.suite_identity = "sha256:fff".into();

        let h1 = payload_hash(&trace1).unwrap();
        let h2 = payload_hash(&trace2).unwrap();
        assert_ne!(h1, h2, "payload hash must change when footer changes");
    }

    #[test]
    fn step_chain_ignores_footer() {
        let trace1 = make_trace(1, vec![initial_frame()]);
        let mut trace2 = make_trace(1, vec![initial_frame()]);
        trace2.footer.suite_identity = "sha256:fff".into();

        let sc1 = step_chain(&trace1).unwrap();
        let sc2 = step_chain(&trace2).unwrap();
        assert_eq!(
            sc1.digest, sc2.digest,
            "step chain depends only on frames, not footer"
        );
    }
}
