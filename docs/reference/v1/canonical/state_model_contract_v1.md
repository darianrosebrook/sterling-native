> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**
>
> **Superseded in v2**: `docs/canonical/bytestate_compilation_boundary.md`, `kernel/src/carrier/` (compile, bytestate, registry), `search/src/` (search, node, frontier, graph).
> v2 evidence: SPINE-001 M1, SC-001 M1; lock tests `s1_m1_golden_fixtures.rs`, `sc1_search_determinism.rs`.

# State Model and Search Contract

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

**Version**: 1.2
**Date**: 2026-02-19
**Author**: @darianrosebrook
**Status**: Implemented

---

## 1. Thesis

Sterling's reasoning engine operates on a layered semantic state model, searched via configurable graph expansion. The state model defines what Sterling reasons about (utterances, world knowledge, annotations). The search infrastructure defines how Sterling explores the space of possible analyses. The value function scores candidate states to guide search toward goals.

This document specifies all three layers as a single contract: state representation, graph topology, and evaluation protocol.

### 1.1 Foundational Types

Source: `core/state_model.py:58-134`

**Invariant enforcement**:

- `STRICT_INVARIANTS` (module-level bool, default `False`): Controls whether critical invariant violations raise exceptions or log warnings.
- `set_strict_invariants(strict: bool)` — Enable/disable strict mode.
- `get_strict_invariants() -> bool` — Query current mode.
- `InvariantViolation(Exception)` — Raised on I1/I2/I4 violations in strict mode.

**Enums**:

| Enum | Source | Values | Used by |
|------|--------|--------|---------|
| `Modality` | `str, Enum` at line 109 | ASSERTED, ASSUMED, DESIRED, FEARED, HYPOTHETICAL | Proposition modality |
| `WorldID` | `str, Enum` at line 119 | ACTUAL, HYPOTHETICAL, COUNTERFACTUAL | World context |
| `UtteranceStage` | `IntEnum` at line 127 | RAW, PARSED, SEMANTIC, GROUNDED, CONTEXTUAL, COMPRESSED | Lifecycle gating |

**Re-exports**:

`OperatorApplication` is re-exported from `core.contracts.operator_application` at line 1372 for backward compatibility. `StateNode.operator_application` uses this type.

---

## 2. UtteranceState: The Primary Object of Analysis

Source: `core/state_model.py:670-934`

An UtteranceState is a single utterance with layered linguistic annotations. It progresses through lifecycle stages as analysis deepens.

### 2.1 Fields

```python
@dataclass
class UtteranceState:
    utterance_id: str                          # Unique identifier
    surface_text: str                          # Raw text
    syntax: Optional[SyntaxLayer] = None       # Layer 1: structural
    semantics: Optional[SemanticIR] = None     # Layer 2: meaning
    semiotics: Optional[SemioticMappings] = None  # Layer 3: grounding
    pragmatics: Optional[PragmaticContext] = None  # Layer 4: context
    _latent: Optional[Any] = None              # Layer 5: compression (read-only)
    _latent_frozen: bool = False               # Immutability flag
    text_ir: Optional[Any] = None              # Full TextIR object (v0)
    score: float = 0.0                         # Search metadata
    visit_count: int = 0                       # Search metadata
    age: int = 0                               # Search metadata
    novelty_flag: bool = False                 # Search metadata
```

### 2.2 Lifecycle Stages

Source: `core/state_model.py:117-130`

```python
class UtteranceStage(Enum):
    RAW = 0           # surface_text only
    PARSED = 1        # + syntax
    SEMANTIC = 2      # + semantics
    GROUNDED = 3      # + semiotics
    CONTEXTUAL = 4    # + pragmatics
    COMPRESSED = 5    # + latent
```

Each stage adds a layer. Stages are monotonically increasing — an utterance at stage SEMANTIC has syntax and semantics populated.

### 2.3 Layer Definitions

**SyntaxLayer** (Lines 138-206):

| Field | Type | Description |
|-------|------|-------------|
| `tokens` | `List[str]` | Tokenized surface |
| `pos_tags` | `List[str]` | UD UPOS preferred |
| `dependency_heads` | `List[int]` | Head indices |
| `dependency_labels` | `List[str]` | Dependency relations |
| `constituent_tree` | `Optional[str]` | S-expression (optional) |
| `morphological_features` | `Optional[Dict[str, Dict[str, str]]]` | Per-token morphology |

