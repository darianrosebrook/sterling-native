> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Value Function Components v1.1

**Status**: Canonical specification — sufficient to rebuild `core/value/` from scratch.
**Scope**: ValueFunction protocol, component heads, hybrid composition, scoring formula, and ablation.
**Layer**: 0 (Reasoning / Value)
**Version**: 1.1 (corrected from 1.0 — TaskType enum, operator features, MDL details, PragmaticFeatures, CoreFeaturizer)

---

## §1 Purpose

Sterling's value function estimates the quality of search states to guide the A*-style reasoning search. The architecture is composable: individual component heads (structural, memory, task, latent, dialogue, MDL, pragmatics) each produce a score, and the `HybridValueFunction` combines them with configurable weights. This design supports ablation studies, progressive capability addition, and domain-specific tuning.

---

## §2 ValueFunction Protocol

```python
class ValueFunction(Protocol):
    @property
    def name(self) -> str: ...

    def evaluate(state: StateNode, context: ValueContext,
                 action: str? = None) -> float: ...

    def evaluate_transition(parent: StateNode, child: StateNode,
                            edge: OperatorEdge, context: ValueContext) -> float: ...

    def supports_latent(self) -> bool: ...
```

All value function components implement this protocol. Scores are in [0, 1] range by convention.

**Commitment 5**: Latent is an optional capability. Components that don't use latent representations return `supports_latent() → False`.

### §2.1 BaseValueFunction

Default implementation:
- `evaluate()` returns 0.5 (neutral)
- `evaluate_transition()` delegates to `evaluate(child, context, action=edge.edge_type)`
- `supports_latent()` returns False

Source: `core/value/protocol.py:193-230`

---

## §3 ValueContext

All information needed to score a state or transition:

```
ValueContext
├── step_idx: int                          # Current step in search
├── total_steps: int                       # Total steps so far
├── max_path_length: int                   # Budget (default 20)
├── max_depth_in_episode: int              # Deepest depth reached
│
├── parent_degree: int                     # Branching factor of parent
├── child_degree: int                      # Branching factor of child
├── candidates_count: int                  # Candidate count (default 1)
│
├── task_type: str                         # Task identifier
│
├── dialogue_fallback_reason: str?         # Why dialogue scoring fell back
├── value_adapter_fallback: bool           # Whether adapter fell back to neutral
├── goal_entity_id: str?                   # Target entity for distance
├── start_entity_id: str?                  # Start entity for distance
│
├── parent_dist_to_goal: int?              # Distance signals
├── child_dist_to_goal: int?
│
├── parent_decay_score: float              # Memory decay (default 1.0)
├── child_decay_score: float
├── parent_visit_count: int               # Visit counts for loop detection
├── child_visit_count: int
├── parent_last_access_age: int           # Recency signals
├── child_last_access_age: int
│
├── path_so_far: list[str]                # Node IDs visited
├── operators_used: list[str]             # Operators applied
└── extra: dict[str, Any]                 # Extension point
```

Source: `core/value/protocol.py:27-72`

---

## §4 ValueScore

Structured score with component breakdown for debugging and ablation:

```
ValueScore
├── combined: float                # What the search uses
│
├── structural: float?             # Feature-based structural head
├── memory: float?                 # SWM-based memory signals
├── task: float?                   # Task-specific reward shaping
├── latent: float?                 # Latent similarity scoring (Stage F)
├── intent: float?                 # Intent-conditioned value
├── dialogue: float?               # Dialogue operator scoring (Stage H)
├── mdl: float?                    # Parsimony line (Stage J)
├── pragmatics: float?             # Pragmatic prior (P1-6)
│
├── weights: dict[str, float]?     # Weights used (reproducibility)
├── confidence: float              # Score confidence (default 1.0)
│
├── dialogue_used: bool            # Whether dialogue scoring participated
├── dialogue_fallback_reason: str? # Why dialogue fell back
└── value_adapter_fallback: bool   # Whether adapter fell back to neutral
```

`__float__()` returns `combined`, allowing `ValueScore` to be used directly as a float.

Source: `core/value/protocol.py:80-117`

---

## §5 Component Heads

### §5.1 StructuralValueHead

Feature-based structural scoring using degree, depth, and progress.

**Name**: `"structural"`
**Features**: degree normalization, depth progress, step progress, goal distance
**Feature modes**: `structural8`, `structural9`, `structural12`
**Model**: Optional trained `TransitionScorer` from `core/reasoning/value_features.py`

