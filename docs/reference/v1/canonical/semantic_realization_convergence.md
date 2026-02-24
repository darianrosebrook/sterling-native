> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Semantic Realization Convergence Design

**Status**: Design Draft
**Date**: 2026-02-17
**Author**: @darianrosebrook
**Depends on**: sterling_architecture_layers.md, MS-01-claim-schema.md, diffusion_realization_v10.md, semantic_working_memory_contract_v0.md, code32_bytestate.md
**Scope**: How Sterling's eight subsystems converge into a unified pipeline for KG-grounded, governance-verified text generation

---

## 0. The Core Thesis

Writing a sentence is like writing an email: first you decide what you mean to say, then you decide how to say it. The meaning doesn't change when you switch from formal to casual tone.

Sterling's novel contribution is that the entire semantic ledger — what we mean to say — exists as a content-addressed, governance-verified artifact *before generation starts*. The output generator determines *how* to say it, but the meaning is traceable back through the reasoning chain to the original semantic commitment. If the output drifts from the commitment, that drift is detectable, auditable, and refusable.

This is not an LLM augmentation. It is a replacement of the generative paradigm: meaning drives expression, not the reverse.

---

## 1. What Converges

Eight subsystems, developed independently, now share enough infrastructure to compose into a single pipeline. This document defines the composition.

| # | Subsystem | Layer | What It Provides |
|---|-----------|-------|------------------|
| 1 | **Intake Pipeline** | L0 input | Deterministic text-to-IR with content-addressed digests |
| 2 | **Reasoning Substrate** | L0 | Governed search, operator calculus, StateGraph |
| 3 | **Semantic Working Memory** | L1 | Lifecycle states, MeaningStateDigest chains, EpisodeTrace |
| 4 | **Claim Schema System** | L1 governance | Typed, schema-validated, evidence-bound semantic claims |
| 5 | **Knowledge Graphs** | L1 grounding | WordNet/Wikidata as landmark worlds for lexical + factual grounding |
| 6 | **ByteState Carrier** | L2 | Code32 atoms, two-plane packed encoding, sub-ms operations |
| 7 | **Diffusion Realization** | L3 | Semantic-conditioned denoising, MaskIntent constraints, validation gates |
| 8 | **Realization Spec** | L1→L3 bridge | MaskSlot/MaskIntent schema, RealizerId, style conditioning |

The dependency chain is strict: **L0 → L1 → L2 → L3**. Layer 3 cannot succeed until Layers 0-2 are proven. Knowledge graphs and claim schemas are transverse concerns that inform L1 but are consumed at every layer.

---

## 2. The Pipeline (End-to-End)

