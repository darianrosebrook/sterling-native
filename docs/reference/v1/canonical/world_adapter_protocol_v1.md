> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**
>
> **Superseded in v2**: `harness/src/contract.rs` (WorldHarnessV1), `search/src/contract.rs` (SearchWorldV1); 3 test worlds in `harness/src/worlds/`.
> v2 evidence: SC-001 M1, M3.1; lock tests `sc1_search_determinism.rs`, `sc1_m3_1_slot_lattice.rs`.

# World Adapter Protocol v1.1

**Status**: Canonical specification — sufficient to rebuild `core/worlds/` from scratch.
**Scope**: WorldAdapter protocol, capabilities, observation/prediction contracts, discourse world, and domain routing.
**Layer**: 2 (Worlds / Domains)
**Version**: 1.1 (corrected from 1.0 — WorldAdapter methods, WorldCapabilities fields, IR dataclass fields, HypothesisCapability, missing adapters)

---

## §1 Purpose

World adapters bridge Sterling's domain-agnostic reasoning engine to specific knowledge domains. Each adapter translates domain-specific state into the universal `StateNode` / `WorldState` representation, provides domain-scoped operators, emits observations for induction, and verifies predictions. The adapter protocol uses structural subtyping (Python `Protocol`), not ABC inheritance.

---

## §2 WorldAdapter Protocol

```python
@runtime_checkable
class WorldAdapter(Protocol):
    @property
    def name(self) -> str: ...

    def parse_input(self, raw: str) -> UtteranceState: ...

    def build_world_state(self, utterances: List[UtteranceState]) -> WorldState: ...

    def get_operators(self) -> List[OperatorSignature]: ...

    def get_registry(self) -> OperatorRegistry: ...

    def get_kernel(self) -> Kernel: ...

    def emit_observations(self, prev_state: StateNode, op_call: Any,
                          next_state: StateNode) -> List[ObservationIR]: ...

    def verify_prediction(self, prediction: Any, state: StateNode) -> TestResultIR: ...

    def supports_latent(self) -> bool: ...
```

### §2.1 Structural Subtyping

Any class that implements these methods satisfies the protocol — no explicit inheritance required. The protocol is `@runtime_checkable`, enabling `isinstance(adapter, WorldAdapter)` checks. This allows worlds to be defined in separate packages without import-time coupling.

### §2.2 Required Method Summary

All methods in the Protocol are required for structural conformance:

| Method | Purpose |
|--------|---------|
| `name` (property) | World's unique identifier string |
| `parse_input(raw) -> UtteranceState` | Parse raw text input into utterance state |
| `build_world_state(utterances) -> WorldState` | Construct WorldState from utterances |
| `get_operators() -> List[OperatorSignature]` | Return available operators in this world |
| `get_registry() -> OperatorRegistry` | Return the operator registry for this world |
| `get_kernel() -> Kernel` | Return the domain kernel for this world |
| `emit_observations(prev, op, next) -> List[ObservationIR]` | Extract deltas from transition (W-OBS-1) |
| `verify_prediction(prediction, state) -> TestResultIR` | Verify prediction (W-TEST-1, W-TEST-2) |
| `supports_latent() -> bool` | Whether world supports latent compression (Stage 5) |

Note: `capabilities` is NOT a protocol method — individual adapter classes define it as an instance attribute or property.

Source: `core/worlds/base.py`

---

## §3 WorldCapabilities

Frozen dataclass replacing string allowlists for Stage K eligibility (Milestone 2).

```
WorldCapabilities (frozen dataclass)
├── supports_discriminative_k1: bool = False  # Supports K1 discriminative evaluation
├── has_stable_abi: bool = False              # Has stable ABI for operator synthesis
├── has_deterministic_transitions: bool = False  # Transitions are deterministic
├── supports_macro_operators: bool = False    # Supports macro-operator composition
├── supports_observation_emission: bool = False  # Emits observations for induction
└── supports_prediction_verification: bool = False  # Can verify predictions
```

### §3.1 Pre-Defined Capability Constants