Falls back to heuristic scoring if no trained model is available.

Source: `core/value/structural.py`

### §5.2 MemoryValueHead

SWM-based memory signals using decay, recency, and visit counts.

**Name**: `"memory"`
**Signals**: novelty (unvisited = high), recency (recently accessed = high), visit penalty (revisit = low)
**Weights**: novelty 0.4, recency 0.3, visit_penalty 0.3 (configurable)
**Decay engine**: Optional `kg.decay.DecayEngine` integration

Source: `core/value/memory.py`

### §5.3 TaskValueHead

Task-specific reward shaping for different reasoning tasks.

**Name**: `"task"`
**Factory**: `get_task_value_head(task_type: str) → TaskValueHead`
**Purpose**: Inject task-specific scoring heuristics (e.g., PN tasks may reward polarity changes, WordNet tasks may reward synset proximity)

Source: `core/value/task_heads.py`

### §5.4 LatentValueHead (Stage F)

Latent similarity scoring using learned embeddings.

**Name**: `"latent"`
**Purpose**: Score states based on learned latent representation similarity
**Protocol**: `LatentValueHead` in `core/value/latent/value_head.py`
**Integration**: Optional — enabled via `use_latent=True` in config

Source: `core/value/latent/`

### §5.5 DialogueOperatorScorer (Stage H)

Dialogue-aware operator scoring for discourse reasoning.

**Name**: `"dialogue"`
**Purpose**: Score operators based on dialogue context and intent features
**Fallback tracking**: Never lets neutral fallback be silent — records `dialogue_fallback_reason`

Source: `core/value/dialogue_scorer.py`

### §5.6 MDL Component (Stage J)

Minimum description length parsimony line — penalizes overly complex solutions.

**Name**: `"mdl"`
**Purpose**: Lower MDL → higher score (reward simpler explanations)

**MDL Formula**: `MDL_total = L_struct + L_params + L_ex`
- `L_struct`: Structure cost (program length, key count, nesting depth)
- `L_params`: Parameter cost (free variables, disjunctions, optional clauses)
- `L_ex`: Exception cost (failures weighted by episode weights)

**Config** (`MDLCostConfig`): `program_length_weight=1.0`, `exception_cost_weight=2.0`, `parameter_entropy_weight=0.5`, `invariant_cost_weight=1.0`, `exception_base_cost=10.0`, `char_cost=0.1`, `key_cost=1.0`, `nesting_cost=0.5`

**MDLScorer methods**: `compute_mdl_score()`, `compute_mdl_breakdown() -> MDLCostBreakdown`, `rank_by_mdl()`, `select_minimum_mdl()`

**WeightApplicationWitness**: Tracks whether episode weights were used, weight checksum, mass, and weighted terms count.

Source: `core/value/mdl.py`

### §5.7 PragmaticsPrior (P1-6)

Pragmatic prior incorporating Gricean maxims and discourse conventions.

**Name**: `"pragmatics"`
**Input**: `PragmaticFeatures` dataclass — `is_interrogative`, `is_imperative`, `has_modal`, `punct_type` (".", "?", "!", ""), `turn_index`
**Scoring**: Scores INFER_INTENT and DETECT_TONE operators based on IR features extracted from morphological features (Mood, VerbType), POS tags, and punctuation. Returns neutral 0.5 for non-dialogue operators. Score range [0.35, 0.65].

Source: `core/value/pragmatics_prior.py`

### §5.8 TaskType Enum

Defined in `core/value/target_contract.py`:

```
TaskType (str, Enum)
├── NAVIGATION, TRANSFORMATION, RETRIEVAL, VERIFICATION
├── WORDNET_NAVIGATION, WORDNET_SIMILARITY
├── PN_VERIFICATION, PN_CANONICALIZE, PN_MINIMAL_FLIP, PN_IDENTITY_INVERT
├── DIALOGUE_ROLLOUT
└── ESCAPE_SOLVE, ESCAPE_OPTIMAL
```

### §5.9 Operator Feature Extraction

39-dimensional feature vectors for operator signatures (`core/value/operator_features.py`):

```
Canonical Vocabularies:
  CATEGORY_ORDER = ["S", "M", "P", "K", "C"]          # 5 dims
  SCOPE_ORDER = ["utterance", "discourse", "world"]     # 3 dims
  LAYER_ORDER = ["syntax", "semantics", "semiotics", "pragmatics", "latent"]  # 5 dims (reads + writes)
  PRECONDITION_ORDER: 13 dims (from registry)
  EFFECT_ORDER: 5 dims (from registry)
  LABEL_CONSTRAINTS: 3 dims
  TOTAL_FEATURE_DIM = 39
```

