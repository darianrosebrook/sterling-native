//! `Code32`: 32-bit concept identifier.
//!
//! Ported from `core/carrier/code32.py` in Sterling v1.
//!
//! # Layout (little-endian, 4 bytes)
//!
//! | Byte | Width | Field    |
//! |------|-------|----------|
//! | 0    | u8    | domain   |
//! | 1    | u8    | kind     |
//! | 2-3  | u16le | `local_id` |
//!
//! Equivalent to `struct.pack("<BBH", domain, kind, local_id)` in Python.
//!
//! # Sentinels (domain=0, kind=0)
//!
//! | Name           | `local_id` | Bytes            | u32 LE       |
//! |----------------|----------|------------------|--------------|
//! | PADDING        | 0        | `[0, 0, 0, 0]`  | `0x00000000` |
//! | `INITIAL_STATE`  | 1        | `[0, 0, 1, 0]`  | `0x00010000` |
//! | TERMINAL       | 2        | `[0, 0, 2, 0]`  | `0x00020000` |
//!
//! # Canonical form
//!
//! The canonical representation is **bytes**, not integers.
//! [`Code32::to_u32_le`] exists for display/debugging only.
//! Serialization and hashing always use [`Code32::to_le_bytes`].

use std::fmt;

/// A 32-bit concept code in the Sterling kernel.
///
/// Stored and serialized as 4 bytes, little-endian.
/// Derives `Ord` for use as `BTreeMap` keys (canonical ordering).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code32([u8; 4]);

impl Code32 {
    /// Padding sentinel: empty/unused slot.
    pub const PADDING: Self = Self([0x00, 0x00, 0x00, 0x00]);

    /// Initial state sentinel: frame 0 operator code (no operator applied).
    pub const INITIAL_STATE: Self = Self([0x00, 0x00, 0x01, 0x00]);

    /// Terminal sentinel: marks terminal/goal states.
    pub const TERMINAL: Self = Self([0x00, 0x00, 0x02, 0x00]);

    /// Construct from domain, kind, and `local_id`.
    #[must_use]
    pub const fn new(domain: u8, kind: u8, local_id: u16) -> Self {
        let id_bytes = local_id.to_le_bytes();
        Self([domain, kind, id_bytes[0], id_bytes[1]])
    }

    /// Construct from raw little-endian bytes.
    #[must_use]
    pub const fn from_le_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Return the raw little-endian bytes.
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 4] {
        self.0
    }

    /// Domain field (byte 0).
    #[must_use]
    pub const fn domain(self) -> u8 {
        self.0[0]
    }

    /// Kind field (byte 1).
    #[must_use]
    pub const fn kind(self) -> u8 {
        self.0[1]
    }

    /// Local ID field (bytes 2-3, little-endian u16).
    #[must_use]
    pub const fn local_id(self) -> u16 {
        u16::from_le_bytes([self.0[2], self.0[3]])
    }

    /// True if this code is a system sentinel (domain=0, kind=0).
    #[must_use]
    pub const fn is_sentinel(self) -> bool {
        self.0[0] == 0 && self.0[1] == 0
    }

    /// Interpret bytes as a little-endian u32.
    ///
    /// **For display and debugging only.** Serialization and hashing
    /// always use [`to_le_bytes`](Self::to_le_bytes), never this integer view.
    #[must_use]
    pub const fn to_u32_le(self) -> u32 {
        u32::from_le_bytes(self.0)
    }
}

impl fmt::Debug for Code32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Code32({},{},{} / 0x{:08x})",
            self.domain(),
            self.kind(),
            self.local_id(),
            self.to_u32_le(),
        )
    }
}

impl fmt::Display for Code32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Code32({},{},{})",
            self.domain(),
            self.kind(),
            self.local_id(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinels_have_expected_bytes() {
        assert_eq!(Code32::PADDING.to_le_bytes(), [0, 0, 0, 0]);
        assert_eq!(Code32::INITIAL_STATE.to_le_bytes(), [0, 0, 1, 0]);
        assert_eq!(Code32::TERMINAL.to_le_bytes(), [0, 0, 2, 0]);
    }

    #[test]
    fn sentinel_u32_values() {
        assert_eq!(Code32::PADDING.to_u32_le(), 0x0000_0000);
        assert_eq!(Code32::INITIAL_STATE.to_u32_le(), 0x0001_0000);
        assert_eq!(Code32::TERMINAL.to_u32_le(), 0x0002_0000);
    }

    #[test]
    fn new_round_trips_fields() {
        let code = Code32::new(0x0A, 0x0B, 0x1234);
        assert_eq!(code.domain(), 0x0A);
        assert_eq!(code.kind(), 0x0B);
        assert_eq!(code.local_id(), 0x1234);
    }

    #[test]
    fn new_2_1_3_u32_matches_v1() {
        // v1: Code32(domain=2, kind=1, local_id=3)
        // struct.pack("<BBH", 2, 1, 3) = b"\x02\x01\x03\x00"
        // struct.unpack("<I", ...) = 0x00030102
        let code = Code32::new(2, 1, 3);
        assert_eq!(code.to_le_bytes(), [0x02, 0x01, 0x03, 0x00]);
        assert_eq!(code.to_u32_le(), 0x0003_0102);
    }

    #[test]
    fn sentinels_are_sentinel() {
        assert!(Code32::PADDING.is_sentinel());
        assert!(Code32::INITIAL_STATE.is_sentinel());
        assert!(Code32::TERMINAL.is_sentinel());
        assert!(!Code32::new(1, 0, 0).is_sentinel());
    }

    #[test]
    fn le_byte_round_trip() {
        let bytes = [0xAA, 0xBB, 0xCC, 0xDD];
        let code = Code32::from_le_bytes(bytes);
        assert_eq!(code.to_le_bytes(), bytes);
    }

    #[test]
    fn domain_zero_nonzero_kind_is_not_sentinel() {
        let code = Code32::new(0, 1, 0);
        assert!(!code.is_sentinel());
    }
}