```
Input (text, game state, structured query) + RealizationTarget
    │
    │  RealizationTarget determines output mode:
    │    PlainText (default) — natural language surface
    │    EmailReply — tone + structure policy over same committed claims
    │    JSONLD — structured emitter with QIDs/PIDs and evidence refs
    │    ClaimTable — rows: {claim_id, value, evidence_refs, status}
    │    CitationGraph — claim dependency graph with evidence links
    │
    ▼
┌─────────────────────────────────────────────────┐
│ Phase 1: Intake                                  │
│   Text → IntakePipeline.parse()                  │
│   → sentence_record (IR, utterance, digests)     │
│   Content-addressed: surface, syntax, semantics, │
│   holes, ir digests per sentence                 │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 2: Semantic Commitment                     │
│   IR → SemanticCommitIR                          │
│   Atoms: entity, event, attribute, polarity,     │
│          pn_type, extracted via IR analysis       │
│   commit_id = SHA-256(canonical(atoms +           │
│     discourse_contract + attribution_contract +   │
│     scope_contract + forbidden_claim_classes +    │
│     parser_policy_digest))                        │
│   This is the commit intent — the first immutable │
│   meaning artifact. Downstream grounding may      │
│   attach evidence but must not mutate the commit. │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 3: KG Grounding (when applicable)          │
│   Atoms → LabelSpace projection → KG query       │
│   WordNet: lexical grounding (synset anchors,    │
│     hypernym paths, sense disambiguation)         │
│   Wikidata: factual grounding (entity IDs,       │
│     property verification, relation anchors)      │
│   Produces GroundedCommitBundle:                  │
│     SemanticCommitIR (unchanged)                  │
│     KGSnapshotRefs (pinned; no live queries       │
│       in cert mode)                               │
│     KGQueryPlanIR (content-addressed query plan   │
│       derived from the commit)                    │
│     EvidenceBundle (results + absence witnesses)  │
│     ClaimQualification per claim:                 │
│       asserted | hypothesis | contradicted        │
│   Bundle digest = the meaning ledger boundary     │
│   consumed by realization and verification.       │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 4: SWM Encoding                            │
│   GroundedCommitBundle → ByteStateV1             │
│   Code32 atoms on identity plane                 │
│   SlotStatus bytes on status plane               │
│   Operator application: SET_POLARITY,            │
│     SET_PN_TYPE, ADD_EDGE, ACTIVATE_ENTITY       │
│   ByteTrace provenance for each operator step    │
│   MeaningStateDigest chain updated per operation │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 5: Realization Spec Construction           │
│   GroundedCommitBundle + ElaborationPolicyIR     │
│   + RealizationTarget → CoreRealizationSpec      │
│   Asserted claims → MUST_INCLUDE slots           │
│   Hypotheses → SHOULD_INCLUDE slots              │
│   Policy vocabulary → MAY_INCLUDE slots          │
│   (multi-word phrases split to individual words) │
│   spec_digest = compute_digest() for             │
│   determinism verification                       │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 6: Constrained Realization                 │
│   CoreRealizationSpec → Realizer                 │
│   (currently: rule-based simulator;              │
│    target: discrete diffusion LM)                │
│   MUST_INCLUDE tokens pinned (semantic anchors)  │
│   Iterative denoising toward surface text        │
│   Multiple seeds → multiple valid realizations   │
│   under same semantic commitment                 │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 7: Validation Gates                        │
│   Gate A: Commit stability (same input →         │
│           same digest)                           │
│   Gate B: Coverage (all required atoms realized) │
│   Gate C0: Lexical leakage (word-level novelty   │
│            tripwire against known vocabulary)      │
│   Gate C1: Claim leakage (reparse → ClaimIR diff;│
│            no new claims beyond commit + policy)  │
│   Gate D: Calibrated refusal (typed refusal      │
│           when coverage/leakage fails)            │
│   Each gate produces typed witness artifact      │
│   Fail-closed: no output without all gates pass  │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 8: Post-Realization Verification           │
│   Reparse surface text → IR                      │
│   IR diff against source commit:                 │
│     entity set, event graph, quantities,         │
│     attribution, discourse relations, coreference│
│   Bidirectional entailment (cautious)            │
│   Quotation/citation invariants                  │
│   Drift detectors: new entity, lost hedge,       │
│     scope shift, modality change                 │
│   Failed verification → refusal or retry         │
│   (max attempts, then refuse with typed reason)  │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│ Phase 9: Certified Sidecar Packaging             │
│   (grounded_bundle_digest, policy_digest,        │
│    render_digest, witness_bundle_digest,          │
│    kg_snapshot_refs, evidence_bundle_digest,      │
│    bytestate_identity_hash, bytetrace_payload)   │
│   Every realization is traceable back to its     │
│   semantic commitment, KG evidence, policy,      │
│   and gate witnesses                             │
└─────────────────────────────────────────────────┘
```

---

## 3. The Six Architectural Seams

These are the integration points where subsystems must agree on contracts, types, and invariants. Getting these wrong breaks the pipeline silently.

### Seam 1: Intake → L0 (Text → Semantic IR)

**Contract**: Content-addressed IR with parser provenance.

Every text input produces a `sentence_record` with five canonical digests: `surface_digest`, `syntax_digest`, `semantics_digest`, `holes_digest`, `ir_digest`. The IR is deterministic given the same parser version and input. Holes are explicit (defeasible sense assignments marked, not guessed).

**Key files**: `core/linguistics/ir_v0/types.py`, `core/domains/language_io_domain.py`

**Risk**: Parser non-determinism or implicit hole resolution. Mitigated by content-addressing every layer independently.

### Seam 2: L0 → L1 (Reasoning → Memory)

**Contract**: Deterministic MeaningStateDigest chains.

Layer 0 produces `OperatorEdge + StateNode` snapshots. Layer 1 wraps these into `EpisodeTrace` (operator_sequence, state_in, state_out, patch, witness). Identical operator sequence + registry → identical MeaningStateDigest chain. No runtime mutation of committed state.

**Key files**: `core/linguistics/ir_v0/meaning_state.py`, `docs/canonical/semantic_working_memory_contract_v0.md`

**Risk**: OperatorEdge provenance loss. Every operator must declare what invariants it preserves/breaks. EpisodeTrace captures the full sequence, not just the final state.

### Seam 3: L1 → L2 (Memory → Carrier)

**Contract**: Epoch-frozen compilation, fail-closed on mismatch.

