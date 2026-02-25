//! `ByteTraceV1` binary writer: serializes traces to `.bst1` format.
//!
//! Pure byte transform. Header/footer canonical JSON produced exclusively
//! by `proof::canon::canonical_json_bytes`. No JSON/serde in frame encoding.
//!
//! # Wire layout
//!
//! ```text
//! [envelope_len:u16le][envelope:JSON]       -- NOT hashed
//! [magic:4 = "BST1"]                        -- hashed
//! [header_len:u16le][header:canonical JSON]  -- hashed
//! [body: fixed-stride frames]               -- hashed
//! [footer_len:u16le][footer:canonical JSON]  -- hashed
//! ```

use crate::carrier::bytetrace::{
    ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceHeaderV1, ByteTraceV1, BYTETRACE_V1_MAGIC,
    MAX_SECTION_LEN,
};
use crate::carrier::code32::Code32;
use crate::proof::canon::canonical_json_bytes;

/// Error during trace serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceWriteError {
    /// A JSON section exceeds `MAX_SECTION_LEN` (u16 max).
    SectionTooLong { section: String, len: usize },
    /// Frame dimensions do not match header.
    FrameDimensionMismatch { frame_index: usize, detail: String },
    /// Canonical JSON serialization failed.
    CanonError { detail: String },
    /// Frame count does not match header `step_count`.
    StepCountMismatch { header: usize, actual: usize },
    /// Size computation overflowed (checked arithmetic).
    DimensionOverflow { detail: String },
    /// Frame 0 does not use `INITIAL_STATE` sentinel or has non-zero `op_args`.
    BadInitialFrame { detail: String },
}

/// Serialize a `ByteTraceV1` to `.bst1` bytes.
///
/// # Errors
///
/// Returns [`TraceWriteError`] if any section exceeds u16 length,
/// frame dimensions mismatch, frame count != `step_count`, frame 0
/// is not the `INITIAL_STATE` sentinel, or dimensions overflow.
pub fn trace_to_bytes(trace: &ByteTraceV1) -> Result<Vec<u8>, TraceWriteError> {
    let stride = validate_trace(trace)?;

    // Serialize sections.
    let envelope_json = envelope_to_json(&trace.envelope);
    check_section_len("envelope", envelope_json.len())?;
    let header_json = header_to_canonical_json(&trace.header)?;
    check_section_len("header", header_json.len())?;
    let footer_json = footer_to_canonical_json(&trace.footer)?;
    check_section_len("footer", footer_json.len())?;

    // Build the byte stream (checked arithmetic for all size computations).
    let body_len = trace.frames.len().checked_mul(stride).ok_or_else(|| {
        TraceWriteError::DimensionOverflow {
            detail: "frames.len() * stride overflows".into(),
        }
    })?;
    let total_len = 2usize
        .checked_add(envelope_json.len())
        .and_then(|n| n.checked_add(4))
        .and_then(|n| n.checked_add(2))
        .and_then(|n| n.checked_add(header_json.len()))
        .and_then(|n| n.checked_add(body_len))
        .and_then(|n| n.checked_add(2))
        .and_then(|n| n.checked_add(footer_json.len()))
        .ok_or_else(|| TraceWriteError::DimensionOverflow {
            detail: "total output length overflows".into(),
        })?;
    let mut buf = Vec::with_capacity(total_len);

    // Envelope (not hashed).
    write_u16le(&mut buf, envelope_json.len());
    buf.extend_from_slice(&envelope_json);

    // Magic.
    buf.extend_from_slice(&BYTETRACE_V1_MAGIC);

    // Header.
    write_u16le(&mut buf, header_json.len());
    buf.extend_from_slice(&header_json);

    // Body (fixed-stride frames).
    for frame in &trace.frames {
        buf.extend_from_slice(&frame.op_code);
        buf.extend_from_slice(&frame.op_args);
        buf.extend_from_slice(&frame.result_identity);
        buf.extend_from_slice(&frame.result_status);
    }

    // Footer.
    write_u16le(&mut buf, footer_json.len());
    buf.extend_from_slice(&footer_json);

    Ok(buf)
}

