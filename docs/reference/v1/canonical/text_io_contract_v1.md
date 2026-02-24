> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Text I/O Contract v1

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

---

## Core Principle

**Text is surface; IR is authority.**

Text intake and realization are I/O boundaries where neural processing may occur, but all reasoning happens on explicit IR + KG structures. Generated text is non-authoritative and cannot mutate semantic state.

## Trust Boundary Definition

Sterling's text I/O operates under a strict trust boundary that separates authoritative symbolic processing from advisory neural processing.

### Authoritative (Symbolic)

These components make binding decisions:

| Component | Authority | Artifact |
|-----------|-----------|----------|
| Semantic IR extraction | Binding | SemanticLayer from SyntaxLayer |
| Operator precondition/effect checking | Binding | InvariantResult |
| StateGraph path selection | Binding | SearchNode sequence |
| Semantic delta generation | Binding | SemanticDeltaIRv0 |
| Hole policy enforcement | Binding | HolesPolicyResult |

### Advisory (Neural)

These components inform but do not decide:

| Component | Authority | Artifact |
|-----------|-----------|----------|
| Raw text → tokens | Advisory | SurfaceLayer |
| Tokens → syntax (UD parsing) | Advisory | SyntaxLayer |
| Token → sense ID (disambiguation hints) | Advisory | Semiotic mappings |
| IR → surface text (realization) | Advisory | RealizationResult |
| Value function scoring | Advisory | Score (float) |

## Key Invariants

### INV-TIO-01: Parser Trust Boundary

Parser selection is an offline decision, not a runtime routing choice:

- **Parser class** determines admissibility (NORMALIZER_ONLY, STRUCTURE_ONLY)
- **Parser identity** is recorded in provenance (ParserProvenance with digest verification)
- **Runtime cannot** change parser mid-episode
- **No "try parser A, fallback to B"** — explicit fail-closed on parser unavailability

This prevents covert routing via "which parser for this input?"

### INV-TIO-02: Hole Semantics

All gaps must be explicit:

| Situation | Result | Never |
|-----------|--------|-------|
| Unsupported syntax | `THole(UNSUPPORTED_SYNTAX)` | Silent fake semantics |
| Parse failures | `THole(PARSE_FAILURE)` | Hallucinated structure |
| Complex nesting | `THole(COMPLEX_NESTING)` | Best-effort semantics |
| Unknown construction | `THole(UNKNOWN)` | Proceed without marking |

**Hole policy** gates search readiness: if holes exceed policy threshold, search must not proceed.

### INV-TIO-03: Digest Visibility Split

Digests exist for audit, not observation:

- **Observation plane** (`to_observation(policy)`): No sha256 strings unless policy explicitly allows
- **Audit plane** (`to_audit()`): Full digests, provenance, witnesses
- **Never**: Single serializer that "sometimes includes digests"

This prevents digest-based routing and agent learning hash patterns.

### INV-TIO-04: Intake Drift Prevention

Text intake cannot become decision-making:

- No "parse differently based on task"
- No "semantic extraction based on expected outcome"
- No "adjust syntax based on downstream needs"

**Parsing is observation, not inference.** The same input text must produce the same TextIntakeIRv1 regardless of task context.

### INV-TIO-05: Realization is Non-Authoritative

Generated text cannot mutate semantic state without re-intake:

- Realization output is a **rendering** of IR, not a new source of truth
- Any text that should affect state must go through `IntakePipeline.parse()`
- Explanation text is descriptive, not prescriptive

**Violation pattern to prevent**: "Generate explanation, extract claims from explanation, use claims as new evidence."

## IR Layer Types

Source: `core/text/ir.py`

### SurfaceLayer

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `"Surface"` | Layer discriminator |
| `raw_text` | str | Original text |
| `tokens` | list[Token] | Tokenized surface |

**Token**: `index`, `text`, `span` (Span), `whitespace_after=""`, `lemma?`, `pos?`

**Span**: `start` (inclusive), `end` (exclusive)

**Surface invariants (I1-I3)**: Non-overlapping token spans, sequential indices, whitespace reconstruction.

### SyntaxLayer

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `"Syntax"` | Layer discriminator |
| `nodes` | list[SyntaxNode] | Dependency nodes |
| `placeholder` | bool | True if placeholder parser used |
| `phrase_spans` | list[tuple]? | Optional phrase structure |

**SyntaxNode**: `token_idx`, `pos` (UD POS tag), `lemma`, `head_idx` (-1 for root), `dep_rel` (UD label), `morph={}` (morphological features)

**Syntax invariants (I4-I7)**: Node count matches tokens, valid head references, exactly one root, no cycles.

### SemanticLayer

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `"Semantic"` | Layer discriminator |
| `entities` | list[SemanticEntity] | Entities in graph |
| `events` | list[SemanticEvent] | Events/states in graph |
| `pn_type` | PNType | PN classification |
| `polarity` | Polarity | Semantic polarity |

