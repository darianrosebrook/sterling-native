//! Value scoring types and traits.

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
}
