//! `OperatorSignature`: typed contract for an operator.
//!
//! Operators are categorized by S/M/P/K/C taxonomy (ADR 0004):
//! - **S** (Seek): explore/navigate state space
//! - **M** (Memorize): commit/consolidate meaning
//! - **P** (Perceive): interpret context, update beliefs
//! - **K** (Knowledge): query/extend world knowledge
//! - **C** (Control): manage search flow

use std::collections::BTreeMap;

use crate::carrier::code32::Code32;

/// Operator category per ADR 0004.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorCategory {
    /// Seek: explore/navigate state space.
    Seek,
    /// Memorize: commit/consolidate meaning.
    Memorize,
    /// Perceive: interpret context, update beliefs.
    Perceive,
    /// Knowledge: query/extend world knowledge.
    Knowledge,
    /// Control: manage search flow.
    Control,
}

impl OperatorCategory {
    /// Single-letter code per ADR 0004.
    #[must_use]
    pub const fn code(self) -> char {
        match self {
            Self::Seek => 'S',
            Self::Memorize => 'M',
            Self::Perceive => 'P',
            Self::Knowledge => 'K',
            Self::Control => 'C',
        }
    }
}

/// Typed operator signature: the contract an operator declares.
///
/// Masks use `BTreeMap` for canonical ordering (deterministic iteration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSignature {
    /// The operator's `Code32` identifier.
    pub op_code: Code32,
    /// Operator category.
    pub category: OperatorCategory,
    /// Human-readable name (for diagnostics, not routing per INV-CORE-04).
    pub name: String,
    /// Precondition mask: slots that must hold specific `Code32` values.
    /// Key: (layer, slot), Value: required `Code32`.
    pub precondition_mask: BTreeMap<(usize, usize), Code32>,
    /// Effect mask: slots written by this operator.
    /// Key: (layer, slot), Value: `Code32` to write.
    pub effect_mask: BTreeMap<(usize, usize), Code32>,
    /// Status effect mask: slot statuses changed by this operator.
    /// Key: (layer, slot), Value: new `SlotStatus` byte.
    pub status_effect_mask: BTreeMap<(usize, usize), u8>,
}
