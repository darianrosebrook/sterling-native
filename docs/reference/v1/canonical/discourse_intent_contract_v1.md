> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Discourse Intent Contract v1.1

**Status**: Canonical specification — sufficient to rebuild `core/linguistics/discourse/` and `core/intent/` from scratch.
**Scope**: Discourse structure, intent classification, speech acts, scope/attribution, and forbidden claim classes.
**Layer**: 2 (Linguistics / Discourse)
**Version**: 1.1 (corrected from 1.0 — dialogue phases, missing operators, intent taxonomy, discourse relations, satisfaction FSM, source paths)

---

## §1 Purpose

Sterling's discourse system transforms raw utterances into structured intent representations. Unlike pipeline systems that classify intent into hard categories, Sterling treats intent as soft feature vectors that feed into the reasoning search. The discourse layer manages dialogue state, turn-taking, topic tracking, and routes goals to domain-specific world adapters.

---

## §2 Intent Classification

### §2.1 Soft Feature Vectors

Intent is represented as a feature vector, not a hard classification. The `DiscourseWorldAdapter.parse_input()` method calls `core.intent.classifier.get_intent_features()` to produce soft features attached to `UtteranceState.pragmatics.intent`.

This design avoids premature commitment to a single intent — the reasoning search explores multiple interpretations.

### §2.2 GoalType

Once discourse reasoning converges, intent features are mapped to a goal type:

```
GoalType (enum)
├── CLARIFY      # Resolve ambiguity in input
├── NAVIGATE     # Move through conceptual/knowledge structure
├── TRANSFORM    # Change representation form
├── VERIFY       # Check correctness or invariants
└── RETRIEVE     # Get information or evidence
```

These are small, operator-oriented categories — not fine-grained intent labels.

Source: `core/worlds/discourse/types.py`

### §2.3 IntentFamily and IntentType Taxonomy

Intent classification uses a two-level hierarchy defined in `core/intent/types.py`:

```
IntentFamily (IntEnum) — 10 families
├── EXPLAIN = 0      # Answer or clarify about existing material
├── EXPAND = 1       # Add detail, fill in, continue
├── COMPRESS = 2     # Summarize or condense
├── TRANSFORM = 3    # Change form without changing meaning
├── PLAN = 4         # Decompose, structure, organize future work
├── EVALUATE = 5     # Critique, verify, test, compare
├── CONSTRAIN = 6    # Tighten specs, choose among options
├── GENERATE = 7     # Create new content from scratch
├── NAVIGATE = 8     # Move/select within a space
└── META = 9         # Talk about the process, rules, or system
```

Each family contains specific `IntentType` values (IntEnum, 40+ types total). Examples:
- EXPLAIN: ANSWER_DIRECT(0), EXPLAIN_CONCEPT(1), EXPLAIN_CODE(2), EXPLAIN_DECISION(3), DEFINE_TERM(4)
- TRANSFORM: REWRITE_STYLE(30), REFORMAT(31), TRANSLATE_LANGUAGE(32), REFACTOR_CODE(34)
- EVALUATE: CRITIQUE(50), VERIFY_FACTS(51), CHECK_CONSISTENCY(52), TEST_CASES(53)
- UNKNOWN = 255

### §2.4 OperationType (Operational Vocabulary)

Maps user-facing operation labels to canonical IntentType:

```
OperationType (str constants): ELABORATE, SPECIFY, PLAN, VERIFY, CRITIQUE,
  ACCEPT, PIVOT, REFRAME, BRANCH, COMPRESS, DEBUG, COMPARE, RESOLVE, UNKNOWN
```

`map_operation_to_intent(operation, raw_prompt, context) -> IntentType` performs the canonical mapping.

### §2.5 IntentFeatures Dataclass

The primary API returns soft features, not hard classifications:

