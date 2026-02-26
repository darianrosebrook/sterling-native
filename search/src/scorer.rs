//! Value scoring types and traits.

use std::collections::BTreeMap;

use sterling_kernel::proof::hash::ContentHash;

use crate::node::{CandidateActionV1, SearchNodeV1};

/// Provenance tag for a candidate score.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreSourceV1 {
    /// Uniform scorer (all candidates scored equally).
    Uniform,
    /// Model-based scorer identified by its digest.
    ModelDigest(ContentHash),
    /// Scorer did not produce a score (panic or contract violation).
    Unavailable,
}

/// A scored candidate with provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateScoreV1 {
    /// Additive priority bonus (integer in Cert mode).
    pub bonus: i64,
    /// Deterministic provenance tag.
    pub source: ScoreSourceV1,
}

/// Trait for candidate scoring.
///
/// Implementations must return one score per candidate (same length as input).
/// Scores are integers — SEARCH-CORE-001 is Cert-only.
pub trait ValueScorer: Send + Sync {
    /// Score a batch of candidates at a given node.
    ///
    /// Must return exactly `candidates.len()` scores.
    fn score_candidates(
        &self,
        node: &SearchNodeV1,
        candidates: &[CandidateActionV1],
    ) -> Vec<CandidateScoreV1>;
}

/// Default scorer: returns 0 bonus for all candidates.
#[derive(Debug, Clone, Copy)]
pub struct UniformScorer;

impl ValueScorer for UniformScorer {
    fn score_candidates(
        &self,
        _node: &SearchNodeV1,
        candidates: &[CandidateActionV1],
    ) -> Vec<CandidateScoreV1> {
        candidates
            .iter()
            .map(|_| CandidateScoreV1 {
                bonus: 0,
                source: ScoreSourceV1::Uniform,
            })
            .collect()
    }
}

/// Table-based scorer: looks up bonus by candidate `canonical_hash`.
///
/// Keys are in `ContentHash.as_str()` format (`"sha256:hex"`).
/// Unknown candidates receive bonus 0. The digest is injected by the
/// harness (bundle-domain hash), not self-computed.
#[derive(Debug, Clone)]
pub struct TableScorer {
    table: BTreeMap<String, i64>,
    digest: ContentHash,
}

impl TableScorer {
    /// Create a new table scorer with an injected digest.
    ///
    /// The `digest` is computed by the harness from the canonical
    /// serialization of the table using `DOMAIN_BUNDLE_ARTIFACT`.
    /// The search crate does not define or duplicate bundle artifact
    /// domain constants.
    #[must_use]
    pub fn new(table: BTreeMap<String, i64>, digest: ContentHash) -> Self {
        Self { table, digest }
    }

    /// The injected digest for provenance tagging.
    #[must_use]
    pub fn digest(&self) -> &ContentHash {
        &self.digest
    }

    /// The underlying score table (read-only).
    #[must_use]
    pub fn table(&self) -> &BTreeMap<String, i64> {
        &self.table
    }

    /// Serialize the table to canonical JSON bytes for artifact generation.
    ///
    /// Schema envelope: `{"schema_version":"scorer.v1","kind":"table","entries":[...]}`
    /// where entries are sorted by key (BTreeMap order).
    pub fn to_canonical_json_bytes(
        &self,
    ) -> Result<Vec<u8>, sterling_kernel::proof::canon::CanonError> {
        let entries: Vec<serde_json::Value> = self
            .table
            .iter()
            .map(|(k, v)| serde_json::json!({"candidate_hash": k, "bonus": v}))
            .collect();
        let value = serde_json::json!({
            "entries": entries,
            "kind": "table",
            "schema_version": "scorer.v1",
        });
        sterling_kernel::proof::canon::canonical_json_bytes(&value)
    }
}

