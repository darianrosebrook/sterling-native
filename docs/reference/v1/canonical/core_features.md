> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Sterling Core Features

**Author**: @darianrosebrook
**Created**: November 27, 2025
**Status**: Canonical Reference

---

## Overview

This document provides detailed explanations of Sterling's core architectural features. These features work together to create a reasoning system where:

1. **Meaning lives in structure** (IR + KG), not transformer weights
2. **Memory is compression-gated** (decay only after summarization)
3. **Learning is edge-relative** (path algebra, not node scores)
4. **Reasoning is auditable** (ProofTraces, not black-box CoT)

---

## 1. Path Algebra

Path algebra is Sterling's mechanism for learning which reasoning paths are productive. Unlike node-level importance scores, path algebra operates on **edges** and tracks **trajectory-level statistics**.

### 1.1 Edge State

Every edge in the knowledge graph carries a state vector:

```python
@dataclass
class EdgeState:
    w_base: float = 1.0      # Structural importance (role-aware floor)
    w_usage: float = 0.0     # How often chosen (0 to 1, EMA)
    w_recency: float = 0.0   # Recency boost (0 to 1, EMA)
    w_novelty: float = 1.0   # Underexplored = high (1/sqrt(1+visits))

    visit_count: int = 0     # Times this edge was traversed
    last_used_step: int = 0  # Step counter when last used

    role: EdgeRole           # STRUCTURAL, EPISODIC, SCRATCH, SUBSUMED
    is_subsumed: bool        # True if rolled into a summary node
```

### 1.2 Update Rules

**When an edge is taken** (src -> dst):

```
w_usage    := (1 - alpha_usage) * w_usage + alpha_usage * 1.0
w_recency  := (1 - alpha_recency) * w_recency + alpha_recency * 1.0
visit_count := visit_count + 1
w_novelty  := 1.0 / sqrt(1 + visit_count)
```

**When an edge is available but NOT taken** (src -> other):

```
w_usage   := w_usage * (1 - branch_decay)
w_recency := w_recency * (1 - branch_decay)
```

Where `branch_decay` depends on node roles:
- `branch_decay_global = 0.01` for GLOBAL_ANCHOR nodes
- `branch_decay_summary = 0.05` for EPISODIC_SUMMARY nodes
- `branch_decay_scratch = 0.15` for SCRATCH nodes

### 1.3 Selection Score

When choosing which edge to traverse, the selection score is:

```
Score(edge) = beta_base * w_base
            + beta_usage * log(1 + w_usage)
            + beta_recency * w_recency
            + beta_novelty * w_novelty
```

The logarithm on `w_usage` creates sublinear reinforcement, preventing "highway" edges from completely dominating.

### 1.4 Dead-End Penalty

When a path terminates at a node with no outgoing edges (dead end), the edge that led there receives additional penalty:

```python
def penalize_dead_end_edge(src, dst):
    penalty = config.dead_end_penalty  # Default: 0.3
    state.w_usage *= (1 - penalty)
    state.w_recency *= (1 - penalty)
    state.w_base *= (1 - penalty * 0.5)  # Base also reduced
```

This teaches the system to avoid paths that lead nowhere.

### 1.5 Key Insight

Path algebra implements **eligibility traces** from reinforcement learning at the graph level. It answers: "Given that we're at node A, which edges have historically led to good outcomes?" This is fundamentally different from asking "How important is node B?" (node-level scoring).

---

## 2. Decay System

Sterling's decay system is **compression-gated**: nodes and edges only become candidates for decay after their content has been summarized into a higher-level representation.

### 2.1 Three-Tier Memory Model

```
Tier 0: GLOBAL_ANCHOR (No Decay)
        - Operators (S/M/P/K/C)
        - PN schemas and templates
        - Grammar universals
        - Domain ontologies

Tier 1: EPISODIC_SUMMARY (Slow Decay)
        - Episode summaries
        - Decision points
        - Conceptual milestones

Tier 2: SCRATCH (Aggressive Decay)
        - Working hypotheses
        - Intermediate states
        - Micro-nodes
```

### 2.2 Decay Rates