```python
DEFAULT_WORLD_CAPABILITIES = WorldCapabilities()  # All False

WORDNET_WORLD_CAPABILITIES = WorldCapabilities(
    supports_discriminative_k1=True, has_stable_abi=True,
    has_deterministic_transitions=True, supports_macro_operators=True,
    supports_observation_emission=True, supports_prediction_verification=True,
)

PN_WORLD_CAPABILITIES = WorldCapabilities(
    supports_discriminative_k1=True, has_stable_abi=True,
    has_deterministic_transitions=True, supports_macro_operators=False,
    supports_observation_emission=True, supports_prediction_verification=True,
)
```

### §3.2 Eligibility Methods

1. **`is_eligible_for_stage_k()`**: Requires `supports_discriminative_k1 AND has_stable_abi AND has_deterministic_transitions`
2. **`is_eligible_for_macro_operators()`**: Requires `supports_macro_operators AND has_stable_abi`
3. **`is_eligible_for_primitive(primitive_id, *, claim_registry=None, domain_id=None)`**: Two-level check:
   - Level 1 (structural): Looks up required flags from `PrimitiveRegistry` — unknown primitives fail-closed
   - Level 2 (claim-based): If `claim_registry` provided, also checks for verified claim
4. **`to_dict()`**: Serializes all 6 boolean fields to dict

Source: `core/worlds/base.py`

---

## §3A HypothesisCapability Protocol

Separate `@runtime_checkable` protocol for worlds that support hypothesis-driven learning. Maintains backward compatibility — worlds without this protocol get an inert hypothesis controller.

```python
@runtime_checkable
class HypothesisCapability(Protocol):
    def extract_observations(self, prev_state: StateNode,
                             operator_id: str, next_state: StateNode) -> List[ObservationIR]: ...

    def verify_hypothesis_prediction(self, hypothesis_id: str,
                                     prediction: Dict[str, Any],
                                     state: StateNode) -> TestResultIR: ...
```

**Usage**: `if isinstance(adapter, HypothesisCapability): observations = adapter.extract_observations(...)`

**Invariants**:
- TC-7A: Hypotheses influence search only after testing
- TC-9A: Applicability set unchanged (influence ranking, not filter)
- ARCH-IND-1: Controller calls these hooks, not world internals
- Worlds without HypothesisCapability still get uniform episode records

Source: `core/worlds/base.py`

---

## §4 Observation Contract (W-OBS-1)

```python
def emit_observations(prev_state, op_call, next_state) -> list[ObservationIR]
```

**Contract W-OBS-1**: After every operator application, the controller calls `emit_observations()`. The world adapter must:

1. Compute deltas between `prev_state` and `next_state`
2. Return `ObservationIR` objects describing what changed
3. Observations contain deltas, not full state snapshots
4. Each observation has a deterministic evidence fingerprint via `compute_evidence_fingerprint()`

**Invariant W-1**: The controller calls observation hooks — world internals never self-invoke (ARCH-IND-1).

### §4.1 ObservationIR Dataclass

```
ObservationIR (frozen dataclass)
├── observation_id: str               # Volatile: excluded from hash (DET-3)
├── kind: str                         # e.g., "ENTITY_RENAME", "POLARITY_FLIP"
├── payload: Dict[str, Any]
├── source_transition: Optional[Tuple[str, str, str]] = None  # (prev_hash, op_id, next_hash)
└── timestamp: float = time.time()    # Volatile: excluded from hash (DET-3)
```

**Methods**:
- `to_canonical_dict()` → excludes `observation_id` and `timestamp` (volatile fields)
- `compute_evidence_fingerprint()` → SHA-256 of `json.dumps(canonical, sort_keys=True)`

### §4.2 DeltaObservationIR

Extends `ObservationIR` for state transformation deltas. Used by operator induction.