**SemanticIR** (Lines 208-394):

| Field | Type | Description |
|-------|------|-------------|
| `events` | `List[Dict[str, Any]]` | Event nodes |
| `entities` | `List[Dict[str, Any]]` | Entity nodes |
| `attributes` | `List[Dict[str, Any]]` | Attribute nodes |
| `propositions` | `List[Dict[str, Any]]` | Proposition nodes |
| `roles` | `List[Dict[str, Any]]` | Role edges |
| `constraints` | `List[Dict[str, Any]]` | Constraint edges |
| `logical_relations` | `List[Dict[str, Any]]` | Logical relation edges |
| `pn_type` | `Optional[str]` | PN classification |
| `polarity` | `str` | `"positive"` or `"negative"` |

**SemioticMappings** (Lines 396-442):

| Field | Type | Description |
|-------|------|-------------|
| `mappings` | `Dict[str, str]` | token_span -> sense_id |
| `sense_metadata` | `Dict[str, Dict[str, Any]]` | Per-sense metadata |

**PragmaticContext** (Lines 444-663):

| Field | Type | Description |
|-------|------|-------------|
| `discourse_relations` | `List[Dict[str, Any]]` | Discourse structure |
| `speaker_id` | `Optional[str]` | Speaker identity |
| `speech_act` | `Optional[str]` | Speech act classification |
| `intent` | `Optional[Any]` | IntentAnnotation (I7 canonical location) |
| `secondary_intents` | `List[Any]` | Additional intents |
| `implicatures` | `List[str]` | Pragmatic inferences |
| `doc_kind` | `Optional[str]` | Document type |
| `role` | `Optional[str]` | Speaker role |
| `tags` | `List[str]` | Annotation tags |
| `decision_ref` | `Optional[str]` | Audit-plane reference |

### 2.4 Invariants

Checked by `UtteranceState.validate()` (lines 774-857):

1. **I1**: Syntax is the sole encoding of phrase structure. *(enforced — critical)*
2. **I2**: Semantics consists of typed graph structures (events, entities, roles, etc.). *(enforced — critical)*
3. **I3**: Every semantic predicate should be anchored to a sense ID. *(enforced — advisory, warning only)*
4. **I4**: Latent vectors are read-only after creation (`_latent_frozen`). *(enforced — critical)*

Additionally, `validate()` checks lifecycle ordering (semiotics requires semantics, pragmatics requires semantics) as critical violations.

**Documentation-only invariants** (not checked by `validate()`):

5. **I5**: If `text_ir` is set, syntax/semantics are derived from it (single source of truth). Enforced by convention and the `linguistic_ir` property.
6. **I7**: Intent canonical location is `pragmatics.intent`. Enforced by the `UtteranceState.intent` convenience property, not by validation logic.

### 2.5 Serialization

- `to_snapshot() -> Dict[str, Any]` (lines 869-891): Canonical serialization for persistence.
- `from_snapshot(data) -> UtteranceState` (lines 893-934): Reconstruction from snapshot.

---

## 3. WorldState: Activated Knowledge

Source: `core/state_model.py:942-1308`

WorldState represents the activated slice of knowledge available for reasoning about an utterance.

### 3.1 Fields

```python
@dataclass
class WorldState:
    world_id: str = "actual"
    activated_entities: Set[str] = set()
    active_frames: Set[str] = set()
    active_rules: Set[str] = set()
    assumptions: Dict[str, Any] = {}
    counterfactuals: Dict[str, Any] = {}
    discourse_state: Optional[Any] = None
    provenance_map: Dict[str, str] = {}
    kg_ref: Optional[KGRef] = None           # Canonical (Phase A+)
    kg_id: Optional[str] = None              # Legacy
    kg_handle: Optional[Any] = None          # Legacy, to be removed
    family_id: Optional[str] = None          # PN-specific
    family_constraints: List[str] = []
```

**`public_assumptions` property** (added in Canonical V2):

Returns `{k: v for k, v in self.assumptions.items() if not k.startswith("_")}`. Keys prefixed with `_` are execution-only service wiring (e.g., `_capability_registry`, `_domain_adapters`) that must never appear in serialized output, canonical dicts, or content hashes. Use `public_assumptions` instead of `assumptions` at all serialization boundaries.