```
IntentFeatures
├── text: str                                    # Raw text for reference
├── family_scores: Dict[IntentFamily, float]     # Soft scores per family (sum ≈ 1.0)
├── type_scores: Dict[IntentFamily, Dict[IntentType, float]]  # Nested scores
├── phrase_features: Dict[IntentFamily, float]   # Phrase match scores (weight 3.0)
├── action_features: Dict[IntentFamily, float]   # Action verb scores (weight 1.5)
└── context_features: Dict[IntentFamily, float]  # Context keyword scores (weight 1.0)
```

**Methods**: `to_feature_vector() -> List[float]` — fixed-size 15-dim vector (10 family + 5 type dims)

**Deprecated**: `IntentClassification` dataclass with hard family/type/confidence — replaced by soft features.

Source: `core/intent/types.py`, `core/intent/classifier.py`

### §2.6 IntentAnnotation

Governance-tracked intent labels for provenance:

```
IntentAnnotation
├── intent_type: IntentType
├── intent_family: IntentFamily
├── confidence: float                    # [0, 1]
├── source: IntentSource                 # HUMAN_LABEL | DISTILL_MODEL | STERLING_MODEL | HEURISTIC | LLM_RECONSTRUCTION | UNKNOWN
├── schema_version: str = "intent-schema-v0.1"
├── span_start: int = 0
├── span_end: int = 0
├── reasoning: Optional[str]
└── raw_operation: Optional[str]         # Original operation type before canonical mapping
```

**Methods**: `from_operation()` factory, `to_dict()`, `to_snapshot()` (canonical for governance), `from_dict()`.

Source: `core/intent/model.py`

---

## §3 Speech Acts

Speech acts parameterize discourse operators. They describe the illocutionary force of an utterance.

### §3.1 Speech Act Types

| Act | Description | Example |
|-----|-------------|---------|
| assertion | Declarative claim about the world | "Clark Kent is Superman" |
| question | Request for information | "Who is Superman?" |
| command | Directive to perform action | "Normalize the copula" |
| declaration | Performative that changes state | "Let X be defined as Y" |
| request | Polite directive | "Could you expand the contraction?" |
| promise | Commitment to future action | "I will verify this claim" |

### §3.2 Tones

Tone is orthogonal to speech act type:

| Tone | Usage |
|------|-------|
| formal | Academic, technical writing |
| casual | Conversational |
| urgent | Time-sensitive or critical |
| technical | Domain-specific precision |
| friendly | Warm, approachable |
| neutral | Default, unmarked |

Speech acts and tones are used as parameters to discourse operators (SELECT_GOAL_TYPE, CLARIFY, ELABORATE, PROPOSE_PLAN).

Source: `core/worlds/discourse/operators.py`

---

## §4 Dialogue State

### §4.1 DiscourseContext

Tracks the evolving state of a dialogue:

```
DiscourseContext
├── turn_count: int                              # Number of dialogue turns
├── topic_stack: list[str]                       # Active topics (stack order)
├── pending_clarifications: list[str]            # Unresolved clarification requests
├── bound_entities: dict[str, EntityBinding]     # Resolved entity references
└── partial_goal: GoalSpec?                      # In-progress goal construction
```

### §4.2 Dialogue Phases

```
DialoguePhase (str, Enum)
├── OPENING                # Initial greeting/setup
├── INFORMATION_SEEKING    # Gathering information
├── TASK_EXECUTION         # Performing requested task
├── CLARIFICATION          # Resolving ambiguity
├── EVALUATION             # Reviewing/critiquing
└── CLOSING                # Wrapping up
```

Phase transitions are detected by `emit_observations()` and reported as PHASE_TRANSITION observations. The `TRANSITION_PHASE` operator explicitly changes the current phase.

Source: `core/discourse/dialogue.py`

### §4.3 WorldState for Discourse

The discourse adapter builds WorldState with:
- `assumptions["dialogue_episode"]` — current dialogue episode (Strategy B)
- `assumptions["discourse_context"]` — serialized DiscourseContext
- `assumptions["domain"]` — always `"discourse"`
- `assumptions["utterance_count"]` — count of utterances
- Operator applicability determined by pragmatics layer presence

