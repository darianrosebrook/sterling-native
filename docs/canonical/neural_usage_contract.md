---
authority: canonical
notice: "This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label."
---
# Neural Usage Contract

---

## Core Principle

**Neural is advisory; Symbolic is authoritative.**

Neural components can inform and accelerate decisions, but they cannot override symbolic constraints or make authoritative state changes.

## Permitted Neural Component Usage

Neural signals MAY be used for:

| Use Case | Description | Authority Level |
|----------|-------------|-----------------|
| **IR intake** | Text -> initial state / claims / spans | Authoritative for parsing only |
| **Explanation generation** | Rendering auditable path into text | Authoritative for surface form only |
| **Compression/indexing** | Latent embeddings, encoder hidden states | Non-authoritative |
| **Value estimation** | Scoring branches for search ordering | Non-authoritative |
| **Move ranking** | Prioritizing already-legal symbolic moves | Non-authoritative |

The last three are explicitly **non-authoritative**: they guide but do not decide.

## Forbidden Neural Component Usage

Neural signals may NOT:

| Forbidden Action | Why It's Forbidden |
|------------------|-------------------|
| **Create new operators** | Operators must come from the governed registry with typed contracts |
| **Bypass operator preconditions** | Preconditions are symbolic constraints that must be satisfied |
| **Mutate KG / UtteranceState directly** | All state changes must go through governed operators |
| **Introduce new facts** | Facts must come from explicit sources with provenance |
| **"Explain away" failed symbolic checks** | Symbolic failures are authoritative; neural cannot override |
| **Override symbolic logic** | Neural can only advise on move ordering, not legality |

## The Hybrid Value Function

Sterling's hybrid value function uses neural heuristics for move ordering. This is permitted because:

1. **Neural ranks already-legal moves**: The symbolic layer determines what moves are legal; neural only orders them
2. **Neural cannot create moves**: If a move isn't symbolically legal, neural cannot make it happen
3. **Neural cannot block moves**: If a move is symbolically required, neural cannot prevent it
4. **Symbolic remains source of truth**: The final decision respects symbolic constraints

Example flow:
```
1. Symbolic layer: "These 5 operators are legal from this state"
2. Neural layer: "Rank them: [op3, op1, op5, op2, op4]"
3. Search: "Try op3 first, then op1, etc."
4. Symbolic layer: "op3 succeeded, new state is X"
```

The neural layer never decided what was legal - only what order to try legal options.

## Transformer Usage Boundaries

Transformers are permitted at **I/O boundaries only**:

| Boundary | Transformer Role |
|----------|------------------|
| **Input** | Parse raw language into structured IR |
| **Output** | Turn IR/paths/summaries back into language |
| **Compression** | Encode IR/ByteState into latent for indexing (non-authoritative) |

Transformers are NOT permitted for:
- Intermediate reasoning steps
- Search expansion decisions
- Operator selection that overrides symbolic logic
- "Thinking harder" inside the loop
- Generating new state transitions directly

## Why This Matters

This contract prevents Sterling from becoming "just another LLM agent with a fancy index."

If neural components could:
- Create operators -> we've reimplemented tool-calling LLMs
- Override symbolic checks -> we've lost auditability
- Mutate state directly -> we've lost the governance boundary

The whole point of Sterling is to prove that structured semantic search can replace transformer-based reasoning. If we let neural components become the cognitive core, we've failed.

## Audit Requirements for Neural Scores

Any neural score that changes branch ordering in search must be auditable. This prevents "hidden routing" where neural components effectively make decisions without symbolic oversight.

### Required Audit Artifacts

| Artifact | Purpose | Location |
|----------|---------|----------|
| **Score value** | The actual score that influenced ordering | `OperatorEdge.score` |
| **Model identity** | Which model produced the score | Edge metadata or episode log |
| **Input snapshot** | What the model saw when scoring | StateNode reference |
| **Score breakdown** | Component scores if composite | Edge metadata |

### Recording Requirements

Neural scores that influence search ordering MUST be:

1. **Recorded** in the StateGraph edge metadata or OperatorEdge.score
2. **Traceable** to specific model weights + input state
3. **Versioned** with model checkpoint hash when in certifying mode
4. **Auditable** via TraceAuditor logging

### Invariant: No Hidden Routing

If a neural score causes branch A to be selected over branch B:
- The score difference must be visible in the StateGraph
- The model identity must be recoverable
- Replay with the same model must produce the same ordering

**Violation pattern to detect**: "Neural component modifies ordering but score is not recorded, making it impossible to explain why path A was chosen over path B."

### Implementation Reference

- `OperatorEdge.score`: Primary location for neural scores
- `OperatorEdge.metadata`: Extended score breakdown
- `core/reasoning/trace_audit.py`: TraceAuditor for logging decisions
- `core/features.py`: Canonical feature extraction for value function

## Enforcement

This contract is enforced through:

1. **Code architecture**: Neural components have no write access to KG/UtteranceState
2. **API design**: Value functions return scores, not actions
3. **Audit trail**: All state changes logged with operator provenance
4. **Testing**: Invariant tests verify neural cannot bypass symbolic constraints
5. **Score recording**: Neural scores recorded on edges for replay verification