### 3.2 KG Ownership Model

KGs are owned by the global KGRegistry, not by WorldState. WorldState stores only a lightweight `kg_ref` (content-addressed reference). Use `get_kg(strict=False)` (lines 1105-1141) to resolve the actual KG from the registry.

### 3.3 Copy Semantics

`__deepcopy__` (lines 1270-1307) implements O(semantic_delta) copying — only modified fields are deeply copied, unchanged fields share references.

**Service ref identity preservation** (added in Canonical V2):

An explicit allowlist controls which `_`-prefixed assumption keys are shared by reference (not cloned) during deepcopy:

```python
_DEEPCOPY_SHARE_BY_REFERENCE_KEYS: frozenset[str] = frozenset({
    "_capability_registry",
    "_domain_adapters",
})
```

Only stateless service facades may be listed here. Mutable scratch structures (e.g., `_pending_observations`) must NOT be allowlisted — they would cause cross-branch contamination if shared by reference. The allowlist is checked in `_copy_assumptions()` alongside the existing `PnLanguagePackRegistry` special case.

---

## 4. StateNode: Semantic State Bundle

Source: `core/state_model.py:1321-1619`

StateNode bundles an utterance state with world state and search metadata. It is the unit of reasoning — operators transform StateNodes into new StateNodes.

### 4.1 Fields

```python
@dataclass
class StateNode:
    node_id: str
    utterance_states: List[UtteranceState]
    world_state: WorldState
    parent_id: Optional[str] = None
    operator_from_parent: Optional[str] = None
    depth: int = 0
    cost: float = 0.0
    operator_application: Optional[OperatorApplication] = None
    visit_count: int = 0
    novelty_flag: bool = False
    _step_record: Optional[StepRecordWithOverlays] = None  # K6.1 proof evidence
    hypothesis_state: Optional[HypothesisState] = None     # Sprint 1
```

### 4.2 Key Properties

| Property | Line | Description |
|----------|------|-------------|
| `state_id` | 1370 | Alias for `node_id` |
| `primary_utterance` | 1393 | First utterance state |
| `latest_text_ir` | 1400 | TextIR from primary utterance |
| `primary_intent` | 1415 | Intent from pragmatics |
| `intent_sequence` | 1432 | List of intent IDs |
| `primary_intent_family` | 1456 | Intent family classification |

### 4.3 Serialization

- `to_canonical_dict()` (lines 1480-1532): Hash-critical fields only, for content addressing.
- `to_snapshot()` (lines 1534-1562): TD-11 handover format.
- `from_snapshot()` (lines 1564-1619): K4 replay reconstruction.

**Canonical V2** (introduced with `CANONICAL_VERSION = 2`):

`to_canonical_dict()` includes a `"canonical_version"` field and uses `WorldState.public_assumptions` (not raw `assumptions`) to exclude `_`-prefixed service wiring from the canonical representation. This prevents non-serializable objects (capability registries, domain adapters) from leaking into content hashes.

`to_snapshot()` similarly iterates `public_assumptions` and does not include `_`-prefixed keys in the snapshot output or in `nonserializable_assumptions_keys`.

**DET-1A enforcement**: Identity-path serialization (in `core/contracts/semantic_edits.py`) uses `json.dumps()` **without** `default=str`. Non-serializable values that reach a content hash or trace hash call will raise `TypeError` rather than being silently coerced. This is a structural guarantee — if `public_assumptions` filtering is ever bypassed, the hash will fail loudly rather than producing a misleading result.

**Invariant**: StateNodes are immutable during search. Operators create new StateNodes; they never modify existing ones.

---

## 5. StateGraph: Reasoning Episode Structure

Source: `core/reasoning/state_graph.py:709-1821`

StateGraph is the directed graph representing a complete reasoning episode. It contains SearchNodes (graph-level) connected by OperatorEdges.

### 5.1 SearchNode (Graph Level)

Source: `core/reasoning/state_graph.py:352-418`

**Naming note**: `state_graph.py` also exports a deprecated alias `StateNode = SearchNode` (line 2045) for backward compatibility. This alias will be removed once all consumers migrate to `SearchNode`. Do not confuse this with the semantic-level `StateNode` from `core/state_model.py`.

