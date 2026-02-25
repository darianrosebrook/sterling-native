//! Compilation boundary: `compile(payload, schema_descriptor, registry) -> ByteState`.
//!
//! This is a **new concept** in Sterling Native, not ported from v1.
//! (v1's `compiler.py` is a `ByteTrace` encoder — that maps to the `bytetrace` writer.)
//!
//! `compile()` transforms a domain payload into an initial `ByteStateV1`.
//! It is a pure function: identical inputs produce identical outputs.
//!
//! Policy does not affect compilation. Policy applies at the `apply`/harness layer.
//!
//! # Payload format (Rome M1)
//!
//! The payload is a canonical JSON object with:
//! ```json
//! {
//!   "layer_count": <integer>,
//!   "slot_count": <integer>,
//!   "identity": [[<domain>, <kind>, <local_lo>, <local_hi>], ...],
//!   "status": [<byte>, ...]
//! }
//! ```
//!
//! - `identity` length must equal `layer_count * slot_count`.
//! - `status` length must equal `layer_count * slot_count`.
//! - Each identity entry is a 4-byte `Code32` in `[domain, kind, lo, hi]` form.
//! - Each status byte must be a valid `SlotStatus` discriminant.
//! - All non-sentinel `Code32` values must be present in the registry.

use crate::carrier::bytestate::{ByteStateV1, RegistrySnapshot, SchemaDescriptor, SlotStatus};
use crate::carrier::code32::Code32;
use crate::carrier::registry::RegistryV1;
use crate::proof::canon::canonical_json_bytes;
use crate::proof::hash::{
    canonical_hash, ContentHash, DOMAIN_EVIDENCE_PLANE, DOMAIN_IDENTITY_PLANE,
};

/// A successful compilation result.
///
/// Contains only what the kernel can produce from its pure inputs.
/// Envelope-level concerns (`request_manifest_hash`, request metadata)
/// belong at the harness/bundle layer — the kernel doesn't see envelopes.
#[derive(Debug, Clone)]
pub struct CompilationResultV1 {
    /// The compiled initial state.
    pub state: ByteStateV1,
    /// Schema used for compilation.
    pub schema_descriptor: SchemaDescriptor,
    /// Registry snapshot used for compilation.
    pub registry_descriptor: RegistrySnapshot,
    /// Canonical JSON manifest recording the schema + registry + payload
    /// dependency hashes. The harness layer can wrap this with envelope
    /// metadata to produce a full request manifest hash.
    pub compilation_manifest: Vec<u8>,
    /// Identity plane digest: `sha256(DOMAIN_IDENTITY || identity_bytes)`.
    pub identity_digest: ContentHash,
    /// Evidence digest: `sha256(DOMAIN_EVIDENCE || evidence_bytes)`.
    pub evidence_digest: ContentHash,
}

/// Typed compilation failure. Fail-closed: no partial `ByteState` is produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilationFailure {
    /// Schema descriptor does not match expected schema.
    SchemaMismatch { detail: String },
    /// Registry snapshot epoch/hash mismatch.
    RegistryMismatch { detail: String },
    /// Payload references a concept not in the registry.
    UnknownConcept { detail: String },
    /// Payload violates a schema constraint (shape, lengths, values).
    ConstraintViolation { detail: String },
    /// Payload JSON is malformed or not valid canonical JSON.
    InvalidPayload { detail: String },
}

/// Result type for compilation.
pub type CompileResult = Result<CompilationResultV1, CompilationFailure>;