SWM artifacts compile through a governed codec into ByteStateV1. The compilation boundary is a pure function: `compile(payload, schema_descriptor, registry_snapshot) → ByteState`. Decompilation preserves domain meaning (declared equivalence per schema). Schema/registry/concept/constraint violations cause fail-closed rejection, not silent corruption.

**Key files**: `core/carrier/bytestate.py`, `core/carrier/code32.py`, `docs/reference/canonical/bytestate_compilation_boundary.md`

**Risk**: Schema version mismatch across epochs. Must ensure promotion pipeline produces versioned artifacts before new epoch activates.

### Seam 4: L2 → L3 (Carrier → Realization)

**Contract**: Only declared artifacts cross this boundary. No implicit state.

Layer 2 produces final ByteState + IR graph. Layer 3 consumes only these declared inputs:

| Input artifact | Digest |
|---------------|--------|
| `GroundedCommitBundle` | `bundle.digest` (commit + evidence + KG snapshot refs) |
| `ByteStateV1` | `identity_hash` (+ schema/registry snapshot refs) |
| `CoreRealizationSpec` | `spec.compute_digest()` |
| `ElaborationPolicyIR` | `policy.digest` |

And produces only these declared outputs:

| Output artifact | Digest |
|----------------|--------|
| `RenderArtifact` | `render_digest` (text + optional confidence payload) |
| `GateWitnessBundle` | `witness_bundle.digest` (all gate witnesses) |
| `ByteTrace` | `payload_hash` (realization trajectory, if applicable) |

`StateLatentComputer` encodes the IR through a shared latent (used by both value function and diffusion conditioning). `RealizationSpec` translates the IR's semantic structure into slot-level constraints (MUST/SHOULD/MAY).

**Key files**: `core/realization/spec.py`, `docs/reference/specifications/ir/diffusion_realization_v10.md`

**Risk**: StateLatent leakage — diffusion learning to reconstruct IR from conditioning alone. Mitigated by information bottleneck, noise injection, diverse realizations, contrastive decoy training, and surface variation audit. Undeclared inputs (e.g., latent computed with a different registry snapshot) are prevented by the digest-pinned input contract.

### Seam 5: L1 ↔ Semantic Validation (Memory ↔ Rewriting)

**Contract**: Operator witnesses prove invariant preservation.

SWM stores claims conforming to registered schemas (MS-01). Rewriting operators produce `SemanticDeltaIR` with witness. Validation gates check IR equivalence, entailment, quotation/citation, and drift. Failed validation triggers refusal or fallback.

**Key files**: `core/worlds/claim_ir.py`, `core/worlds/claim_verifier.py`

**Risk**: Semantic leakage if operator witnesses are insufficient to prove invariant preservation. Every rewriting operator must emit a typed witness.

### Seam 6: LabelSpace → All Layers

**Contract**: Canonical vocabulary coordination across all layers.

LabelSpace defines stable IDs for concepts, entity types, properties, relations, operations, and intents. Every layer (syntax, semantics, semiotics, pragmatics, latent) projects into this space. WordNet, JSON-LD, Text IR, ClaimIR, and Pseudocode IR all use the same identifiers for the same concepts.

**Key files**: `docs/theory/wordnet_harness_not_an_oracle.md` (LabelSpace section)

**Risk**: Without explicit LabelSpace, each world/layer invents its own semantic bindings, resulting in opaque enums and failed cross-domain transfer. This is the highest-priority transverse concern.

---

## 4. Knowledge Graph Integration

### 4.1 KGs as Upstream Semantic Augmentation

WordNet and Wikidata serve three roles in this pipeline:

1. **Lexical grounding** (WordNet): Surface words → stable concept anchors via synset resolution. Provides hypernym paths, troponyms, and sense disambiguation that the Intake Pipeline consumes to produce unambiguous semantic IR.

2. **Factual grounding** (Wikidata): Entity IDs, property verification, relation anchors. When SemanticCommitIR contains claims about entities or relations, KG facts provide evidence atoms that the Claim Schema Registry (MS-01) uses to distinguish assertions from hypotheses.

3. **Landmark world** (WordNet): When ClaimIR cannot cleanly represent a concept, WordNet subgraphs provide routing patterns — conceptual waypoints that operators can traverse to reach representable claims. These are bridges, not truth sources.

### 4.2 KGs as Downstream Structured Emitters

The same KG infrastructure that grounds upstream semantics can produce structured output:

- **ClaimIR → KG query plan**: Given a SemanticCommitIR, derive which KG facts are needed to verify/ground the claims. The query plan is itself a governance artifact — it declares what evidence was sought and what was found.