```python
@dataclass
class SearchNode:
    state_id: str                              # Opaque ("s0", "s1", ...)
    node_type: SearchNodeType = WORLD_STATE
    payload_kind: Optional[str] = None
    payload_ref: Optional[str] = None          # Reference to object store
    payload_hash: Optional[str] = None         # Semantic hash (DET-1)
    schema_version: str = "1.0"
    kg_node_id: Optional[str] = None           # KG node ID (for WORLD_STATE)
    parent_state_id: Optional[str] = None
    depth: int = 0
    score: float = 0.0
    is_goal: bool = False
    is_dead_end: bool = False
    semantic_state: Optional[SemanticStateNode] = None  # Legacy, to be moved to object store
    intent_distribution: Optional[Dict[str, float]] = None  # Intent-guided search
    predicted_intent: Optional[str] = None
    predicted_intent_family: Optional[str] = None
    metadata: Dict[str, Any] = {}
```

**SearchNodeType** (lines 299-311):

| Value | Description |
|-------|-------------|
| `WORLD_STATE` | Expandable reasoning state |
| `OBSERVATION` | Fact/delta from transition |
| `HYPOTHESIS` | Induced rule or program |
| `TEST_RESULT` | Verification outcome |
| `INVARIANCE_WITNESS` | Counterexample/evidence |
| `PREDICTION` | Forward-looking claim |

### 5.2 OperatorEdge

Source: `core/reasoning/state_graph.py:420-707`

```python
@dataclass(init=False)
class OperatorEdge:
    edge_id: str                               # Content-addressed
    src_state: str
    dst_state: str
    edge_kind: EdgeKind = EXPANDS_TO
    operator_category: Union[OperatorCategory, List[OperatorCategory]]
    operator_type: str
    navigation_type: EdgeType = WORLD_LOCAL
    kg_edge_id: Optional[str] = None           # KG edge ID (bridge navigation)
    score: float = 0.0
    source_world: Optional[str] = None         # Bridge: originating world
    target_world: Optional[str] = None         # Bridge: destination world
    landmark_id: Optional[str] = None          # Bridge: landmark used
    metadata: Dict[str, Any] = {}
    semantic_delta_ref: Optional[str] = None   # M3.1.6 derivation tracking
```

Note: `init=False` — OperatorEdge uses a custom `__init__` that handles backward compatibility (e.g., the deprecated `edge_type` parameter maps to `operator_type`).

**EdgeKind** (lines 314-336):

| Value | Description |
|-------|-------------|
| `EXPANDS_TO` | world_state -> world_state (search path) |
| `DERIVES_FROM` | product -> source (provenance) |
| `EVALUATES_ON` | test_result -> world_state |
| `WITNESS_FOR` | witness -> test_result |
| `REFINES` | hypothesis_v2 -> hypothesis_v1 |
| `PREDICTS` | hypothesis -> prediction |

**Edge ID generation**:
- V1 (`compute_edge_id`, lines 131-181): Based on state IDs.
- V2 (`compute_edge_id_v2`, lines 183-246): Governance-grade, requires `semantic_delta_ref`. Content-addressed: `sha256:{hash[:16]}` (dev) or `sha256:{hash[:24]}` (cert).

### 5.3 StateGraph Fields

```python
@dataclass
class StateGraph:
    root_id: str
    nodes: Dict[str, SearchNode] = {}
    edges: List[OperatorEdge] = []
    goal_nodes: List[str] = []
    dead_ends: List[str] = []
    _edges_from: Dict[str, List[OperatorEdge]] = {}  # Index
    _object_store: EpisodeObjectStore
    delta_enforcement_policy: Optional[DeltaEnforcementPolicy] = None
    governance_strict: bool = False
    governance_witnesses: List[Dict[str, Any]] = []
```

### 5.4 StateGraph Invariants

Enforced by validation methods (lines 1243-1558):

1. **SG-1**: Payloads stored by reference in object store (not inline).
2. **SG-2**: Only `WORLD_STATE` nodes may enter the frontier.
3. **SG-3**: Canonical path contains only `WORLD_STATE` nodes + `EXPANDS_TO` edges.
4. **SG-4/SG-4A**: Edge kinds match node type constraints (e.g., `DERIVES_FROM` must point from product to source).

**Certification validation** (`validate_for_certification`, lines 1564-1660) checks all invariants. Returns `(bool, List[Violation])`.