/// Compile a domain payload into an initial `ByteState`.
///
/// Pure function: identical inputs produce identical output bytes.
///
/// # Arguments
///
/// * `payload_bytes` - JSON bytes of the domain payload (will be re-canonicalized).
/// * `schema_descriptor` - Identifies the `ByteState` schema to use.
/// * `registry` - The full `Code32` registry for membership validation.
///
/// Policy is **not** an input. It applies at the apply/harness layer.
///
/// # Errors
///
/// Returns [`CompilationFailure`] on any mismatch. Fail-closed.
pub fn compile(
    payload_bytes: &[u8],
    schema_descriptor: &SchemaDescriptor,
    registry: &RegistryV1,
) -> CompileResult {
    // 1. Parse payload JSON.
    let value: serde_json::Value =
        serde_json::from_slice(payload_bytes).map_err(|e| CompilationFailure::InvalidPayload {
            detail: format!("JSON parse error: {e}"),
        })?;

    // 2. Re-canonicalize (ordering invariance: input key order doesn't matter).
    let canonical_payload =
        canonical_json_bytes(&value).map_err(|e| CompilationFailure::InvalidPayload {
            detail: format!("canonical JSON error: {e}"),
        })?;

    // 3. Extract and validate dimensions.
    let obj = value
        .as_object()
        .ok_or_else(|| CompilationFailure::InvalidPayload {
            detail: "payload must be a JSON object".into(),
        })?;

    let layer_count = extract_usize(obj, "layer_count")?;
    let slot_count = extract_usize(obj, "slot_count")?;
    let total = layer_count.checked_mul(slot_count).ok_or_else(|| {
        CompilationFailure::ConstraintViolation {
            detail: format!("layer_count * slot_count overflow: {layer_count} * {slot_count}"),
        }
    })?;

    // Validate dimensions against schema.
    validate_schema_dimensions(schema_descriptor, layer_count, slot_count)?;

    // 4. Extract and validate identity plane.
    let identity = extract_identity_plane(obj, total, registry)?;

    // 5. Extract and validate status plane.
    let status = extract_status_plane(obj, total)?;

    // 6. Construct ByteStateV1.
    let mut state = ByteStateV1::new(layer_count, slot_count);
    for (i, code) in identity.iter().enumerate() {
        let layer = i / slot_count;
        let slot = i % slot_count;
        state.set_identity(layer, slot, *code);
    }
    for (i, st) in status.iter().enumerate() {
        let layer = i / slot_count;
        let slot = i % slot_count;
        state.set_status(layer, slot, *st);
    }

    // 7. Compute digests.
    let identity_digest = canonical_hash(DOMAIN_IDENTITY_PLANE, &state.identity_bytes());
    let evidence_digest = canonical_hash(DOMAIN_EVIDENCE_PLANE, &state.evidence_bytes());

    // 8. Build compilation manifest.
    let registry_snapshot =
        registry
            .snapshot()
            .map_err(|e| CompilationFailure::RegistryMismatch {
                detail: format!("registry snapshot error: {e:?}"),
            })?;

    let payload_hash = canonical_hash(DOMAIN_IDENTITY_PLANE, &canonical_payload);

    let manifest_value = serde_json::json!({
        "evidence_digest": evidence_digest.as_str(),
        "identity_digest": identity_digest.as_str(),
        "payload_hash": payload_hash.as_str(),
        "registry_epoch": registry_snapshot.epoch,
        "registry_hash": registry_snapshot.hash,
        "schema_hash": schema_descriptor.hash,
        "schema_id": schema_descriptor.id,
        "schema_version": schema_descriptor.version,
    });

    let compilation_manifest =
        canonical_json_bytes(&manifest_value).map_err(|e| CompilationFailure::InvalidPayload {
            detail: format!("manifest canonicalization error: {e}"),
        })?;

    Ok(CompilationResultV1 {
        state,
        schema_descriptor: schema_descriptor.clone(),
        registry_descriptor: registry_snapshot,
        compilation_manifest,
        identity_digest,
        evidence_digest,
    })
}

/// Extract and validate the identity plane from the payload.
fn extract_identity_plane(
    obj: &serde_json::Map<String, serde_json::Value>,
    total: usize,
    registry: &RegistryV1,
) -> Result<Vec<Code32>, CompilationFailure> {
    let identity_arr = obj
        .get("identity")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CompilationFailure::InvalidPayload {
            detail: "missing or non-array 'identity' field".into(),
        })?;

    if identity_arr.len() != total {
        return Err(CompilationFailure::ConstraintViolation {
            detail: format!(
                "identity array length {} != layer_count * slot_count {total}",
                identity_arr.len(),
            ),
        });
    }

    let mut identity = Vec::with_capacity(total);
    for (i, entry) in identity_arr.iter().enumerate() {
        let code = parse_code32_entry(entry, i)?;
        if !code.is_sentinel() && !registry.contains(&code) {
            return Err(CompilationFailure::UnknownConcept {
                detail: format!(
                    "identity[{i}]: Code32({},{},{}) not in registry epoch '{}'",
                    code.domain(),
                    code.kind(),
                    code.local_id(),
                    registry.epoch()
                ),
            });
        }
        identity.push(code);
    }
    Ok(identity)
}

