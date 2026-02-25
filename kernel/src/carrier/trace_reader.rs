//! `ByteTraceV1` binary reader: deserializes `.bst1` bytes into structs.
//!
//! Fail-closed: rejects truncated input, bad magic, wrong body length,
//! invalid `SlotStatus` discriminants, non-canonical header/footer bytes,
//! bad initial frame, and trailing bytes. No partial frames. No panics
//! on malformed input — all failure paths return typed [`TraceParseError`].
//!
//! # Canonical enforcement
//!
//! Header and footer bytes are verified to be in canonical JSON form
//! (sorted keys, compact, integers only) by re-serializing through
//! `proof::canon::canonical_json_bytes` and comparing byte-for-byte.
//! This ensures the payload hash claim surface is unambiguous: "hash of
//! bytes in the `.bst1`" and "hash of canonicalized semantics" are the
//! same thing, because non-canonical bytes are rejected at parse time.
//!
//! # Wire layout (same as `trace_writer`)
//!
//! ```text
//! [envelope_len:u16le][envelope:JSON]       -- NOT hashed
//! [magic:4 = "BST1"]                        -- hashed
//! [header_len:u16le][header:canonical JSON]  -- hashed
//! [body: fixed-stride frames]               -- hashed
//! [footer_len:u16le][footer:canonical JSON]  -- hashed
//! ```

use crate::carrier::bytestate::SlotStatus;
use crate::carrier::bytetrace::{
    ByteTraceEnvelopeV1, ByteTraceFooterV1, ByteTraceFrameV1, ByteTraceHeaderV1, ByteTraceV1,
    TraceParseError, BYTETRACE_V1_MAGIC, MAX_SECTION_LEN,
};
use crate::carrier::code32::Code32;
use crate::proof::canon::canonical_json_bytes;

/// Parse `.bst1` bytes into a `ByteTraceV1`.
///
/// # Errors
///
/// Returns [`TraceParseError`] if the input is truncated, has bad magic,
/// mismatched body length, contains invalid `SlotStatus` bytes,
/// non-canonical header/footer JSON, bad initial frame, or trailing bytes.
pub fn bytes_to_trace(data: &[u8]) -> Result<ByteTraceV1, TraceParseError> {
    let mut cursor = 0usize;

    // --- Envelope ---
    let envelope_len = read_u16le(data, &mut cursor, "envelope")?;
    check_section_len("envelope", envelope_len)?;
    let envelope_bytes = read_slice(data, &mut cursor, envelope_len, "envelope")?;
    let envelope = parse_envelope(envelope_bytes)?;

    // --- Magic ---
    let magic_bytes = read_slice(data, &mut cursor, 4, "magic")?;
    let mut magic = [0u8; 4];
    magic.copy_from_slice(magic_bytes);
    if magic != BYTETRACE_V1_MAGIC {
        return Err(TraceParseError::BadMagic { found: magic });
    }

    // --- Header (canonical enforcement) ---
    let header_len = read_u16le(data, &mut cursor, "header")?;
    check_section_len("header", header_len)?;
    let header_bytes = read_slice(data, &mut cursor, header_len, "header")?;
    let header = parse_header(header_bytes)?;
    verify_canonical(header_bytes, "header")?;

    // --- Body ---
    let stride = header
        .frame_stride()
        .ok_or_else(|| TraceParseError::DimensionOverflow {
            detail: "header dimensions cause arithmetic overflow".into(),
        })?;

    let expected_body_len =
        header
            .expected_body_len()
            .ok_or_else(|| TraceParseError::DimensionOverflow {
                detail: "step_count * stride overflows".into(),
            })?;

    let remaining = data.len().saturating_sub(cursor);
    if remaining < expected_body_len {
        return Err(TraceParseError::Truncated {
            detail: format!("body: need {expected_body_len} bytes but only {remaining} remain"),
        });
    }

    let body_bytes = read_slice(data, &mut cursor, expected_body_len, "body")?;
    let frames = parse_frames(body_bytes, &header, stride)?;

    // --- Frame 0 sentinel validation ---
    if !frames.is_empty() {
        validate_initial_frame(&frames[0], header.arg_slot_count)?;
    }

    // --- Footer (canonical enforcement) ---
    let footer_len = read_u16le(data, &mut cursor, "footer")?;
    check_section_len("footer", footer_len)?;
    let footer_bytes = read_slice(data, &mut cursor, footer_len, "footer")?;
    let footer = parse_footer(footer_bytes)?;
    verify_canonical(footer_bytes, "footer")?;

    // --- Reject trailing bytes ---
    if cursor != data.len() {
        return Err(TraceParseError::TrailingBytes {
            excess: data.len() - cursor,
        });
    }

    Ok(ByteTraceV1 {
        envelope,
        header,
        frames,
        footer,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn read_u16le(data: &[u8], cursor: &mut usize, section: &str) -> Result<usize, TraceParseError> {
    if *cursor + 2 > data.len() {
        return Err(TraceParseError::Truncated {
            detail: format!("{section}: need 2 bytes for length at offset {}", *cursor),
        });
    }
    let len = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]) as usize;
    *cursor += 2;
    Ok(len)
}

fn read_slice<'a>(
    data: &'a [u8],
    cursor: &mut usize,
    len: usize,
    section: &str,
) -> Result<&'a [u8], TraceParseError> {
    if *cursor + len > data.len() {
        return Err(TraceParseError::Truncated {
            detail: format!(
                "{section}: need {len} bytes at offset {} but only {} remain",
                *cursor,
                data.len() - *cursor
            ),
        });
    }
    let slice = &data[*cursor..*cursor + len];
    *cursor += len;
    Ok(slice)
}