```
DeltaObservationIR(ObservationIR) (frozen dataclass)
├── prev_state_hash: str = ""         # REQUIRED (post_init validates non-empty)
├── next_state_hash: str = ""         # REQUIRED (post_init validates non-empty)
├── semantic_edit: Any = None         # SemanticEdit or dict
├── triggering_operator: Optional[str] = None
├── operator_category: Optional[str] = None  # Must be S/M/P/K/C if provided
├── improved_score: bool = False      # NON-CANONICAL annotation
└── score_delta: float = 0.0          # NON-CANONICAL annotation
```

**FOOTGUN A FIX**: `improved_score` and `score_delta` are NON-CANONICAL annotations excluded from `to_canonical_dict()` to prevent hash poisoning from policy/scoring changes.

**Canonical dict** includes: kind, payload, source_transition, prev_state_hash, next_state_hash, triggering_operator, operator_category, and canonicalized semantic_edit (volatile fields stripped from edit). Semantic edit must be dict or have `to_dict()` — `str()` fallback raises `ValueError` (prevents non-deterministic hashing).

### §4.3 ObservationIR Types

Observations record domain-specific deltas:

| Observation Type | Detected When | Example |
|-----------------|---------------|---------|
| GOAL_SPEC_CHANGE | Goal count changes in discourse | New goal added or resolved |
| INTENT_SATISFACTION | Pending → satisfied intents | Clarification satisfied |
| PHASE_TRANSITION | Dialogue phase changes | Opening → exploration |
| SYNSET_TRANSITION | Current synset changes in KG | Traversed hypernym edge |
| LAYER_CHANGE | Utterance layer presence changes | Semantics added |

Source: `core/worlds/discourse/world.py`, `core/worlds/wordnet.py`

---

## §5 Prediction Contract (W-TEST-1, W-TEST-2)

```python
def verify_prediction(prediction, state) -> TestResultIR
```

**Contract W-TEST-1**: Worlds verify predictions against their domain state.

**Contract W-TEST-2**: Predictions must be pure and side-effect free.

### §5.1 TestResultIR Dataclass

```
TestResultIR (frozen dataclass)
├── outcome: Literal["PASS", "FAIL", "UNKNOWN"]
├── episode_id: Optional[str] = None       # First-class episode attribution
├── witness_ref: Optional[str] = None      # Pointer to failure witness
├── details_hash: Optional[str] = None
├── message: Optional[str] = None          # For UNKNOWN: must include reason code
└── cost: float = 0.0
```

**Post-init validation (W-TEST-1A)**:
- `FAIL` requires `witness_ref` (counterexample reference)
- `UNKNOWN` requires `message` with reason code (e.g., `"UNIMPLEMENTED"`, `"NONDETERMINISTIC_CHECK"`, `"MISSING_CONTEXT"`)

**Methods**:
- `to_canonical_dict()` → includes all fields; `episode_id` only if not None
- `compute_evidence_fingerprint()` → SHA-256 of canonical dict

### §5.2 Search Influence Rules

**TC-7A**: Hypotheses influence search ranking only after testing — not before.
**TC-9A**: Hypotheses influence operator ranking, not filtering (applicability set unchanged).

Source: `core/worlds/base.py`

---

## §6 Registered World Adapters

### §6.1 DiscourseWorldAdapter

Entry point for Sterling reasoning. Transforms raw utterances and dialogue context into domain-agnostic `GoalSpec` objects.

**Name**: `"discourse"`
**Capabilities**: All flags true (observation, prediction, latent encoding)

#### Goal Specification

```
GoalType (enum)
├── CLARIFY      # Resolve ambiguity
├── NAVIGATE     # Move through structure
├── TRANSFORM    # Change representation
├── VERIFY       # Check correctness/invariants
└── RETRIEVE     # Get information/evidence

GoalSpec
├── goal_type: GoalType
├── entities: list[EntityBinding]
├── success_criteria: SuccessCriteria?
├── confidence: float                    # 0.0–1.0
├── suggested_steps: list[str]           # Plan sketch
└── source_text: str?                    # For debugging
```

**Invariants**:
- All entities must be bound OR explicitly marked UNKNOWN via `BindingStatus`
- If `success_criteria` is None, `confidence` must be < 0.5