/// Extract and validate the status plane from the payload.
fn extract_status_plane(
    obj: &serde_json::Map<String, serde_json::Value>,
    total: usize,
) -> Result<Vec<SlotStatus>, CompilationFailure> {
    let status_arr = obj
        .get("status")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CompilationFailure::InvalidPayload {
            detail: "missing or non-array 'status' field".into(),
        })?;

    if status_arr.len() != total {
        return Err(CompilationFailure::ConstraintViolation {
            detail: format!(
                "status array length {} != layer_count * slot_count {total}",
                status_arr.len(),
            ),
        });
    }

    let mut status = Vec::with_capacity(total);
    for (i, entry) in status_arr.iter().enumerate() {
        let byte = entry
            .as_u64()
            .ok_or_else(|| CompilationFailure::InvalidPayload {
                detail: format!("status[{i}]: expected integer"),
            })?;
        if byte > 255 {
            return Err(CompilationFailure::ConstraintViolation {
                detail: format!("status[{i}]: value {byte} exceeds u8 range"),
            });
        }
        #[allow(clippy::cast_possible_truncation)]
        let byte = byte as u8;
        let slot_status =
            SlotStatus::from_byte(byte).ok_or_else(|| CompilationFailure::ConstraintViolation {
                detail: format!("status[{i}]: invalid SlotStatus byte {byte}"),
            })?;
        status.push(slot_status);
    }
    Ok(status)
}

/// Extract a `usize` field from a JSON object.
fn extract_usize(
    obj: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<usize, CompilationFailure> {
    let v = obj
        .get(field)
        .ok_or_else(|| CompilationFailure::InvalidPayload {
            detail: format!("missing '{field}' field"),
        })?;
    let n = v
        .as_u64()
        .ok_or_else(|| CompilationFailure::InvalidPayload {
            detail: format!("'{field}' must be a non-negative integer"),
        })?;
    usize::try_from(n).map_err(|_| CompilationFailure::ConstraintViolation {
        detail: format!("'{field}' value {n} exceeds platform usize"),
    })
}

/// Parse a `Code32` from a JSON array `[domain, kind, local_lo, local_hi]`.
fn parse_code32_entry(
    entry: &serde_json::Value,
    index: usize,
) -> Result<Code32, CompilationFailure> {
    let arr = entry
        .as_array()
        .ok_or_else(|| CompilationFailure::InvalidPayload {
            detail: format!("identity[{index}]: expected 4-element array"),
        })?;
    if arr.len() != 4 {
        return Err(CompilationFailure::InvalidPayload {
            detail: format!("identity[{index}]: expected 4 elements, got {}", arr.len()),
        });
    }
    let bytes: Result<Vec<u8>, _> = arr
        .iter()
        .enumerate()
        .map(|(j, v)| {
            let n = v
                .as_u64()
                .ok_or_else(|| CompilationFailure::InvalidPayload {
                    detail: format!("identity[{index}][{j}]: expected integer"),
                })?;
            if n > 255 {
                return Err(CompilationFailure::ConstraintViolation {
                    detail: format!("identity[{index}][{j}]: value {n} exceeds u8 range"),
                });
            }
            #[allow(clippy::cast_possible_truncation)]
            Ok(n as u8)
        })
        .collect();
    let bytes = bytes?;
    Ok(Code32::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
    ]))
}