| Node Role | Branch Decay | Long-Horizon Decay | Base Floor |
|-----------|--------------|-------------------|------------|
| GLOBAL_ANCHOR | 0.01 | 0.001 | 0.5 |
| EPISODIC_SUMMARY | 0.05 | 0.01 | 0.5 |
| SCRATCH | 0.15 | 0.1 | 0.1 |

### 2.3 Compression-Gated Decay

The critical constraint: **decay is blocked until compression**.

The implementation is `run_compression_gated_decay()` in `kg/path_algebra.py`, which checks `src_state.summary_of` to gate decay — a node's edges only become decay-eligible after the node's content has been rolled into a summary.

> **Note**: There is no standalone `should_decay()` predicate function. The gating logic is integrated into `run_compression_gated_decay()`.

This ensures we never "forget before understanding" — the semantic content of a path segment is preserved in a summary node before the micro-nodes decay.

### 2.4 Subsumed Edge Decay

Once a path segment is rolled into a summary node, the original edges are marked as `is_subsumed = True`. These edges decay faster:

```
subsumed_decay = base_decay * subsumed_decay_multiplier  # Default: 2.0x
```

This gradually removes the detailed micro-structure while preserving the compressed representation.

### 2.5 Key Insight

Traditional memory systems treat decay as "forgetting" - a loss of information. Sterling treats decay as **sculpting** - preserving the main chain while pruning branches. The summary nodes ensure nothing semantically important is lost; only the redundant micro-structure fades.

---

## 3. Episodic Summaries

Episodic summaries are **first-class KG objects**, not text blobs in a database. They participate in reasoning, carry latent vectors, and connect to both the micro-nodes they compress and the broader KG structure.

> **Implementation note**: There is no dedicated `SummaryNode` class. Summary nodes are represented as regular `KGNode` objects with a `NodeState` (in `kg/path_algebra.py`) that carries `role`, `level`, `summary_of`, and `latent` fields. The pseudocode below illustrates the design intent.

### 3.1 Summary Node Structure (Conceptual)

Summary nodes are `KGNode` instances with `NodeState` carrying:

```python
# Actual representation in kg/path_algebra.py (NodeState dataclass)
@dataclass
class NodeState:
    role: NodeRole = SCRATCH
    level: int = 0
    summary_of: Set[str] = field(default_factory=set)
    latent: Optional[List[float]] = None
    # ... other fields
```

### 3.2 Summary Creation

When a path segment meets the threshold for summarization (see `create_summary_node()` in `kg/path_algebra.py`):

1. A new KG node is created with `NodeState(role=EPISODIC_SUMMARY, summary_of=set(segment))`
2. Routing edges of type `SUMMARIZES` connect the segment endpoints through the summary
3. Back-link edges of type `CONTAINS` connect the summary to segment members
4. **Edges** within the segment are marked as subsumed (via `_mark_segment_subsumed()`), not interior nodes
5. Optionally, a latent vector is computed via a provided `latent_fn` callable

### 3.3 Latent Attachment

Summary nodes can carry compressed latent vectors. The `compress_episode_with_latent()` function in `kg/path_algebra.py` accepts a `latent_fn` callable and stores the result in `NodeState.latent`.

> **Note**: There is no `unified_encoder.encode_sequence()` API. Latent computation is injected via callable, keeping the path algebra module encoder-agnostic.

This latent enables:
- Fast similarity search over episodes
- Value estimation for planning
- Cross-episode pattern matching

### 3.4 Role in SWM Seeding

Summary nodes are preferred seeds for Semantic Working Memory. The design intent is:

1. Always include relevant global anchors
2. Include recent episode summaries
3. Include semantically similar summaries (via latent similarity)

> **Implementation status**: SWM seed selection is handled by `SWMSelector` in `core/reasoning/loop/components.py`, which is currently a stub/passthrough implementation with a `max_nodes` limit. The full activation-spreading algorithm described below (Section 4.2) is planned but not yet implemented.

### 3.5 Key Insight

By treating summaries as KG nodes with latent vectors, Sterling creates a **two-level semantic hierarchy**:
1. Detailed micro-paths (subject to decay)
2. Compressed episode summaries (persistent, searchable)

This enables reasoning over "what happened" without re-ingesting the full history.

---

## 4. Semantic Working Memory (SWM)