fn check_section_len(section: &str, len: usize) -> Result<(), TraceParseError> {
    if len > MAX_SECTION_LEN {
        return Err(TraceParseError::SectionTooLong {
            section: section.into(),
            len,
        });
    }
    Ok(())
}

/// Verify that raw bytes are in canonical JSON form by re-serializing
/// and comparing byte-for-byte.
fn verify_canonical(raw_bytes: &[u8], section: &str) -> Result<(), TraceParseError> {
    let value: serde_json::Value =
        serde_json::from_slice(raw_bytes).map_err(|e| TraceParseError::NonCanonical {
            section: section.into(),
            detail: format!("JSON re-parse failed: {e}"),
        })?;
    let canonical = canonical_json_bytes(&value).map_err(|e| TraceParseError::NonCanonical {
        section: section.into(),
        detail: format!("canonical_json_bytes failed: {e:?}"),
    })?;
    if canonical != raw_bytes {
        return Err(TraceParseError::NonCanonical {
            section: section.into(),
            detail: "bytes are not in canonical JSON form".into(),
        });
    }
    Ok(())
}

/// Validate frame 0 sentinel: `op_code` must be `INITIAL_STATE`, `op_args` must be zero-filled.
fn validate_initial_frame(
    frame: &ByteTraceFrameV1,
    arg_slot_count: usize,
) -> Result<(), TraceParseError> {
    let expected_op_code = Code32::INITIAL_STATE.to_le_bytes();
    if frame.op_code != expected_op_code {
        return Err(TraceParseError::BadInitialFrame {
            detail: format!(
                "frame 0 op_code {:?} != INITIAL_STATE {:?}",
                frame.op_code, expected_op_code
            ),
        });
    }
    let expected_args_len = arg_slot_count * 4;
    if frame.op_args.len() != expected_args_len {
        return Err(TraceParseError::BadInitialFrame {
            detail: format!(
                "frame 0 op_args length {} != expected {}",
                frame.op_args.len(),
                expected_args_len
            ),
        });
    }
    if frame.op_args.iter().any(|&b| b != 0) {
        return Err(TraceParseError::BadInitialFrame {
            detail: "frame 0 op_args must be zero-filled".into(),
        });
    }
    Ok(())
}

