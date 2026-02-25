//! `ByteStateV1`: the two-plane deterministic state tensor.
//!
//! Ported from `core/carrier/bytestate.py` in Sterling v1.
//!
//! # Layout
//!
//! - Identity plane: `layer_count * slot_count * 4` bytes (`Code32` per slot)
//! - Status plane: `layer_count * slot_count * 1` byte (`SlotStatus` per slot)
//!
//! Default: 4 layers x 32 slots = 512 + 128 = 640 bytes.
//!
//! # Equality semantics
//!
//! `ByteStateV1` intentionally does **not** derive `Eq` or `Hash`.
//!
//! - State equality (for search/dedup): identity plane only — use [`ByteStateV1::identity_eq`].
//! - Evidence equality (for replay verification): both planes — use [`ByteStateV1::bitwise_eq`].
//!
//! This prevents the conflation bug where governance status changes inflate
//! search frontiers or break cycle detection.

use crate::carrier::code32::Code32;

/// Slot governance status.
///
/// Ordered by promotion level. Higher values = more trust.
/// Separated from identity (`Code32`) to prevent the conflation bug:
/// status changes must NOT alter identity hashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SlotStatus {
    /// Empty / unresolved semantics.
    Hole = 0,
    /// Exploratory only, lowest confidence.
    Shadow = 64,
    /// Under evaluation, tentatively placed.
    Provisional = 128,
    /// Promoted through evaluation gate, awaiting full certification.
    Promoted = 192,
    /// Fully certified and grounded, highest confidence.
    Certified = 255,
}

impl SlotStatus {
    /// Convert from raw byte. Returns `None` for unrecognized values.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Hole),
            64 => Some(Self::Shadow),
            128 => Some(Self::Provisional),
            192 => Some(Self::Promoted),
            255 => Some(Self::Certified),
            _ => None,
        }
    }

    /// Convert to raw byte.
    #[must_use]
    pub const fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Schema descriptor: identifies which `ByteState` layout to use.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaDescriptor {
    pub id: String,
    pub version: String,
    pub hash: String,
}

/// Registry snapshot: identifies the `Code32` <-> `ConceptID` mapping epoch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegistrySnapshot {
    pub epoch: String,
    pub hash: String,
}

/// `ByteStateV1`: two-plane deterministic state tensor.
///
/// Does **not** implement `Eq` or `Hash`. Use the explicit comparison
/// methods to choose the correct semantics for your context:
///
/// - [`identity_eq`](Self::identity_eq) for search/dedup (identity plane only)
/// - [`bitwise_eq`](Self::bitwise_eq) for replay verification (both planes)
#[derive(Debug, Clone)]
pub struct ByteStateV1 {
    layer_count: usize,
    slot_count: usize,
    /// Identity plane: flattened `[layer_count * slot_count]` `Code32` values.
    identity: Vec<Code32>,
    /// Status plane: flattened `[layer_count * slot_count]` status values.
    status: Vec<SlotStatus>,
}

impl ByteStateV1 {
    /// Create a new `ByteState` with all slots set to `PADDING`/`Hole`.
    #[must_use]
    pub fn new(layer_count: usize, slot_count: usize) -> Self {
        let total = layer_count * slot_count;
        Self {
            layer_count,
            slot_count,
            identity: vec![Code32::PADDING; total],
            status: vec![SlotStatus::Hole; total],
        }
    }

    /// Number of layers.
    #[must_use]
    pub const fn layer_count(&self) -> usize {
        self.layer_count
    }

    /// Number of slots per layer.
    #[must_use]
    pub const fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// Total byte size of the identity plane.
    #[must_use]
    pub const fn identity_byte_len(&self) -> usize {
        self.layer_count * self.slot_count * 4
    }

    /// Total byte size of the status plane.
    #[must_use]
    pub const fn status_byte_len(&self) -> usize {
        self.layer_count * self.slot_count
    }

    /// Total byte size of both planes combined.
    #[must_use]
    pub const fn total_byte_len(&self) -> usize {
        self.identity_byte_len() + self.status_byte_len()
    }