- **Evidence coverage gate**: Before realization, check whether the commit's claims have sufficient KG backing. Claims without evidence are flagged as hypotheses. The realization policy can treat hypotheses differently (hedging language, explicit uncertainty markers).

- **Derivability gate**: After realization, verify that the surface text's factual claims are derivable from the KG evidence cited in the sidecar. This closes the loop: KG evidence → semantic commit → realization → verification back to KG.

### 4.3 What KGs Do NOT Do

KGs are not truth sources. They are structured evidence that the governance pipeline consumes. A KG fact being present does not make a claim true; it makes it *grounded*. The distinction matters:

**Determinism constraint (cert mode)**:
- All KG reads must be executed against pinned snapshots referenced by `KGSnapshotRefs`.
- No network queries; no "latest" resolution. Absence must be recorded as a witness, not silently ignored.
- `EvidenceBundle.digest` must be included in the downstream sidecar to make grounding auditable.
- `KGSnapshotRef` schema: `{kg_id, snapshot_digest, schema_version, build_provenance}`

Examples:

- **Grounded claim**: "John is a doctor" + KG evidence `wikidata:Q12345 instance_of physician` → the claim has evidence.
- **Ungrounded claim**: "John is a doctor" + no KG evidence → the claim is a hypothesis. It may still be asserted, but the sidecar will record the absence of evidence.
- **Contradicted claim**: "John is a doctor" + KG evidence `wikidata:Q12345 instance_of engineer` → the claim conflicts with evidence. The system either refuses or flags the contradiction.

---

## 5. Claim Schema Integration (MS-01)

### 5.1 Claims as the Governance API

Every semantic atom in the pipeline — from intake IR through SWM encoding to realization — is or contains a claim. MS-01 defines the system that makes these claims governable:

- **Schema registry**: Every claim type must be registered with a `SchemaDef` specifying slots, constraints, evidence policy, and migration policy.
- **Evidence binding**: Claims reference evidence atoms. The evidence policy (per schema) determines whether a claim can be asserted without evidence or must remain a hypothesis.
- **Versioned migration**: Schema changes are explicit operations with migration metadata, not silent mutations.
- **Fail-closed validation**: Invalid claim instances are rejected before entering memory.

### 5.2 How Claims Flow Through the Pipeline

| Phase | Claim role |
|-------|-----------|
| Intake | IR atoms are implicit claims about text structure ("this word is an entity", "this phrase is a predicate") |
| Semantic Commitment | Atoms become explicit claims in SemanticCommitIR — typed, content-addressed, with forbidden claim classes |
| KG Grounding | Claims acquire evidence atoms from WordNet/Wikidata |
| SWM Encoding | Claims are encoded as Code32 governance markers (polarity, PN type) on the ByteState governance layer |
| Realization Spec | Claims determine slot intent: claims with evidence → MUST_INCLUDE; hypotheses → SHOULD_INCLUDE |
| Validation Gates | Gate B verifies all MUST_INCLUDE claims are realized; Gate C0 checks lexical novelty; Gate C1 checks no unauthorized claims leaked (reparse → diff) |
| Post-Realization | Reparsed surface text produces new claims; diff against source commit detects drift |
| Sidecar | Final artifact records which claims were committed, which evidence was cited, and which gates passed |

### 5.3 The ComparisonRegistry Bridge

`core/worlds/claim_ir.py` already implements `ComparisonRegistry` with strategies for `numeric_exact`, `numeric_range`, `boolean`, `string_normalized`, and `entity_id` comparisons. This registry is the mechanism for Seam 5 (semantic validation): when a rewriting operator changes surface form, the `ComparisonRegistry` verifies that the underlying claim values haven't drifted.

---

## 6. What Exists Today vs. What Needs Building

### Already built and proven

| Component | Evidence | Location |
|-----------|----------|----------|
| Intake Pipeline (rule_tokenizer:v1) | Used in toy-e2e-demo, produces content-addressed IR | `test-scenarios/toy-e2e-demo/` |
| SWM contract + lifecycle | 17 invariants, 24 tests, 3 system gates | `docs/canonical/semantic_working_memory_contract_v0.md` |
| ByteStateV1 + Code32 | Round-trip verified, 176-340x speedup over Python | `test-scenarios/bytestate-benchmark/` |
| ByteTraceV1 | Envelope/payload split, deterministic replay | `core/carrier/bytetrace.py` |
| ClaimIR + ComparisonRegistry | Schema-validated claims with typed comparisons | `core/worlds/claim_ir.py` |
| WordNet adapter | Synset resolution, hypernym traversal, KG registration | `core/worlds/wordnet.py` |
| RealizationSpec + MaskSlot | Core types for slot-level constraint specification | `core/realization/spec.py` |
| Diffusion demo (simulated) | 60 realizations, 100% gate pass, 8 phases | `test-scenarios/diffusion-demo/` |
| SWM I/O demo | 8-phase operator pipeline with ByteTrace provenance | `test-scenarios/swm-io-demo/` |
| Toy E2E demo | Joins SWM + diffusion + lemma-grammar via governance crosscheck; SAP-1a four-plane agreement proof; langpack sidecar (bridge profiling, role-typing analysis, projection-only diagnostics) | `test-scenarios/toy-e2e-demo/` |
| Realization LanguagePacks demo | Standalone realization operator chain: planner → SELECT_FRAMES → LINEARIZE → LEXICALIZE → INFLECT → ASSEMBLE → REALIZE_CONSTITUENTS; 147 fixtures, 107 tests | `test-scenarios/realization-languagepacks-demo/` |

