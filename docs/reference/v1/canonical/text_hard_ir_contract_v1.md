> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Text Hard IR Contract v1.1

**Status**: Canonical specification — sufficient to rebuild `core/text/` from scratch.
**Scope**: Layered Text IR (Surface, Syntax, Semantics), Holes, and Hard Language IR sidecar.
**Layer**: 2 (Text / Linguistics)
**Version**: 1.1 (corrected from 1.0 — missing contracts, factory functions, validation helper, FigurativeContext methods)

---

## §1 Purpose

Sterling's Text IR provides a deterministic, canonical representation for natural language with explicit holes for unsupported phenomena. The representation is layered (Surface → Syntax → Semantics) with a sidecar "Hard IR" for pragmatic phenomena (implicature, figurative language, rhetoric, claims) that cannot be represented in the core semantic layer without corrupting it.

**Design principle**: Mark unsupported phenomena with explicit holes — do not smooth them away (C5).

---

## §2 Layer 1: Surface

Raw text and tokenization.

### §2.1 Span

```
Span
├── start: int     # Inclusive character offset
└── end: int       # Exclusive character offset
```

Methods: `text(source) → str`, `overlaps(other) → bool`

### §2.2 Token

```
Token
├── index: int              # 0-based token index
├── text: str               # Surface form
├── span: Span              # Character span in source
└── whitespace_after: str   # Trailing whitespace (default "")
```

### §2.3 SurfaceLayer

```
SurfaceLayer
├── kind: Literal["Surface"]
├── raw_text: str              # Original input (never modified)
└── tokens: list[Token]
```

**Invariants (I1–I3)**:
- **I1**: Token spans are non-overlapping and cover source text
- **I2**: Token indices are sequential starting at 0
- **I3**: Concatenating tokens + whitespace reconstructs `raw_text`

Source: `core/text/__init__.py`

---

## §3 Layer 2: Syntax

UD-style dependency tree structure.

### §3.1 SyntaxNode

```
SyntaxNode
├── token_idx: int           # Index into SurfaceLayer
├── pos: str                 # UD POS tag
├── lemma: str               # Lemmatized form
├── head_idx: int            # Syntactic head (-1 = root)
├── dep_rel: str             # UD dependency label
└── morph: dict[str, str]    # Morphological features
```

### §3.2 SyntaxLayer

```
SyntaxLayer
├── kind: Literal["Syntax"]
├── nodes: list[SyntaxNode]
├── placeholder: bool                              # True if from placeholder parser
└── phrase_spans: list[tuple[int, int, str]]?      # Constituency (optional)
```

Methods: `get_root()`, `get_dependents(head_idx)`, `get_tree_depth()`, `count_clauses()`

**Invariants (I4–I7)**:
- **I4**: `len(nodes)` equals surface token count
- **I5**: `head_idx` references valid index or -1 for root
- **I6**: Exactly one root node (`head_idx == -1`)
- **I7**: No cycles in dependency graph

Source: `core/text/__init__.py`

---

## §4 Layer 3: Semantics

Minimal event/role graph focused on predicate nominals (PN).

### §4.1 Enums

```
SemanticRole (enum)
├── THEME          # Subject of copular
├── PREDICATIVE    # Predicate nominal
├── AGENT          # For non-copular (future)
└── PATIENT        # For non-copular (future)

EventType (enum)
├── COPULAR        # be-predication
├── STATE          # Generic state
└── UNKNOWN

PNType (enum)
├── IDENTITY       # "Clark Kent is Superman"
├── CHARACTERIZING # "John is a doctor"
├── DEFINING       # "A bachelor is an unmarried man"
└── UNKNOWN

Polarity (enum)
├── POSITIVE
└── NEGATIVE
```

### §4.2 SemanticEntity

```
SemanticEntity
├── id: str                          # Canonical: "{start}:{end}"
├── token_span: tuple[int, int]      # (start_idx, end_idx)
├── lemma: str                       # Head word lemma
├── entity_type: str?                # "person", "org", etc.
└── sense_candidates: list[str]?     # Layer 4 stub
```

### §4.3 SemanticEvent

```
SemanticEvent
├── id: str
├── event_type: EventType
├── predicate_token_idx: int                    # Token index of predicate (not copula)
└── roles: dict[SemanticRole, str]              # role → entity_id
```

### §4.4 SemanticLayer

