//! Core search node and candidate action types.

use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::proof::hash::ContentHash;

/// Domain prefix for search node fingerprints.
pub const DOMAIN_SEARCH_NODE: &[u8] = b"STERLING::SEARCH_NODE::V1\0";

/// Domain prefix for candidate action content hashing.
/// Distinct from `DOMAIN_SEARCH_NODE` to prevent cross-domain collisions.
pub const DOMAIN_SEARCH_CANDIDATE: &[u8] = b"STERLING::SEARCH_CANDIDATE::V1\0";

/// An immutable search node in the frontier.
///
/// Ordering for frontier extraction uses `(f_cost, depth, creation_order)`
/// where `f_cost = g_cost + h_cost`. Lower is better; ties broken by
/// shallower depth, then older creation order.
#[derive(Debug, Clone)]
pub struct SearchNodeV1 {
    /// Monotonic node identifier assigned by the frontier.
    pub node_id: u64,
    /// Parent node ID (`None` for root).
    pub parent_id: Option<u64>,
    /// Full immutable state at this node.
    pub state: ByteStateV1,
    /// Canonical hash of identity bytes under the dedup policy.
    pub state_fingerprint: ContentHash,
    /// Tree depth (root = 0).
    pub depth: u32,
    /// Cumulative path cost (integer; +1 per applied step in M1).
    pub g_cost: i64,
    /// Heuristic estimate (integer; 0 default, world may override).
    pub h_cost: i64,
    /// Global counter for deterministic tie-breaking.
    pub creation_order: u64,
    /// The action that produced this node from its parent.
    pub producing_action: Option<CandidateActionV1>,
}

impl SearchNodeV1 {
    /// Compute `f_cost = g_cost + h_cost` (the frontier ordering key).
    #[must_use]
    pub fn f_cost(&self) -> i64 {
        self.g_cost.saturating_add(self.h_cost)
    }
}

/// A candidate operator application proposed by a world.
///
/// Candidates are sorted by `canonical_hash` for deterministic enumeration
/// before scorer bias is applied.
///
/// Construct via [`CandidateActionV1::new`] — the `canonical_hash` is computed
/// internally to prevent callers from supplying incorrect hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateActionV1 {
    /// The operator code to apply.
    pub op_code: Code32,
    /// Serialized operator arguments (must be kernel-canonical bytes).
    pub op_args: Vec<u8>,
    /// Content-addressed hash of `(op_code, op_args)` for dedup and ordering.
    /// Computed by [`CandidateActionV1::new`]; not directly settable outside this crate.
    pub(crate) canonical_hash: ContentHash,
}

impl CandidateActionV1 {
    /// Construct a candidate action, computing the canonical hash from `op_code` and `op_args`.
    ///
    /// `op_args` must be kernel-canonical bytes (operator ABI encoding). Worlds must
    /// not pass ad-hoc structured serialization — use the operator's canonical arg
    /// encoder (e.g., `set_slot_args`).
    #[must_use]
    pub fn new(op_code: Code32, op_args: Vec<u8>) -> Self {
        let canonical_hash = candidate_canonical_hash(op_code, &op_args);
        Self {
            op_code,
            op_args,
            canonical_hash,
        }
    }

    /// Read-only access to the canonical hash.
    #[must_use]
    pub fn canonical_hash(&self) -> &ContentHash {
        &self.canonical_hash
    }
}

impl PartialOrd for CandidateActionV1 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CandidateActionV1 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.canonical_hash.cmp(&other.canonical_hash)
    }
}

/// Compute the canonical hash for a candidate action.
///
/// Hash = `canonical_hash(DOMAIN_SEARCH_CANDIDATE, op_code_le_bytes || op_args)`.
#[must_use]
pub fn candidate_canonical_hash(op_code: Code32, op_args: &[u8]) -> ContentHash {
    let mut data = Vec::with_capacity(4 + op_args.len());
    data.extend_from_slice(&op_code.to_le_bytes());
    data.extend_from_slice(op_args);
    sterling_kernel::proof::hash::canonical_hash(DOMAIN_SEARCH_CANDIDATE, &data)
}

