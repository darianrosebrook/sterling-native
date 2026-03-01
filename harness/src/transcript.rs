//! Tool transcript rendering: derived projection from `SearchTape`.
//!
//! `render_tool_transcript()` is a pure, deterministic function that
//! extracts tool-operator frames from a parsed `SearchTape` and renders
//! them as canonical JSON bytes. The transcript is a derived artifact
//! (like `SearchGraphV1`): the tape is authoritative, the transcript is
//! a human-readable derived view that must be byte-identical to a
//! deterministic rendering from the tape.
//!
//! # Cert equivalence
//!
//! The verifier independently renders the transcript from the tape +
//! operator registry, then asserts byte-identical match to the
//! bundle-shipped `tool_transcript.json`. This closes the correspondence
//! gap (analogous to tape→graph equivalence).

use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::operators::apply::{OP_COMMIT, OP_ROLLBACK, OP_STAGE};
use sterling_kernel::operators::operator_registry::OperatorRegistryV1;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_search::tape::{
    SearchTapeV1, TapeCandidateOutcomeV1, TapeCandidateV1, TapeRecordV1,
};

/// Tool operator codes that are recorded in the transcript.
const TOOL_OPS: [Code32; 3] = [OP_STAGE, OP_COMMIT, OP_ROLLBACK];

/// Check whether a `Code32` is a tool operator.
fn is_tool_op(code: Code32) -> bool {
    TOOL_OPS.contains(&code)
}

/// Render `tool_transcript.json` canonical bytes from a parsed tape.
///
/// Iterates expansion records on the winning path (applied candidates),
/// extracts frames where `op_code` matches a tool operator
/// (`OP_STAGE`/`OP_COMMIT`/`OP_ROLLBACK`), and renders each as a
/// transcript entry. Output is canonical JSON bytes.
///
/// Returns `None` if the tape contains no tool-operator frames (callers
/// should not emit the artifact in that case, unless `evidence_obligations`
/// requires it — in which case an empty-entries transcript is valid).
///
/// # Errors
///
/// Returns an error string if canonical JSON serialization fails.
pub fn render_tool_transcript(
    tape: &SearchTapeV1,
    operator_registry: &OperatorRegistryV1,
    world_id: &str,
) -> Result<Vec<u8>, String> {
    let mut entries = Vec::new();

    // Walk expansions in tape order (which is expansion_order).
    // For each expansion, check applied candidates for tool ops.
    for record in &tape.records {
        let TapeRecordV1::Expansion(expansion) = record else {
            continue;
        };

        for candidate in &expansion.candidates {
            // Only include applied candidates (successful operations).
            if !matches!(candidate.outcome, TapeCandidateOutcomeV1::Applied { .. }) {
                continue;
            }

            let op_code = Code32::from_le_bytes(candidate.op_code_bytes);
            if !is_tool_op(op_code) {
                continue;
            }

            let entry = build_transcript_entry(
                expansion.expansion_order,
                candidate,
                op_code,
                operator_registry,
            );
            entries.push(entry);
        }
    }

    let entry_count = entries.len();

    let transcript = serde_json::json!({
        "entries": entries,
        "entry_count": entry_count,
        "schema_version": "tool_transcript.v1",
        "txn_epoch": 0,
        "world_id": world_id,
    });

    canonical_json_bytes(&transcript).map_err(|e| format!("canonical JSON error: {e:?}"))
}

/// Check if a tape contains any applied tool-operator frames.
///
/// Used for the belt-and-suspenders cross-check: if the tape contains
/// tool ops but `evidence_obligations` doesn't include `tool_transcript_v1`,
/// Cert fails with `ObligationMismatch`.
#[must_use]
pub fn tape_contains_tool_ops(tape: &SearchTapeV1) -> bool {
    for record in &tape.records {
        let TapeRecordV1::Expansion(expansion) = record else {
            continue;
        };
        for candidate in &expansion.candidates {
            if !matches!(candidate.outcome, TapeCandidateOutcomeV1::Applied { .. }) {
                continue;
            }
            let op_code = Code32::from_le_bytes(candidate.op_code_bytes);
            if is_tool_op(op_code) {
                return true;
            }
        }
    }
    false
}

