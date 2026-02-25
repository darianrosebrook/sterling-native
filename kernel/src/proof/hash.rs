//! Canonical hashing types and domain separation constants.
//!
//! Domain prefix constants match v1 exactly (byte-for-byte, null-terminated).
//! Algorithm: SHA-256 for all V1 artifacts. Blake3 reserved for future V2.
//!
//! **Exactly one place defines canonical hashing** (SPINE-001 invariant).
//!
//! # M0 scope
//!
//! Types and constants only. `sha2` implementation is M1 scope.

/// A content-addressed hash with algorithm identifier.
///
/// Format: `"algorithm:hex_digest"` (e.g., `"sha256:abcdef..."`)
///
/// Invariant: the inner string always contains exactly one `:` separator,
/// with non-empty substrings on both sides (enforced by [`ContentHash::parse`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash {
    /// Full string in `"algorithm:hex_digest"` format.
    full: String,
    /// Byte offset of the `:` separator (cached from parse).
    colon: usize,
}

impl ContentHash {
    /// Parse from `"algorithm:hex"` format.
    ///
    /// Returns `None` if the format is invalid (missing colon,
    /// empty algorithm, or empty digest).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let colon = s.find(':')?;
        if colon == 0 || colon == s.len() - 1 {
            return None;
        }
        Some(Self {
            full: s.to_string(),
            colon,
        })
    }

    /// The algorithm portion (e.g., "sha256").
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.full[..self.colon]
    }

    /// The hex digest portion.
    #[must_use]
    pub fn hex_digest(&self) -> &str {
        &self.full[self.colon + 1..]
    }

    /// The full string representation (`"algorithm:hex_digest"`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.full
    }
}

// Domain separation constants.
// These match v1 exactly: `core/carrier/bytestate.py` and `core/carrier/bytetrace.py`.
// Each prefix is null-terminated.

/// Domain prefix for `ByteState` identity plane hashing.
pub const DOMAIN_IDENTITY_PLANE: &[u8] = b"STERLING::BYTESTATE_IDENTITY::V1\0";

/// Domain prefix for `ByteState` evidence hashing (identity + status).
pub const DOMAIN_EVIDENCE_PLANE: &[u8] = b"STERLING::BYTESTATE_EVIDENCE::V1\0";

/// Domain prefix for `ByteTrace` payload hashing.
pub const DOMAIN_BYTETRACE: &[u8] = b"STERLING::BYTETRACE::V1\0";

/// Domain prefix for registry snapshot hashing.
pub const DOMAIN_REGISTRY_SNAPSHOT: &[u8] = b"STERLING::REGISTRY_SNAPSHOT::V1\0";

/// Domain prefix for schema bundle hashing.
pub const DOMAIN_SCHEMA_BUNDLE: &[u8] = b"STERLING::BYTESTATE_SCHEMA_BUNDLE::V1\0";

/// Compute the canonical hash of a byte slice with domain separation.
///
/// Algorithm: SHA-256 (V1-compatible).
/// Result format: `"sha256:<hex_digest>"`.
///
/// # Errors
///
/// This function does not fail; it always produces a valid hash.
///
/// # Panics
///
/// M0 stub. Will panic until M1 provides the `sha2` implementation.
#[must_use]
pub fn canonical_hash(_domain: &[u8], _data: &[u8]) -> ContentHash {
    todo!("M1: implement sha256 canonical hashing with domain separation")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_parse_valid() {
        let h = ContentHash::parse("sha256:abcdef0123456789").unwrap();
        assert_eq!(h.algorithm(), "sha256");
        assert_eq!(h.hex_digest(), "abcdef0123456789");
        assert_eq!(h.as_str(), "sha256:abcdef0123456789");
    }

    #[test]
    fn content_hash_parse_rejects_bad_format() {
        assert!(ContentHash::parse("nocolon").is_none());
        assert!(ContentHash::parse(":noalg").is_none());
        assert!(ContentHash::parse("nodigest:").is_none());
    }

    #[test]
    fn domain_prefixes_are_null_terminated() {
        assert!(DOMAIN_IDENTITY_PLANE.ends_with(&[0]));
        assert!(DOMAIN_EVIDENCE_PLANE.ends_with(&[0]));
        assert!(DOMAIN_BYTETRACE.ends_with(&[0]));
        assert!(DOMAIN_REGISTRY_SNAPSHOT.ends_with(&[0]));
        assert!(DOMAIN_SCHEMA_BUNDLE.ends_with(&[0]));
    }

    #[test]
    fn domain_prefixes_match_v1() {
        // Cross-reference with v1: core/carrier/bytestate.py lines 28-30
        assert_eq!(DOMAIN_IDENTITY_PLANE, b"STERLING::BYTESTATE_IDENTITY::V1\0");
        assert_eq!(DOMAIN_EVIDENCE_PLANE, b"STERLING::BYTESTATE_EVIDENCE::V1\0");
        // Cross-reference with v1: core/carrier/bytetrace.py line 36
        assert_eq!(DOMAIN_BYTETRACE, b"STERLING::BYTETRACE::V1\0");
    }
}