### Needs building

| Component | Why | Depends on |
|-----------|-----|-----------|
| **LabelSpace** (canonical vocabulary layer) | Without it, each layer invents its own semantic bindings | Architecture decision — this is Seam 6 |
| **ClaimInstance + ClaimDelta** (MS-01 S2) | Bridge between operators and claim schema | MS-01 S1 (Claim Schema Registry) |
| **StateLatentComputer** | Shared conditioning vector for value function + diffusion | L2 carrier compilation |
| **Real diffusion LM integration** | Replace rule-based simulator with SEDD/LLaDA | Tokenization alignment, mask schedule |
| **Post-realization IR reparse** | Self-auditing: generate → reparse → IR diff | Intake Pipeline + semantic diff |
| **GroundedCommitBundle** | Immutable meaning ledger boundary for realization | SemanticCommitIR + KG infrastructure |
| **KGSnapshotRef + KGQueryPlanIR** | Pinned KG snapshots and deterministic query plans | KG adapters + content addressing |
| **KG query plan generation** | Derive evidence needs from SemanticCommitIR | ClaimIR + KG adapters |
| **Evidence coverage gate** | Pre-realization check for KG backing | KG query plan + MS-01 evidence policy |
| **Gate C1 (claim leakage)** | Reparse → ClaimIR diff; no new claims beyond policy | Post-realization IR reparse |
| **Cross-domain LabelSpace projection** | ClaimWorld ↔ WordNet ↔ TextIR ↔ PseudocodeIR mapping | LabelSpace definition |
| **RealizationTarget modes** | Structured output (JSONLD, ClaimTable) alongside PlainText | RealizationSpec + KG evidence |

---

## 7. Sequencing

The dependency chain constrains the order. Work proceeds in tiers:

### Tier 0: Foundation (no code changes, design only)

1. **LabelSpace definition**: Define `ConceptID`, `EntityTypeID`, `PropertyID`, `RelationID`, `OperationID`, `IntentID` as the canonical vocabulary. Map existing enums and string constants from all layers to this space.

2. **Claim Schema Registry design** (MS-01 S1): Formalize `SchemaDef`, `SlotDef`, mixins, evidence policies. This is design work — the contract definition, not the implementation.

3. **GroundedCommitBundle specification**: Define the immutable meaning ledger boundary — `SemanticCommitIR` + `KGSnapshotRefs` + `KGQueryPlanIR` + `EvidenceBundle` + `ClaimQualification`. Pin the digest scheme and the cert-mode constraint (no live KG queries).

4. **StateLatent specification**: Define the conditioning vector format that bridges L2 carrier output and L3 diffusion input. Must be shared with value function.

### Tier 1: Schema + Vocabulary Infrastructure

5. **LabelSpace implementation**: Small, frozen dataclasses. Projection maps from existing enums.

6. **Claim Schema Registry MVP** (MS-01 S1): Register / validate / version / migrate.

7. **ClaimInstance + ClaimDelta** (MS-01 S2): Bridge operators to claim schema. Every semantic mutation emits a `ClaimDelta` with evidence links.

8. **GroundedCommitBundle implementation**: Wire KGSnapshotRefs, KGQueryPlanIR, EvidenceBundle into the commit pipeline. Pinned snapshot infrastructure for cert mode.

### Tier 2: Pipeline Integration

9. **Post-realization IR reparse**: Intake Pipeline running on generated text, producing IR for diff against source commit.

10. **KG query plan generation**: Given SemanticCommitIR, derive KG queries for evidence grounding.

11. **Evidence coverage gate**: Pre-realization check that claims have sufficient backing.

