//! Canonical hashing types and domain separation constants.
//!
//! Domain prefix constants match v1 exactly (byte-for-byte, null-terminated).
//! Algorithm: SHA-256 for all V1 artifacts. Blake3 reserved for future V2.
//!
//! **Exactly one place defines canonical hashing** (SPINE-001 invariant).

use sha2::{Digest, Sha256};

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
    /// Parse from `"algorithm:hex_digest"` format.
    ///
    /// Validation rules (enforced to prevent "almost-valid" artifacts):
    /// - Exactly one `:` separator.
    /// - Algorithm: non-empty, ASCII lowercase alphanumeric only (e.g., `sha256`, `blake3`).
    /// - Digest: non-empty, lowercase hex only (`[0-9a-f]+`).
    ///
    /// Returns `None` if the format is invalid.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let colon = s.find(':')?;

        // Exactly one colon.
        if s[colon + 1..].contains(':') {
            return None;
        }

        let algorithm = &s[..colon];
        let digest = &s[colon + 1..];

        // Algorithm: non-empty, lowercase ASCII alphanumeric.
        if algorithm.is_empty()
            || !algorithm
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return None;
        }

        // Digest: non-empty, lowercase hex.
        if digest.is_empty()
            || !digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
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

/// Domain prefix for compilation payload commitment.
///
/// This is a Native-originated prefix (not from v1) — the `compile()` boundary
/// is a new concept. Using a dedicated prefix prevents domain collapse between
/// "payload commitment in compilation manifest" and "identity plane digest."
pub const DOMAIN_COMPILATION_PAYLOAD: &[u8] = b"STERLING::COMPILATION_PAYLOAD::V1\0";

/// Compute the canonical hash of a byte slice with domain separation.
///
/// Algorithm: SHA-256 (V1-compatible).
/// Computes `sha256(domain_prefix || data)` and returns `"sha256:<hex_digest>"`.
///
/// The domain prefix must include the null terminator (all `DOMAIN_*` constants
/// in this module already do). This matches v1's hashing exactly.
#[must_use]
pub fn canonical_hash(domain: &[u8], data: &[u8]) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(data);
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    let full = format!("sha256:{hex}");
    let colon = 6; // "sha256" is 6 bytes
    ContentHash { full, colon }
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
        // Missing colon.
        assert!(ContentHash::parse("nocolon").is_none());
        // Empty algorithm or digest.
        assert!(ContentHash::parse(":noalg").is_none());
        assert!(ContentHash::parse("nodigest:").is_none());
        // Multiple colons.
        assert!(ContentHash::parse("sha256:abc:def").is_none());
        // Uppercase algorithm.
        assert!(ContentHash::parse("SHA256:abcdef").is_none());
        // Uppercase hex in digest.
        assert!(ContentHash::parse("sha256:ABCDEF").is_none());
        // Non-hex in digest.
        assert!(ContentHash::parse("sha256:xyz123").is_none());
        // Non-alphanumeric algorithm.
        assert!(ContentHash::parse("sha-256:abcdef").is_none());
    }

    #[test]
    fn domain_prefixes_are_null_terminated() {
        assert!(DOMAIN_IDENTITY_PLANE.ends_with(&[0]));
        assert!(DOMAIN_EVIDENCE_PLANE.ends_with(&[0]));
        assert!(DOMAIN_BYTETRACE.ends_with(&[0]));
        assert!(DOMAIN_REGISTRY_SNAPSHOT.ends_with(&[0]));
        assert!(DOMAIN_SCHEMA_BUNDLE.ends_with(&[0]));
        assert!(DOMAIN_COMPILATION_PAYLOAD.ends_with(&[0]));
    }

    #[test]
    fn domain_prefixes_match_v1() {
        // Cross-reference with v1: core/carrier/bytestate.py lines 28-30
        assert_eq!(DOMAIN_IDENTITY_PLANE, b"STERLING::BYTESTATE_IDENTITY::V1\0");
        assert_eq!(DOMAIN_EVIDENCE_PLANE, b"STERLING::BYTESTATE_EVIDENCE::V1\0");
        // Cross-reference with v1: core/carrier/bytetrace.py line 36
        assert_eq!(DOMAIN_BYTETRACE, b"STERLING::BYTETRACE::V1\0");
    }

    // --- V1 parity test vectors (S1-M1-HASH-V1-VECTORS) ---
    // Generated offline by Python: hashlib.sha256(prefix + data).hexdigest()
    // These prove domain-separated SHA-256 matches v1 oracle output,
    // including the null-terminated prefix bytes.

    #[test]
    fn hash_vector_identity_prefix_empty_data() {
        let h = canonical_hash(DOMAIN_IDENTITY_PLANE, b"");
        assert_eq!(h.algorithm(), "sha256");
        assert_eq!(
            h.hex_digest(),
            "31bd6f65a99fde83bdf0daf1097ae7a125293da9560fc22fc6d04f1f1cce813c"
        );
    }

    #[test]
    fn hash_vector_evidence_prefix_hello() {
        let h = canonical_hash(DOMAIN_EVIDENCE_PLANE, b"hello");
        assert_eq!(
            h.hex_digest(),
            "a602de1de411d50e90ff92d29b09e310b853b530b5946b9ffacefa12ddea1b48"
        );
    }

    #[test]
    fn hash_vector_bytetrace_prefix_bytes() {
        let h = canonical_hash(DOMAIN_BYTETRACE, &[0x00, 0x01, 0x02, 0x03]);
        assert_eq!(
            h.hex_digest(),
            "44f05a34c7e7f00aa1e415f2ca50b5a7e9757eda94357c9064ec7fe9cee55cfc"
        );
    }

    #[test]
    fn hash_vector_registry_prefix_json() {
        let h = canonical_hash(DOMAIN_REGISTRY_SNAPSHOT, br#"{"epoch":"test"}"#);
        assert_eq!(
            h.hex_digest(),
            "a32aab7658bb3b8ad8cdbad70ad071ef9a17a5a560ad91394dfead7e9249caa2"
        );
    }

    #[test]
    fn hash_vector_compilation_payload_prefix() {
        let h = canonical_hash(DOMAIN_COMPILATION_PAYLOAD, b"test-payload");
        assert_eq!(
            h.hex_digest(),
            "0bcc379abca7b4f1c5abf704feb81362f1f2b406435661666c10ae240960f275"
        );
    }

    #[test]
    fn canonical_hash_returns_valid_content_hash() {
        let h = canonical_hash(DOMAIN_IDENTITY_PLANE, b"test");
        // Must be parseable.
        assert!(ContentHash::parse(h.as_str()).is_some());
        assert_eq!(h.algorithm(), "sha256");
        // SHA-256 digest is always 64 hex chars.
        assert_eq!(h.hex_digest().len(), 64);
    }

    #[test]
    fn canonical_hash_deterministic() {
        let first = canonical_hash(DOMAIN_EVIDENCE_PLANE, b"determinism");
        for _ in 0..10 {
            assert_eq!(canonical_hash(DOMAIN_EVIDENCE_PLANE, b"determinism"), first);
        }
    }
}