Provides `cosine_similarity()`, `operator_similarity()`, `find_similar_operators()` for operator comparison.

### §5.10 CoreFeaturizer

Feature extraction with namespaced output (`core/value/featurizers/core.py`):

| Namespace | Features |
|-----------|----------|
| `core.constraint.*` | preconditions_satisfied, active_rules_count, precondition_rate |
| `core.novelty.*` | visit_count, is_novel, step_fraction, depth_normalized, loop_risk |
| `core.operator.*` | applicable_count, applicability_rate, category_*_total/applicable |
| `core.coverage.*` | activated_entities, activation_rate, has_kg_handle |
| `core.graph.*` | entity_count, proposition_count, edge_count, density, pn_type, polarity |

---

## §6 HybridValueFunction

Multi-factor composition of all component heads.

### §6.1 HybridValueConfig

```
HybridValueConfig
├── w_structural: float = 0.5       # Primary signal (50%)
├── w_memory: float = 0.2           # Secondary signal (20%)
├── w_task: float = 0.2             # Secondary signal (20%)
├── w_latent: float = 0.1           # Optional enhancement (10%)
├── w_intent: float = 0.1           # Optional enhancement (10%)
├── w_dialogue: float = 0.1         # Dialogue scoring (10%)
├── w_mdl: float = 0.1              # Parsimony (10%)
├── w_pragmatics: float = 0.15      # Pragmatic prior (15%)
│
├── use_structural: bool = True
├── use_memory: bool = True
├── use_task: bool = True
├── use_latent: bool = False         # Disabled by default (Stage F)
├── use_intent: bool = False         # Disabled by default
├── use_dialogue: bool = False       # Disabled by default (Stage H)
├── use_mdl: bool = False            # Disabled by default (Stage J)
├── use_pragmatics: bool = False     # Disabled by default (P1-6)
│
├── structural_model_path: Path?
├── structural_feature_mode: str = "structural9"
├── structural_hidden_dim: int = 32
│
├── memory_novelty_weight: float = 0.4
├── memory_recency_weight: float = 0.3
└── memory_visit_penalty_weight: float = 0.3
```

**Weight semantics**: Weights are relative importance, not absolute probabilities. Normalized at evaluation time based on which components are enabled.

### §6.2 Composition Formula

At evaluation time:

```
active_components = {name: (head, weight) for each enabled component}
weight_sum = sum(weight for _, weight in active_components)
combined = sum(weight * head.evaluate(state, context) for head, weight in active_components) / weight_sum
```

### §6.3 Factory Functions

| Factory | Components Enabled |
|---------|-------------------|
| `create_hybrid_value_function(config)` | structural + memory + task |
| `create_hybrid_with_teacher(config)` | + teacher latent head |
| `create_hybrid_with_student(config)` | + student latent head |
| `create_hybrid_full_stack(config)` | All components |

**INV-F5**: With `use_latent=False`, behavior is identical to pre-Stage-F.

Source: `core/value/hybrid.py`

---

## §7 ConfigurableValueModel (TD-6)

Alternative scoring model with explicit formula:

```
combined = value_weight * value_score - heuristic_weight * heuristic_distance - penalties
```

### §7.1 Sign Convention

| Component | Higher Means | Combined Effect |
|-----------|-------------|-----------------|
| value_score | Better (learned desirability) | Added |
| heuristic_distance | Farther from goal | Subtracted |
| penalties | Worse (loops, depth) | Subtracted |
| combined | Better overall | Used by search |

### §7.2 ValueModelConfig

```
ValueModelConfig
├── value_weight: float = 1.0
├── heuristic_weight: float = 0.5
├── enable_value: bool = True           # Ablation toggle
├── enable_heuristic: bool = True       # Ablation toggle
├── loop_penalty: float = 0.1
├── depth_penalty: float = 0.01
├── dead_end_penalty: float = 0.5
├── normalize_scores: bool = True
└── score_range: (0.0, 1.0)
```

### §7.3 Penalties

```python
penalty = depth_penalty * state.depth + loop_penalty * max(0, state.visit_count - 1)
```

Source: `core/value/protocol.py:313-542`

---

## §8 StateValueModel Protocol (TD-6)