```
SemanticLayer
├── kind: Literal["Semantic"]
├── entities: list[SemanticEntity]
├── events: list[SemanticEvent]
├── pn_type: PNType                  # Default UNKNOWN
└── polarity: Polarity               # Default POSITIVE
```

**Invariants (I8–I11)**:
- **I8**: All entity IDs referenced in events exist in entities list
- **I9**: Token spans reference valid indices
- **I10**: For copular events, both THEME and PREDICATIVE roles required
- **I11**: Entity IDs are unique and follow canonical format `"{start}:{end}"`

Source: `core/text/__init__.py`

---

## §5 Holes

Explicit gaps where analysis failed or is unsupported.

### §5.1 THoleReason

```
THoleReason (enum)
├── UNSUPPORTED_SYNTAX    # Syntactic structure not handled
├── PARSE_FAILURE         # Parser failed to produce output
├── COMPLEX_NESTING       # Nesting too deep for current analyzer
└── NON_COPULAR           # Non-copular predication (not yet supported)
```

### §5.2 THole

```
THole
├── kind: Literal["Hole"]
├── reason: THoleReason
├── description: str                      # Human-readable explanation
└── token_span: tuple[int, int]?          # None = whole utterance
```

**Semantics**:
- Operators MUST NOT introspect inside holes
- WorldAdapter MAY reject programs containing holes for certain tasks
- Holes are honest — they say "we don't know" rather than fabricating an analysis

Source: `core/text/__init__.py`

---

## §6 TextIR Container

Complete intermediate representation for an utterance:

```
TextIR
├── surface: SurfaceLayer              # Always present
├── syntax: SyntaxLayer?               # Present for parsed utterances
├── semantics: SemanticLayer?          # Present for copular sentences
└── holes: list[THole]                 # Explicit gaps
```

Methods:
- `stage() → str`: Returns `"semantic"`, `"syntactic"`, or `"surface"` based on what layers exist
- `is_valid_for_pn_reasoning() → bool`: v0 contract check
- `validate() → list[str]`: Cross-layer validation
- `has_holes() → bool`
- `get_holes_for_span(start_idx, end_idx) → list[THole]`

### §6.1 v0 Contract for PN Reasoning

- Surface MUST be present
- Syntax MUST be present and valid (unless entire utterance unsupported)
- Semantics MUST be present for simple be-predications; MAY be None otherwise
- Holes MUST cover any spans where semantics intentionally not constructed

Source: `core/text/__init__.py`

---

## §7 Hard Language IR (Sidecar)

The Hard IR captures pragmatic phenomena as parallel annotations without mutating the core SemanticLayer.

**Contract C1**: Literal content lives in SemanticLayer only. Hard IR provides alternative views, not replacements.

### §7.1 Reference Types

```
TokenSpan = tuple[int, int]        # (start_token_idx, end_token_idx)
PropositionId = str                # Reference to proposition in side graph
WorldNodeId = str                  # Reference to KG node or fact ID
UtteranceId = str                  # Reference to utterance
```

---

## §8 Implicature

Conversational implicatures — pragmatic inferences not directly stated.

### §8.1 ImplicatureKind

```
ImplicatureKind (enum)
├── SCALAR          # "some" → "not all"
├── RELEVANCE       # Relevance-based inference
├── QUANTITY        # Quantity maxim inference
├── MANNER          # Manner maxim inference
├── CONVENTIONAL    # Conventional implicature
├── EXHAUSTIVITY    # Exhaustive interpretation
└── OTHER
```

### §8.2 ImplicatureHypothesis

```
ImplicatureHypothesis
├── id: str
├── utterance_id: UtteranceId
├── kind: ImplicatureKind
├── proposition_id: PropositionId?         # Implied proposition (side graph)
├── proposition_text: str?                 # Human-readable
├── trigger_span: TokenSpan?              # Text region triggering implicature
├── based_on_proposition: PropositionId?  # Literal semantics it derives from
├── confidence: float                     # [0, 1]
├── reasoning: str?                       # Explanation
└── source_module: str                    # Default "unknown"
```

**Invariant I-IMP-1**: Must have `trigger_span` OR `based_on_proposition` (at least one).

Source: `core/text/hard_ir.py`

---

## §9 Ellipsis

Marked omissions with reconstruction hypotheses.

### §9.1 EllipsisKind

