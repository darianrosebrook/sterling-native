# Search Complexity Reference (v1)

Algorithmic complexity analysis for the v1 best-first search implementation
(SC-001). All references are to `sterling-native` source files.

## Variables

| Symbol | Meaning | Bounded by |
|--------|---------|------------|
| **E** | Total expansions (loop iterations) | `max_expansions` (default 1000) |
| **c** | Candidates per node | `max_candidates_per_node` (default 1000) |
| **F** | Frontier size | `max_frontier_size` (default 10,000) |
| **V** | Unique visited states | Practical: E × b |
| **T** | Scorer table entries | Input-dependent |
| **s** | State fingerprint size | Constant (identity bytes, 2 planes) |
| **b** | Average branching factor | World-dependent |
| **d** | Search depth | `max_depth` (default 100) |
| **N** | Total nodes created | E × c worst case |

## Overall Complexity

```
Time:  O(E × c × (log c + log V + log F + s))
Space: O(V + F + E×c)
```

The algorithm is bounded best-first search (A\* with h=0). With default
budget caps: roughly O(2×10^7) time, O(10^6) space.

## Data Structures

| Component | Structure | Rationale |
|-----------|-----------|-----------|
| Frontier | `BinaryHeap<Reverse<FrontierKey>>` | Min-heap by (f\_cost, depth, creation\_order) |
| Visited set | `BTreeSet<String>` | Deterministic iteration order |
| Dead ends | `BTreeSet<String>` | Deterministic iteration order |
| Scorer table | `BTreeMap<String, i64>` | Deterministic serialization |
| Expansion log | `Vec<ExpandEventV1>` | Append-only normative record |
| Node list | `Vec<SearchNodeV1>` | Append-only |

All ordered structures (`BTreeSet`, `BTreeMap`, sorted `Vec`) are
deliberate: they guarantee deterministic replay at the cost of O(log n) vs
O(1) amortized for hash-based alternatives.

## Per-Operation Costs

| Operation | Complexity | Source |
|-----------|-----------|--------|
| Frontier push | O(log F) | `search/src/frontier.rs` `push()` |
| Frontier pop | O(log F) | `search/src/frontier.rs` `pop()` |
| Frontier prune | O(F log F) | `search/src/frontier.rs` `prune_to()` |
| Visited lookup | O(log V) | `search/src/frontier.rs` `is_visited()` |
| Visited insert | O(log V) | `search/src/frontier.rs` `push()` |
| Candidate enumerate | O(c) | World-provided via `SearchWorldV1` |
| Candidate sort (determinism) | O(c log c) | `search/src/search.rs` pre-score sort |
| Candidate sort (score order) | O(c log c) | `search/src/search.rs` post-score sort |
| Score (Uniform) | O(c) | `search/src/scorer.rs` `UniformScorer` |
| Score (Table) | O(c log T) | `search/src/scorer.rs` `TableScorer` |
| Fingerprint hash | O(s) | SHA-256 on `identity_bytes()` |
| Goal check | O(1) | World-provided via `SearchWorldV1` |
| Expansion record | O(1) amortized | Vec append |

## Per-Expansion Breakdown

Each main loop iteration (pop one node, expand, push children):

```
  O(log F)                            pop from frontier
+ O(c)                                enumerate candidates
+ O(c log c)                          sort by canonical_hash (determinism)
+ O(c log T) or O(c)                  score (table vs uniform)
+ O(c log c)                          sort by (-bonus, canonical_hash)
+ c × O(s + log V + log F)            per-candidate: hash + dedup + push
─────────────────────────────────────
= O(c × (log c + log V + log F + s))  per expansion
```

## Search Loop Structure