```python
class StateValueModel(Protocol):
    @property
    def model_id(self) -> str: ...
    def score_state(state, context?) -> float: ...
    def score_with_features(features: dict) -> float: ...
    def get_score_breakdown(state, context?) -> ValueScore: ...
```

Enables swapping value model implementations while maintaining consistent API for search infrastructure.

Source: `core/value/protocol.py:238-305`

---

## §9 Invariants

1. **V-1**: All component `evaluate()` methods return values in [0, 1].
2. **V-2**: `HybridValueFunction` normalizes weights at evaluation time — only active components participate.
3. **V-3**: INV-F5 — with `use_latent=False`, hybrid behavior is identical to pre-Stage-F.
4. **V-4**: `ConfigurableValueModel` subtracts heuristic distance (lower distance = better score).
5. **V-5**: Dialogue scoring fallback is never silent — always records `dialogue_fallback_reason`.
6. **V-6**: `ValueScore.__float__()` returns `combined`, ensuring transparent use as a float.

---

## §10 Related Documents

- [State Model Contract](state_model_contract_v1.md) — StateNode that value functions score
- [Reasoning Framework](reasoning_framework.md) — Search that uses value scores for node ordering

---

## §11 Source File Index

| File | Defines |
|------|---------|
| `core/value/protocol.py` | ValueFunction Protocol, BaseValueFunction, ValueContext, ValueScore, StateValueModel Protocol, ConfigurableValueModel, ValueModelConfig |
| `core/value/hybrid.py` | HybridValueFunction, HybridValueConfig, factory functions |
| `core/value/structural.py` | StructuralValueHead |
| `core/value/memory.py` | MemoryValueHead |
| `core/value/task_heads.py` | TaskValueHead, get_task_value_head |
| `core/value/dialogue_scorer.py` | DialogueOperatorScorer |
| `core/value/mdl.py` | MDL parsimony component |
| `core/value/pragmatics_prior.py` | PragmaticsPrior |
| `core/value/latent/value_head.py` | LatentValueHead |
| `core/value/latent/protocol.py` | Latent scoring protocol |
| `core/value/adapters.py` | Value adapter utilities |
| `core/value/target_contract.py` | Target contract for value training |
| `core/value/featurizers/core.py` | CoreFeaturizer, feature namespace functions |
| `core/value/featurizers/kernel_augment.py` | Kernel-augmented features |
| `core/value/feature_classification.py` | Feature classification utilities |
| `core/value/grouped_head.py` | Grouped value head |
| `core/value/landmark_embeddings.py` | Landmark embedding integration |
| `core/value/operator_features.py` | OperatorFeatures, extract_operator_features, canonical vocabularies, cosine_similarity |
| `core/value/target_contract.py` | TaskType, ValueTargetConfig, ValueTarget, ValueTargetContract, CorrelationMetrics |
| `core/value/latent/ablation_modes.py` | Latent ablation mode definitions |
| `core/value/latent/ir_bottleneck_encoder.py` | IR bottleneck encoder |
| `core/value/latent/ir_latent_v1.py` | IR latent v1 format |
| `core/value/latent/latent_value_model_v2.py` | Latent value model v2 |
| `core/value/latent/latent_value_model_v3.py` | Latent value model v3 |
| `core/value/latent/serialization.py` | Latent model serialization |
| `core/value/latent/sterling_encoder.py` | Sterling encoder for latent features |
| `core/value/latent/student_head.py` | Student value head (distillation) |
| `core/value/latent/teacher_head.py` | Teacher value head (distillation) |
| `core/value/latent/training_dataset.py` | Training dataset for latent models |
| `core/value/latent/training_dataset_v2.py` | Training dataset v2 |
| `core/value/latent/archive/latent_value_model.py` | Archived latent value model v1 |
| `core/reasoning/value_features.py` | FeatureSpec, TransitionScorer, make_structural_features |

---

## Changelog

### v1.1 (2026-02-17)
- **§5.6**: Expanded MDL component with MDLCostBreakdown formula, MDLCostConfig fields, MDLScorer methods, WeightApplicationWitness
- **§5.7**: Expanded PragmaticsPrior with PragmaticFeatures dataclass, scoring range, IR feature extraction details
- **§5.8**: Added TaskType enum (13 task types from target_contract.py)
- **§5.9**: Added operator feature extraction system (39-dim vectors, canonical vocabularies, similarity functions)
- **§5.10**: Added CoreFeaturizer with 5 feature namespaces (constraint, novelty, operator, coverage, graph)
- **§11**: Added 3 missing source files (operator_features.py, target_contract.py, value_features.py)