**SemanticEntity**: `id` ("{start}:{end}"), `token_span`, `lemma`, `entity_type?`, `sense_candidates?`

**SemanticEvent**: `id`, `event_type` (EventType), `predicate_token_idx`, `roles` (dict[SemanticRole, str])

**Enums**: `EventType` (COPULAR, STATE, UNKNOWN), `SemanticRole` (THEME, PREDICATIVE, AGENT, PATIENT), `PNType` (IDENTITY, CHARACTERIZING, DEFINING, UNKNOWN), `Polarity` (POSITIVE, NEGATIVE)

**Semantic invariants (I8-I11)**: Entity IDs in events exist, valid token spans, copular role completeness, unique canonical IDs.

### THole

Explicit holes in analysis:

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `"Hole"` | Discriminator |
| `reason` | THoleReason | Why analysis failed |
| `description` | str | Human-readable detail |
| `token_span` | tuple? | Affected span (None = entire utterance) |

**THoleReason** (enum): `UNSUPPORTED_SYNTAX`, `PARSE_FAILURE`, `COMPLEX_NESTING`, `NON_COPULAR`

### TextIR Container

`TextIR` combines all layers: `surface`, `syntax?`, `semantics?`, `holes`. Methods: `stage()` → "semantic"/"syntactic"/"surface", `is_valid_for_pn_reasoning()`, `validate()`.

## TextIntakeIRv1

Source: `core/text/intake_ir.py`

**Schema**: `sterling.text_intake_ir.v1` (version `1.0.0`)

Versioned, content-addressed, provenance-stamped intake artifact.

| Field | Type | Description |
|-------|------|-------------|
| `schema_id` | str | `"sterling.text_intake_ir.v1"` |
| `schema_version` | str | `"1.0.0"` |
| `raw_text` | str | Surface text (immutable) |
| `surface_digest` | str | SHA256 of surface (audit-plane only) |
| `syntax` | SyntaxLayer? | Derived syntax layer |
| `syntax_digest` | str | SHA256 of syntax (audit-plane only) |
| `semantics` | SemanticLayer? | Derived semantic layer |
| `semantics_digest` | str | SHA256 of semantics (audit-plane only) |
| `holes` | list[THole] | Explicit unsupported regions |
| `holes_digest` | str | SHA256 of holes (audit-plane only) |
| `holes_policy_result` | HolesPolicyResult? | Policy validation result |
| `parser_provenance` | ParserProvenance? | Parser identity and digests |
| `canonicalization_version` | str | `"canon/v1"` |
| `ir_digest` | str | Composite digest of all layers |

**Intake invariants (TI-1 through TI-8)**: Surface immutable, syntax derived from parser, semantics derived from syntax, stable digests, explicit holes, parser class gates admissibility, digests audit-plane only, holes policy enforced.

### ParserProvenance

| Field | Type | Description |
|-------|------|-------------|
| `parser_id` | str | e.g., `"spacy:en_core_web_sm:3.7.0"` |
| `parser_class` | ParserClass | NORMALIZER_ONLY, STRUCTURE_ONLY, SEMANTIC_PROVIDER |
| `parser_binary_digest` | str? | SHA256 of wheel/container |
| `parser_model_digest` | str? | SHA256 of model package |
| `parser_config_digest` | str? | SHA256 of pipeline config |
| `single_threaded` | bool | Default True |
| `python_hash_seed` | int? | PYTHONHASHSEED (default 0) |
| `gpu_disabled` | bool | Default True |

## Data Flow Contract

### Intake Pipeline (Text → IR)

```
Raw Text
  ↓ (TextParser.parse)
SurfaceLayer + SyntaxLayer
  ↓ (SemanticExtractor.extract)
SemanticLayer + holes
  ↓ (TextIntakeIRv1.create)
TextIntakeIRv1 (versioned, content-addressed, provenance-stamped)
  ↓ (to_utterance_state)
UtteranceState (search-ready)
```

### Realization Pipeline (IR → Text)

```
SemanticLayer
  ↓ (TemplateRealizer.realize or DiffusionRealizer.realize)
RealizationResult
  ↓ (verify if round-trip enabled)
Verification (re-parse → compare semantics)
  ↓
Surface Text (non-authoritative)
```

## Acceptance Tests

### Digest Visibility

- **T-TIO-01**: `to_observation(policy)` contains no sha256 strings when `expose_digests_to_agent=False`
- **T-TIO-02**: `to_audit()` always includes full digests and provenance
- **T-TIO-03**: Guard check count increments (guard is not dead code)

### Parser Trust

- **T-TIO-04**: STRUCTURE_ONLY produces SyntaxLayer for supported inputs
- **T-TIO-05**: Missing configured parser fails closed in certifying mode
- **T-TIO-06**: Parser provenance records parser_id actually used

### Hole Semantics