#### Entity Binding

```
BindingStatus (enum)
├── BOUND       # Entity resolved to reference
├── UNKNOWN     # Entity mentioned but not resolved
└── IMPLICIT    # Entity inferred from context

EntityBinding
├── entity_id: str
├── mention_text: str?        # Surface form (debugging only)
├── status: BindingStatus
├── entity_type: str?         # e.g., "concept", "symbol", "claim"
└── properties: dict[str, Any]
```

#### Discourse Operators

All category P (Pragmatic), scope DISCOURSE:

| Operator | Reads | Writes | Precondition |
|----------|-------|--------|-------------|
| SELECT_GOAL_TYPE | pragmatics | pragmatics | has_pragmatics |
| BIND_ENTITIES | semantics, pragmatics | pragmatics | has_semantics |
| SET_SUCCESS_CRITERIA | pragmatics | pragmatics | has_pragmatics |
| CLARIFY | pragmatics | pragmatics | has_pragmatics |
| ELABORATE | pragmatics | pragmatics | has_pragmatics |
| PROPOSE_PLAN | pragmatics | pragmatics | has_pragmatics |

**Speech acts** (parameterization): assertion, question, command, declaration, request, promise
**Tones** (parameterization): formal, casual, urgent, technical, friendly, neutral

#### World Entry Routing

```
WorldEntry
├── target_world: str             # World to enter
├── landmark_id: str?             # Optional routing landmark
├── goal_spec: GoalSpec           # Domain-agnostic goal
├── initial_state: WorldState?
└── confidence: float
```

Flow: Discourse → Landmark → Domain World

#### Discourse Context

```
DiscourseContext
├── turn_count: int
├── topic_stack: list[str]
├── pending_clarifications: list[str]
├── bound_entities: dict[str, EntityBinding]
└── partial_goal: GoalSpec?
```

Source: `core/worlds/discourse/world.py`, `core/worlds/discourse/types.py`, `core/worlds/discourse/operators.py`

### §6.2 PnWorldAdapter

Predicate nominal reasoning over copular sentences.

**Name**: `"linguistics"` (not "pn")
**Capabilities**: `PN_WORLD_CAPABILITIES` — supports_discriminative_k1, has_stable_abi, has_deterministic_transitions, supports_observation_emission, supports_prediction_verification (no macro-operators)
**Operators**: APPLY_NEGATION, SWAP_SUBJECT_PREDICATE, EXPAND_CONTRACTION, NORMALIZE_COPULA, STANDARDIZE_DETERMINER, NORMALIZE_CLITIC_COPULA

Source: `core/worlds/pn.py`

### §6.3 WordNetWorldAdapter

Knowledge graph navigation over WordNet synsets.

**Name**: `"wordnet"`
**Capabilities**: All flags true (fully certified)
**Operators**: HYPERNYM_OF, HYPONYM_OF, MERONYM_OF, HOLONYM_OF, SIMILAR_TO, ANTONYM_OF

Emits observations for synset transitions. Verifies predictions about hypernym/hyponym relations.

Source: `core/worlds/wordnet.py`

### §6.4 TextWorldAdapter

Text IR layer transformations.

**Name**: `"text"`
**Operators**: Shared PN operators (APPLY_NEGATION, EXPAND_CONTRACTION, etc.)

Source: `core/worlds/text.py`

### §6.5 LandmarkWorldAdapter

Routing hub between discourse and domain worlds.

Source: `core/worlds/landmarks.py`

### §6.6 CodeRefactoringWorldAdapter

Code transformation operations.

**Name**: `"code_refactoring"`
**Operators**: RENAME_SYMBOL, EXTRACT_FUNCTION, INLINE_VARIABLE, EXTRACT_VARIABLE

Source: `core/worlds/code_refactoring.py`

### §6.7 PseudocodeWorldAdapter

Code reasoning over Pseudocode IR (`PProgram` / `PModule` / `PFunction`).