```
EllipsisKind (enum)
├── ANSWER_ELIDED       # "And then?"
├── SUBJECT_ELIDED      # Subject omitted
├── VERB_ELIDED         # Verb omitted
├── OBJECT_ELIDED       # Object omitted
├── SLUICE              # "I know who"
├── FRAGMENT            # Incomplete utterance
└── OTHER
```

### §9.2 EllipsisAnnotation

```
EllipsisAnnotation
├── id: str
├── utterance_id: UtteranceId
├── span: TokenSpan
├── kind: EllipsisKind
├── reconstructed_proposition_id: PropositionId?
├── reconstructed_text: str?                      # Hypothesis (NOT in SemanticLayer)
├── antecedent_utterance_id: UtteranceId?
├── antecedent_span: TokenSpan?
├── confidence: float
└── source_module: str
```

Source: `core/text/hard_ir.py`

---

## §10 Figurative Language

### §10.1 InterpretationKind

```
InterpretationKind (enum)
├── LITERAL       # Compositional, truth-conditional
├── METAPHORIC    # "A is B" non-literally
├── IDIOMATIC     # Fixed expression, non-compositional
├── MALAPHOR      # Blended/confused idioms
├── HYPERBOLE     # Exaggeration
├── IRONY         # Opposite/orthogonal to literal
├── SARCASM       # Mocking irony
└── NONSENSE      # Compositionally incoherent
```

### §10.2 Interpretation

```
Interpretation
├── id: str
├── utterance_id: UtteranceId
├── kind: InterpretationKind
├── span: TokenSpan
├── literal_proposition_ids: list[PropositionId]    # Propositions it re-reads
├── semantic_proposition_ids: list[PropositionId]   # Side semantic graph
├── paraphrase: str?                                # Alternative phrasing
├── source_domain: str?                             # For METAPHORIC: "journey", "war"
├── target_domain: str?                             # For METAPHORIC: "life", "argument"
├── idiom_id: str?                                  # For IDIOMATIC
├── idiom_canonical_form: str?                      # e.g., "cross that bridge"
├── confidence: float
├── reasoning: str?
└── source_module: str
```

### §10.3 FigurativeContext

```
FigurativeContext
├── utterance_id: UtteranceId
├── literal_interpretation_id: str?         # At most one designated literal
├── literal_is_coherent: bool = True        # False for malaphors, nonsense
├── interpretations: list[Interpretation]
└── active_interpretation_id: str?          # Current active reading
```

**Methods**: `get_active() -> Optional[Interpretation]`, `get_by_kind(kind) -> List[Interpretation]`

**Invariants (I-FIG)**:
- **I-FIG-1**: At most one designated literal interpretation
- **I-FIG-2**: Value head must know which interpretation it uses
- **I-FIG-3**: Nonsense gets NONSENSE tag, not fake coherence
- **I-FIG-4**: No interpretation laundering — figurative readings never silently promoted to literal truth

Source: `core/text/hard_ir.py`

---

## §11 Rhetoric and Argumentation

### §11.1 RhetoricalFunction

```
RhetoricalFunction (enum)
├── CLAIM        # Main assertion
├── EVIDENCE     # Support for claim
├── WARRANT      # Reasoning connecting evidence to claim
├── BACKING      # Support for warrant
├── QUALIFIER    # Hedging, limiting scope
├── REBUTTAL     # Anticipating counterarguments
├── ATTACK       # Challenging claim
├── SUPPORT      # Endorsing claim
├── RIDICULE     # Mocking
├── HEDGE        # Weakening commitment
├── QUESTION     # Genuine or rhetorical
├── META         # About discourse itself
└── OTHER
```

### §11.2 RhetoricalAnnotation

```
RhetoricalAnnotation
├── id: str
├── utterance_id: UtteranceId
├── function: RhetoricalFunction
├── trigger_span: TokenSpan?                      # Rhetorical cues
├── local_proposition_ids: list[PropositionId]    # Propositions in this utterance
├── target_proposition_ids: list[PropositionId]   # Prior propositions targeted
├── target_utterance_ids: list[UtteranceId]
├── confidence: float
└── source_module: str
```

**Invariant I-RHE-2**: Must have at least one of: `local_proposition_ids`, `target_proposition_ids`, `target_utterance_ids`, or `trigger_span`.

### §11.3 ArgumentRelationKind

