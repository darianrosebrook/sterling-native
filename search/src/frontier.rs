//! Best-first frontier with loop detection and dead-end tracking.
//!
//! Uses `BTreeSet`-based visited set (not `HashSet`) for deterministic
//! iteration order at serialization boundaries.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use crate::node::{FrontierKey, SearchNodeV1};

/// A frontier entry wrapping a node with its ordering key.
///
/// `BinaryHeap` is a max-heap, so we use `Reverse<FrontierKey>` to get
/// min-heap behavior (lowest `f_cost` first).
#[derive(Debug)]
struct FrontierEntry {
    key: Reverse<FrontierKey>,
    node: SearchNodeV1,
}

impl PartialEq for FrontierEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for FrontierEntry {}

impl PartialOrd for FrontierEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FrontierEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

/// Best-first frontier manager.
///
/// Maintains:
/// - A `BinaryHeap` for O(log n) pop of the best node
/// - A `BTreeSet<String>` of visited state fingerprint hex digests
/// - A `BTreeSet<String>` of dead-end fingerprint hex digests
pub struct BestFirstFrontier {
    heap: BinaryHeap<FrontierEntry>,
    visited: BTreeSet<String>,
    dead_ends: BTreeSet<String>,
    high_water: u64,
}

impl BestFirstFrontier {
    /// Create a new empty frontier.
    #[must_use]
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            visited: BTreeSet::new(),
            dead_ends: BTreeSet::new(),
            high_water: 0,
        }
    }

    /// Push a node onto the frontier and mark its fingerprint as visited.
    ///
    /// Returns `false` if the fingerprint was already visited (node not added).
    pub fn push(&mut self, node: SearchNodeV1) -> bool {
        let fp = node.state_fingerprint.hex_digest().to_string();
        if !self.visited.insert(fp) {
            return false;
        }
        self.heap.push(FrontierEntry {
            key: Reverse(FrontierKey::from(&node)),
            node,
        });
        let size = self.heap.len() as u64;
        if size > self.high_water {
            self.high_water = size;
        }
        true
    }

    /// Pop the best (lowest `f_cost`) node from the frontier.
    #[must_use]
    pub fn pop(&mut self) -> Option<SearchNodeV1> {
        self.heap.pop().map(|e| e.node)
    }

    /// Check if a fingerprint has been visited.
    #[must_use]
    pub fn is_visited(&self, fingerprint_hex: &str) -> bool {
        self.visited.contains(fingerprint_hex)
    }

    /// Mark a fingerprint as a dead end.
    pub fn mark_dead_end(&mut self, fingerprint_hex: &str) {
        self.dead_ends.insert(fingerprint_hex.to_string());
    }

    /// Check if a fingerprint is a known dead end.
    #[must_use]
    pub fn is_dead_end(&self, fingerprint_hex: &str) -> bool {
        self.dead_ends.contains(fingerprint_hex)
    }

    /// Current frontier size.
    #[must_use]
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Whether the frontier is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// High-water mark of frontier size.
    #[must_use]
    pub fn high_water(&self) -> u64 {
        self.high_water
    }

    /// Number of dead ends recorded.
    #[must_use]
    pub fn dead_end_count(&self) -> usize {
        self.dead_ends.len()
    }

    /// Prune frontier to at most `max_size` entries.
    ///
    /// Keeps the best nodes by frontier key ordering.
    /// Returns the `node_ids` of pruned nodes.
    pub fn prune_to(&mut self, max_size: usize) -> Vec<u64> {
        if self.heap.len() <= max_size {
            return Vec::new();
        }

        // Drain all entries, sort by key, keep best max_size
        let mut entries: Vec<FrontierEntry> = self.heap.drain().collect();
        // BinaryHeap is a max-heap with Reverse keys, so the "best" have the
        // largest Reverse<FrontierKey> — but sorting the entries ascending by
        // key.0 (the raw FrontierKey) gives us lowest-f first.
        entries.sort_by(|a, b| a.key.0.cmp(&b.key.0));

        let pruned_ids: Vec<u64> = entries[max_size..].iter().map(|e| e.node.node_id).collect();

        entries.truncate(max_size);
        self.heap = entries.into_iter().collect();

        pruned_ids
    }
}

impl Default for BestFirstFrontier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::DOMAIN_SEARCH_NODE;
    use sterling_kernel::carrier::bytestate::ByteStateV1;
    use sterling_kernel::proof::hash::{canonical_hash, ContentHash};

    fn make_node(id: u64, g_cost: i64, depth: u32) -> SearchNodeV1 {
        let state = ByteStateV1::new(1, 2);
        // Each node gets a unique fingerprint by hashing its id
        let fp = canonical_hash(DOMAIN_SEARCH_NODE, &id.to_le_bytes());
        SearchNodeV1 {
            node_id: id,
            parent_id: None,
            state,
            state_fingerprint: fp,
            depth,
            g_cost,
            h_cost: 0,
            creation_order: id,
            producing_action: None,
        }
    }

    #[test]
    fn pop_returns_lowest_f_cost_first() {
        let mut frontier = BestFirstFrontier::new();
        frontier.push(make_node(0, 10, 0));
        frontier.push(make_node(1, 5, 0));
        frontier.push(make_node(2, 15, 0));

        let first = frontier.pop().unwrap();
        assert_eq!(first.g_cost, 5, "lowest f_cost node should pop first");
    }

    #[test]
    fn duplicate_fingerprint_rejected() {
        let mut frontier = BestFirstFrontier::new();
        let node = make_node(0, 1, 0);
        let fp_hex = node.state_fingerprint.hex_digest().to_string();

        assert!(frontier.push(node));

        // Same fingerprint, different node_id — should be rejected
        let mut dup = make_node(1, 2, 0);
        dup.state_fingerprint = ContentHash::parse(&format!("sha256:{fp_hex}")).unwrap();
        assert!(!frontier.push(dup));
    }

    #[test]
    fn prune_keeps_best_nodes() {
        let mut frontier = BestFirstFrontier::new();
        frontier.push(make_node(0, 10, 0));
        frontier.push(make_node(1, 5, 0));
        frontier.push(make_node(2, 1, 0));
        frontier.push(make_node(3, 20, 0));

        let pruned = frontier.prune_to(2);
        assert_eq!(pruned.len(), 2, "should prune 2 nodes");
        assert_eq!(frontier.len(), 2);

        // The two remaining should be the ones with lowest f_cost
        let a = frontier.pop().unwrap();
        let b = frontier.pop().unwrap();
        assert!(a.g_cost <= b.g_cost);
        assert!(b.g_cost <= 5, "only g_cost 1 and 5 should remain");
    }

    #[test]
    fn high_water_tracks_max_size() {
        let mut frontier = BestFirstFrontier::new();
        frontier.push(make_node(0, 1, 0));
        frontier.push(make_node(1, 2, 0));
        frontier.push(make_node(2, 3, 0));
        assert_eq!(frontier.high_water(), 3);

        let _ = frontier.pop();
        assert_eq!(
            frontier.high_water(),
            3,
            "high water should not decrease on pop"
        );
    }

    #[test]
    fn dead_end_tracking() {
        let mut frontier = BestFirstFrontier::new();
        assert!(!frontier.is_dead_end("abc123"));
        frontier.mark_dead_end("abc123");
        assert!(frontier.is_dead_end("abc123"));
    }
}