impl ValueScorer for TableScorer {
    fn score_candidates(
        &self,
        _node: &SearchNodeV1,
        candidates: &[CandidateActionV1],
    ) -> Vec<CandidateScoreV1> {
        candidates
            .iter()
            .map(|c| {
                let bonus = self
                    .table
                    .get(c.canonical_hash.as_str())
                    .copied()
                    .unwrap_or(0);
                CandidateScoreV1 {
                    bonus,
                    source: ScoreSourceV1::ModelDigest(self.digest.clone()),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sterling_kernel::carrier::bytestate::ByteStateV1;
    use sterling_kernel::carrier::code32::Code32;
    use sterling_kernel::proof::hash::ContentHash;

    fn dummy_node() -> SearchNodeV1 {
        SearchNodeV1 {
            node_id: 0,
            parent_id: None,
            state: ByteStateV1::new(1, 2),
            state_fingerprint: ContentHash::parse(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            depth: 0,
            g_cost: 0,
            h_cost: 0,
            creation_order: 0,
            producing_action: None,
        }
    }

    fn dummy_digest() -> ContentHash {
        ContentHash::parse(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap()
    }

    #[test]
    fn uniform_scorer_returns_zero_for_all() {
        let scorer = UniformScorer;
        let node = dummy_node();
        let candidates = vec![
            CandidateActionV1::new(Code32::new(1, 1, 1), vec![0u8; 12]),
            CandidateActionV1::new(Code32::new(2, 1, 1), vec![0u8; 12]),
        ];

        let result = scorer.score_candidates(&node, &candidates);
        assert_eq!(result.len(), 2);
        for s in &result {
            assert_eq!(s.bonus, 0);
            assert!(matches!(s.source, ScoreSourceV1::Uniform));
        }
    }

    #[test]
    fn uniform_scorer_returns_correct_length() {
        let scorer = UniformScorer;
        let node = dummy_node();
        let result = scorer.score_candidates(&node, &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn table_scorer_returns_configured_bonus() {
        let c1 = CandidateActionV1::new(Code32::new(1, 1, 1), vec![0u8; 12]);
        let hash_key = c1.canonical_hash.as_str().to_string();
        let mut table = BTreeMap::new();
        table.insert(hash_key, 42);
        let scorer = TableScorer::new(table, dummy_digest());
        let node = dummy_node();
        let result = scorer.score_candidates(&node, &[c1]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bonus, 42);
    }

    #[test]
    fn table_scorer_returns_zero_for_unknown() {
        let scorer = TableScorer::new(BTreeMap::new(), dummy_digest());
        let node = dummy_node();
        let c1 = CandidateActionV1::new(Code32::new(1, 1, 1), vec![0u8; 12]);
        let result = scorer.score_candidates(&node, &[c1]);
        assert_eq!(result[0].bonus, 0);
    }

    #[test]
    fn table_scorer_source_is_model_digest() {
        let digest = dummy_digest();
        let scorer = TableScorer::new(BTreeMap::new(), digest.clone());
        let node = dummy_node();
        let c1 = CandidateActionV1::new(Code32::new(1, 1, 1), vec![0u8; 12]);
        let result = scorer.score_candidates(&node, &[c1]);
        assert_eq!(result[0].source, ScoreSourceV1::ModelDigest(digest));
    }

    #[test]
    fn table_scorer_canonical_json_is_deterministic() {
        let mut table = BTreeMap::new();
        table.insert("sha256:bbbb".to_string(), 10);
        table.insert("sha256:aaaa".to_string(), 20);
        let scorer = TableScorer::new(table, dummy_digest());
        let bytes1 = scorer.to_canonical_json_bytes().unwrap();
        let bytes2 = scorer.to_canonical_json_bytes().unwrap();
        assert_eq!(bytes1, bytes2);
        // Verify sorted order (aaaa before bbbb)
        let json: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
        let entries = json["entries"].as_array().unwrap();
        assert_eq!(entries[0]["candidate_hash"], "sha256:aaaa");
        assert_eq!(entries[1]["candidate_hash"], "sha256:bbbb");
    }

    #[test]
    fn table_scorer_uses_injected_digest() {
        let digest = dummy_digest();
        let scorer = TableScorer::new(BTreeMap::new(), digest.clone());
        assert_eq!(scorer.digest().as_str(), digest.as_str());
    }

    #[test]
    fn table_scorer_empty_table_works() {
        let scorer = TableScorer::new(BTreeMap::new(), dummy_digest());
        let node = dummy_node();
        let result = scorer.score_candidates(&node, &[]);
        assert!(result.is_empty());
    }
}
