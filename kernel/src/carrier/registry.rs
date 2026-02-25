//! `Code32` registry: the bijective `Code32` to `ConceptID` mapping for a given epoch.
//!
//! The registry proves the bijection. Code allocations within a certification
//! epoch are append-only. Remapping requires a version bump and replay
//! verification against the prior epoch.
//!
//! The kernel uses the registry to validate that payloads only reference
//! allocated Code32 values. Unknown codes cause fail-closed compilation
//! (no auto-allocation).

use std::collections::BTreeMap;

use crate::carrier::bytestate::RegistrySnapshot;
use crate::carrier::code32::Code32;
use crate::proof::canon::canonical_json_bytes;
use crate::proof::hash::{canonical_hash, ContentHash, DOMAIN_REGISTRY_SNAPSHOT};

/// A `Code32` registry for a given epoch.
///
/// Maps `Code32` to `ConceptID` (string). The bijection is enforced at
/// construction time: duplicate `Code32` values or duplicate concept IDs
/// are rejected.
///
/// Allocations are stored in a `BTreeMap` for deterministic iteration
/// order (sorted by Code32 bytes, which is the same as their Ord impl).
#[derive(Debug, Clone)]
pub struct RegistryV1 {
    epoch: String,
    allocations: BTreeMap<Code32, String>,
    reverse: BTreeMap<String, Code32>,
}

/// Error type for registry construction and lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// A Code32 was allocated more than once.
    DuplicateCode32 { code: Code32, concept_id: String },
    /// A `ConceptID` was mapped to more than one Code32.
    DuplicateConceptId {
        concept_id: String,
        existing_code: Code32,
        new_code: Code32,
    },
    /// Canonical JSON serialization failed (non-integer number).
    CanonicalizationError { detail: String },
}

impl RegistryV1 {
    /// Create a new registry from an epoch and a set of allocations.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the bijection is violated (duplicate
    /// Code32 or duplicate `ConceptID`).
    pub fn new(epoch: String, allocations: Vec<(Code32, String)>) -> Result<Self, RegistryError> {
        let mut forward: BTreeMap<Code32, String> = BTreeMap::new();
        let mut reverse: BTreeMap<String, Code32> = BTreeMap::new();

        for (code, concept_id) in allocations {
            if let Some(existing) = forward.get(&code) {
                return Err(RegistryError::DuplicateCode32 {
                    code,
                    concept_id: existing.clone(),
                });
            }
            if let Some(&existing_code) = reverse.get(&concept_id) {
                return Err(RegistryError::DuplicateConceptId {
                    concept_id,
                    existing_code,
                    new_code: code,
                });
            }
            forward.insert(code, concept_id.clone());
            reverse.insert(concept_id, code);
        }

        Ok(Self {
            epoch,
            allocations: forward,
            reverse,
        })
    }

    /// The epoch identifier.
    #[must_use]
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// Whether a Code32 is allocated in this registry.
    #[must_use]
    pub fn contains(&self, code: &Code32) -> bool {
        self.allocations.contains_key(code)
    }

    /// Look up a Code32 by `ConceptID`.
    #[must_use]
    pub fn code_for_concept(&self, concept_id: &str) -> Option<Code32> {
        self.reverse.get(concept_id).copied()
    }

    /// Look up a `ConceptID` by Code32.
    #[must_use]
    pub fn concept_for_code(&self, code: &Code32) -> Option<&str> {
        self.allocations.get(code).map(String::as_str)
    }

