//! `OperatorRegistryV1`: the normative operator catalog.
//!
//! Maps `Code32` operator IDs to their declared contracts (name, category,
//! argument layout, effect kind). Content-addressed via canonical JSON for
//! inclusion as a normative bundle artifact (`operator_registry.json`).
//!
//! The registry is the **contract surface**; the dispatch table in `apply.rs`
//! is the **implementation**. Verification can assert "every registry entry
//! used in this run had an implementation present."
//!
//! See: ADR 0008 (schema extension via additive fields), parity audit §Operator
//! Registry MVP.

use std::collections::BTreeMap;

use crate::carrier::code32::Code32;
use crate::operators::signature::{IdentityMaskV1, OperatorCategory, StatusMaskV1};
use crate::proof::canon::canonical_json_bytes;

// ---------------------------------------------------------------------------
// EffectKind — mechanically checkable effect contract
// ---------------------------------------------------------------------------

/// How an operator's effects can be mechanically validated post-apply.
///
/// Each variant defines an invariant that `apply()` checks after the
/// dispatch handler returns. This is the "effects bounded by declaration"
/// contract: the implementation cannot produce effects outside what the
/// registry entry declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    /// Writes exactly one identity slot (to a non-PADDING value) and promotes
    /// exactly one status slot to `Provisional`. Which slot is determined by
    /// the operator's arguments at runtime.
    WritesOneSlotFromArgs,

    /// Stages exactly one identity slot and promotes exactly one status slot
    /// to `Provisional`. Mechanically identical to `WritesOneSlotFromArgs`
    /// but semantically distinct: marks the write as a staging action for
    /// transcript categorization.
    StagesOneSlot,

    /// Commits a transaction by writing the `txn_marker` slot. Validates:
    /// exactly 1 identity diff (marker slot) and 1 status diff
    /// (marker Hole→Provisional). Additionally requires at least one
    /// non-marker slot on the target layer is already Provisional.
    /// Validated using only pre/post state bytes + args + schema-known offsets.
    CommitsTransaction,

    /// Rolls back a transaction by writing the `txn_marker` slot. Validates:
    /// exactly 1 identity diff (marker slot) and 1 status diff
    /// (marker Hole→Provisional). No precondition on staged slots —
    /// empty rollbacks are permitted.
    RollsBackTransaction,
}

impl EffectKind {
    /// Canonical string for JSON serialization.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WritesOneSlotFromArgs => "writes_one_slot_from_args",
            Self::StagesOneSlot => "stages_one_slot",
            Self::CommitsTransaction => "commits_transaction",
            Self::RollsBackTransaction => "rolls_back_transaction",
        }
    }

    /// Parse from canonical string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "writes_one_slot_from_args" => Some(Self::WritesOneSlotFromArgs),
            "stages_one_slot" => Some(Self::StagesOneSlot),
            "commits_transaction" => Some(Self::CommitsTransaction),
            "rolls_back_transaction" => Some(Self::RollsBackTransaction),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// OperatorEntry
// ---------------------------------------------------------------------------

/// A single entry in the operator registry.
///
/// Declares an operator's contract: what it is, how many argument bytes it
/// expects, what kind of effects it produces, and optional mask constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorEntry {
    /// The operator's `Code32` identifier.
    pub op_id: Code32,
    /// Human-readable name (diagnostic only, not routing per INV-CORE-04).
    pub name: String,
    /// Operator category per ADR 0004 (S/M/P/K/C).
    pub category: OperatorCategory,
    /// Exact expected byte length of `op_args` passed to `apply()`.
    pub arg_byte_count: usize,
    /// Mechanically checkable effect contract.
    pub effect_kind: EffectKind,
    /// Precondition mask: slots that must hold specific values.
    pub precondition_mask: IdentityMaskV1,
    /// Effect mask: slots written by this operator.
    pub effect_mask: IdentityMaskV1,
    /// Status effect mask: slot statuses changed by this operator.
    pub status_effect_mask: StatusMaskV1,
    /// Cost model identifier (e.g., `"unit"`, `"proportional"`).
    pub cost_model: String,
    /// Schema epoch for this entry's contract.
    pub contract_epoch: String,
}