Source: `core/worlds/discourse/world.py`

---

## §5 Entity Binding

### §5.1 BindingStatus

```
BindingStatus (enum)
├── BOUND       # Entity resolved to a concrete reference
├── UNKNOWN     # Entity mentioned but not yet resolved
└── IMPLICIT    # Entity inferred from context (not explicitly mentioned)
```

### §5.2 EntityBinding

```
EntityBinding
├── entity_id: str               # Unique identifier
├── mention_text: str?           # Surface form (debugging only, not for operator use)
├── status: BindingStatus        # Current binding state
├── entity_type: str?            # e.g., "concept", "symbol", "claim"
└── properties: dict[str, Any]   # Domain-agnostic properties
```

**Invariant D-1**: All entities in a GoalSpec must be BOUND or explicitly marked UNKNOWN. No entity may have an implicit, unacknowledged binding state.

### §5.3 Binding Lifecycle

1. Entity mentioned in utterance → created with status UNKNOWN
2. BIND_ENTITIES operator resolves reference → status becomes BOUND
3. Context inference → status becomes IMPLICIT (with explicit marking)
4. CLARIFY operator may request user resolution of UNKNOWN entities

Source: `core/worlds/discourse/types.py`

---

## §6 Success Criteria

```
SuccessCriteria
├── description: str                   # What "success" means (human-readable)
├── predicates: list[str]              # Typed predicate names to check
└── threshold: float                   # Fraction of predicates that must pass (0.0–1.0, default 1.0)
```

```python
def is_satisfiable() -> bool:
    return len(predicates) > 0 or threshold < 1.0
```

**Invariant D-2**: If `success_criteria` is None on a GoalSpec, the `confidence` field must be < 0.5. A goal without success criteria is by definition low-confidence.

Source: `core/worlds/discourse/types.py`

---

## §7 GoalSpec Validation

```python
def validate() -> list[str]:
    violations = []
    # D-1: All entities bound or explicitly UNKNOWN
    for entity in entities:
        if entity.status not in (BindingStatus.BOUND, BindingStatus.UNKNOWN):
            if entity.status == BindingStatus.IMPLICIT and not entity.properties.get("acknowledged"):
                violations.append(f"Entity {entity.entity_id} implicitly bound without acknowledgment")
    # D-2: No success criteria → low confidence
    if success_criteria is None and confidence >= 0.5:
        violations.append("GoalSpec without success_criteria must have confidence < 0.5")
    return violations
```

Source: `core/worlds/discourse/types.py`

---

## §8 Scope and Attribution

### §8.1 Claim Scope

Every claim in Sterling carries explicit scope:

- **Temporal scope**: When the claim is valid (see Claim Schema System)
- **Modal scope**: Under what conditions the claim holds
- **Attribution**: Who asserted the claim (speaker, document, inference)

### §8.2 Speaker Attribution

Claims track their source via `ClaimSource`:

| Source | Meaning |
|--------|---------|
| USER_ASSERTION | Claimed by the user in dialogue |
| ASSISTANT_ASSERTION | Claimed by the system |
| DOCUMENT | Extracted from referenced document |
| INFERENCE | Derived by reasoning |
| EXTERNAL_API | From external knowledge source |
| UNKNOWN | Source not determined |

**Invariant D-3**: No claim may be promoted from UNKNOWN source to a truth status other than UNKNOWN without explicit evidence (see I-MIS-2 in Text Hard IR).

### §8.3 Discourse-Level Scope

The discourse system maintains scope boundaries:

1. **Turn scope**: Claims valid within a single turn
2. **Episode scope**: Claims valid within the dialogue episode
3. **Persistent scope**: Claims carried across episodes (requires explicit promotion)

---

## §9 Forbidden Claim Classes

Sterling enforces strict boundaries on what the discourse system may assert:

### §9.1 Forbidden Operations