/// Build a single transcript entry JSON value from a tape candidate.
fn build_transcript_entry(
    expansion_order: u64,
    candidate: &TapeCandidateV1,
    op_code: Code32,
    operator_registry: &OperatorRegistryV1,
) -> serde_json::Value {
    let [d, k, lo, hi] = op_code.to_le_bytes();

    // Look up operator name from registry; fall back to hex if not found.
    let operator_name = operator_registry
        .get(&op_code)
        .map_or_else(
            || format!("unknown_0x{d:02x}{k:02x}{lo:02x}{hi:02x}"),
            |e| e.name.clone(),
        );

    // Build structured args based on operator type.
    let args = build_structured_args(op_code, &candidate.op_args);

    serde_json::json!({
        "args": args,
        "op_code": [d, k, lo, hi],
        "operator": operator_name,
        "outcome": "applied",
        "step_index": expansion_order,
    })
}

/// Build structured args object from raw `op_args` bytes.
///
/// Operator-specific parsing:
/// - `OP_STAGE`: `{layer, slot, value}` (3 × 4 bytes = 12)
/// - `OP_COMMIT`: `{layer}` (1 × 4 bytes = 4)
/// - `OP_ROLLBACK`: `{layer}` (1 × 4 bytes = 4)
fn build_structured_args(op_code: Code32, op_args: &[u8]) -> serde_json::Value {
    if op_code == OP_STAGE && op_args.len() >= 12 {
        let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]);
        let slot = u32::from_le_bytes([op_args[4], op_args[5], op_args[6], op_args[7]]);
        let value_bytes = [op_args[8], op_args[9], op_args[10], op_args[11]];
        let value = Code32::from_le_bytes(value_bytes);
        let [vd, vk, vlo, vhi] = value.to_le_bytes();
        return serde_json::json!({
            "layer": layer,
            "slot": slot,
            "value": [vd, vk, vlo, vhi],
        });
    } else if (op_code == OP_COMMIT || op_code == OP_ROLLBACK) && op_args.len() >= 4 {
        let layer = u32::from_le_bytes([op_args[0], op_args[1], op_args[2], op_args[3]]);
        return serde_json::json!({
            "layer": layer,
        });
    }

    // Fallback: hex-encode raw args.
    serde_json::json!({
        "raw_hex": hex::encode(op_args),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sterling_kernel::operators::operator_registry::kernel_operator_registry;

    #[test]
    fn is_tool_op_identifies_tool_operators() {
        assert!(is_tool_op(OP_STAGE));
        assert!(is_tool_op(OP_COMMIT));
        assert!(is_tool_op(OP_ROLLBACK));
        assert!(!is_tool_op(sterling_kernel::operators::apply::OP_SET_SLOT));
    }

    #[test]
    fn build_structured_args_stage() {
        let args = sterling_kernel::operators::apply::stage_args(1, 2, Code32::new(2, 1, 0));
        let result = build_structured_args(OP_STAGE, &args);
        assert_eq!(result["layer"], 1);
        assert_eq!(result["slot"], 2);
        assert_eq!(result["value"], serde_json::json!([2, 1, 0, 0]));
    }

    #[test]
    fn build_structured_args_commit() {
        let args = sterling_kernel::operators::apply::commit_args(1);
        let result = build_structured_args(OP_COMMIT, &args);
        assert_eq!(result["layer"], 1);
    }

    #[test]
    fn build_structured_args_rollback() {
        let args = sterling_kernel::operators::apply::rollback_args(1);
        let result = build_structured_args(OP_ROLLBACK, &args);
        assert_eq!(result["layer"], 1);
    }

    #[test]
    fn render_empty_tape_produces_empty_entries() {
        use sterling_search::tape::{SearchTapeFooterV1, SearchTapeHeaderV1, SearchTapeV1};

        let tape = SearchTapeV1 {
            header: SearchTapeHeaderV1 {
                json_bytes: vec![],
                json: serde_json::json!({}),
            },
            records: vec![],
            footer: SearchTapeFooterV1 {
                record_count: 0,
                final_chain_hash: [0u8; 32],
            },
        };

        let registry = kernel_operator_registry();
        let bytes = render_tool_transcript(&tape, &registry, "test_world").unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["entry_count"], 0);
        assert_eq!(parsed["entries"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["schema_version"], "tool_transcript.v1");
        assert_eq!(parsed["txn_epoch"], 0);
        assert_eq!(parsed["world_id"], "test_world");
    }

    #[test]
    fn tape_contains_tool_ops_empty() {
        use sterling_search::tape::{SearchTapeFooterV1, SearchTapeHeaderV1, SearchTapeV1};

        let tape = SearchTapeV1 {
            header: SearchTapeHeaderV1 {
                json_bytes: vec![],
                json: serde_json::json!({}),
            },
            records: vec![],
            footer: SearchTapeFooterV1 {
                record_count: 0,
                final_chain_hash: [0u8; 32],
            },
        };
        assert!(!tape_contains_tool_ops(&tape));
    }
}