// ---------------------------------------------------------------------------
// OperatorRegistryError
// ---------------------------------------------------------------------------

/// Error type for operator registry construction and serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorRegistryError {
    /// Two entries share the same `op_id`.
    DuplicateOpCode { op_id: Code32 },
    /// Canonical JSON serialization failed.
    CanonicalizationError { detail: String },
}

impl std::fmt::Display for OperatorRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateOpCode { op_id } => {
                write!(f, "duplicate op_id in operator registry: {op_id:?}")
            }
            Self::CanonicalizationError { detail } => {
                write!(f, "operator registry canonicalization failed: {detail}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OperatorRegistryV1
// ---------------------------------------------------------------------------

/// The operator registry: maps `Code32` `op_id` → `OperatorEntry`.
///
/// Content-addressed via canonical JSON serialization. `BTreeMap` for
/// deterministic iteration order (Code32 Ord impl).
#[derive(Debug, Clone)]
pub struct OperatorRegistryV1 {
    entries: BTreeMap<Code32, OperatorEntry>,
    schema_version: String,
}

impl OperatorRegistryV1 {
    /// Build a registry from a list of entries.
    ///
    /// # Errors
    ///
    /// Returns [`OperatorRegistryError::DuplicateOpCode`] if two entries
    /// share the same `op_id`.
    pub fn new(
        schema_version: String,
        entries: Vec<OperatorEntry>,
    ) -> Result<Self, OperatorRegistryError> {
        let mut map = BTreeMap::new();
        for entry in entries {
            let op_id = entry.op_id;
            if map.insert(op_id, entry).is_some() {
                return Err(OperatorRegistryError::DuplicateOpCode { op_id });
            }
        }
        Ok(Self {
            entries: map,
            schema_version,
        })
    }

    /// Look up an entry by `op_code`.
    #[must_use]
    pub fn get(&self, op_code: &Code32) -> Option<&OperatorEntry> {
        self.entries.get(op_code)
    }

    /// Whether `op_code` is registered.
    #[must_use]
    pub fn contains(&self, op_code: &Code32) -> bool {
        self.entries.contains_key(op_code)
    }

    /// Number of registered operators.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Produce canonical JSON bytes for content-addressing and artifact generation.
    ///
    /// Format: sorted top-level keys, entries array sorted by Code32 bytes
    /// (`BTreeMap` iteration order).
    ///
    /// # Errors
    ///
    /// Returns [`OperatorRegistryError::CanonicalizationError`] if canonical
    /// JSON serialization fails.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, OperatorRegistryError> {
        let entries_json: Vec<serde_json::Value> = self
            .entries
            .values()
            .map(entry_to_json)
            .collect();

        let value = serde_json::json!({
            "entries": entries_json,
            "schema_version": self.schema_version,
        });

        canonical_json_bytes(&value).map_err(|e| OperatorRegistryError::CanonicalizationError {
            detail: e.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Canonical JSON helpers
// ---------------------------------------------------------------------------

fn entry_to_json(entry: &OperatorEntry) -> serde_json::Value {
    let op_id_bytes = entry.op_id.to_le_bytes();
    serde_json::json!({
        "arg_byte_count": entry.arg_byte_count as u64,
        "category": entry.category.code().to_string(),
        "contract_epoch": entry.contract_epoch,
        "cost_model": entry.cost_model,
        "effect_kind": entry.effect_kind.as_str(),
        "effect_mask": mask_to_json(&entry.effect_mask),
        "name": entry.name,
        "op_id": [
            u64::from(op_id_bytes[0]),
            u64::from(op_id_bytes[1]),
            u64::from(op_id_bytes[2]),
            u64::from(op_id_bytes[3]),
        ],
        "precondition_mask": mask_to_json(&entry.precondition_mask),
        "status_effect_mask": status_mask_to_json(&entry.status_effect_mask),
    })
}

fn mask_to_json(mask: &IdentityMaskV1) -> serde_json::Value {
    let (layers, slots) = mask.dimensions();
    let mut active: Vec<serde_json::Value> = Vec::new();
    let mut values: Vec<serde_json::Value> = Vec::new();

    for layer in 0..layers {
        for slot in 0..slots {
            active.push(serde_json::json!(u64::from(mask.is_active(layer, slot))));
            let code = mask.get(layer, slot).unwrap_or(Code32::PADDING);
            let bytes = code.to_le_bytes();
            values.push(serde_json::json!([
                u64::from(bytes[0]),
                u64::from(bytes[1]),
                u64::from(bytes[2]),
                u64::from(bytes[3]),
            ]));
        }
    }

    serde_json::json!({
        "active": active,
        "dimensions": [layers as u64, slots as u64],
        "values": values,
    })
}

fn status_mask_to_json(mask: &StatusMaskV1) -> serde_json::Value {
    let (layers, slots) = mask.dimensions();
    let mut active: Vec<serde_json::Value> = Vec::new();
    let mut values: Vec<serde_json::Value> = Vec::new();

    for layer in 0..layers {
        for slot in 0..slots {
            active.push(serde_json::json!(u64::from(mask.is_active(layer, slot))));
            let val = mask.get(layer, slot).unwrap_or(0);
            values.push(serde_json::json!(u64::from(val)));
        }
    }

    serde_json::json!({
        "active": active,
        "dimensions": [layers as u64, slots as u64],
        "values": values,
    })
}

// ---------------------------------------------------------------------------
// kernel_operator_registry() — the canonical V1 kernel registry
// ---------------------------------------------------------------------------

use crate::operators::apply::{OP_COMMIT, OP_ROLLBACK, OP_SET_SLOT, OP_STAGE};

/// Build the canonical operator registry for the V1 kernel.
///
/// Contains four entries: `OP_SET_SLOT`, `OP_STAGE`, `OP_COMMIT`, `OP_ROLLBACK`.
///
/// The harness calls this to get the registry for bundle artifact generation.
/// Worlds call this (or receive it from the harness) for operator legality
/// checks during candidate enumeration.
///
/// # Panics
///
/// Panics if the static registry construction fails (programming error).
#[must_use]
pub fn kernel_operator_registry() -> OperatorRegistryV1 {
    // Masks are 0×0 (dimension-free) for all operators: their effects depend
    // on which (layer, slot) args specify at runtime. The effect_kind +
    // post-apply check substitutes for static mask checking.
    let empty_id_mask = || IdentityMaskV1::new(0, 0);
    let empty_st_mask = || StatusMaskV1::new(0, 0);

    let set_slot = OperatorEntry {
        op_id: OP_SET_SLOT,
        name: "SET_SLOT".into(),
        category: OperatorCategory::Memorize,
        arg_byte_count: 12, // 3 × 4 bytes (layer u32, slot u32, value Code32)
        effect_kind: EffectKind::WritesOneSlotFromArgs,
        precondition_mask: empty_id_mask(),
        effect_mask: empty_id_mask(),
        status_effect_mask: empty_st_mask(),
        cost_model: "unit".into(),
        contract_epoch: "v1".into(),
    };

    let stage = OperatorEntry {
        op_id: OP_STAGE,
        name: "STAGE".into(),
        category: OperatorCategory::Memorize,
        arg_byte_count: 12, // 3 × 4 bytes (layer u32, slot u32, value Code32)
        effect_kind: EffectKind::StagesOneSlot,
        precondition_mask: empty_id_mask(),
        effect_mask: empty_id_mask(),
        status_effect_mask: empty_st_mask(),
        cost_model: "unit".into(),
        contract_epoch: "v1".into(),
    };

    let commit = OperatorEntry {
        op_id: OP_COMMIT,
        name: "COMMIT".into(),
        category: OperatorCategory::Control,
        arg_byte_count: 4, // 1 × 4 bytes (layer u32)
        effect_kind: EffectKind::CommitsTransaction,
        precondition_mask: empty_id_mask(),
        effect_mask: empty_id_mask(),
        status_effect_mask: empty_st_mask(),
        cost_model: "unit".into(),
        contract_epoch: "v1".into(),
    };

    let rollback = OperatorEntry {
        op_id: OP_ROLLBACK,
        name: "ROLLBACK".into(),
        category: OperatorCategory::Control,
        arg_byte_count: 4, // 1 × 4 bytes (layer u32)
        effect_kind: EffectKind::RollsBackTransaction,
        precondition_mask: empty_id_mask(),
        effect_mask: empty_id_mask(),
        status_effect_mask: empty_st_mask(),
        cost_model: "unit".into(),
        contract_epoch: "v1".into(),
    };

    OperatorRegistryV1::new(
        "operator_registry.v1".into(),
        vec![set_slot, stage, commit, rollback],
    )
    .expect("kernel_operator_registry: static invariant violated")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn set_slot_entry() -> OperatorEntry {
        OperatorEntry {
            op_id: OP_SET_SLOT,
            name: "SET_SLOT".into(),
            category: OperatorCategory::Memorize,
            arg_byte_count: 12,
            effect_kind: EffectKind::WritesOneSlotFromArgs,
            precondition_mask: IdentityMaskV1::new(0, 0),
            effect_mask: IdentityMaskV1::new(0, 0),
            status_effect_mask: StatusMaskV1::new(0, 0),
            cost_model: "unit".into(),
            contract_epoch: "v1".into(),
        }
    }

    #[test]
    fn new_accepts_valid_entry() {
        let reg = OperatorRegistryV1::new(
            "test.v1".into(),
            vec![set_slot_entry()],
        )
        .unwrap();
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
        assert!(reg.contains(&OP_SET_SLOT));
    }

    #[test]
    fn new_rejects_duplicate_op_id() {
        let err = OperatorRegistryV1::new(
            "test.v1".into(),
            vec![set_slot_entry(), set_slot_entry()],
        )
        .unwrap_err();
        assert!(matches!(err, OperatorRegistryError::DuplicateOpCode { .. }));
    }

    #[test]
    fn empty_registry_valid() {
        let reg = OperatorRegistryV1::new("test.v1".into(), vec![]).unwrap();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        let bytes = reg.canonical_bytes().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn get_present_and_absent() {
        let reg = kernel_operator_registry();
        assert!(reg.get(&OP_SET_SLOT).is_some());
        assert_eq!(reg.get(&OP_SET_SLOT).unwrap().name, "SET_SLOT");
        assert!(reg.get(&Code32::new(9, 9, 9)).is_none());
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let reg = kernel_operator_registry();
        let first = reg.canonical_bytes().unwrap();
        for _ in 0..10 {
            assert_eq!(reg.canonical_bytes().unwrap(), first);
        }
    }

    #[test]
    fn canonical_bytes_insertion_order_independent() {
        let entry_a = OperatorEntry {
            op_id: Code32::new(1, 0, 0),
            name: "OP_A".into(),
            category: OperatorCategory::Seek,
            arg_byte_count: 4,
            effect_kind: EffectKind::WritesOneSlotFromArgs,
            precondition_mask: IdentityMaskV1::new(0, 0),
            effect_mask: IdentityMaskV1::new(0, 0),
            status_effect_mask: StatusMaskV1::new(0, 0),
            cost_model: "unit".into(),
            contract_epoch: "v1".into(),
        };
        let entry_b = OperatorEntry {
            op_id: Code32::new(2, 0, 0),
            name: "OP_B".into(),
            category: OperatorCategory::Memorize,
            arg_byte_count: 8,
            effect_kind: EffectKind::WritesOneSlotFromArgs,
            precondition_mask: IdentityMaskV1::new(0, 0),
            effect_mask: IdentityMaskV1::new(0, 0),
            status_effect_mask: StatusMaskV1::new(0, 0),
            cost_model: "unit".into(),
            contract_epoch: "v1".into(),
        };

        let reg1 = OperatorRegistryV1::new(
            "test.v1".into(),
            vec![entry_a.clone(), entry_b.clone()],
        )
        .unwrap();
        let reg2 = OperatorRegistryV1::new(
            "test.v1".into(),
            vec![entry_b, entry_a],
        )
        .unwrap();

        assert_eq!(reg1.canonical_bytes().unwrap(), reg2.canonical_bytes().unwrap());
    }

    #[test]
    fn canonical_bytes_sorted_keys() {
        let reg = kernel_operator_registry();
        let bytes = reg.canonical_bytes().unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        // Top-level keys must be sorted: "entries" < "schema_version"
        let entries_pos = s.find("\"entries\"").unwrap();
        let schema_pos = s.find("\"schema_version\"").unwrap();
        assert!(entries_pos < schema_pos);
        // Entry keys must be sorted: arg_byte_count < category < ... < status_effect_mask
        let name_pos = s.find("\"name\"").unwrap();
        let cat_pos = s.find("\"category\"").unwrap();
        let abc_pos = s.find("\"arg_byte_count\"").unwrap();
        assert!(abc_pos < cat_pos);
        assert!(cat_pos < name_pos);
    }

    #[test]
    fn kernel_operator_registry_has_all_operators() {
        let reg = kernel_operator_registry();
        assert_eq!(reg.len(), 4);

        let set_slot = reg.get(&OP_SET_SLOT).unwrap();
        assert_eq!(set_slot.name, "SET_SLOT");
        assert_eq!(set_slot.category, OperatorCategory::Memorize);
        assert_eq!(set_slot.arg_byte_count, 12);
        assert_eq!(set_slot.effect_kind, EffectKind::WritesOneSlotFromArgs);
        assert_eq!(set_slot.contract_epoch, "v1");

        let stage = reg.get(&OP_STAGE).unwrap();
        assert_eq!(stage.name, "STAGE");
        assert_eq!(stage.category, OperatorCategory::Memorize);
        assert_eq!(stage.arg_byte_count, 12);
        assert_eq!(stage.effect_kind, EffectKind::StagesOneSlot);

        let commit = reg.get(&OP_COMMIT).unwrap();
        assert_eq!(commit.name, "COMMIT");
        assert_eq!(commit.category, OperatorCategory::Control);
        assert_eq!(commit.arg_byte_count, 4);
        assert_eq!(commit.effect_kind, EffectKind::CommitsTransaction);

        let rollback = reg.get(&OP_ROLLBACK).unwrap();
        assert_eq!(rollback.name, "ROLLBACK");
        assert_eq!(rollback.category, OperatorCategory::Control);
        assert_eq!(rollback.arg_byte_count, 4);
        assert_eq!(rollback.effect_kind, EffectKind::RollsBackTransaction);
    }

    #[test]
    fn effect_kind_round_trip() {
        for kind in [
            EffectKind::WritesOneSlotFromArgs,
            EffectKind::StagesOneSlot,
            EffectKind::CommitsTransaction,
            EffectKind::RollsBackTransaction,
        ] {
            assert_eq!(EffectKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(EffectKind::parse("unknown"), None);
    }

    #[test]
    fn canonical_bytes_golden() {
        // Lock the canonical bytes for the kernel registry.
        // If this changes, it means the registry schema or content changed —
        // which is a deliberate schema version bump.
        let reg = kernel_operator_registry();
        let bytes = reg.canonical_bytes().unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();

        // Verify key structural properties rather than exact bytes.
        assert!(s.starts_with('{'));
        assert!(s.ends_with('}'));
        assert!(s.contains("\"schema_version\":\"operator_registry.v1\""));
        // All four operators present
        assert!(s.contains("\"name\":\"SET_SLOT\""));
        assert!(s.contains("\"name\":\"STAGE\""));
        assert!(s.contains("\"name\":\"COMMIT\""));
        assert!(s.contains("\"name\":\"ROLLBACK\""));
        // Effect kinds
        assert!(s.contains("\"effect_kind\":\"writes_one_slot_from_args\""));
        assert!(s.contains("\"effect_kind\":\"stages_one_slot\""));
        assert!(s.contains("\"effect_kind\":\"commits_transaction\""));
        assert!(s.contains("\"effect_kind\":\"rolls_back_transaction\""));
        // Op IDs: SET_SLOT(1,1,1), STAGE(1,1,2), COMMIT(1,1,3), ROLLBACK(1,1,4)
        assert!(s.contains("\"op_id\":[1,1,1,0]"));
        assert!(s.contains("\"op_id\":[1,1,2,0]"));
        assert!(s.contains("\"op_id\":[1,1,3,0]"));
        assert!(s.contains("\"op_id\":[1,1,4,0]"));
    }
}