### 5.5 Serialization (DET-2)

- `to_proof_dict()` (lines 1673+): Hash-critical fields only (no scores, no volatile metadata).
- `to_debug_dict()`: All fields including volatile.
- `compute_proof_hash() -> str`: Stable content hash for replay verification.

---

## 6. GraphEmitter: StateGraph Construction Protocol

Source: `core/reasoning/graph_emitter.py`

StateGraph is not populated directly. All node and edge creation during a reasoning episode goes through a `GraphEmitter` — an event sink that prevents ad hoc conversion between search-level and graph-level nodes (ENG-3A).

### 6.1 GraphEmitter (Abstract Base)

Source: `core/reasoning/graph_emitter.py:23-114`

```python
class GraphEmitter(ABC):
    @abstractmethod
    def on_world_node_created(
        self, state: StateNode, runtime_node_id: str,
        content_hash: Optional[str] = None, parent_id=None, depth=0,
        score=0.0, is_goal=False, is_dead_end=False, metadata=None,
        state_fingerprint: Optional[str] = None,
    ) -> None: ...

    @abstractmethod
    def on_world_edge_expanded(
        self, parent_id: str, child_id: str, operator_id: str,
        operator_signature_hash: str, operator_category=None,
        score_delta=0.0, metadata=None,
    ) -> None: ...

    @abstractmethod
    def on_meta_payload(
        self, payload: Any, payload_kind: str, node_type: SearchNodeType,
        attach_to: str, edge_kind: EdgeKind, metadata=None,
    ) -> tuple[str, str]: ...
```

### 6.2 StateGraphEmitter

Source: `core/reasoning/graph_emitter.py:117-652`

`StateGraphEmitter(GraphEmitter)` is the concrete implementation that populates a `StateGraph` instance. It binds to a graph at construction and uses its `_object_store`.

**Event methods** (core protocol):

| Method | Creates | Edge Kind |
|--------|---------|-----------|
| `on_world_node_created()` | `SearchNode(WORLD_STATE)` | n/a (node only) |
| `on_world_edge_expanded()` | `OperatorEdge` | `EXPANDS_TO` |
| `on_meta_payload()` | `SearchNode(any meta type)` + `OperatorEdge` | Caller-specified |

**Convenience methods** (typed wrappers over `on_meta_payload`):

| Method | Node Type | Edge Kind | ID Format |
|--------|-----------|-----------|-----------|
| `emit_observation()` | `OBSERVATION` | `DERIVES_FROM` | Auto-generated |
| `emit_hypothesis()` | `HYPOTHESIS` | `DERIVES_FROM` | `hyp::{hypothesis_id}` (stable) |
| `emit_prediction()` | `PREDICTION` | `PREDICTS` | Auto-generated |
| `emit_test_result()` | `TEST_RESULT` | `EVALUATES_ON` | Auto-generated |
| `emit_invariance_witness()` | `INVARIANCE_WITNESS` | `WITNESS_FOR` | Auto-generated |
| `emit_hypothesis_refinement()` | `HYPOTHESIS` | `REFINES` + `DERIVES_FROM` | `hyp::{hypothesis_id}` |

**Design rule**: Search code never calls `StateGraph.add_node()` or `StateGraph.add_edge()` directly. All population goes through the emitter.

---

## 7. Search Infrastructure

Source: `core/reasoning/search.py`

### 7.1 SearchNode (Tree Level)

Source: `core/reasoning/search.py:270-463`

This is distinct from the graph-level SearchNode in state_graph.py. The tree-level SearchNode wraps a StateNode with search cost metadata.

```python
@dataclass
class SearchNode:
    state: StateNode                           # Immutable semantic state
    parent: Optional[SearchNode] = None
    operator_from_parent: Optional[str] = None
    g_cost: float = 0.0                        # Path cost from root
    h_cost: float = 0.0                        # Heuristic (LOWER = closer)
    value_score: float = 0.0                   # Value estimate (HIGHER = better)
    depth: int = 0
    creation_order: int = 0                    # For deterministic ordering
```

### 7.2 Scoring

**Sign convention**:
- `value_score`: HIGHER = BETTER
- `h_cost`: LOWER = BETTER (distance, subtracted)
- `g_cost`: LOWER = BETTER (cost, subtracted)
- Combined `score`: HIGHER = BETTER