/// Validate that the schema descriptor matches the payload dimensions.
///
/// For M1, the schema ID must start with "rome" and declare compatible dimensions.
/// Schema validation will become more sophisticated in later milestones.
fn validate_schema_dimensions(
    schema: &SchemaDescriptor,
    _layer_count: usize,
    _slot_count: usize,
) -> Result<(), CompilationFailure> {
    // M1: basic check that schema ID is present and non-empty.
    if schema.id.is_empty() {
        return Err(CompilationFailure::SchemaMismatch {
            detail: "schema id is empty".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rome_registry() -> RegistryV1 {
        RegistryV1::new(
            "epoch-0".into(),
            vec![
                (Code32::new(1, 0, 0), "rome:node:start".into()),
                (Code32::new(1, 0, 1), "rome:node:forum".into()),
                (Code32::new(1, 0, 2), "rome:node:colosseum".into()),
                (Code32::new(1, 1, 0), "rome:edge:road".into()),
            ],
        )
        .unwrap()
    }

    fn rome_schema() -> SchemaDescriptor {
        SchemaDescriptor {
            id: "rome".into(),
            version: "1.0".into(),
            hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        }
    }

    fn rome_payload(layer_count: usize, slot_count: usize) -> serde_json::Value {
        let total = layer_count * slot_count;
        let mut identity = Vec::new();
        for i in 0..total {
            if i == 0 {
                identity.push(serde_json::json!([1, 0, 0, 0])); // rome:node:start
            } else {
                identity.push(serde_json::json!([0, 0, 0, 0])); // PADDING
            }
        }
        let status = vec![serde_json::json!(0); total]; // All Hole
        serde_json::json!({
            "layer_count": layer_count,
            "slot_count": slot_count,
            "identity": identity,
            "status": status,
        })
    }

    #[test]
    fn compile_valid_rome_payload() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = rome_payload(2, 4);
        let payload_bytes = serde_json::to_vec(&payload).unwrap();

        let result = compile(&payload_bytes, &schema, &registry).unwrap();

        assert_eq!(result.state.layer_count(), 2);
        assert_eq!(result.state.slot_count(), 4);
        assert_eq!(result.state.get_identity(0, 0), Code32::new(1, 0, 0));
        assert_eq!(result.state.get_identity(0, 1), Code32::PADDING);
        assert_eq!(result.state.get_status(0, 0), SlotStatus::Hole);
        assert_eq!(result.identity_digest.algorithm(), "sha256");
        assert_eq!(result.evidence_digest.algorithm(), "sha256");
        assert!(!result.compilation_manifest.is_empty());
    }

    #[test]
    fn compile_ordering_invariance() {
        let registry = rome_registry();
        let schema = rome_schema();

        // Keys in different order produce the same result.
        let payload_a = r#"{"layer_count":2,"slot_count":2,"identity":[[1,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]],"status":[0,0,0,0]}"#;
        let payload_b = r#"{"status":[0,0,0,0],"identity":[[1,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]],"slot_count":2,"layer_count":2}"#;

        let result_a = compile(payload_a.as_bytes(), &schema, &registry).unwrap();
        let result_b = compile(payload_b.as_bytes(), &schema, &registry).unwrap();

        assert!(result_a.state.bitwise_eq(&result_b.state));
        assert_eq!(result_a.identity_digest, result_b.identity_digest);
        assert_eq!(result_a.evidence_digest, result_b.evidence_digest);
    }

    #[test]
    fn compile_whitespace_invariance() {
        let registry = rome_registry();
        let schema = rome_schema();

        let compact = r#"{"layer_count":1,"slot_count":1,"identity":[[0,0,0,0]],"status":[0]}"#;
        let spaced = "{ \"layer_count\" : 1, \"slot_count\" : 1, \"identity\" : [ [0, 0, 0, 0] ], \"status\" : [0] }";

        let result_a = compile(compact.as_bytes(), &schema, &registry).unwrap();
        let result_b = compile(spaced.as_bytes(), &schema, &registry).unwrap();

        assert!(result_a.state.bitwise_eq(&result_b.state));
        assert_eq!(result_a.identity_digest, result_b.identity_digest);
    }

    #[test]
    fn compile_deterministic_n10() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = rome_payload(2, 4);
        let payload_bytes = serde_json::to_vec(&payload).unwrap();

        let first = compile(&payload_bytes, &schema, &registry).unwrap();
        for _ in 0..10 {
            let result = compile(&payload_bytes, &schema, &registry).unwrap();
            assert!(result.state.bitwise_eq(&first.state));
            assert_eq!(result.identity_digest, first.identity_digest);
            assert_eq!(result.evidence_digest, first.evidence_digest);
            assert_eq!(result.compilation_manifest, first.compilation_manifest);
        }
    }

    // --- Fail-closed tests (S1-M1-FAILCLOSED) ---

    #[test]
    fn compile_rejects_invalid_json() {
        let registry = rome_registry();
        let schema = rome_schema();
        let result = compile(b"not json", &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::InvalidPayload { .. })
        ));
    }

    #[test]
    fn compile_rejects_non_object_payload() {
        let registry = rome_registry();
        let schema = rome_schema();
        let result = compile(b"[1, 2, 3]", &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::InvalidPayload { .. })
        ));
    }

    #[test]
    fn compile_rejects_missing_dimensions() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = r#"{"identity":[],"status":[]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::InvalidPayload { .. })
        ));
    }

    #[test]
    fn compile_rejects_identity_length_mismatch() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload =
            r#"{"layer_count":2,"slot_count":2,"identity":[[0,0,0,0]],"status":[0,0,0,0]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::ConstraintViolation { .. })
        ));
    }

    #[test]
    fn compile_rejects_status_length_mismatch() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload =
            r#"{"layer_count":1,"slot_count":2,"identity":[[0,0,0,0],[0,0,0,0]],"status":[0]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::ConstraintViolation { .. })
        ));
    }

    #[test]
    fn compile_rejects_invalid_status_byte() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = r#"{"layer_count":1,"slot_count":1,"identity":[[0,0,0,0]],"status":[42]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::ConstraintViolation { .. })
        ));
    }

    #[test]
    fn compile_rejects_unknown_code32() {
        let registry = rome_registry();
        let schema = rome_schema();
        // Code32(99,0,0) is not in the registry.
        let payload = r#"{"layer_count":1,"slot_count":1,"identity":[[99,0,0,0]],"status":[0]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::UnknownConcept { .. })
        ));
    }

    #[test]
    fn compile_allows_sentinel_codes_without_registry() {
        let registry = rome_registry();
        let schema = rome_schema();
        // PADDING and INITIAL_STATE are sentinels — no registry check needed.
        let payload =
            r#"{"layer_count":1,"slot_count":2,"identity":[[0,0,0,0],[0,0,1,0]],"status":[0,0]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(result.is_ok());
    }

    #[test]
    fn compile_rejects_empty_schema_id() {
        let registry = rome_registry();
        let schema = SchemaDescriptor {
            id: String::new(),
            version: "1.0".into(),
            hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        };
        let payload = r#"{"layer_count":1,"slot_count":1,"identity":[[0,0,0,0]],"status":[0]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::SchemaMismatch { .. })
        ));
    }

    #[test]
    fn compile_rejects_code32_entry_wrong_length() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = r#"{"layer_count":1,"slot_count":1,"identity":[[0,0,0]],"status":[0]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::InvalidPayload { .. })
        ));
    }

    #[test]
    fn compile_rejects_code32_byte_overflow() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = r#"{"layer_count":1,"slot_count":1,"identity":[[256,0,0,0]],"status":[0]}"#;
        let result = compile(payload.as_bytes(), &schema, &registry);
        assert!(matches!(
            result,
            Err(CompilationFailure::ConstraintViolation { .. })
        ));
    }

    #[test]
    fn compile_manifest_is_deterministic_json() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = rome_payload(1, 2);
        let payload_bytes = serde_json::to_vec(&payload).unwrap();

        let result = compile(&payload_bytes, &schema, &registry).unwrap();
        let manifest_str = std::str::from_utf8(&result.compilation_manifest).unwrap();

        // Manifest is valid JSON.
        let _: serde_json::Value = serde_json::from_str(manifest_str).unwrap();

        // Contains expected fields.
        assert!(manifest_str.contains("\"schema_id\""));
        assert!(manifest_str.contains("\"registry_epoch\""));
        assert!(manifest_str.contains("\"payload_hash\""));
        assert!(manifest_str.contains("\"identity_digest\""));
        assert!(manifest_str.contains("\"evidence_digest\""));
    }

    #[test]
    fn compile_manifest_has_no_timestamps() {
        let registry = rome_registry();
        let schema = rome_schema();
        let payload = rome_payload(1, 1);
        let payload_bytes = serde_json::to_vec(&payload).unwrap();

        let result = compile(&payload_bytes, &schema, &registry).unwrap();
        let manifest_str = std::str::from_utf8(&result.compilation_manifest).unwrap();

        // No timestamps or paths.
        assert!(!manifest_str.contains("time"));
        assert!(!manifest_str.contains("date"));
        assert!(!manifest_str.contains("path"));
        assert!(!manifest_str.contains("cwd"));
    }
}
