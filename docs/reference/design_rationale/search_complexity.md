---
authority: reference
status: advisory
---

# Search Complexity Reference

**Advisory -- not normative.** This document describes algorithmic complexity
analysis for Sterling's best-first search implementation. Do not cite as
canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for
capability status.

## Variables

| Symbol | Meaning | Bounded by |
|--------|---------|------------|
| **E** | Total expansions (loop iterations) | `max_expansions` (default 1000) |
| **c** | Candidates per node | `max_candidates_per_node` (default 1000) |
| **F** | Frontier size | `max_frontier_size` (default 10,000) |
| **V** | Unique visited states | Practical: E x b |
| **T** | Scorer table entries | Input-dependent |
| **s** | State fingerprint size | Constant (identity bytes, 1 plane) |
| **b** | Average branching factor | World-dependent |
| **d** | Search depth | `max_depth` (default 100) |
| **N** | Total nodes created | E x c worst case |

## Overall Complexity

```
Time:  O(E x c x (log c + log V + log F + s))
Space: O(V + F + E x c)
```

The algorithm is bounded best-first search (A* with h=0). With default
budget caps: roughly O(2x10^7) time, O(10^6) space.

## Data Structures

| Component | Structure | Rationale |
|-----------|-----------|-----------|
| Frontier | `BinaryHeap<Reverse<FrontierKey>>` | Min-heap by (f_cost, depth, creation_order) |
| Visited set | `BTreeSet<String>` | Deterministic iteration order |
| Dead ends | `BTreeSet<String>` | Deterministic iteration order |
| Scorer table | `BTreeMap<String, i64>` | Deterministic serialization |
| Expansion log | `Vec<ExpandEventV1>` | Append-only normative record |
| Node list | `Vec<SearchNodeV1>` | Append-only |

All ordered structures (BTreeSet, BTreeMap, sorted Vec) are deliberate: they
guarantee deterministic replay at the cost of O(log n) vs O(1) amortized for
hash-based alternatives.

## Per-Operation Costs

| Operation | Complexity | Source |
|-----------|-----------|--------|
| Frontier push | O(log F) | `search/src/frontier.rs` |
| Frontier pop | O(log F) | `search/src/frontier.rs` |
| Frontier prune | O(F log F) | `search/src/frontier.rs` |
| Visited lookup | O(log V) | `search/src/frontier.rs` |
| Visited insert | O(log V) | `search/src/frontier.rs` |
| Candidate enumerate | O(c) | World-provided via SearchWorldV1 |
| Candidate sort (determinism) | O(c log c) | `search/src/search.rs` |
| Candidate sort (score order) | O(c log c) | `search/src/search.rs` |
| Score (Uniform) | O(c) | `search/src/scorer.rs` |
| Score (Table) | O(c log T) | `search/src/scorer.rs` |
| Fingerprint hash | O(s) | SHA-256 on identity_bytes() |
| Goal check | O(1) | World-provided via SearchWorldV1 |
| Expansion record | O(1) amortized | Vec append |

## Per-Expansion Breakdown

Each main loop iteration (pop one node, expand, push children):

```
  O(log F)                            pop from frontier
+ O(c)                                enumerate candidates
+ O(c log c)                          sort by canonical_hash (determinism)
+ O(c log T) or O(c)                  score (table vs uniform)
+ O(c log c)                          sort by (-bonus, canonical_hash)
+ c x O(s + log V + log F)            per-candidate: hash + dedup + push
----------------------------------------------
= O(c x (log c + log V + log F + s))  per expansion
```

## Budget Controls

Policy fields in SearchPolicyV1 bound the search to finite time and space:

| Policy field | Default | Effect |
|-------------|---------|--------|
| `max_expansions` | 1000 | Hard cap on loop iterations |
| `max_frontier_size` | 10,000 | Triggers prune when exceeded |
| `max_depth` | 100 | Candidates beyond depth are skipped |
| `max_candidates_per_node` | 1000 | Truncates enumeration |

Without budgets, best-first search on a graph with branching factor b
explores O(b^d) nodes to reach depth d -- exponential. The budget caps
convert this to a linear scan bounded by E x c.

## Deduplication

- **Key:** SHA-256 fingerprint of identity_bytes() (identity plane only)
- **Policy:** DedupKeyV1::IdentityOnly (default)
- **Semantics:** First-seen-wins. Once a state fingerprint enters the
  visited set, all subsequent arrivals at the same state are suppressed.
- **Cost:** O(s) to hash + O(log V) to check/insert

## Comparison to Standard Algorithms

| Algorithm | Time | Space | Difference |
|-----------|------|-------|------------|
| BFS | O(V + E_graph) | O(V) | Sterling is cost-ordered, not level-ordered |
| Dijkstra | O((V + E_graph) log V) | O(V) | Sterling uses budget caps instead of relaxation |
| A* (good heuristic) | O(b^d) | O(b^d) | Sterling uses h=0 (no heuristic) |
| IDA* | O(b^d) | O(d) | Sterling uses explicit frontier, not iterative deepening |

Sterling's search is closest to bounded uniform-cost search (A* with h=0 and
budget caps). The budget caps make worst-case complexity linear in E x c
rather than exponential in d.

## Known Inefficiencies

### ~~Node summary construction: O(N x E)~~ (resolved)

Node summaries are now built via `expansion_index` HashMap lookup, reducing
this from O(N x E) to O(N). See `search/src/search.rs`.

### Double candidate sort

Candidates are sorted twice per expansion: once by canonical_hash for
determinism (pre-score), once by (-bonus, canonical_hash) for score order
(post-score). Both are O(c log c). The first sort is required by the
determinism contract; the second is required by the advisory scoring
contract. Combined cost is still O(c log c).

### BTreeSet vs HashSet for visited set

BTreeSet gives O(log V) lookups where HashSet would give O(1) amortized.
BTreeSet is chosen for deterministic iteration order, required for
reproducible graph construction. The log factor is bounded by
log(max_expansions x branching), which is small in practice.