**Score computation** (lines 388-411):
```
score = value_weight * value_score
      - g_cost
      - heuristic_weight * h_cost
      + novelty_score
      + operator_weight_bonus
      - invariant_penalty
```

**Priority key** (lines 412-430):
```python
def priority_key(self) -> Tuple[float, int, int, str]:
    return (-score, depth, creation_order, branch_id)
```

Min-heap ordering: lower `priority_key` = higher score. Ties broken by depth (shallower first), then creation order (earlier first), then branch_id (deterministic string comparison).

### 7.3 SearchConfig

Source: `core/reasoning/search.py:101-263`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_depth` | `int` | 50 | Maximum search depth |
| `beam_width` | `int` | 5 | Beam width (BEAM strategy) |
| `max_frontier_size` | `int` | 100 | Frontier capacity |
| `max_expansions` | `int` | 1000 | Total expansion budget |
| `strategy` | `SearchStrategy` | `BEST_FIRST` | Search strategy |
| `value_weight` | `float` | 1.0 | Weight for value score |
| `heuristic_weight` | `float` | 0.5 | Weight for heuristic |
| `enable_loop_detection` | `bool` | `True` | Cycle detection |
| `use_stable_tiebreaker` | `bool` | `True` | Deterministic ties |
| `enable_novelty_tiebreak` | `bool` | `False` | TD-6.8 novelty |

**SearchStrategy** (lines 67-74):

| Strategy | Description |
|----------|-------------|
| `BEST_FIRST` | Priority queue by value |
| `BEAM` | Top-K at each depth |
| `BREADTH_FIRST` | FIFO queue |
| `DEPTH_FIRST` | LIFO stack |

### 7.4 Expansion Protocol

The core search cycle:

1. **Select**: Pop best node from frontier (`heappop` using `priority_key`).
2. **Expand**: Get applicable operators, apply to create successor StateNodes.
3. **Evaluate**: Score each successor via value function (`evaluate_transition`).
4. **Update**: Add scored children to frontier (`heappush`), mark parent as visited.

**Open set (frontier)**: Priority queue (heapq) of SearchNodes.
**Closed set (visited)**: Set of state fingerprints for loop detection.
**Loop detection**: Via canonical state hashing (`state_fingerprint()`).

---

## 8. Value Function Protocol

Source: `core/value/protocol.py`

### 8.1 ValueFunction Interface

Source: `core/value/protocol.py:125-186`

```python
class ValueFunction(Protocol):
    @property
    def name(self) -> str: ...

    def evaluate(
        self, state: StateNode, context: ValueContext, action: Optional[str]
    ) -> float: ...

    def evaluate_transition(
        self, parent: StateNode, child: StateNode,
        edge: OperatorEdge, context: ValueContext
    ) -> float: ...

    def supports_latent(self) -> bool: ...
