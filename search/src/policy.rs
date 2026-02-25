//! Search policy types.

use crate::error::SearchError;

/// Search-specific budget and dedup configuration.
///
/// Extends the harness `PolicySnapshotV1` with search budgets.
/// SEARCH-CORE-001 is Cert-only: integer scores, total ordering, bit-reproducible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPolicyV1 {
    /// Hard cap on node expansions.
    pub max_expansions: u64,
    /// Frontier prune threshold.
    pub max_frontier_size: u64,
    /// Depth cutoff.
    pub max_depth: u32,
    /// Candidate generation cap per node.
    pub max_candidates_per_node: u64,
    /// Dedup key policy (M1 default: `IdentityOnly`).
    pub dedup_key: DedupKeyV1,
    /// Pruning policy for visited nodes (M1 default: `KeepVisited`).
    pub prune_visited_policy: PruneVisitedPolicyV1,
}

impl SearchPolicyV1 {
    /// Validate that this policy uses only M1-supported options.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::UnsupportedPolicyMode`] if reserved options
    /// (`FullState`, `ReleaseVisited`) are selected.
    pub fn validate_m1(&self) -> Result<(), SearchError> {
        if self.dedup_key == DedupKeyV1::FullState {
            return Err(SearchError::UnsupportedPolicyMode {
                detail: "DedupKeyV1::FullState is reserved and not supported in M1".into(),
            });
        }
        if self.prune_visited_policy == PruneVisitedPolicyV1::ReleaseVisited {
            return Err(SearchError::UnsupportedPolicyMode {
                detail: "PruneVisitedPolicyV1::ReleaseVisited is reserved and not supported in M1"
                    .into(),
            });
        }
        Ok(())
    }
}

impl Default for SearchPolicyV1 {
    fn default() -> Self {
        Self {
            max_expansions: 1000,
            max_frontier_size: 10_000,
            max_depth: 100,
            max_candidates_per_node: 1000,
            dedup_key: DedupKeyV1::IdentityOnly,
            prune_visited_policy: PruneVisitedPolicyV1::KeepVisited,
        }
    }
}

/// Dedup key policy: how state fingerprints are computed for loop detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupKeyV1 {
    /// `canonical_hash(DOMAIN_SEARCH_NODE, state.identity_bytes())`.
    /// Default for M1.
    IdentityOnly,
    /// Reserved for future use. Selecting this in M1 is a hard error.
    FullState,
}

/// Pruning policy for visited nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneVisitedPolicyV1 {
    /// Pruned nodes remain in the visited set (irreversible pruning).
    /// Default for M1.
    KeepVisited,
    /// Reserved for future use. Selecting this in M1 is a hard error.
    ReleaseVisited,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_passes_m1_validation() {
        let policy = SearchPolicyV1::default();
        assert!(policy.validate_m1().is_ok());
    }

    // ACCEPTANCE: SC1-M1-RESERVED-POLICY-HARDERROR
    #[test]
    fn fullstate_dedup_key_rejected_in_m1() {
        let policy = SearchPolicyV1 {
            dedup_key: DedupKeyV1::FullState,
            ..SearchPolicyV1::default()
        };
        let err = policy.validate_m1().unwrap_err();
        assert!(
            matches!(err, SearchError::UnsupportedPolicyMode { .. }),
            "expected UnsupportedPolicyMode, got {err:?}"
        );
    }

    // ACCEPTANCE: SC1-M1-RESERVED-POLICY-HARDERROR
    #[test]
    fn release_visited_rejected_in_m1() {
        let policy = SearchPolicyV1 {
            prune_visited_policy: PruneVisitedPolicyV1::ReleaseVisited,
            ..SearchPolicyV1::default()
        };
        let err = policy.validate_m1().unwrap_err();
        assert!(
            matches!(err, SearchError::UnsupportedPolicyMode { .. }),
            "expected UnsupportedPolicyMode, got {err:?}"
        );
    }
}