    /// Serialize identity plane to bytes (little-endian).
    ///
    /// Used for identity hashing (dedup, cycle detection).
    #[must_use]
    pub fn identity_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.identity_byte_len());
        for code in &self.identity {
            buf.extend_from_slice(&code.to_le_bytes());
        }
        buf
    }

    /// Serialize status plane to bytes.
    #[must_use]
    pub fn status_bytes(&self) -> Vec<u8> {
        self.status.iter().map(|s| s.to_byte()).collect()
    }

    /// Serialize both planes concatenated: identity ++ status.
    ///
    /// Used for evidence hashing (replay verification).
    #[must_use]
    pub fn evidence_bytes(&self) -> Vec<u8> {
        let mut buf = self.identity_bytes();
        buf.extend(self.status_bytes());
        buf
    }

    /// Identity-only equality: true if both states have identical identity planes.
    ///
    /// Use this for search/dedup. Status differences are intentionally ignored.
    #[must_use]
    pub fn identity_eq(&self, other: &Self) -> bool {
        self.layer_count == other.layer_count
            && self.slot_count == other.slot_count
            && self.identity == other.identity
    }

    /// Bitwise equality: true if both planes are identical.
    ///
    /// Use this for replay verification where governance status matters.
    #[must_use]
    pub fn bitwise_eq(&self, other: &Self) -> bool {
        self.layer_count == other.layer_count
            && self.slot_count == other.slot_count
            && self.identity == other.identity
            && self.status == other.status
    }

    /// Get `Code32` at (layer, slot). Panics if out of bounds.
    #[must_use]
    pub fn get_identity(&self, layer: usize, slot: usize) -> Code32 {
        self.identity[layer * self.slot_count + slot]
    }

    /// Get `SlotStatus` at (layer, slot). Panics if out of bounds.
    #[must_use]
    pub fn get_status(&self, layer: usize, slot: usize) -> SlotStatus {
        self.status[layer * self.slot_count + slot]
    }

    /// Set `Code32` at (layer, slot). Panics if out of bounds.
    pub fn set_identity(&mut self, layer: usize, slot: usize, code: Code32) {
        self.identity[layer * self.slot_count + slot] = code;
    }

    /// Set `SlotStatus` at (layer, slot). Panics if out of bounds.
    pub fn set_status(&mut self, layer: usize, slot: usize, status: SlotStatus) {
        self.status[layer * self.slot_count + slot] = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bytestate_dimensions() {
        let bs = ByteStateV1::new(4, 32);
        assert_eq!(bs.identity_byte_len(), 512);
        assert_eq!(bs.status_byte_len(), 128);
        assert_eq!(bs.total_byte_len(), 640);
    }

    #[test]
    fn default_slots_are_padding_and_hole() {
        let bs = ByteStateV1::new(2, 4);
        for layer in 0..2 {
            for slot in 0..4 {
                assert_eq!(bs.get_identity(layer, slot), Code32::PADDING);
                assert_eq!(bs.get_status(layer, slot), SlotStatus::Hole);
            }
        }
    }

    #[test]
    fn slot_status_byte_round_trip() {
        for expected in [
            SlotStatus::Hole,
            SlotStatus::Shadow,
            SlotStatus::Provisional,
            SlotStatus::Promoted,
            SlotStatus::Certified,
        ] {
            let byte = expected.to_byte();
            let actual = SlotStatus::from_byte(byte).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn invalid_status_byte_returns_none() {
        assert!(SlotStatus::from_byte(1).is_none());
        assert!(SlotStatus::from_byte(100).is_none());
        assert!(SlotStatus::from_byte(254).is_none());
    }

    #[test]
    fn identity_eq_ignores_status() {
        let mut a = ByteStateV1::new(2, 4);
        let mut b = ByteStateV1::new(2, 4);

        // Same identity, different status.
        a.set_identity(0, 0, Code32::new(1, 2, 3));
        b.set_identity(0, 0, Code32::new(1, 2, 3));
        a.set_status(0, 0, SlotStatus::Certified);
        b.set_status(0, 0, SlotStatus::Shadow);

        assert!(a.identity_eq(&b));
        assert!(!a.bitwise_eq(&b));
    }

    #[test]
    fn identity_eq_detects_identity_difference() {
        let mut a = ByteStateV1::new(2, 4);
        let mut b = ByteStateV1::new(2, 4);

        a.set_identity(0, 0, Code32::new(1, 2, 3));
        b.set_identity(0, 0, Code32::new(1, 2, 4));

        assert!(!a.identity_eq(&b));
    }

    #[test]
    fn evidence_bytes_includes_both_planes() {
        let mut bs = ByteStateV1::new(1, 1);
        bs.set_identity(0, 0, Code32::new(0x0A, 0x0B, 0x0C0D));
        bs.set_status(0, 0, SlotStatus::Certified);

        let evidence = bs.evidence_bytes();
        // 4 bytes identity + 1 byte status.
        assert_eq!(evidence.len(), 5);
        assert_eq!(&evidence[..4], &[0x0A, 0x0B, 0x0D, 0x0C]); // LE u16
        assert_eq!(evidence[4], 255); // Certified
    }
}