12. **ClaimTable realization target**: Structured emitter mode — deterministic, schema-validated, exercises KG query planning + evidence qualification + sidecar packaging without requiring neural generation. Cert-grade fallback when text generation refuses or drifts.

### Tier 3: Neural Realization

13. **Tokenization alignment**: Map MaskSlot positions to diffusion model token indices.

14. **Real mask schedule**: Replace fixed-step simulation with learned denoising schedule.

15. **Discrete diffusion LM integration**: SEDD or LLaDA conditioned on RealizationSpec.

16. **Gate upgrades**: Gate C0 (lexical leakage) must handle token-level novelty with real model vocabulary; Gate C1 (claim leakage) becomes mandatory in cert-grade runs; Gate D (refusal) taxonomy aligns with MS-01 claim statuses.

---

## 8. Realization Spec Contract

Source: `core/realization/spec.py`

The RealizationSpec is the bridge between semantic IR (L1) and text generation (L3). It translates committed claims into slot-level constraints that any realizer — template-based or diffusion-based — must respect.

### 8.1 MaskIntent

```python
class MaskIntent(str, Enum):
    MUST_INCLUDE = "must_include"     # Priority 1.0 — content must appear verbatim
    SHOULD_INCLUDE = "should_include" # Priority 0.7 — content should appear, paraphrase OK
    MAY_INCLUDE = "may_include"       # Priority 0.4 — include if natural
```

**Semantics by realizer mode**:
- **Template realizers**: Only `MUST_INCLUDE` is binding. `SHOULD_INCLUDE` and `MAY_INCLUDE` are best-effort.
- **Diffusion realizers**: All intents influence conditioning strength. Priority values map to mask retention probabilities.

### 8.2 MaskSlot

A slot in the realization mask representing content that should appear in output:

| Field | Type | Description |
|-------|------|-------------|
| `slot_id` | `str` | Semantic role identifier (e.g., `"subject"`, `"predicate"`, `"polarity"`) |
| `content` | `str` | The content that should appear |
| `intent` | `MaskIntent` | How strictly to preserve this content (default: `MUST_INCLUDE`) |
| `span_hint` | `Optional[Tuple[int, int]]` | Character span hint for positioning |

### 8.3 RealizerMode and RealizerId

```python
class RealizerMode(str, Enum):
    TEMPLATE_V0 = "template_v0"   # Template-based, deterministic (current)
    DIFFUSION_V1 = "diffusion_v1" # Block diffusion-based (future)
    PLANNING_V1 = "planning_v1"   # Prerequisite planning-based (lemma-grammar demo)
```

`RealizerId` binds a realizer to a specific mode and implementation:
- `mode`: `RealizerMode` enum value
- `impl_id`: Implementation identifier (e.g., `"template:pn_copular:v0"`, `"lemma_grammar:prerequisite:v1"`)

**No hidden routing**: Realizer selection is explicit via `RealizerId`, never implicit based on content analysis. This prevents non-deterministic realizer dispatch.

### 8.4 RealizationSpec

The top-level specification consumed by all realizers:

| Field | Type | Description |
|-------|------|-------------|
| `slots` | `Tuple[MaskSlot, ...]` | Ordered content slots |
| `style_id` | `str` | Style identifier (default: `"sterling.light.explain.v0"`) |
| `domain_id` | `int` | Domain identifier (0 = text default) |
| `metadata` | `Dict[str, Any]` | Additional configuration (authoritative; included in digest) |
| `realizer_id` | `Optional[RealizerId]` | Explicit realizer identity |

**Digest computation** (`compute_digest()`):
1. Slots are sorted by canonical JSON serialization (deterministic ordering regardless of insertion order).
2. The canonical JSON of the full spec is computed via `canonical_json_dumps()`.
3. Domain prefix `REALIZATION_SPEC_V1|` is prepended to the canonical bytes.
4. SHA-256 is computed; output format: `"sha256:<hex>"`.

**Claim-to-slot mapping** (Phase 5 in the pipeline):
- Asserted claims → `MUST_INCLUDE` slots
- Hypotheses → `SHOULD_INCLUDE` slots
- Policy vocabulary → `MAY_INCLUDE` slots
- Multi-word phrases are split to individual words for slot-level constraint specification.

### 8.5 Realization Spec Invariants

1. **Deterministic digest**: Same slots + style + domain + metadata + realizer → same digest. Slot ordering does not affect the digest (canonical sort).
2. **No implicit state**: The spec is the complete contract between semantic IR and realizer. No ambient configuration influences realization behavior.
3. **Realizer binding**: When `realizer_id` is set, only that realizer may consume the spec. Mismatch is a contract violation.
4. **Metadata authority**: The `metadata` dict is authoritative (included in digest). It is not advisory — changing metadata changes the spec's identity.