```
ArgumentRelationKind (enum)
├── SUPPORTS      # Provides evidence for
├── ATTACKS       # Contradicts or undermines
├── UNDERCUTS     # Weakens evidence-claim link
├── REBUTS        # Direct contradiction
├── CONCEDES      # Acknowledges while maintaining position
├── REPHRASES     # Restates in different terms
└── QUESTIONS     # Challenges without asserting contrary
```

### §11.4 ArgumentRelation

```
ArgumentRelation
├── id: str
├── kind: ArgumentRelationKind
├── source_utterance_id: UtteranceId
├── source_proposition_ids: list[PropositionId]    # Non-empty
├── target_utterance_id: UtteranceId
├── target_proposition_ids: list[PropositionId]    # Non-empty
├── confidence: float
└── source_module: str
```

Source: `core/text/hard_ir.py`

---

## §12 Claims and Misinformation

### §12.1 ClaimTruthStatus

```
ClaimTruthStatus (enum)
├── UNKNOWN         # Not yet checked
├── CONSISTENT      # Aligns with world model
├── INCONSISTENT    # Conflicts with world model
├── DISPUTED        # Multiple conflicting sources
├── UNSUPPORTED     # No evidence either way
└── UNVERIFIABLE    # Cannot be checked
```

### §12.2 ClaimSource

```
ClaimSource (enum)
├── USER_ASSERTION
├── ASSISTANT_ASSERTION
├── DOCUMENT
├── INFERENCE
├── EXTERNAL_API
└── UNKNOWN
```

### §12.3 WorldEvidenceRef

```
WorldEvidenceRef
├── id: str
├── world_node_id: WorldNodeId     # KG node or fact ID
├── support: float                 # [-1.0, 1.0]: positive=supports, negative=contradicts
└── fact_text: str?
```

### §12.4 ClaimNode

```
ClaimNode
├── id: str
├── utterance_id: UtteranceId
├── proposition_id: PropositionId              # Link to literal semantics
├── proposition_text: str?
├── source: ClaimSource
├── speaker_id: str?
├── span: TokenSpan?
├── truth_status: ClaimTruthStatus             # Default UNKNOWN
├── checked_against_world_version: str?        # Required if status ≠ UNKNOWN (I-MIS-2)
├── evidence: list[WorldEvidenceRef]
├── assessment_method: str?                    # "rule_check", "kg_lookup", "human_label"
├── assessment_confidence: float               # Default 0.0
└── assessment_timestamp: str?
```

**Invariants (I-MIS)**:
- **I-MIS-1**: World graph never overwritten by untrusted claim
- **I-MIS-2**: Truth status is relative to world model — `checked_against_world_version` required if status ≠ UNKNOWN

**Contracts**:
- **C3**: WorldState is home of "what's true" — claims are separate
- **C8**: Claims are explicit ClaimNodes, not implicit facts
- **C9**: Misinformation is a relation between claim and world model, not a sentence-level label

Source: `core/text/hard_ir.py`

---

## §13 HardLanguageIR Container

```
HardLanguageIR
├── utterance_id: UtteranceId
│
│  ── Implicature and Omissions ──
├── implicatures: list[ImplicatureHypothesis]
├── ellipsis: list[EllipsisAnnotation]
│
│  ── Figurative Language ──
├── figurative: FigurativeContext?
├── interpretations: list[Interpretation]
│
│  ── Rhetoric and Argumentation ──
├── rhetoric: list[RhetoricalAnnotation]
├── argument_relations_out: list[ArgumentRelation]
│
│  ── Claims and Truth Status ──
├── claims: list[ClaimNode]
│
│  ── Metadata ──
└── version: str                       # "hard-ir-v0.1"
```

Convenience properties: `has_implicatures`, `has_ellipsis`, `has_figurative_language`, `has_misinformation_risk`, `has_unverified_claims`.

Query methods: `get_claims_by_status(status)`, `get_interpretations_by_kind(kind)`, `get_rhetoric_by_function(function)`.

**Contract**: Never mutates core SemanticLayer. Provides parallel/alternative views, not replacements.

Source: `core/text/hard_ir.py`

### §13.1 Validation Helper

```python
def validate_hard_ir_against_text_ir(hard_ir: HardLanguageIR, text_ir: TextIR) -> list[str]
```

Cross-layer validation: checks all spans within SurfaceLayer bounds, all proposition_id references exist in SemanticLayer.events, implicature/interpretation/rhetoric/claim proposition references valid. Returns list of errors (empty if valid).