```
initialize root node
push root to frontier
while frontier not empty AND budget not exceeded:
    pop best node                          O(log F)
    enumerate candidates from world        O(c)
    sort candidates by canonical_hash      O(c log c)
    cap to max_candidates_per_node         O(1)
    score candidates                       O(c) or O(c log T)
    sort by (-bonus, canonical_hash)       O(c log c)
    for each candidate:
        apply operator                     O(s)
        compute fingerprint (SHA-256)      O(s)
        check visited set                  O(log V)
        if duplicate: skip
        check depth limit                  O(1)
        check goal                         O(1)
        push to frontier                   O(log F)
    record expansion event                 O(c)
build node summaries                       O(N × E) [see known issues]
build graph                                O(E × c)
```

## Space Breakdown

| Allocation | Size | Lifetime |
|------------|------|----------|
| Frontier heap | O(F) | Pruned to `max_frontier_size` |
| Visited set | O(V) | Grows monotonically (no removal in M1) |
| Dead ends set | O(D) where D ≤ V | Grows monotonically |
| All nodes | O(N) | Retained for graph construction |
| Expansion events | O(E × c) | Candidate records per expansion |

Total: **O(V + F + E×c)**. With default budgets and typical branching,
V dominates.

## Budget Controls

Policy fields in `SearchPolicyV1` (`search/src/policy.rs`) bound the
search to finite time and space:

| Policy field | Default | Effect |
|-------------|---------|--------|
| `max_expansions` | 1000 | Hard cap on loop iterations |
| `max_frontier_size` | 10,000 | Triggers `prune_to()` when exceeded |
| `max_depth` | 100 | Candidates beyond depth are skipped |
| `max_candidates_per_node` | 1000 | Truncates enumeration |

Without budgets, best-first search on a graph with branching factor b
explores O(b^d) nodes to reach depth d — exponential. The budget caps
convert this to a linear scan bounded by E × c.

## Deduplication

- **Key**: SHA-256 fingerprint of `identity_bytes()` (identity + status planes)
- **Policy**: `DedupKeyV1::IdentityOnly` (M1 default)
- **Semantics**: First-seen-wins. Once a state fingerprint enters the
  visited set, all subsequent arrivals at the same state are suppressed.
- **Cost**: O(s) to hash + O(log V) to check/insert

## Comparison to Standard Algorithms

| Algorithm | Time | Space | v1 difference |
|-----------|------|-------|---------------|
| BFS | O(V + E\_graph) | O(V) | v1 is cost-ordered, not level-ordered |
| Dijkstra | O((V + E\_graph) log V) | O(V) | v1 uses budget caps instead of relaxation |
| A\* (good heuristic) | O(b^d) | O(b^d) | v1 uses h=0 (no heuristic) |
| IDA\* | O(b^d) | O(d) | v1 uses explicit frontier, not iterative deepening |

v1 is closest to **bounded uniform-cost search** (A\* with h=0 and budget
caps). The budget caps make worst-case complexity linear in E×c rather
than exponential in d.

## Known Inefficiencies

### Node summary construction: O(N×E)

`search/src/search.rs` builds `node_summaries` after search completes by
scanning all expansions per node via linear search:

```rust
let expansion_order = expansions
    .iter()
    .find(|e| e.node_id == n.node_id)
    .map(|e| e.expansion_order);
```

This is O(N×E). A `HashMap<u64, usize>` built during expansion recording
would reduce it to O(N). Impact is post-search only, not in the hot loop.

### Double candidate sort

Candidates are sorted twice per expansion: once by `canonical_hash` for
determinism (pre-score), once by `(-bonus, canonical_hash)` for score
order (post-score). Both are O(c log c). The first sort is required by the
determinism contract; the second is required by the advisory scoring
contract. Combined cost is 2×O(c log c), which is still O(c log c).

### BTreeSet vs HashSet for visited set

`BTreeSet<String>` gives O(log V) lookups where `HashSet` would give O(1)
amortized. The BTreeSet is chosen for deterministic iteration order, which
is required for reproducible graph construction. The log factor is bounded
by log(max\_expansions × branching), which is small in practice.