fn parse_envelope(bytes: &[u8]) -> Result<ByteTraceEnvelopeV1, TraceParseError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| TraceParseError::InvalidEnvelope {
            detail: format!("JSON parse: {e}"),
        })?;
    let obj = value
        .as_object()
        .ok_or_else(|| TraceParseError::InvalidEnvelope {
            detail: "expected JSON object".into(),
        })?;

    let timestamp = get_string(obj, "timestamp", "envelope")?;
    let trace_id = get_string(obj, "trace_id", "envelope")?;
    let runner_version = get_string(obj, "runner_version", "envelope")?;
    let wall_time_ms = get_u64(obj, "wall_time_ms", "envelope")?;

    Ok(ByteTraceEnvelopeV1 {
        timestamp,
        trace_id,
        runner_version,
        wall_time_ms,
    })
}

fn parse_header(bytes: &[u8]) -> Result<ByteTraceHeaderV1, TraceParseError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| TraceParseError::InvalidHeader {
            detail: format!("JSON parse: {e}"),
        })?;
    let obj = value
        .as_object()
        .ok_or_else(|| TraceParseError::InvalidHeader {
            detail: "expected JSON object".into(),
        })?;

    let schema_version = get_string(obj, "schema_version", "header")?;
    let domain_id = get_string(obj, "domain_id", "header")?;
    let registry_epoch_hash = get_string(obj, "registry_epoch_hash", "header")?;
    let codebook_hash = get_string(obj, "codebook_hash", "header")?;
    let fixture_hash = get_string(obj, "fixture_hash", "header")?;
    let step_count = get_usize(obj, "step_count", "header")?;
    let layer_count = get_usize(obj, "layer_count", "header")?;
    let slot_count = get_usize(obj, "slot_count", "header")?;
    let arg_slot_count = get_usize(obj, "arg_slot_count", "header")?;

    Ok(ByteTraceHeaderV1 {
        schema_version,
        domain_id,
        registry_epoch_hash,
        codebook_hash,
        fixture_hash,
        step_count,
        layer_count,
        slot_count,
        arg_slot_count,
    })
}

fn parse_footer(bytes: &[u8]) -> Result<ByteTraceFooterV1, TraceParseError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| TraceParseError::InvalidFooter {
            detail: format!("JSON parse: {e}"),
        })?;
    let obj = value
        .as_object()
        .ok_or_else(|| TraceParseError::InvalidFooter {
            detail: "expected JSON object".into(),
        })?;

    let suite_identity = get_string(obj, "suite_identity", "footer")?;

    // Reject explicit null for witness_store_digest — spec says omit, not null.
    if let Some(v) = obj.get("witness_store_digest") {
        if v.is_null() {
            return Err(TraceParseError::InvalidFooter {
                detail: "witness_store_digest must be omitted when absent, not null".into(),
            });
        }
        let digest = v.as_str().ok_or_else(|| TraceParseError::InvalidFooter {
            detail: "witness_store_digest must be a string".into(),
        })?;
        return Ok(ByteTraceFooterV1 {
            suite_identity,
            witness_store_digest: Some(digest.to_string()),
        });
    }

    Ok(ByteTraceFooterV1 {
        suite_identity,
        witness_store_digest: None,
    })
}