**Name**: `"pseudocode"`
**Operators**: LIFT_PATTERN (M), EXTRACT_BLOCK (S), INLINE_CALL (S), ADD_GUARD (M)
**Features**: Includes `PythonLowerer` for lowering Python AST to Pseudocode IR, symbol table construction, scope analysis.

Source: `core/worlds/pseudocode.py`

### §6.8 ClaimKGWorldAdapter

Claim verification over the Truth KG and optional external knowledge graphs.

**Name**: configurable via `ClaimWorldConfig.world_kind`
**Config**: `ClaimWorldConfig` dataclass with world_kind, claim_types, evidence_modes, scoring parameters
**Operators**: Dynamic based on `claim_types` — e.g., VERIFY_CLAIM, FIND_EVIDENCE, CHECK_CONSISTENCY
**Features**: `ClaimWorldStatePayload` for claim-specific world state, entity-to-claim matching with word boundary checks.

Source: `core/worlds/claims.py`

### §6.9 MastermindWorldAdapter

Mastermind (code-breaking) game with partial observability.

**Name**: `"mastermind"`
**Config**: `MastermindWorldConfig` dataclass
**Operators**: APPLY_GUESS (category K — Knowledge)

Source: `core/worlds/mastermind.py`

---

## §7 Discourse → Domain Transition

The standard flow for processing natural language input:

1. **DiscourseWorldAdapter** receives raw utterance + context
2. `parse_input()` extracts soft intent features (not hard classifications)
3. `build_world_state()` constructs WorldState with dialogue_episode in assumptions
4. Discourse operators produce `GoalSpec` (domain-agnostic)
5. `WorldEntry` routes to target domain world via LandmarkWorldAdapter
6. Domain world operators work toward `goal_predicate`

**Invariant W-2**: Discourse output is domain-agnostic. GoalSpec contains no domain-specific types.