1. **No world state override from untrusted claims** (I-MIS-1): The world graph is the home of "what's true." User assertions are tracked as claims with truth status, not injected into world state.

2. **No implicit fact promotion** (C8): Claims are explicit `ClaimNode` objects, never implicit facts in the semantic layer.

3. **No sentence-level misinformation labels** (C9): Misinformation is a relation between a claim and the world model, not a sentence-level property.

4. **No interpretation laundering** (I-FIG-4): Figurative interpretations remain hypotheses — they are never silently promoted to literal truth.

5. **No raw token preconditions** (Discourse Invariant #3): No operator may depend on raw token substrings as a precondition. Operators work on structured representations.

### §9.2 Enforcement

These forbidden classes are checked by:
- Governance gates at operator execution time
- Discourse world adapter's `emit_observations()` and `verify_prediction()`
- Hard IR invariants (I-IMP-1, I-FIG-1 through I-FIG-4, I-MIS-1 through I-MIS-2)

---

## §10 Discourse Operators

### §10.1 Core Discourse Operators

All core discourse operators have category `P` (Pragmatic) and scope `DISCOURSE`, defined in `core/worlds/discourse/operators.py`:

| Operator | Purpose | Reads | Writes | Precondition |
|----------|---------|-------|--------|-------------|
| SELECT_GOAL_TYPE | Choose goal category from intent features | pragmatics | pragmatics | has_pragmatics |
| BIND_ENTITIES | Resolve entity references | semantics, pragmatics | pragmatics | has_semantics |
| SET_SUCCESS_CRITERIA | Define success predicates | pragmatics | pragmatics | has_pragmatics |
| CLARIFY | Request disambiguation | pragmatics | pragmatics | has_pragmatics |
| ELABORATE | Expand on partial goal | pragmatics | pragmatics | has_pragmatics |
| PROPOSE_PLAN | Suggest execution steps | pragmatics | pragmatics | has_pragmatics |
| INFER_INTENT | Infer speech act (no phrase dicts — INV-CORE-04) | pragmatics | pragmatics | has_pragmatics |
| RESOLVE_REFERENCE | Resolve anaphoric/coreference | pragmatics, semantics | pragmatics | has_pragmatics |
| DETECT_TONE | Classify utterance tone (no phrase dicts — INV-CORE-04) | pragmatics | pragmatics | has_pragmatics |

Note: INFER_INTENT and DETECT_TONE have scope `UTTERANCE`, not DISCOURSE. Both enforce INV-CORE-04 (no phrase dictionaries or keyword heuristics).

### §10.2 Multi-Turn Dialogue Operators

Defined in `core/discourse/operators.py`, these handle multi-turn dialogue state:

**Discourse-Structure Operators** (category K — Knowledge):

| Operator | Purpose | Precondition |
|----------|---------|-------------|
| RESOLVE_COREFERENCE | Pronoun/reference resolution with salience scoring | has_semantics, has_unresolved_references |
| LINK_DISCOURSE_RELATION | Link discourse relations between utterances | has_semantics, has_multiple_utterances |
| UPDATE_TOPIC | Update topic state | — |
| MERGE_SEGMENTS | Merge discourse segments | has_multiple_utterances |

**Intent-Aware Dialogue Operators** (H.3):

| Operator | Category | Purpose |
|----------|----------|---------|
| CLASSIFY_INTENT | K | Set intent family/type on dialogue turn |
| SATISFY_INTENT | P | Mark intent as pending/partial/satisfied/abandoned |
| PREDICT_NEXT_INTENT | K | Use intent transition model to predict likely next intents |
| TRANSITION_PHASE | P | Explicitly transition dialogue phase |

### §10.3 Observation Types

The discourse adapter emits observations for operator induction:

| Type | Trigger | Detection |
|------|---------|-----------|
| GOAL_SPEC_CHANGE | Goal count changes | Compare goal lists pre/post |
| INTENT_SATISFACTION | Pending → satisfied | Compare pending intent counts |
| PHASE_TRANSITION | Dialogue phase changes | Compare phase labels |
| OPERATOR_APPLICABLE | Precondition check | Check operator preconditions |

### §10.4 Prediction Verification

The discourse adapter verifies predictions about:
- GOAL_SPEC_CHANGE: Predicted goal count matches actual
- INTENT_SATISFACTION: Predicted satisfaction matches actual
- PHASE_TRANSITION: Predicted phase matches actual
- OPERATOR_APPLICABLE: Predicted applicability matches precondition check

Source: `core/worlds/discourse/operators.py`, `core/worlds/discourse/world.py`, `core/discourse/operators.py`

---

## §10A Discourse Relations

Defined in `core/discourse/state.py`:

```
DiscourseRelationType (str, Enum) — 20 relation types in 3 groups:

Nucleus-Satellite: ELABORATION, EXPLANATION, EVIDENCE, BACKGROUND, CIRCUMSTANCE,
  CAUSE, RESULT, PURPOSE, CONDITION, CONCESSION, CONTRAST

Multi-Nuclear: SEQUENCE, JOINT, DISJUNCTION, LIST

Dialogue-Specific: QUESTION_ANSWER, REQUEST_RESPONSE, CLARIFICATION, CORRECTION,
  ACKNOWLEDGMENT
```

```
CoherenceType (str, Enum): ENTITY_CONTINUITY, TOPIC_CONTINUITY, TEMPORAL_CONTINUITY,
  CAUSAL_CHAIN, PARALLEL, SHIFT
```

```
DiscourseRelation
├── relation_id: str
├── relation_type: DiscourseRelationType
├── source_position: int          # Source utterance index
├── target_position: int          # Target utterance index
├── nuclearity: str = "nucleus-satellite"  # or "satellite-nucleus", "multi-nuclear"
├── confidence: float = 1.0
├── source_span: Optional[Tuple[int, int]]
├── target_span: Optional[Tuple[int, int]]
└── metadata: Dict[str, Any]
```

```
ReferenceChain
├── chain_id: str
├── entity_id: str               # Canonical entity ID
├── mentions: List[Tuple[int, Tuple[int, int], str]]  # (position, span, mention_type)
├── entity_type: Optional[str]   # "person", "organization", etc.
├── salience: float = 0.5        # Entity prominence (0–1)
└── is_focus: bool = False        # Current focus entity
```

Source: `core/discourse/state.py`

---

## §10B Intent Satisfaction FSM

Tracks goal satisfaction using typed evidence from execution outcomes (never surface text heuristics).

```
SatisfactionState (str, Enum)
├── PENDING
├── PARTIALLY_SATISFIED
├── SATISFIED
└── ABANDONED
```

```
TypedEvidence — all evidence from execution outcomes, NOT surface text
├── goal_achieved: bool
├── constraints_satisfied: bool
├── bindings_resolved: bool
├── partial_progress: float       # [0, 1] — % of goal criteria met
├── abandonment_signals: int
├── step_count: int = 0
├── world_name: Optional[str]
└── operators_applied: List[str]
```

**IntentSatisfactionFSM**: Per-goal FSM with `process_execution_result(result) -> SatisfactionState`. Thresholds: PARTIAL_PROGRESS_THRESHOLD=0.3, ABANDONMENT_THRESHOLD=2.

**GoalTracker**: Manages multiple FSMs. Methods: `register_goal()`, `process_result()`, `get_active_goals()`, `get_satisfied_goals()`, `get_satisfaction_rate()`.

**WeakLexicalFeatures** (static, scoring only — never gating): `closing_indicator_score()`, `clarification_request_score()`, `abandonment_indicator_score()`.

Source: `core/discourse/intent_satisfaction.py`

---

## §11 Invariants

1. **D-1**: All entities in GoalSpec must be BOUND or explicitly UNKNOWN.
2. **D-2**: GoalSpec without success_criteria must have confidence < 0.5.
3. **D-3**: Claims with UNKNOWN source cannot be promoted to non-UNKNOWN truth status without evidence.
4. **D-4**: Discourse output (GoalSpec) is domain-agnostic — no domain-specific types.
5. **D-5**: No operator may depend on raw token substrings as precondition.
6. **D-6**: World state is never overridden by untrusted claims (I-MIS-1).
7. **D-7**: Intent features are soft vectors, not hard classifications.

---

## §12 Related Documents

- [World Adapter Protocol](world_adapter_protocol_v1.md) — Base protocol discourse adapter implements
- [Text Hard IR Contract](text_hard_ir_contract_v1.md) — Hard language phenomena (implicature, figurative, claims)
- [Claim Schema System](claim_schema_system_v1.md) — Claim lifecycle and conflict resolution
- [Operator Registry Contract](operator_registry_contract_v1.md) — Where discourse operators are registered

---

## §13 Source File Index

| File | Defines |
|------|---------|
| `core/worlds/discourse/world.py` | DiscourseWorldAdapter |
| `core/worlds/discourse/types.py` | GoalType, GoalSpec, EntityBinding, BindingStatus, SuccessCriteria, WorldEntry, DiscourseContext |
| `core/worlds/discourse/operators.py` | Core discourse operator signatures (9 operators), speech acts, tones |
| `core/intent/types.py` | IntentFamily, IntentType, OperationType, map_operation_to_intent |
| `core/intent/classifier.py` | IntentClassifier, IntentFeatures, get_intent_features() |
| `core/intent/model.py` | IntentSource, IntentAnnotation, StateIntentDistribution |
| `core/discourse/state.py` | DiscourseRelationType, CoherenceType, DiscourseRelation, ReferenceChain, TopicState |
| `core/discourse/dialogue.py` | DialoguePhase, TurnRole, IntentSatisfaction, DialogueTurn, DialogueState, IntentTransition |
| `core/discourse/operators.py` | Multi-turn dialogue operators (RESOLVE_COREFERENCE, CLASSIFY_INTENT, SATISFY_INTENT, etc.) |
| `core/discourse/intent_satisfaction.py` | SatisfactionState, TypedEvidence, ExecutionResult, IntentSatisfactionFSM, GoalTracker |
| `core/text/hard_ir.py` | ClaimNode, ClaimSource, ClaimTruthStatus |

---

## Changelog

### v1.1 (2026-02-17)
- **§2.3–2.6**: Added IntentFamily (10 families), IntentType (40+ types), OperationType operational vocabulary, IntentFeatures dataclass with soft scoring weights, IntentAnnotation with governance provenance
- **§4.2**: Replaced 4 conceptual phases (Opening/Exploration/Convergence/Routing) with actual DialoguePhase enum (6 values: OPENING, INFORMATION_SEEKING, TASK_EXECUTION, CLARIFICATION, EVALUATION, CLOSING)
- **§4.3**: Fixed source path from `discourse/adapter.py` to `discourse/world.py`; added `domain` and `utterance_count` assumptions
- **§10.1**: Added 3 missing core discourse operators: INFER_INTENT, RESOLVE_REFERENCE, DETECT_TONE (with INV-CORE-04 enforcement note)
- **§10.2**: Added multi-turn dialogue operators from `core/discourse/operators.py`: RESOLVE_COREFERENCE, LINK_DISCOURSE_RELATION, UPDATE_TOPIC, MERGE_SEGMENTS, CLASSIFY_INTENT, SATISFY_INTENT, PREDICT_NEXT_INTENT, TRANSITION_PHASE
- **§10A**: Added DiscourseRelationType (20 relation types), CoherenceType, DiscourseRelation, ReferenceChain dataclasses
- **§10B**: Added IntentSatisfactionFSM, SatisfactionState, TypedEvidence, ExecutionResult, GoalTracker
- **§13**: Fixed discourse adapter path, added 6 missing source files
