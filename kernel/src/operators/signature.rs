//! `OperatorSignature`: typed contract for an operator.
//!
//! Operators are categorized by S/M/P/K/C taxonomy (ADR 0004):
//! - **S** (Seek): explore/navigate state space
//! - **M** (Memorize): commit/consolidate meaning
//! - **P** (Perceive): interpret context, update beliefs
//! - **K** (Knowledge): query/extend world knowledge
//! - **C** (Control): manage search flow
//!
//! # Mask representation
//!
//! Operator masks are full-width packed vectors matching the `ByteStateV1`
//! dimensions (`layer_count * slot_count`). This avoids a future API break
//! when SIMD-friendly application is needed, and matches the canonical doc's
//! `uint32[layer_count * slot_count]` layout.

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

/// Full-width operator mask over the identity plane.
///
/// Each entry corresponds to a `(layer, slot)` position in the `ByteStateV1`.
/// `None` means "don't care" (slot is not constrained / not written).
/// `Some(code)` means "this slot must be / will be set to `code`".
///
/// Flat layout: index = `layer * slot_count + slot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityMaskV1 {
    layer_count: usize,
    slot_count: usize,
    entries: Vec<Option<Code32>>,
}

impl IdentityMaskV1 {
    /// Create an empty mask (all entries `None`).
    #[must_use]
    pub fn new(layer_count: usize, slot_count: usize) -> Self {
        Self {
            layer_count,
            slot_count,
            entries: vec![None; layer_count * slot_count],
        }
    }

    /// Set a single entry. Panics if out of bounds.
    pub fn set(&mut self, layer: usize, slot: usize, code: Code32) {
        self.entries[layer * self.slot_count + slot] = Some(code);
    }

    /// Get an entry. Panics if out of bounds.
    #[must_use]
    pub fn get(&self, layer: usize, slot: usize) -> Option<Code32> {
        self.entries[layer * self.slot_count + slot]
    }

    /// Number of non-`None` entries.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    /// Dimensions.
    #[must_use]
    pub const fn dimensions(&self) -> (usize, usize) {
        (self.layer_count, self.slot_count)
    }
}

/// Full-width operator mask over the status plane.
///
/// Same layout as [`IdentityMaskV1`] but with `u8` (`SlotStatus` byte) values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMaskV1 {
    layer_count: usize,
    slot_count: usize,
    entries: Vec<Option<u8>>,
}

impl StatusMaskV1 {
    /// Create an empty mask (all entries `None`).
    #[must_use]
    pub fn new(layer_count: usize, slot_count: usize) -> Self {
        Self {
            layer_count,
            slot_count,
            entries: vec![None; layer_count * slot_count],
        }
    }

    /// Set a single entry. Panics if out of bounds.
    pub fn set(&mut self, layer: usize, slot: usize, status_byte: u8) {
        self.entries[layer * self.slot_count + slot] = Some(status_byte);
    }

    /// Get an entry. Panics if out of bounds.
    #[must_use]
    pub fn get(&self, layer: usize, slot: usize) -> Option<u8> {
        self.entries[layer * self.slot_count + slot]
    }

    /// Number of non-`None` entries.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    /// Dimensions.
    #[must_use]
    pub const fn dimensions(&self) -> (usize, usize) {
        (self.layer_count, self.slot_count)
    }
}

/// Typed operator signature: the contract an operator declares.
///
/// Masks are full-width packed vectors matching `ByteStateV1` dimensions.
/// `None` entries mean "don't care"; `Some` entries are constraints/effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSignature {
    /// The operator's `Code32` identifier.
    pub op_code: Code32,
    /// Operator category.
    pub category: OperatorCategory,
    /// Human-readable name (for diagnostics, not routing per INV-CORE-04).
    pub name: String,
    /// Precondition mask: slots that must hold specific `Code32` values.
    pub precondition_mask: IdentityMaskV1,
    /// Effect mask: slots written by this operator.
    pub effect_mask: IdentityMaskV1,
    /// Status effect mask: slot statuses changed by this operator.
    pub status_effect_mask: StatusMaskV1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_mask_sparse_set_get() {
        let mut mask = IdentityMaskV1::new(4, 32);
        assert_eq!(mask.active_count(), 0);

        mask.set(0, 5, Code32::new(2, 1, 3));
        assert_eq!(mask.get(0, 5), Some(Code32::new(2, 1, 3)));
        assert_eq!(mask.get(0, 0), None);
        assert_eq!(mask.active_count(), 1);
    }

    #[test]
    fn status_mask_sparse_set_get() {
        let mut mask = StatusMaskV1::new(4, 32);
        mask.set(1, 10, 255); // Certified
        assert_eq!(mask.get(1, 10), Some(255));
        assert_eq!(mask.get(0, 0), None);
        assert_eq!(mask.active_count(), 1);
    }

    #[test]
    fn mask_dimensions_match_construction() {
        let mask = IdentityMaskV1::new(4, 32);
        assert_eq!(mask.dimensions(), (4, 32));
    }
}