**Invariant W-3**: No operator may depend on raw token substrings as a precondition (Discourse Invariant #3).

---

## §8 Text IR Integration

Text processing follows a governance-tracked pipeline:

1. Text parsed to `TextIntakeIRv1` (governance-tracked)
2. Converted to `UtteranceState` — **no embedded TextIR** (governance rule)
3. TextIR passed as artifact reference in request input
4. World adapters access TextIR via `request.get_input_value("text_intake_ref")`

### §8.1 SterlingTextPipeline

```python
class SterlingTextPipeline:
    def parse_text(raw_text: str) -> TextIntakeIRv1
    def text_to_utterance(raw_text, utterance_id?, include_text_ir?) -> (TextIntakeIRv1, UtteranceState)
    def explain_reasoning(reasoning_result, query?) -> ExplanationResult
    def realize_answer(reasoning_result) -> RealizationResult
    def process(input_text, engine?, task_type?, options?, explain?, realize?,
                governance_context?) -> TextPipelineResult
```

### §8.2 TextPipelineResult

```
TextPipelineResult
├── input_text: str
├── intake_ir: TextIntakeIRv1?
├── utterance_state: UtteranceState?
├── reasoning_result: SterlingResponse?
├── run_result: RunResultV1?
├── explanation: ExplanationResult?
├── answer_text: str?
├── success: bool
├── failures: list[PipelineFailure]
├── parse_status: PipelineStageStatus      # PASS | FAIL | SKIPPED
├── reasoning_status: PipelineStageStatus
├── explain_status: PipelineStageStatus
└── realize_status: PipelineStageStatus
```

Source: `core/text/pipeline.py`

---

## §9 Invariants

1. **W-1**: Controller calls observation/prediction hooks — world internals never self-invoke (ARCH-IND-1).
2. **W-2**: Discourse output (GoalSpec) is domain-agnostic.
3. **W-3**: No operator may depend on raw token substrings as precondition.
4. **W-4**: Predictions must be pure and side-effect free (W-TEST-2).
5. **W-5**: Hypotheses influence search only after testing (TC-7A).
6. **W-6**: Hypotheses influence operator ranking, not filtering (TC-9A — applicability set unchanged).
7. **W-7**: Canonical dict representations exclude volatile fields for deterministic hashing (DET-1A, DET-3).
8. **W-8**: All hashes deterministic via `json.dumps(sort_keys=True)`.

---

## §10 Related Documents

- [Operator Registry Contract](operator_registry_contract_v1.md) — Where world operators are registered
- [State Model Contract](state_model_contract_v1.md) — StateNode/WorldState that adapters populate
- [Discourse Intent Contract](discourse_intent_contract_v1.md) — Discourse structure and intent classification
- [Text Hard IR Contract](text_hard_ir_contract_v1.md) — Hard language IR sidecar
- [Knowledge Graph Contract](knowledge_graph_contract_v1.md) — KG that WordNet adapter traverses

---

## §11 Source File Index

| File | Defines |
|------|---------|
| `core/worlds/base.py` | WorldAdapter Protocol, HypothesisCapability Protocol, WorldCapabilities, ObservationIR, DeltaObservationIR, TestResultIR, GoalSpec, SearchPolicy, SterlingOptions, SterlingRequest, SterlingResponse |
| `core/worlds/discourse/world.py` | DiscourseWorldAdapter |
| `core/worlds/discourse/types.py` | GoalType, GoalSpec, EntityBinding, BindingStatus, SuccessCriteria, WorldEntry, DiscourseContext |
| `core/worlds/discourse/operators.py` | Discourse operator signatures |
| `core/worlds/pn.py` | PnWorldAdapter |
| `core/worlds/wordnet.py` | WordNetWorldAdapter |
| `core/worlds/text.py` | TextWorldAdapter |
| `core/worlds/landmarks.py` | LandmarkWorldAdapter |
| `core/worlds/code_refactoring.py` | CodeRefactoringWorldAdapter |
| `core/worlds/pseudocode.py` | PseudocodeWorldAdapter, PythonLowerer, Pseudocode IR types |
| `core/worlds/claims.py` | ClaimKGWorldAdapter, ClaimWorldConfig, ClaimWorldStatePayload |
| `core/worlds/mastermind.py` | MastermindWorldAdapter, MastermindWorldConfig |
| `core/worlds/delta_observation_helpers.py` | create_delta_observations_from_edit_delta, create_delta_observation_from_semantic_edit |
| `core/text/pipeline.py` | SterlingTextPipeline, TextPipelineResult |

---

## Changelog

### v1.1 (2026-02-17)
- **§2**: Fixed WorldAdapter protocol — added `parse_input`, `build_world_state`, `get_registry`, `get_kernel`, `supports_latent` as required methods (were listed as "optional" or missing); added `@runtime_checkable`; noted `capabilities` is not a protocol method
- **§3**: Replaced incorrect WorldCapabilities fields (`supports_observation`, `supports_prediction`, etc.) with actual fields (`supports_discriminative_k1`, `has_stable_abi`, `has_deterministic_transitions`, `supports_macro_operators`, `supports_observation_emission`, `supports_prediction_verification`); added pre-defined constants and `is_eligible_for_macro_operators()` method; fixed `is_eligible_for_stage_k()` logic
- **§3A**: Added HypothesisCapability protocol (separate `@runtime_checkable` protocol for hypothesis-driven learning)
- **§4.1–4.2**: Added ObservationIR and DeltaObservationIR frozen dataclass fields, volatile field exclusions, FOOTGUN A FIX documentation
- **§5.1**: Added TestResultIR frozen dataclass fields with post-init validation rules (W-TEST-1A)
- **§6.2**: Fixed PnWorldAdapter name from `"pn"` to `"linguistics"`; updated capabilities to actual field names
- **§6.5**: Fixed file path from `landmark.py` to `landmarks.py`
- **§6.7–6.9**: Added missing adapters: PseudocodeWorldAdapter, ClaimKGWorldAdapter, MastermindWorldAdapter
- **§11**: Fixed discourse adapter path (`discourse/adapter.py` → `discourse/world.py`), added 4 missing source files, expanded `base.py` defines list