- **T-TIO-07**: Unsupported constructions emit THole with reason code
- **T-TIO-08**: Hole policy failure blocks search readiness
- **T-TIO-09**: No silent "None" for missing semantics

### Realization Contract

- **T-TIO-10**: Realization is deterministic (same input → same output)
- **T-TIO-11**: Round-trip verification passes for supported PN patterns
- **T-TIO-12**: Unsupported semantics return failure, not "best effort" text

## Realizer Contract (RZ)

### RZ-0: No Hidden Router

Realizer selection is explicit via `RealizerId`. No fallback chains like "try diffusion, then template."

### RZ-1: Deterministic in Certifying Mode

In `RunIntent.CERTIFYING`:
- Same input artifacts → identical text output
- No timestamps, UUIDs, randomized synonyms, or reorderings

### RZ-2: Observation vs Audit Split

- `RealizationResult.to_observation_dict(policy)` excludes digests unless policy allows
- `RealizationResult.to_audit_dict()` includes full witness and digests

### RZ-3: Semantic Fidelity (v0 Template Policy)

For supported patterns, realized text must be round-trip stable:
1. `realize(sem, surface)` → text
2. `parse(text)` → surface2, syntax2
3. `extract_semantics(surface2, syntax2)` → sem2
4. `sem2` equivalent to `sem` (digest match for PN v0)

### RZ-4: Fail Closed on Unsupported Semantics

Unsupported event types or holes beyond policy → return failure, not partial text.

### RZ-5: MUST_INCLUDE Slot Semantics

Template realization uses MUST_INCLUDE slots (subject/predicate/polarity) compatible with future diffusion mask model.

### RZ-7: Narration Coverage

If trace has N steps, explanation has N ordered step descriptions (no drop/reorder) and is deterministic.

## Enforcement

This contract is enforced through:

1. **Code architecture**: Intake and realization are separate pipelines with typed interfaces
2. **Guard classes**: `DigestVisibilityGuard` at observation boundaries
3. **Policy objects**: `DigestVisibilityPolicy`, `HolesPolicy` control behavior
4. **Schema versioning**: TextIntakeIRv1, SemanticDeltaIRv0 with explicit versions
5. **Testing**: Invariant tests in `tests/unit/test_text_*` and `tests/integration/test_text_*`

## Relationship to Other Contracts

- **Neural Usage Contract**: Text I/O is a permitted neural use case at I/O boundaries
- **Core Constraints v1**: Meaning in IR+KG, not text (constraint 2)
- **Evaluation Gates v1**: Certifying mode requires fail-closed intake behavior

## TextToSearch Pipeline

Source: `core/text/text_to_search.py`

End-to-end orchestrator: text intake → IR conversion → digest visibility enforcement → certifying governance → search readiness.

`TextToSearchResult`: `utterance_state`, `intake_ir`, `parser_attestation?`, `holes_policy_result?`, `certifying_validation?`, `processing_timestamp`. Methods: `is_search_ready()`, `get_violations()`, `to_observation_dict(policy)`, `to_audit_dict()`.

## Trace Narration

Source: `core/text/trace_narrator.py`

`TraceNarrator` converts StateGraph reasoning traces to natural language explanations. Output is `ExplanationResult`: `narrative`, `provenance`, `operators_applied`, `semantic_deltas`, `holes`, `metadata`. Non-authoritative artifact (INV-TIO-05).

## Source File Index

| File | Defines |
|------|---------|
| `core/text/ir.py` | SurfaceLayer, SyntaxLayer, SemanticLayer, TextIR, Token, Span, SyntaxNode, SemanticEntity, SemanticEvent, THole, THoleReason, enums |
| `core/text/intake_ir.py` | TextIntakeIRv1, ParserProvenance, ParserClass, digest computation, observation/audit serialization |
| `core/text/parser.py` | TextParser protocol, SpacyTextParser, PlaceholderTextParser, ParseResult, get_text_parser() |
| `core/text/semantics.py` | SemanticExtractor (copular extraction, polarity detection, PN classification) |
| `core/text/realizer.py` | TemplateRealizer, RealizationResult |
| `core/text/pipeline.py` | SterlingTextPipeline (intake orchestration) |
| `core/text/hard_ir.py` | HardLanguageIR, ImplicatureHypothesis, FigurativeContext, RhetoricalAnnotation, ClaimNode |
| `core/text/text_to_search.py` | TextToSearchPipeline, TextToSearchResult |
| `core/text/trace_narrator.py` | TraceNarrator, ExplanationResult |

## Version History

| Version | Date | Changes |
|---------|------|---------|
| v1.0 | 2026-01 | Initial canonicalization |
| v1.1 | 2026-02 | Added IR layer types (SurfaceLayer, SyntaxLayer, SemanticLayer, THole), TextIntakeIRv1 fields, ParserProvenance, layer invariants I1-I11, TI-1 through TI-8, TextToSearch pipeline, TraceNarration, source file index (9 files) |