SWM is the bounded active subgraph used for reasoning on a specific task. It's not the full KG, but a focused slice selected based on salience and relevance.

> **Implementation status**: The `SemanticWorkingMemory` class described here is a design target. The current implementation uses `SWMPolicy` in `kg/swm_policy.py` (with `max_nodes`, `max_depth`, `max_edges`, and bias fields) and `SWMSelector` in `core/reasoning/loop/components.py` (currently a stub passthrough). Salience computation exists as `compute_node_importance_from_edges()` and `get_node_salience_for_swm()` in `kg/path_algebra.py` with different signatures and formulas than shown below.

### 4.1 SWM Structure (Design Target)

The current configuration is `SWMPolicy` in `kg/swm_policy.py`:

```python
@dataclass
class SWMPolicy:
    max_nodes: int = 50
    max_depth: int = 5
    max_edges: int = 200
    # ... bias fields
```

### 4.2 Selection Algorithm (Design Target)

The intended algorithm is activation spreading from seed nodes. The current `SWMSelector.select_nodes()` is a passthrough with `max_nodes` limiting. The full priority-queue algorithm below is planned:

1. Start from seed nodes with salience 1.0
2. Spread activation along edges, decaying by edge salience
3. Stop when `max_nodes` budget is reached or salience drops below threshold
4. Only nodes above `min_node_salience` are admitted

### 4.3 Salience Computation

Node importance is computed in `kg/path_algebra.py` via `compute_node_importance_from_edges()` and `get_node_salience_for_swm()`. Edge salience uses the same selection score formula as path algebra (Section 1.3).

### 4.4 SWM vs Full KG

| Aspect | Full KG | SWM |
|--------|---------|-----|
| Size | 500-10,000 nodes | 20-50 nodes |
| Access | All nodes available | Only salient nodes |
| Update | Persistent | Per-task, rebuilt |
| Purpose | Long-term memory | Working memory |

### 4.5 Key Insight

SWM implements **attention at the graph level**. Instead of attending over tokens (transformer attention), Sterling attends over semantic structures (nodes and edges). This enables bounded reasoning without re-ingesting the full context.

---

## 5. Value Function

Sterling's value function is a hybrid of symbolic constraints and learned heuristics. See [Value Function Components v1](value_function_components_v1.md) for the authoritative specification.

> **Implementation note**: The conceptual formula below describes the design intent. The actual implementation uses `HybridValueFunction` (in `core/value/hybrid.py`) which composes multiple named component heads (structural, memory, task, latent, dialogue, MDL, pragmatics) with configurable weights. See the canonical value function spec for full details.

### 5.1 Value Function Components (Conceptual)

The hybrid value function combines:

- **Structural head** (`core/value/structural.py`): Feature-based scoring using degree, depth, progress, goal distance
- **Memory head** (`core/value/memory.py`): SWM-based novelty, recency, and visit penalty signals
- **Task head** (`core/value/task_heads.py`): Task-specific reward shaping
- **Latent head** (`core/value/latent/`, optional): Learned embedding similarity
- **Dialogue head** (`core/value/dialogue_scorer.py`, optional): Dialogue-aware scoring
- **MDL head** (`core/value/mdl.py`, optional): Parsimony penalty
- **Pragmatics head** (`core/value/pragmatics_prior.py`, optional): Gricean maxim scoring

All component `evaluate()` methods return values in [0, 1]. Weights are normalized at evaluation time.

### 5.2 Value Head Architecture

The neural value head in `models/light/value_head.py`:

```python
class ValueHead(nn.Module):
    def __init__(self, latent_dim: int = 128):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(latent_dim, 64),
            nn.GELU(),
            nn.Dropout(p=0.1),
            nn.Linear(64, 1),
            nn.Sigmoid(),
        )
```

Note: `core/reasoning/loop/components.py` also defines a `ValueHead` class that is a higher-level interface wrapping scoring logic, not the neural network itself.

### 5.3 Training Signal

The value head is trained on successful and failed reasoning traces using discounted rewards. The training pipeline uses `ValueTargetContract` and `ValueTarget` from `core/value/target_contract.py`.

### 5.4 Key Insight