---

## 9. Honest Framing

What this document describes is the **target architecture**. Here is what each piece actually delivers today:

| Claim | Status | Evidence |
|-------|--------|----------|
| Semantic ledger exists before generation | **Proven (commit intent)** | SemanticCommitIR is content-addressed, immutable after Phase 2; GroundedCommitBundle (Phase 3) is the full meaning ledger boundary — not yet implemented |
| Meaning is traceable through the chain | **Proven for demos** | Toy E2E demo links every realization back to its commit via ByteTrace |
| KG grounding provides evidence atoms | **Infrastructure exists** | ClaimIR + WordNet adapter + ComparisonRegistry are implemented; lemma-grammar demo uses pinned WordNet synset snapshot for lexical grounding |
| Semantic identity is backend-invariant | **Proven (three-way + SAP-1a)** | Toy E2E three-way governance crosscheck: SWM carrier, diffusion simulation, and prerequisite planning all produce identical governance markers from the same upstream parse (12/12 polarity + PN type triple match). SAP-1a four-plane agreement proof confirms alignment across anchor, carrier, governance, and realization planes |
| Surface realization as prerequisite planning | **Proven for narrow scope** | Lemma-grammar demo produces grammatical English for 10/12 toy-e2e corpus sentences via operator chain with fail-closed leakage. 2 refusals are due to role-typing mismatches (MODIFIER unlicensed in copular template, frame underspecified for intransitive), not planner limitation. Canon/v2 expanded corpus from 10→12 sentences and resolved contraction tokenization gap |
| Langpack projection diagnostics | **Proven (sidecar)** | Bridge profiling (STRICT vs COERCED) with typed BridgeAudit; role-typing analysis extracts frame choice and role assignments from realization trace; COERCED 7/12 linearized, STRICT 0/12; 32 report-level lock tests |
| Validation gates catch semantic drift | **Heuristic, not formal** | Gates B/C operate at word/atom level, not logical entailment. No syntactic well-formedness gate yet — diffusion branch passes gates with ungrammatical output |
| Post-realization verification closes the loop | **Not yet built** | IR reparse pipeline exists for intake, not yet for generated text |
| LabelSpace unifies all layers | **Not yet built** | Concept is documented, implementation is future work |
| Diffusion LM replaces simulator | **Not yet built** | Architecture designed, simulator proves governance surfaces |

### What the four-branch evidence proves (2026-02-23)

The toy-e2e demo joins four independent realization backends under one shared parse. The results:

- **Governance convergence**: All three primary backends (SWM carrier, diffusion simulation, prerequisite planning) produce identical polarity and PN type governance markers for all 12 corpus sentences. Lemma-grammar's richer internal vocabulary (`modality`, `quantified`) is explicitly collapsed to the canonical governance map — fail-closed on unmapped types.
- **SAP-1a four-plane agreement**: The SAP-1a claim proof verifies alignment across anchor, carrier, governance, and realization planes for each sentence. This is a stronger convergence result than the three-way crosscheck alone: it proves that semantic identity is preserved through the full encoding/decoding pipeline, not just at the governance surface.
- **Langpack sidecar diagnostics**: The langpack branch (sidecar mode) runs bridge profiling in STRICT and COERCED modes, producing role-typing analysis for each sentence. COERCED profile: 7/12 linearized, 2 frame_refused, 3 linearize_refused. STRICT profile: 0/12 linearized (9 frame_refused, 3 bridge_refused). The gap between COERCED and STRICT quantifies how much semantic coercion the bridge performs. BridgeAudit records every injection, collapse, and mapping.
- **Surface quality gap**: The lemma-grammar branch produces exact source reproductions for 8/12 sentences and close approximations for 2 more. The diffusion branch produces word salad under non-technical policies. Both pass all gates because gates check semantic content presence, not syntactic well-formedness.
- **Lemma-set overlap**: Diagnostic crosscheck between lemma-grammar's `committed_lemmas` and diffusion's semantic atoms averages 0.87 overlap rate. Mismatches are normalization differences (different lemmatizers), not semantic disagreements.

### Known gaps to full grammatical output