    /// Number of allocations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.allocations.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty()
    }

    /// Produce canonical JSON bytes for this registry.
    ///
    /// Format: `{"allocations":[["<concept_id>",[d,k,lo,hi]],...], "epoch":"<epoch>"}`
    ///
    /// Allocations are sorted by `Code32` bytes (`BTreeMap` iteration order).
    /// Each allocation is `[concept_id, [domain, kind, local_lo, local_hi]]`.
    /// Top-level keys are sorted ("allocations" < "epoch").
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::CanonicalizationError`] if canonical JSON
    /// serialization fails (should not happen for well-formed registries).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RegistryError> {
        let allocs: Vec<serde_json::Value> = self
            .allocations
            .iter()
            .map(|(code, concept_id)| {
                let bytes = code.to_le_bytes();
                serde_json::json!([
                    concept_id,
                    [
                        u64::from(bytes[0]),
                        u64::from(bytes[1]),
                        u64::from(bytes[2]),
                        u64::from(bytes[3])
                    ]
                ])
            })
            .collect();

        let value = serde_json::json!({
            "allocations": allocs,
            "epoch": self.epoch,
        });

        canonical_json_bytes(&value).map_err(|e| RegistryError::CanonicalizationError {
            detail: e.to_string(),
        })
    }

    /// Compute the canonical digest for this registry.
    ///
    /// `sha256(DOMAIN_REGISTRY_SNAPSHOT || canonical_bytes)`
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::CanonicalizationError`] if canonical bytes
    /// cannot be produced.
    pub fn digest(&self) -> Result<ContentHash, RegistryError> {
        let bytes = self.canonical_bytes()?;
        Ok(canonical_hash(DOMAIN_REGISTRY_SNAPSHOT, &bytes))
    }

    /// Produce a compact [`RegistrySnapshot`] descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::CanonicalizationError`] if the digest
    /// cannot be computed.
    pub fn snapshot(&self) -> Result<RegistrySnapshot, RegistryError> {
        let digest = self.digest()?;
        Ok(RegistrySnapshot {
            epoch: self.epoch.clone(),
            hash: digest.as_str().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> RegistryV1 {
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

    #[test]
    fn registry_membership() {
        let reg = sample_registry();
        assert!(reg.contains(&Code32::new(1, 0, 0)));
        assert!(reg.contains(&Code32::new(1, 1, 0)));
        // Not allocated.
        assert!(!reg.contains(&Code32::new(2, 0, 0)));
        assert!(!reg.contains(&Code32::PADDING));
    }

    #[test]
    fn registry_bijection() {
        let reg = sample_registry();
        assert_eq!(
            reg.code_for_concept("rome:node:start"),
            Some(Code32::new(1, 0, 0))
        );
        assert_eq!(
            reg.concept_for_code(&Code32::new(1, 0, 1)),
            Some("rome:node:forum")
        );
        assert_eq!(reg.code_for_concept("nonexistent"), None);
        assert_eq!(reg.concept_for_code(&Code32::new(99, 0, 0)), None);
    }

    #[test]
    fn registry_rejects_duplicate_code32() {
        let result = RegistryV1::new(
            "epoch-0".into(),
            vec![
                (Code32::new(1, 0, 0), "concept_a".into()),
                (Code32::new(1, 0, 0), "concept_b".into()),
            ],
        );
        assert!(matches!(result, Err(RegistryError::DuplicateCode32 { .. })));
    }

    #[test]
    fn registry_rejects_duplicate_concept_id() {
        let result = RegistryV1::new(
            "epoch-0".into(),
            vec![
                (Code32::new(1, 0, 0), "same_concept".into()),
                (Code32::new(1, 0, 1), "same_concept".into()),
            ],
        );
        assert!(matches!(
            result,
            Err(RegistryError::DuplicateConceptId { .. })
        ));
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let reg = sample_registry();
        let first = reg.canonical_bytes().unwrap();
        for _ in 0..10 {
            assert_eq!(reg.canonical_bytes().unwrap(), first);
        }
    }

    #[test]
    fn canonical_bytes_key_sorted() {
        let reg = sample_registry();
        let bytes = reg.canonical_bytes().unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        // Top-level keys: "allocations" < "epoch" (sorted).
        assert!(s.starts_with("{\"allocations\":"));
        assert!(s.contains("\"epoch\":\"epoch-0\""));
    }

    #[test]
    fn digest_is_sha256() {
        let reg = sample_registry();
        let h = reg.digest().unwrap();
        assert_eq!(h.algorithm(), "sha256");
        assert_eq!(h.hex_digest().len(), 64);
    }

    #[test]
    fn digest_deterministic() {
        let reg = sample_registry();
        let first = reg.digest().unwrap();
        for _ in 0..10 {
            assert_eq!(reg.digest().unwrap(), first);
        }
    }

    #[test]
    fn snapshot_matches_digest() {
        let reg = sample_registry();
        let snap = reg.snapshot().unwrap();
        let digest = reg.digest().unwrap();
        assert_eq!(snap.epoch, "epoch-0");
        assert_eq!(snap.hash, digest.as_str());
    }

    #[test]
    fn canonical_bytes_independent_of_insertion_order() {
        // Same allocations, different insertion order.
        let reg1 = RegistryV1::new(
            "epoch-0".into(),
            vec![
                (Code32::new(1, 0, 2), "c".into()),
                (Code32::new(1, 0, 0), "a".into()),
                (Code32::new(1, 0, 1), "b".into()),
            ],
        )
        .unwrap();
        let reg2 = RegistryV1::new(
            "epoch-0".into(),
            vec![
                (Code32::new(1, 0, 0), "a".into()),
                (Code32::new(1, 0, 1), "b".into()),
                (Code32::new(1, 0, 2), "c".into()),
            ],
        )
        .unwrap();
        assert_eq!(
            reg1.canonical_bytes().unwrap(),
            reg2.canonical_bytes().unwrap()
        );
        assert_eq!(reg1.digest().unwrap(), reg2.digest().unwrap());
    }

    // --- Golden fixture (S1-M1-REGISTRY-HASH-GOLDEN) ---
    // Generated offline by Python: hashlib.sha256(prefix + canonical_json).hexdigest()
    // where prefix = b"STERLING::REGISTRY_SNAPSHOT::V1\0"

    #[test]
    fn golden_registry_digest() {
        let reg = sample_registry();
        let h = reg.digest().unwrap();
        assert_eq!(
            h.as_str(),
            "sha256:956f5485105685cd78391cc3b2212c474fdc02aab3363aa980d7f198407e6a74"
        );
    }

    #[test]
    fn empty_registry() {
        let reg = RegistryV1::new("epoch-empty".into(), vec![]).unwrap();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        // Should still produce valid canonical bytes and digest.
        let bytes = reg.canonical_bytes().unwrap();
        assert!(!bytes.is_empty());
        let h = reg.digest().unwrap();
        assert_eq!(h.algorithm(), "sha256");
    }
}
