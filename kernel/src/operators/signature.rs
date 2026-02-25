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

impl std::fmt::Display for OperatorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Seek => "S(Seek)",
            Self::Memorize => "M(Memorize)",
            Self::Perceive => "P(Perceive)",
            Self::Knowledge => "K(Knowledge)",
            Self::Control => "C(Control)",
        })
    }
}

/// Full-width operator mask over the identity plane.
///
/// Each entry corresponds to a `(layer, slot)` position in the `ByteStateV1`.
/// Inactive entries mean "don't care" (slot is not constrained / not written).
/// Active entries mean "this slot must be / will be set to the stored `Code32`".
///
/// Flat layout: index = `layer * slot_count + slot`.
///
/// Internal representation uses separate `values` and `active` arrays so that
/// "constraint equals `PADDING`" is distinguishable from "no constraint," and
/// the `active` bitvec can later be SIMD-scanned without touching the values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityMaskV1 {
    layer_count: usize,
    slot_count: usize,
    values: Vec<Code32>,
    active: Vec<bool>,
}

impl IdentityMaskV1 {
    /// Create an empty mask (all entries inactive, values default to `PADDING`).
    #[must_use]
    pub fn new(layer_count: usize, slot_count: usize) -> Self {
        let len = layer_count * slot_count;
        Self {
            layer_count,
            slot_count,
            values: vec![Code32::PADDING; len],
            active: vec![false; len],
        }
    }

    /// Set a single entry as active. Panics if out of bounds.
    pub fn set(&mut self, layer: usize, slot: usize, code: Code32) {
        let idx = layer * self.slot_count + slot;
        self.values[idx] = code;
        self.active[idx] = true;
    }

    /// Clear a single entry (mark inactive). Panics if out of bounds.
    pub fn clear(&mut self, layer: usize, slot: usize) {
        let idx = layer * self.slot_count + slot;
        self.values[idx] = Code32::PADDING;
        self.active[idx] = false;
    }

    /// Get an entry. Returns `None` if inactive. Panics if out of bounds.
    #[must_use]
    pub fn get(&self, layer: usize, slot: usize) -> Option<Code32> {
        let idx = layer * self.slot_count + slot;
        if self.active[idx] {
            Some(self.values[idx])
        } else {
            None
        }
    }

    /// Whether a slot is active (constrained). Panics if out of bounds.
    #[must_use]
    pub fn is_active(&self, layer: usize, slot: usize) -> bool {
        self.active[layer * self.slot_count + slot]
    }

    /// Number of active (constrained) entries.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
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
/// Separate `values`/`active` arrays for the same reasons as `IdentityMaskV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMaskV1 {
    layer_count: usize,
    slot_count: usize,
    values: Vec<u8>,
    active: Vec<bool>,
}

impl StatusMaskV1 {
    /// Create an empty mask (all entries inactive, values default to 0).
    #[must_use]
    pub fn new(layer_count: usize, slot_count: usize) -> Self {
        let len = layer_count * slot_count;
        Self {
            layer_count,
            slot_count,
            values: vec![0; len],
            active: vec![false; len],
        }
    }

    /// Set a single entry as active. Panics if out of bounds.
    pub fn set(&mut self, layer: usize, slot: usize, status_byte: u8) {
        let idx = layer * self.slot_count + slot;
        self.values[idx] = status_byte;
        self.active[idx] = true;
    }

    /// Clear a single entry (mark inactive). Panics if out of bounds.
    pub fn clear(&mut self, layer: usize, slot: usize) {
        let idx = layer * self.slot_count + slot;
        self.values[idx] = 0;
        self.active[idx] = false;
    }

    /// Get an entry. Returns `None` if inactive. Panics if out of bounds.
    #[must_use]
    pub fn get(&self, layer: usize, slot: usize) -> Option<u8> {
        let idx = layer * self.slot_count + slot;
        if self.active[idx] {
            Some(self.values[idx])
        } else {
            None
        }
    }

    /// Whether a slot is active (constrained). Panics if out of bounds.
    #[must_use]
    pub fn is_active(&self, layer: usize, slot: usize) -> bool {
        self.active[layer * self.slot_count + slot]
    }

    /// Number of active (constrained) entries.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
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

    #[test]
    fn category_codes_match_adr_0004() {
        assert_eq!(OperatorCategory::Seek.code(), 'S');
        assert_eq!(OperatorCategory::Memorize.code(), 'M');
        assert_eq!(OperatorCategory::Perceive.code(), 'P');
        assert_eq!(OperatorCategory::Knowledge.code(), 'K');
        assert_eq!(OperatorCategory::Control.code(), 'C');
    }

    #[test]
    fn category_display_includes_code() {
        let s = format!("{}", OperatorCategory::Seek);
        assert!(s.starts_with('S'));
        let m = format!("{}", OperatorCategory::Memorize);
        assert!(m.starts_with('M'));
    }

    #[test]
    fn identity_mask_padding_vs_inactive() {
        let mut mask = IdentityMaskV1::new(2, 4);

        // Inactive slot returns None.
        assert_eq!(mask.get(0, 0), None);
        assert!(!mask.is_active(0, 0));

        // Set slot to PADDING — now it's active with value PADDING.
        mask.set(0, 0, Code32::PADDING);
        assert_eq!(mask.get(0, 0), Some(Code32::PADDING));
        assert!(mask.is_active(0, 0));
        assert_eq!(mask.active_count(), 1);

        // Clear returns it to inactive.
        mask.clear(0, 0);
        assert_eq!(mask.get(0, 0), None);
        assert!(!mask.is_active(0, 0));
        assert_eq!(mask.active_count(), 0);
    }

    #[test]
    fn status_mask_zero_vs_inactive() {
        let mut mask = StatusMaskV1::new(2, 4);

        // Inactive slot returns None.
        assert_eq!(mask.get(0, 0), None);
        assert!(!mask.is_active(0, 0));

        // Set slot to 0 (Hole status) — now active.
        mask.set(0, 0, 0);
        assert_eq!(mask.get(0, 0), Some(0));
        assert!(mask.is_active(0, 0));

        // Clear returns it to inactive.
        mask.clear(0, 0);
        assert_eq!(mask.get(0, 0), None);
        assert!(!mask.is_active(0, 0));
    }
}