1. **Unlicensed roles in copular template**: The copular realization template licenses SUBJECT and PREDICATE_NOMINAL but not MODIFIER. Sentences with MODIFIER roles (e.g., "The big dog is happy") refuse at LINEARIZE with `unlicensed_roles`. This is a template coverage issue — the policy needs an expanded copular template or a modifier-aware adjunct slot.
2. **Frame underspecification**: Intransitive events (only AGENT, no PATIENT) fall to the transitive default rule which expects AGENT+PATIENT. These refuse at SELECT_FRAMES with `frame_underspecified`. Fix is to add an intransitive frame rule to the EN policy.
3. **Syntactic well-formedness gate**: Gates check semantic content but not grammaticality. A reparse-and-diff gate (Phase 8 in this spec) would catch the diffusion branch's ungrammatical output. Architecture supports it; not yet wired.
4. **Template coverage**: The lemma-grammar planner supports 5 template patterns. Broader English requires either many more templates or integration with a conditioned generative model. The `MaskSlot`/`RealizationSpec` interface is designed for the latter.
5. **Shared normalizer**: Lemma-grammar and diffusion use different lemmatizers (`_simple_lemmatize` vs. `_derive_lemma`). A versioned, shared normalizer would close the lemma-set overlap gap.
6. **Discourse composition**: Current system operates sentence-by-sentence. Long-form output requires discourse planning (topic management, referent tracking, coherence constraints) on top of per-sentence realization.

The governance infrastructure (commit/policy/gates/witnesses), determinism, semantic identity convergence across backends, carrier/provenance integration, and langpack diagnostic instrumentation are proven. Syntactic well-formedness enforcement, broader template coverage, and neural model integration are the next frontier.

---

## 10. Success Criteria

The convergence is complete when:

1. **LabelSpace is first-class**: Every concept, entity type, property, relation, operation, and intent referenced across all layers uses a `LabelSpace` identifier, not a local enum or string constant.

2. **MeaningStateDigest chains are deterministic**: Identical operator sequence + registry → identical chain. Proven by replay verification.

3. **ByteState compilation is a pure function**: Identical inputs → identical bytes. No runtime state leakage.

4. **Claims gate all memory writes**: No claim enters memory without conforming to a registered schema (MS-01). Invalid claims fail closed.

5. **Evidence is explicit**: Every asserted claim cites evidence atoms. Hypotheses are marked as such. The distinction is visible in the sidecar.

6. **Post-realization verification works**: Generated text is reparsed to IR, diffed against source commit, and drift is detected and handled (refuse or retry).

7. **All layers respect the Neural Usage Contract**: No neural component writes to authoritative state. Neural outputs are proposals that must pass governance gates before affecting memory.

8. **End-to-end pipeline runs**: Input text → semantic commit → KG grounding → SWM encoding → realization spec → constrained generation → validation → certified sidecar. All phases produce auditable artifacts.

---

## 11. Related Documents

### Canonical (authoritative)
- [Sterling Architecture Layers](sterling_architecture_layers.md) — four-layer definition
- [Code32 and ByteStateV1](code32_bytestate.md) — carrier substrate spec
- [ByteState Compilation Boundary](bytestate_compilation_boundary.md) — codec spec
- [SWM Contract v0](../../canonical/semantic_working_memory_contract_v0.md) — memory architecture contract

### Specifications
- [Diffusion Realization v10](../specifications/ir/diffusion_realization_v10.md) — Layer 3 spec
- [Realization IR Text Contract](../specifications/text-semantic-ir/realization-ir-text-contract.md) — realization bridge
- [Semantic Text Contract](../specifications/text-semantic-ir/semantic-text-contract.md) — intake pipeline

### Design (active)
- [MS-00 Thesis](../specifications/memory-substrate/MS-00-thesis.md) — memory substrate thesis
- [MS-01 Claim Schema](../specifications/memory-substrate/MS-01-claim-schema.md) — claim governance
- [MS Roadmap](../specifications/memory-substrate/MS-Roadmap.md) — workstream sequencing

### Theory / Experimental
- [Generative Inversion](../../theory/generative-inversion.md) — Frazier inversion thesis
- [WordNet Harness](../../theory/wordnet_harness_not_an_oracle.md) — LabelSpace and KG roles

### Evidence (demos)
- [Diffusion Demo](../../../test-scenarios/diffusion-demo/README.md) — 60 realizations, 8 phases
- [SWM I/O Demo](../../../test-scenarios/swm-io-demo/README.md) — operator pipeline, 8 phases
- [Lemma-Grammar Demo](../../../test-scenarios/lemma-grammar-demo/README.md) — prerequisite planning realization, WordNet grounding, 8 phases
- [Realization LanguagePacks Demo](../../../test-scenarios/realization-languagepacks-demo/README.md) — standalone realization operator chain, 107 tests, trace schema locks
- [Toy E2E Demo](../../../test-scenarios/toy-e2e-demo/README.md) — four-branch joined pipeline with SAP-1a proof + langpack sidecar (UnifiedReportV2)