```

### 8.2 ValueContext

Source: `core/value/protocol.py:27-73`

Provides evaluation context to value functions. See also: [Value Function Components v1.1](value_function_components_v1.md) for the authoritative field list.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `step_idx` | `int` | 0 | Current step in episode |
| `total_steps` | `int` | 0 | Total steps so far |
| `max_path_length` | `int` | 20 | Path length budget |
| `max_depth_in_episode` | `int` | 0 | Max depth reached in episode |
| `parent_degree` | `int` | 0 | Parent branching degree |
| `child_degree` | `int` | 0 | Child branching degree |
| `candidates_count` | `int` | 1 | Number of candidates |
| `task_type` | `str` | `""` | Task classification |
| `dialogue_fallback_reason` | `Optional[str]` | `None` | Reason for dialogue scoring fallback |
| `value_adapter_fallback` | `bool` | `False` | Whether value adapter fell back to neutral |
| `goal_entity_id` | `Optional[str]` | `None` | Target entity |
| `start_entity_id` | `Optional[str]` | `None` | Start entity |
| `parent_dist_to_goal` | `Optional[int]` | `None` | Parent distance |
| `child_dist_to_goal` | `Optional[int]` | `None` | Child distance |
| `parent_decay_score` | `float` | 1.0 | SWM decay signal for parent |
| `child_decay_score` | `float` | 1.0 | SWM decay signal for child |
| `parent_visit_count` | `int` | 0 | Parent visit count |
| `child_visit_count` | `int` | 0 | Child visit count |
| `parent_last_access_age` | `int` | 0 | Parent last access age |
| `child_last_access_age` | `int` | 0 | Child last access age |
| `path_so_far` | `List[str]` | `[]` | Operator path |
| `operators_used` | `List[str]` | `[]` | Operator history |
| `extra` | `Dict[str, Any]` | `{}` | Additional metadata |

### 8.3 ValueScore

Source: `core/value/protocol.py:80-118`

Structured breakdown of value computation:

| Field | Type | Description |
|-------|------|-------------|
| `combined` | `float` | Final score used by search |
| `structural` | `Optional[float]` | Feature-based structural score |
| `memory` | `Optional[float]` | SWM decay signals |
| `task` | `Optional[float]` | Per-task reward shaping |
| `latent` | `Optional[float]` | Stage F latent similarity |
| `intent` | `Optional[float]` | Intent conditioning |
| `dialogue` | `Optional[float]` | Stage H dialogue scoring |
| `mdl` | `Optional[float]` | Stage J parsimony |
| `pragmatics` | `Optional[float]` | P1-6 pragmatic scoring |
| `weights` | `Optional[Dict[str, float]]` | Component weights |
| `confidence` | `float` | Score confidence |

### 8.4 HybridValueFunction

Source: `core/value/hybrid.py:192-766`

Multi-factor value function combining all component heads:

| Component | Default Weight | Source |
|-----------|---------------|--------|
| Structural | 50% | `StructuralValueHead` |
| Memory | 20% | `MemoryValueHead` |
| Task | 20% | `TaskValueHead` |
| Latent | 10% | `LatentValueHead` (Stage F) |
| Intent | 10% | `IntentConditionedValueHead` |
| Dialogue | 10% | `DialogueOperatorScorer` (Stage H) |
| MDL | 10% | Parsimony line |
| Pragmatics | 15% | `PragmaticsPrior` (P1-6) |

**Formula**: `score = sum(weight_i * component_i for each active component)`

Key methods:
- `evaluate(state, context, action) -> float` (line 384)
- `evaluate_detailed(state, context, action) -> ValueScore` (line 403)
- `evaluate_transition(parent, child, edge, context) -> float` (line 550)

---

## 9. Frontier Snapshot

Source: `core/reasoning/loop/frontier_snapshot.py`

### 9.1 FrontierSnapshot

Source: lines 61-119

```python
@dataclass
class FrontierSnapshot:
    schema: str = "packed_frontier_snapshot.v1"
    trigger: str = ""                          # "end_of_run", "shadow_divergence", etc.
    step: int = 0
    mode: str = "legacy"                       # "parity" or "legacy"
    ordering_fields: list[str] = []
    counter_state: int | None = None           # Parity mode only
    frontier_size: int = 0
    entries: list[FrontierSnapshotEntry] = []
    frontier_fingerprint: str | None = None
    truncated: bool = False