fn parse_frames(
    body: &[u8],
    header: &ByteTraceHeaderV1,
    stride: usize,
) -> Result<Vec<ByteTraceFrameV1>, TraceParseError> {
    // Verify body length matches exactly.
    let expected = header.step_count * stride;
    if body.len() != expected {
        return Err(TraceParseError::BodyLengthMismatch {
            expected,
            actual: body.len(),
        });
    }

    let total_slots = header.layer_count * header.slot_count;
    let arg_bytes = header.arg_slot_count * 4;
    let identity_bytes = total_slots * 4;
    let status_bytes = total_slots;

    let mut frames = Vec::with_capacity(header.step_count);
    for i in 0..header.step_count {
        let offset = i * stride;
        let mut pos = offset;

        // op_code: 4 bytes
        let mut op_code = [0u8; 4];
        op_code.copy_from_slice(&body[pos..pos + 4]);
        pos += 4;

        // op_args
        let op_args = body[pos..pos + arg_bytes].to_vec();
        pos += arg_bytes;

        // result_identity
        let result_identity = body[pos..pos + identity_bytes].to_vec();
        pos += identity_bytes;

        // result_status — validate each byte
        let status_slice = &body[pos..pos + status_bytes];
        for &b in status_slice {
            if SlotStatus::from_byte(b).is_none() {
                return Err(TraceParseError::InvalidSlotStatus {
                    frame_index: i,
                    byte_value: b,
                });
            }
        }
        let result_status = status_slice.to_vec();

        frames.push(ByteTraceFrameV1 {
            op_code,
            op_args,
            result_identity,
            result_status,
        });
    }

    Ok(frames)
}

// ---------------------------------------------------------------------------
// JSON field helpers
// ---------------------------------------------------------------------------

fn get_string(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    section: &str,
) -> Result<String, TraceParseError> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| invalid_section(section, &format!("missing or non-string field \"{key}\"")))
}

fn get_u64(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    section: &str,
) -> Result<u64, TraceParseError> {
    obj.get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| invalid_section(section, &format!("missing or non-integer field \"{key}\"")))
}

fn get_usize(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    section: &str,
) -> Result<usize, TraceParseError> {
    let val = get_u64(obj, key, section)?;
    usize::try_from(val)
        .map_err(|_| invalid_section(section, &format!("\"{key}\" too large for usize")))
}