The value function bridges symbolic and neural reasoning:
- Symbolic component ensures hard constraints are never violated
- Neural component learns soft heuristics from experience
- Together, they guide search toward promising states without hallucinating

---

## 6. Integration: How Features Work Together

### 6.1 Reasoning Loop

```
1. Receive query -> Parse to IR
2. Select SWM seeds (global anchors + relevant summaries)
3. Build SWM by activation spreading
4. Search within SWM using value-guided A*
   - Path algebra scores guide edge selection
   - Value function estimates state quality
5. On success:
   - Reinforce taken edges
   - Consider creating summary node
6. On failure:
   - Penalize dead-end edges
   - Expand SWM or report failure
7. Return result with ProofTrace
```

### 6.2 Learning Loop

```
1. Process multiple reasoning episodes
2. Edge weights converge:
   - High-value paths get reinforced
   - Dead ends get penalized
   - Unused branches decay
3. Summary nodes accumulate:
   - Frequent patterns get compressed
   - Micro-nodes become decay-eligible
4. Value head improves:
   - Better state estimates
   - Faster search convergence
```

### 6.3 Memory Management

```
1. Short-term: SWM (task-specific, rebuilt each query)
2. Medium-term: Path algebra weights (persist across queries)
3. Long-term: Summary nodes + global anchors (permanent)
4. Decay: Micro-nodes after summarization (gradual removal)
```

---

## 7. Configuration Reference

### 7.1 Path Algebra Config

```python
@dataclass
class PathAlgebraConfig:
    # Reinforcement
    alpha_usage: float = 0.1      # EMA factor for usage
    alpha_recency: float = 0.2    # EMA factor for recency

    # Decay by role
    branch_decay_global: float = 0.01
    branch_decay_summary: float = 0.05
    branch_decay_scratch: float = 0.15

    # Long-horizon decay
    eps_very_small: float = 0.001  # Global anchor
    eps_small: float = 0.01        # Summary
    eps_large: float = 0.1         # Scratch

    # Floors
    base_floor_high: float = 0.5   # Global/Summary
    base_floor_low: float = 0.1    # Scratch

    # Selection weights
    beta_base: float = 1.0
    beta_usage: float = 0.5
    beta_recency: float = 0.3
    beta_novelty: float = 0.2

    # Exploration
    epsilon_explore: float = 0.1

    # Summary
    min_segment_length: int = 3
    promotion_usage_threshold: int = 10

    # Dead end
    dead_end_penalty: float = 0.3
```

### 7.2 SWM Config

The actual SWM configuration is `SWMPolicy` in `kg/swm_policy.py`:

```python
@dataclass
class SWMPolicy:
    max_nodes: int = 50
    max_depth: int = 5
    max_edges: int = 200
    # ... bias fields for node/edge selection
```

---

## 8. Related Documents

- `docs/DECAY_REDESIGN.md` - Detailed decay system design
- `docs/GLOSSARY.md` - Canonical terminology
- `../../theory/reasoning_framework.md` - Theoretical foundations
- `docs/internal/not-another-graph-rag.md` - Differentiation from standard approaches
- `kg/path_algebra.py` - Implementation

---

**Version**: 1.1
**Last Updated**: February 17, 2026

### Changelog

#### v1.1 (2026-02-17)
- §2.3: Corrected compression-gated decay — `should_decay()` does not exist; implemented as `run_compression_gated_decay()` in `kg/path_algebra.py`
- §3.1: Corrected SummaryNode — no dedicated class; uses `KGNode` + `NodeState` in `kg/path_algebra.py`
- §3.2: Corrected edge types (`SUMMARIZES`/`CONTAINS`, not `SUMMARIZED_BY`/`SUMMARIZES_TO`); marks edges not nodes as subsumed
- §3.3: Corrected latent API — no `unified_encoder.encode_sequence()`; uses injectable `latent_fn` callable
- §3.4: Added implementation status note — `SWMSelector` is a stub passthrough
- §4: Added implementation status — `SemanticWorkingMemory` class is design target; current impl uses `SWMPolicy` + `SWMSelector`
- §5: Corrected ValueHead architecture (128-dim, 2 layers, GELU, dropout, sigmoid); linked to canonical value function spec
- §7.2: Corrected SWM config to reference actual `SWMPolicy` class