```

### 9.2 Ordering Modes

| Mode | Ordering Key | Tie-breaking |
|------|-------------|-------------|
| Legacy | `(neg_score, depth, node_id)` | String comparison |
| Parity | `(neg_priority_q4, counter, node_key)` | Quantized, hash-based |

### 9.3 Snapshot Verifier

Source: `core/reasoning/loop/snapshot_verifier.py`

`verify_snapshot(snapshot)` validates:
- Schema version matches expected
- Mode-specific fields are present
- Entries are sorted by ordering key
- Raises `SnapshotVerificationError` on failure

---

## 10. Search Health

Source: `core/search_health.py:36-165`

`SearchHealthAccumulator` collects O(1)-per-expansion metrics:

| Method | Timing | Description |
|--------|--------|-------------|
| `on_pop(h, f, frontier_size)` | After heappop | Records h/f costs, frontier size |
| `on_generate()` | Per successor | Counts generated nodes |
| `on_enqueue()` | At heappush | Counts enqueued nodes |
| `finalize(reason)` | End of search | Produces final metrics dict |

**Output schema** (`searchHealthVersion: 1`):

| Metric | Description |
|--------|-------------|
| `nodesExpanded` | Total nodes popped from frontier |
| `frontierPeak` | Maximum frontier size observed |
| `hMin` / `hMax` / `hMean` / `hVariance` | Heuristic distribution (Welford's algorithm) |
| `fMin` / `fMax` | f-cost range |
| `pctSameH` | Fraction sharing modal h-value |
| `branchingEstimate` | `total_generated / nodes_expanded` |
| `terminationReason` | Why search stopped |

---

## 11. Invariants Summary

### State Model

1. **Immutability**: StateNodes are never modified during search. Operators produce new StateNodes.
2. **Layer monotonicity**: UtteranceState layers accumulate (PARSED implies syntax exists). *(enforced by `validate()`)*
3. **Latent read-only**: Once `_latent_frozen` is set, the latent vector cannot be modified. *(enforced — I4)*
4. **Single source**: If `text_ir` is set, syntax/semantics are derived from it. *(convention only — I5, not checked by `validate()`)*
5. **Intent location**: Canonical intent lives at `pragmatics.intent` (I7). *(convention only, not checked by `validate()`)*
6. **Service ref exclusion** (V2): `_`-prefixed assumption keys never appear in canonical dicts, snapshots, or content hashes. *(enforced by `public_assumptions` property)*
7. **DET-1A**: Identity serialization paths use `json.dumps()` without `default=str`. Non-serializable values cause `TypeError`, not silent coercion. *(enforced in `core/contracts/semantic_edits.py`)*
8. **Deepcopy allowlist**: Only keys in `_DEEPCOPY_SHARE_BY_REFERENCE_KEYS` are shared by reference on deepcopy. Mutable `_`-prefixed keys are cloned normally. *(enforced in `_copy_assumptions()`)*

### StateGraph

6. **SG-1**: Payloads stored by reference in object store.
7. **SG-2**: Only WORLD_STATE nodes may enter frontier.
8. **SG-3**: Canonical path uses only WORLD_STATE + EXPANDS_TO.
9. **SG-4**: Edge kinds match node type constraints.

### Search

10. **Determinism**: Priority key includes stable tie-breakers (`creation_order`, `branch_id`).
11. **Single score source**: `SearchNode.score` is the authoritative ranking value.
12. **Sign convention**: Higher score = better candidate. h_cost and g_cost are subtracted.

---

## 12. Source File Index

| File | Purpose |
|------|---------|
| `core/state_model.py` | UtteranceState, WorldState, StateNode, all layers |
| `core/reasoning/state_graph.py` | StateGraph, SearchNode (graph), OperatorEdge |
| `core/reasoning/graph_emitter.py` | GraphEmitter, StateGraphEmitter (construction protocol) |
| `core/reasoning/search.py` | SearchNode (tree), SearchConfig, ImmutableSearchTree |
| `core/reasoning/loop/frontier_snapshot.py` | FrontierSnapshot, capture |
| `core/reasoning/loop/snapshot_verifier.py` | Snapshot ordering verification |
| `core/search_health.py` | SearchHealthAccumulator |
| `core/value/protocol.py` | ValueFunction, ValueContext, ValueScore |
| `core/value/hybrid.py` | HybridValueFunction |
| `core/value/structural.py` | StructuralValueHead |
| `core/value/memory.py` | MemoryValueHead |
| `core/value/task_heads.py` | TaskValueHead |
| `core/value/dialogue_scorer.py` | DialogueOperatorScorer |
| `core/value/pragmatics_prior.py` | PragmaticsPrior |
| `core/contracts/operator_application.py` | OperatorApplication (re-exported by state_model.py) |

---

## 13. Relationship to Other Canonical Documents

| Document | Relationship |
|----------|-------------|
| [Reasoning Framework](reasoning_framework.md) | High-level architecture; this doc specifies the data structures and protocols |
| [Code32 and ByteState](code32_bytestate.md) | ByteState is the packed carrier encoding of the state defined here |
| [Hashing Contracts](hashing_contracts_v1.md) | Content hashing of StateNodes uses the canonicalization contracts |
| [Semantic Working Memory](semantic_working_memory_contract_v0.md) | SWM manages persistence and retrieval of states across episodes |
| [Evaluation Gates](evaluation_gates_v1.md) | Gates validate search outcomes; search health feeds gate decisions |
| [Rust Parity Audit](../../rust_parity_audit.md) | Defines Rust certification-boundary scope and tracks current runtime parity status for native benchmark comparisons |