/// Validate trace structure and return the frame stride.
fn validate_trace(trace: &ByteTraceV1) -> Result<usize, TraceWriteError> {
    if trace.frames.len() != trace.header.step_count {
        return Err(TraceWriteError::StepCountMismatch {
            header: trace.header.step_count,
            actual: trace.frames.len(),
        });
    }

    let stride = trace
        .header
        .frame_stride()
        .ok_or_else(|| TraceWriteError::DimensionOverflow {
            detail: "header dimensions cause arithmetic overflow in frame_stride".into(),
        })?;

    let total_slots = trace
        .header
        .layer_count
        .checked_mul(trace.header.slot_count)
        .ok_or_else(|| TraceWriteError::DimensionOverflow {
            detail: "layer_count * slot_count overflows".into(),
        })?;
    let identity_len =
        total_slots
            .checked_mul(4)
            .ok_or_else(|| TraceWriteError::DimensionOverflow {
                detail: "total_slots * 4 overflows".into(),
            })?;
    let status_len = total_slots;
    let args_len = trace.header.arg_slot_count.checked_mul(4).ok_or_else(|| {
        TraceWriteError::DimensionOverflow {
            detail: "arg_slot_count * 4 overflows".into(),
        }
    })?;

    // Validate frame 0 sentinel.
    if let Some(frame_0) = trace.frames.first() {
        let expected_op = Code32::INITIAL_STATE.to_le_bytes();
        if frame_0.op_code != expected_op {
            return Err(TraceWriteError::BadInitialFrame {
                detail: format!(
                    "frame 0 op_code {:?} != INITIAL_STATE {:?}",
                    frame_0.op_code, expected_op
                ),
            });
        }
        if frame_0.op_args.iter().any(|&b| b != 0) {
            return Err(TraceWriteError::BadInitialFrame {
                detail: "frame 0 op_args must be zero-filled".into(),
            });
        }
    }

    for (i, frame) in trace.frames.iter().enumerate() {
        let frame_len =
            4 + frame.op_args.len() + frame.result_identity.len() + frame.result_status.len();
        if frame_len != stride {
            return Err(TraceWriteError::FrameDimensionMismatch {
                frame_index: i,
                detail: format!("frame {i} size {frame_len} != stride {stride}"),
            });
        }
        if frame.op_args.len() != args_len {
            return Err(TraceWriteError::FrameDimensionMismatch {
                frame_index: i,
                detail: format!(
                    "frame {i} op_args len {} != expected {args_len}",
                    frame.op_args.len()
                ),
            });
        }
        if frame.result_identity.len() != identity_len {
            return Err(TraceWriteError::FrameDimensionMismatch {
                frame_index: i,
                detail: format!(
                    "frame {i} identity len {} != expected {identity_len}",
                    frame.result_identity.len()
                ),
            });
        }
        if frame.result_status.len() != status_len {
            return Err(TraceWriteError::FrameDimensionMismatch {
                frame_index: i,
                detail: format!(
                    "frame {i} status len {} != expected {status_len}",
                    frame.result_status.len()
                ),
            });
        }
    }

    Ok(stride)
}