fn invalid_section(section: &str, detail: &str) -> TraceParseError {
    match section {
        "header" => TraceParseError::InvalidHeader {
            detail: detail.into(),
        },
        "footer" => TraceParseError::InvalidFooter {
            detail: detail.into(),
        },
        "envelope" => TraceParseError::InvalidEnvelope {
            detail: detail.into(),
        },
        _ => TraceParseError::Truncated {
            detail: detail.into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carrier::bytetrace::ByteTraceFrameV1;
    use crate::carrier::code32::Code32;
    use crate::carrier::trace_writer::trace_to_bytes;

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

    fn make_trace() -> ByteTraceV1 {
        ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(1),
            frames: vec![initial_frame()],
            footer: test_footer(),
        }
    }

    #[test]
    fn round_trip_single_frame() {
        let original = make_trace();
        let bytes = trace_to_bytes(&original).unwrap();
        let parsed = bytes_to_trace(&bytes).unwrap();
        assert_eq!(parsed.envelope, original.envelope);
        assert_eq!(parsed.header, original.header);
        assert_eq!(parsed.frames, original.frames);
        assert_eq!(parsed.footer, original.footer);
    }

    #[test]
    fn round_trip_multi_frame() {
        let frame2 = ByteTraceFrameV1 {
            op_code: Code32::new(1, 1, 0).to_le_bytes(),
            op_args: vec![0; 4],
            result_identity: vec![2, 0, 0, 0, 0, 0, 0, 0],
            result_status: vec![0, 64], // Hole, Shadow
        };
        let trace = ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(2),
            frames: vec![initial_frame(), frame2],
            footer: test_footer(),
        };
        let bytes = trace_to_bytes(&trace).unwrap();
        let parsed = bytes_to_trace(&bytes).unwrap();
        assert_eq!(parsed.frames.len(), 2);
        assert_eq!(parsed.frames[1].result_status, vec![0, 64]);
    }

    #[test]
    fn round_trip_footer_with_witness() {
        let trace = ByteTraceV1 {
            envelope: test_envelope(),
            header: test_header(1),
            frames: vec![initial_frame()],
            footer: ByteTraceFooterV1 {
                suite_identity: "sha256:ddd".into(),
                witness_store_digest: Some("sha256:eee".into()),
            },
        };
        let bytes = trace_to_bytes(&trace).unwrap();
        let parsed = bytes_to_trace(&bytes).unwrap();
        assert_eq!(
            parsed.footer.witness_store_digest,
            Some("sha256:eee".into())
        );
    }

    #[test]
    fn rejects_empty_input() {
        let err = bytes_to_trace(&[]).unwrap_err();
        assert!(matches!(err, TraceParseError::Truncated { .. }));
    }

    #[test]
    fn rejects_bad_magic() {
        let trace = make_trace();
        let mut bytes = trace_to_bytes(&trace).unwrap();
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let magic_offset = 2 + env_len;
        bytes[magic_offset] = b'X';
        let err = bytes_to_trace(&bytes).unwrap_err();
        assert!(matches!(err, TraceParseError::BadMagic { .. }));
    }

    #[test]
    fn rejects_truncated_body() {
        let trace = make_trace();
        let bytes = trace_to_bytes(&trace).unwrap();
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let header_offset = 2 + env_len + 4;
        let header_len =
            u16::from_le_bytes([bytes[header_offset], bytes[header_offset + 1]]) as usize;
        let body_start = header_offset + 2 + header_len;
        let truncated = &bytes[..body_start + 2];
        let err = bytes_to_trace(truncated).unwrap_err();
        assert!(matches!(err, TraceParseError::Truncated { .. }));
    }

    #[test]
    fn rejects_invalid_slot_status() {
        let trace = make_trace();
        let mut bytes = trace_to_bytes(&trace).unwrap();
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let header_offset = 2 + env_len + 4;
        let header_len =
            u16::from_le_bytes([bytes[header_offset], bytes[header_offset + 1]]) as usize;
        let body_start = header_offset + 2 + header_len;
        // Status bytes at body_start + 4 + 4 + 8 = body_start + 16
        let status_offset = body_start + 16;
        bytes[status_offset] = 42;
        let err = bytes_to_trace(&bytes).unwrap_err();
        assert!(matches!(
            err,
            TraceParseError::InvalidSlotStatus {
                frame_index: 0,
                byte_value: 42
            }
        ));
    }

    #[test]
    fn rejects_truncated_footer() {
        let trace = make_trace();
        let bytes = trace_to_bytes(&trace).unwrap();
        let truncated = &bytes[..bytes.len() - 1];
        let err = bytes_to_trace(truncated).unwrap_err();
        assert!(matches!(err, TraceParseError::Truncated { .. }));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let trace = make_trace();
        let mut bytes = trace_to_bytes(&trace).unwrap();
        bytes.push(0xFF); // garbage suffix
        let err = bytes_to_trace(&bytes).unwrap_err();
        assert!(matches!(err, TraceParseError::TrailingBytes { excess: 1 }));
    }

    #[test]
    fn rejects_non_canonical_header() {
        let trace = make_trace();
        let bytes = trace_to_bytes(&trace).unwrap();

        // Find header section and inject whitespace to break canonicality.
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let header_offset = 2 + env_len + 4; // after envelope + magic
        let header_len =
            u16::from_le_bytes([bytes[header_offset], bytes[header_offset + 1]]) as usize;
        let header_start = header_offset + 2;

        // Build a non-canonical header: add a space after first `{`.
        let header_json = &bytes[header_start..header_start + header_len];
        let mut bad_header = Vec::with_capacity(header_json.len() + 1);
        bad_header.push(b'{');
        bad_header.push(b' '); // whitespace = non-canonical
        bad_header.extend_from_slice(&header_json[1..]);

        // Rebuild the bytes with the bad header and updated length.
        #[allow(clippy::cast_possible_truncation)]
        let new_header_len_u16 = bad_header.len() as u16; // test value known < u16::MAX
        let mut rebuilt = Vec::new();
        rebuilt.extend_from_slice(&bytes[..header_offset]);
        rebuilt.extend_from_slice(&new_header_len_u16.to_le_bytes());
        rebuilt.extend_from_slice(&bad_header);
        rebuilt.extend_from_slice(&bytes[header_start + header_len..]);

        let err = bytes_to_trace(&rebuilt).unwrap_err();
        assert!(matches!(err, TraceParseError::NonCanonical { .. }));
    }

    #[test]
    fn rejects_null_witness_store_digest() {
        // Build valid bytes, then manually replace footer with one that has null.
        let trace = make_trace();
        let bytes = trace_to_bytes(&trace).unwrap();

        // Find footer section.
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let header_offset = 2 + env_len + 4;
        let header_len =
            u16::from_le_bytes([bytes[header_offset], bytes[header_offset + 1]]) as usize;
        let body_start = header_offset + 2 + header_len;
        let stride = trace.header.frame_stride().unwrap();
        let body_len = trace.header.step_count * stride;
        let footer_offset = body_start + body_len;

        // Build footer with explicit null (not canonical, but test the null rejection).
        let bad_footer = br#"{"suite_identity":"sha256:ddd","witness_store_digest":null}"#;
        #[allow(clippy::cast_possible_truncation)]
        let footer_len_u16 = bad_footer.len() as u16; // test value known < u16::MAX
        let mut rebuilt = Vec::new();
        rebuilt.extend_from_slice(&bytes[..footer_offset]);
        rebuilt.extend_from_slice(&footer_len_u16.to_le_bytes());
        rebuilt.extend_from_slice(bad_footer);

        let err = bytes_to_trace(&rebuilt).unwrap_err();
        // Could be InvalidFooter (null rejection) or NonCanonical (not canonical form).
        // The null check happens before canonical check in parse_footer, so expect InvalidFooter.
        assert!(
            matches!(err, TraceParseError::InvalidFooter { .. })
                || matches!(err, TraceParseError::NonCanonical { .. })
        );
    }

    #[test]
    fn rejects_bad_initial_frame_sentinel() {
        let mut trace = make_trace();
        trace.frames[0].op_code = Code32::new(1, 1, 1).to_le_bytes();
        // Can't use trace_to_bytes (writer also validates), so build raw bytes manually.
        // Actually — the writer doesn't validate frame 0 sentinel yet (that's fix #31).
        // For now, test that the reader rejects it by building bytes that bypass the writer.
        // Let's build from a valid trace and patch the op_code in the body.
        let valid_trace = make_trace();
        let mut bytes = trace_to_bytes(&valid_trace).unwrap();
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let header_offset = 2 + env_len + 4;
        let header_len =
            u16::from_le_bytes([bytes[header_offset], bytes[header_offset + 1]]) as usize;
        let body_start = header_offset + 2 + header_len;
        // Patch frame 0 op_code (first 4 bytes of body).
        let bad_code = Code32::new(1, 1, 1).to_le_bytes();
        bytes[body_start..body_start + 4].copy_from_slice(&bad_code);
        let err = bytes_to_trace(&bytes).unwrap_err();
        assert!(matches!(err, TraceParseError::BadInitialFrame { .. }));
    }

    #[test]
    fn rejects_nonzero_initial_frame_args() {
        let valid_trace = make_trace();
        let mut bytes = trace_to_bytes(&valid_trace).unwrap();
        let env_len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        let header_offset = 2 + env_len + 4;
        let header_len =
            u16::from_le_bytes([bytes[header_offset], bytes[header_offset + 1]]) as usize;
        let body_start = header_offset + 2 + header_len;
        // op_args start at body_start + 4 (after op_code)
        bytes[body_start + 4] = 0xFF; // non-zero arg
        let err = bytes_to_trace(&bytes).unwrap_err();
        assert!(matches!(err, TraceParseError::BadInitialFrame { .. }));
    }

    #[test]
    fn deterministic_round_trip_n10() {
        let trace = make_trace();
        let bytes = trace_to_bytes(&trace).unwrap();
        for _ in 0..10 {
            let parsed = bytes_to_trace(&bytes).unwrap();
            let rebytes = trace_to_bytes(&parsed).unwrap();
            assert_eq!(bytes, rebytes);
        }
    }
}