### §13.2 Factory Functions

```python
def create_scalar_implicature(utterance_id, trigger_span, literal_prop_id,
                              implied_text, confidence=0.8) -> ImplicatureHypothesis

def create_metaphor_interpretation(utterance_id, span, literal_prop_ids,
                                   paraphrase, source_domain, target_domain,
                                   confidence=0.7) -> Interpretation

def create_claim_node(utterance_id, proposition_id, proposition_text,
                      source=ClaimSource.USER_ASSERTION, speaker_id=None) -> ClaimNode
```

Source: `core/text/hard_ir.py`

---

## §14 Linguistic Contracts Summary

| ID | Contract |
|----|----------|
| C1 | Literal content in SemanticLayer only |
| C3 | WorldState is home of "what's true" — claims are separate |
| C5 | Mark unsupported phenomena, don't smooth them |
| C6 | Figurative readings are hypotheses, not replacements |
| C7 | Implicatures never silently folded into semantics |
| C8 | Claims are explicit ClaimNodes, not implicit facts |
| C9 | Misinformation is a relation, not sentence-level label |
| C12 | Hypotheses carry provenance and uncertainty |
| I-IMP-1 | Implicatures have trigger_span OR based_on_proposition |
| I-IMP-3 | Omissions are hypotheses, not filled semantics |
| I-FIG-1 | At most one designated literal interpretation |
| I-FIG-2 | Value head must know which interpretation it uses |
| I-FIG-3 | Nonsense gets NONSENSE tag, not fake coherence |
| I-FIG-4 | No interpretation laundering |
| I-RHE-1 | Rhetorical function labels never change truth-conditions |
| I-RHE-2 | Rhetorical annotations have local propositions OR target refs OR trigger_span |
| I-MIS-1 | World graph never overwritten by untrusted claim |
| I-MIS-2 | Truth status relative to world model version |

---

## §15 Determinism Contracts

- **DET-1A**: Canonical dict representations exclude volatile fields (ids, timestamps)
- **DET-3**: SHA-256 of canonical JSON for fingerprinting (not `str(dict)`)
- All hashes deterministic via `json.dumps(sort_keys=True)`
- Evidence fingerprints computed via `compute_evidence_fingerprint()` for deduplication

---

## §16 Related Documents

- [World Adapter Protocol](world_adapter_protocol_v1.md) — How TextIR enters the reasoning pipeline
- [Discourse Intent Contract](discourse_intent_contract_v1.md) — Discourse structure built on top of TextIR
- [Semantic Realization Convergence](semantic_realization_convergence.md) — How IR converges to text output
- [Claim Schema System](claim_schema_system_v1.md) — Claim lifecycle in the reasoning system

---

## §17 Source File Index

| File | Defines |
|------|---------|
| `core/text/__init__.py` | TextIR, SurfaceLayer, SyntaxLayer, SemanticLayer, Token, Span, SyntaxNode, SemanticEntity, SemanticEvent, THole, THoleReason, SemanticRole, EventType, PNType, Polarity |
| `core/text/hard_ir.py` | HardLanguageIR, ImplicatureKind, ImplicatureHypothesis, EllipsisKind, EllipsisAnnotation, InterpretationKind, Interpretation, FigurativeContext, RhetoricalFunction, RhetoricalAnnotation, ArgumentRelationKind, ArgumentRelation, ClaimTruthStatus, ClaimSource, WorldEvidenceRef, ClaimNode, validate_hard_ir_against_text_ir, create_scalar_implicature, create_metaphor_interpretation, create_claim_node |
| `core/text/pipeline.py` | SterlingTextPipeline, TextPipelineResult, PipelineStageStatus, PipelineFailure |

---

## Changelog

### v1.1 (2026-02-17)
- **§10.3**: Added FigurativeContext methods (`get_active()`, `get_by_kind()`)
- **§11.2**: Fixed I-RHE-2 — also accepts `trigger_span` as alternative to propositions/targets
- **§13.1**: Added `validate_hard_ir_against_text_ir()` cross-layer validation helper
- **§13.2**: Added factory functions: `create_scalar_implicature()`, `create_metaphor_interpretation()`, `create_claim_node()`
- **§14**: Added missing contracts to summary table: C3, C7, C12, I-IMP-3, I-RHE-1
- **§17**: Updated hard_ir.py defines to include validation/factory functions