/// The frontier ordering key: `(f_cost, depth, creation_order)`.
///
/// Lower `f_cost` first, then shallower depth, then older `creation_order`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontierKey {
    pub f_cost: i64,
    pub depth: u32,
    pub creation_order: u64,
}

impl PartialOrd for FrontierKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FrontierKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.f_cost
            .cmp(&other.f_cost)
            .then(self.depth.cmp(&other.depth))
            .then(self.creation_order.cmp(&other.creation_order))
    }
}

impl From<&SearchNodeV1> for FrontierKey {
    fn from(node: &SearchNodeV1) -> Self {
        Self {
            f_cost: node.f_cost(),
            depth: node.depth,
            creation_order: node.creation_order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontier_key_lower_f_cost_wins() {
        let a = FrontierKey {
            f_cost: 1,
            depth: 5,
            creation_order: 10,
        };
        let b = FrontierKey {
            f_cost: 2,
            depth: 1,
            creation_order: 1,
        };
        assert!(a < b, "lower f_cost should sort first");
    }

    #[test]
    fn frontier_key_ties_broken_by_depth_then_creation_order() {
        let a = FrontierKey {
            f_cost: 1,
            depth: 2,
            creation_order: 5,
        };
        let b = FrontierKey {
            f_cost: 1,
            depth: 3,
            creation_order: 1,
        };
        assert!(a < b, "shallower depth should sort first on f_cost tie");

        let c = FrontierKey {
            f_cost: 1,
            depth: 2,
            creation_order: 3,
        };
        assert!(
            c < a,
            "older creation_order should sort first on f_cost+depth tie"
        );
    }

    #[test]
    fn candidate_canonical_hash_is_deterministic() {
        let code = Code32::new(1, 1, 1);
        let args = vec![0u8; 12];
        let h1 = candidate_canonical_hash(code, &args);
        let h2 = candidate_canonical_hash(code, &args);
        assert_eq!(h1, h2, "same inputs must produce same hash");
    }

    #[test]
    fn candidate_ordering_uses_canonical_hash() {
        let code_a = Code32::new(1, 1, 1);
        let code_b = Code32::new(2, 1, 1);
        let args = vec![0u8; 12];

        let ca = CandidateActionV1::new(code_a, args.clone());
        let cb = CandidateActionV1::new(code_b, args.clone());

        let mut candidates = [cb.clone(), ca.clone()];
        candidates.sort();
        // Should be sorted by canonical_hash, which is deterministic
        assert_eq!(
            candidates[0]
                .canonical_hash
                .cmp(&candidates[1].canonical_hash),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn domain_separation_candidate_vs_node() {
        let code = Code32::new(1, 1, 1);
        let args = vec![0u8; 12];
        let candidate_hash = candidate_canonical_hash(code, &args);

        // Same input bytes through the node domain must differ
        let mut data = Vec::with_capacity(4 + args.len());
        data.extend_from_slice(&code.to_le_bytes());
        data.extend_from_slice(&args);
        let node_hash = sterling_kernel::proof::hash::canonical_hash(DOMAIN_SEARCH_NODE, &data);

        assert_ne!(
            candidate_hash.as_str(),
            node_hash.as_str(),
            "DOMAIN_SEARCH_CANDIDATE and DOMAIN_SEARCH_NODE must produce different hashes for same input"
        );
    }

    #[test]
    fn f_cost_is_sum_of_g_and_h() {
        let node = SearchNodeV1 {
            node_id: 0,
            parent_id: None,
            state: ByteStateV1::new(1, 2),
            state_fingerprint: ContentHash::parse(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            depth: 0,
            g_cost: 3,
            h_cost: 7,
            creation_order: 0,
            producing_action: None,
        };
        assert_eq!(node.f_cost(), 10);
    }
}