/// Extract the hashed payload bytes from a serialized trace.
///
/// Returns `magic || header_json || body || footer_json` — the exact input
/// to `sha256(DOMAIN_BYTETRACE || ...)` for the payload hash.
///
/// This skips the envelope prefix and reassembles the hashed sections.
///
/// # Errors
///
/// Returns [`TraceWriteError::CanonError`] if header dimensions overflow.
pub fn extract_payload_bytes(trace: &ByteTraceV1) -> Result<Vec<u8>, TraceWriteError> {
    let header_json = header_to_canonical_json(&trace.header)?;
    let footer_json = footer_to_canonical_json(&trace.footer)?;

    let stride = trace
        .header
        .frame_stride()
        .ok_or_else(|| TraceWriteError::CanonError {
            detail: "header dimensions cause overflow".into(),
        })?;

    let body_len = trace.frames.len() * stride;
    let total = 4 + header_json.len() + body_len + footer_json.len();
    let mut buf = Vec::with_capacity(total);

    buf.extend_from_slice(&BYTETRACE_V1_MAGIC);
    buf.extend_from_slice(&header_json);
    for frame in &trace.frames {
        buf.extend_from_slice(&frame.op_code);
        buf.extend_from_slice(&frame.op_args);
        buf.extend_from_slice(&frame.result_identity);
        buf.extend_from_slice(&frame.result_status);
    }
    buf.extend_from_slice(&footer_json);

    Ok(buf)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn write_u16le(buf: &mut Vec<u8>, len: usize) {
    #[allow(clippy::cast_possible_truncation)]
    let len_u16 = len as u16; // caller must check via check_section_len first
    buf.extend_from_slice(&len_u16.to_le_bytes());
}

fn check_section_len(section: &str, len: usize) -> Result<(), TraceWriteError> {
    if len > MAX_SECTION_LEN {
        return Err(TraceWriteError::SectionTooLong {
            section: section.into(),
            len,
        });
    }
    Ok(())
}

/// Serialize envelope to JSON bytes (NOT canonical — envelope is not hashed).
///
/// Uses `serde_json` directly since envelope is observability-only.
fn envelope_to_json(envelope: &ByteTraceEnvelopeV1) -> Vec<u8> {
    let value = serde_json::json!({
        "runner_version": envelope.runner_version,
        "timestamp": envelope.timestamp,
        "trace_id": envelope.trace_id,
        "wall_time_ms": envelope.wall_time_ms,
    });
    // Envelope is not hashed, so we use serde_json compact serialization.
    // This is NOT canonical_json_bytes — envelope is explicitly excluded from
    // all hashing and is allowed to use serde_json directly.
    serde_json::to_vec(&value).expect("envelope serialization cannot fail")
}

/// Serialize header to canonical JSON bytes via `proof::canon::canonical_json_bytes`.
fn header_to_canonical_json(header: &ByteTraceHeaderV1) -> Result<Vec<u8>, TraceWriteError> {
    let value = serde_json::json!({
        "arg_slot_count": header.arg_slot_count,
        "codebook_hash": header.codebook_hash,
        "domain_id": header.domain_id,
        "fixture_hash": header.fixture_hash,
        "layer_count": header.layer_count,
        "registry_epoch_hash": header.registry_epoch_hash,
        "schema_version": header.schema_version,
        "slot_count": header.slot_count,
        "step_count": header.step_count,
    });
    canonical_json_bytes(&value).map_err(|e| TraceWriteError::CanonError {
        detail: format!("header: {e}"),
    })
}

/// Serialize footer to canonical JSON bytes via `proof::canon::canonical_json_bytes`.
fn footer_to_canonical_json(footer: &ByteTraceFooterV1) -> Result<Vec<u8>, TraceWriteError> {
    let mut map = serde_json::Map::new();
    map.insert(
        "suite_identity".into(),
        serde_json::Value::String(footer.suite_identity.clone()),
    );
    if let Some(ref digest) = footer.witness_store_digest {
        map.insert(
            "witness_store_digest".into(),
            serde_json::Value::String(digest.clone()),
        );
    }
    let value = serde_json::Value::Object(map);
    canonical_json_bytes(&value).map_err(|e| TraceWriteError::CanonError {
        detail: format!("footer: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carrier::bytetrace::ByteTraceFrameV1;
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
            op_args: vec![0; 4],                           // 1 arg slot * 4 bytes
            result_identity: vec![1, 0, 0, 0, 0, 0, 0, 0], // 1 layer * 2 slots * 4
            result_status: vec![0, 0],                     // 1 layer * 2 slots
        }
    }

    #[test]
    fn write_single_frame_trace() {
        let trace = ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(1),
            frames: vec![initial_frame()],
            footer: test_footer(),
        };
        let bytes = trace_to_bytes(&trace).unwrap();

        // Check magic is present.
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let magic_offset = 2 + env_len;
        assert_eq!(&bytes[magic_offset..magic_offset + 4], b"BST1");
    }

    #[test]
    fn write_rejects_step_count_mismatch() {
        let trace = ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(5),        // claims 5 steps
            frames: vec![initial_frame()], // only 1 frame
            footer: test_footer(),
        };
        let err = trace_to_bytes(&trace).unwrap_err();
        assert!(matches!(err, TraceWriteError::StepCountMismatch { .. }));
    }

    #[test]
    fn write_rejects_wrong_frame_dimensions() {
        let bad_frame = ByteTraceFrameV1 {
            op_code: Code32::INITIAL_STATE.to_le_bytes(),
            op_args: vec![0; 8], // wrong: should be 4 (1 arg slot)
            result_identity: vec![0; 8],
            result_status: vec![0, 0],
        };
        let trace = ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(1),
            frames: vec![bad_frame],
            footer: test_footer(),
        };
        let err = trace_to_bytes(&trace).unwrap_err();
        assert!(matches!(
            err,
            TraceWriteError::FrameDimensionMismatch { .. }
        ));
    }

    #[test]
    fn header_footer_use_canonical_json() {
        let header = test_header(1);
        let header_json = header_to_canonical_json(&header).unwrap();
        let header_str = std::str::from_utf8(&header_json).unwrap();

        // Canonical JSON: sorted keys, compact.
        assert!(header_str.starts_with("{\"arg_slot_count\":"));
        assert!(!header_str.contains(' ')); // no whitespace

        let footer = test_footer();
        let footer_json = footer_to_canonical_json(&footer).unwrap();
        let footer_str = std::str::from_utf8(&footer_json).unwrap();
        assert!(footer_str.starts_with("{\"suite_identity\":"));
    }

    #[test]
    fn footer_with_witness_digest() {
        let footer = ByteTraceFooterV1 {
            suite_identity: "sha256:ddd".into(),
            witness_store_digest: Some("sha256:eee".into()),
        };
        let json = footer_to_canonical_json(&footer).unwrap();
        let s = std::str::from_utf8(&json).unwrap();
        assert!(s.contains("\"witness_store_digest\":\"sha256:eee\""));
    }

    #[test]
    fn footer_without_witness_omits_field() {
        let footer = test_footer();
        let json = footer_to_canonical_json(&footer).unwrap();
        let s = std::str::from_utf8(&json).unwrap();
        assert!(!s.contains("witness_store_digest"));
    }

    #[test]
    fn extract_payload_excludes_envelope() {
        let trace = ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(1),
            frames: vec![initial_frame()],
            footer: test_footer(),
        };
        let payload = extract_payload_bytes(&trace).unwrap();

        // Payload starts with magic.
        assert_eq!(&payload[..4], b"BST1");

        // Full serialized bytes include envelope prefix; payload does not.
        let full = trace_to_bytes(&trace).unwrap();
        assert!(full.len() > payload.len());
    }

    #[test]
    fn write_deterministic_n10() {
        let trace = ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(1),
            frames: vec![initial_frame()],
            footer: test_footer(),
        };
        let first = trace_to_bytes(&trace).unwrap();
        for _ in 0..10 {
            assert_eq!(trace_to_bytes(&trace).unwrap(), first);
        }
    }
}
