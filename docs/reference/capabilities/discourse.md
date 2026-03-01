---
authority: reference
status: advisory
date: 2026-03-01
capability: discourse
parity_capabilities: [H2]
---

# Discourse and Intent

**Advisory — not normative.** This document describes proof obligations for future v2 work. Do not cite as canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability status.

## Overview

Discourse processing covers how Sterling interprets user intent, manages dialogue state, and coordinates multi-turn interactions. The sterling Python repo defined a detailed discourse contract with 10 intent families, 40+ intent types, goal type enumerations, dialogue phase tracking, entity binding, speech act classification, and discourse-level operators. This reference captures the proof obligations that model imposes on the native substrate.

Discourse is the furthest-out capability in the parity audit. No implementation work has started. This document is intentionally lighter on prescriptive design and heavier on the minimal proof obligations that any future implementation must satisfy.

## Key Concepts

### Intent Taxonomy

The sterling Python repo organized user intents into families (e.g., informational, procedural, creative, analytical) with specific types within each family (e.g., factual query, how-to request, open-ended generation). Each intent type has a satisfaction model — a finite state machine defining what it means for the intent to be addressed.

The key design question for the native substrate is how much of this taxonomy carries forward. The taxonomy is large (40+ types), and some types may be artifacts of the sterling Python repo's specific use cases rather than fundamental categories. The proof obligation is not "implement all 40 types" but "intents are typed, registered, and governed."

### Goal Types

Intents map to goal types that the planning/search system can pursue. A goal type defines what success looks like in terms of state predicates — conditions on ByteState that constitute goal satisfaction. The mapping from intent to goal type is itself a governed operation: it must be auditable and deterministic given the same discourse context.

In the native substrate, goal evaluation already exists via `SearchWorldV1::is_goal()`. Discourse would extend this by providing the mechanism that selects which goal predicate to evaluate, based on interpreted intent.

### Dialogue Phases

Multi-turn interactions move through phases: opening (intent recognition), elaboration (clarification, entity binding), execution (search/planning against the interpreted goal), and closing (result delivery, satisfaction assessment). Phase transitions are governed: each transition produces a state change that is recorded in the discourse trace.

### Entity Binding

Discourse references entities — objects, concepts, prior utterances — that must be resolved to specific referents before reasoning can proceed. Entity binding is the process of mapping discourse references to content-addressed identifiers (analogous to Code32 concept IDs or KGRef content-addressed entity identity).

Unresolved entity references are holes in the discourse state, analogous to ByteState slots in Hole status or text IR holes. The system must track which references are resolved and which remain open.

### Discourse Operators

The sterling Python repo defined discourse-level operators: SELECT_GOAL_TYPE, BIND_ENTITIES, CLARIFY, ACKNOWLEDGE, REDIRECT, and others. These are operators in the same sense as SET_SLOT — they transform state under witnessed contracts. The key obligation is that discourse operators are registered operators with signatures in OperatorRegistryV1, not ad hoc functions.

## Design Decisions (Open)

| Decision | Options | Constraint |
|----------|---------|------------|
| Taxonomy scope | Full 40+ types, compressed taxonomy, or extensible base | Must be typed and registered; size is a pragmatic choice |
| Intent as world or as harness layer | Intent resolution as a SearchWorldV1 or as pre-search harness logic | If world: search finds the best interpretation. If harness: interpretation is fixed before search |
| Discourse state representation | ByteState-encoded or dedicated DiscourseStateV1 | ByteState reuse gets verification for free; dedicated type allows richer structure |
| Phase tracking | Explicit FSM in state or implicit from operator sequence | Explicit FSM is auditable; implicit is simpler but harder to verify |
| Entity binding mechanism | Reuse RegistryV1 concept mapping or define EntityRegistryV1 | RegistryV1 is proven for Code32 mapping; entities may need richer structure |
| Clarification model | Clarification as operator (transforms state) or as world action (generates observation) | Operator model integrates with existing apply() contract |

## Proof Obligations for v2

1. **Intents are typed and registered.** Every intent recognized by the system has a type identifier registered in a discourse registry (analogous to OperatorRegistryV1). No intent processing occurs for unregistered intent types — fail-closed on unknown intents.

2. **Discourse operators are registered operators.** SELECT_GOAL_TYPE, BIND_ENTITIES, CLARIFY, and all other discourse-level state transformations are operators with signatures, preconditions, and effect contracts. They are registered in OperatorRegistryV1 (or an equivalent discourse-specific registry) and produce witnessed state changes.

3. **Entity bindings are content-addressed.** Resolved entity references produce content-addressed bindings (ContentHash or Code32-based). The binding between a discourse reference and its referent is auditable and deterministic.

4. **Dialogue state is traceable.** The discourse state at any point in a multi-turn interaction is reconstructable from the initial state plus the sequence of discourse operators applied. This is the same replay guarantee that holds for ByteState execution.

5. **Goal selection is governed.** The mapping from interpreted intent to goal predicate is a witnessed operation. Given the same discourse context and the same intent interpretation, the same goal predicate is selected. The selection is recorded in the evidence chain.

6. **Satisfaction is verifiable.** Intent satisfaction (did the system address what the user asked?) is defined by a satisfaction model — a set of conditions over discourse state. The model is content-addressed and evaluable against the final discourse state.

7. **No implicit discourse state.** All discourse state (phase, bound entities, active intent, clarification history) is explicit and content-addressed. No reasoning path depends on implicit dialogue context that is not represented in the auditable state.

## Parity Audit Reference

This document covers capability **H2** (Discourse / speech act contracts) from the [parity audit](../../architecture/v1_v2_parity_audit.md).

Current status: **Not started.** The parity audit notes that the sterling Python repo's contract promotion queue marks this capability as "Rewrite" — indicating that the design should be reconsidered rather than ported directly.

### What exists today (verifiable)

- Operator taxonomy with category codes — `kernel/src/operators/operator_registry.rs` (M, S, P, K, C categories)
- Advisory/authoritative trust boundary — `docs/adr/0003-neural-advisory-not-authoritative.md`

### What is proposed (not implemented)

- An IntentFamily taxonomy (Inform, Query, Direct, Commit, Meta) for speech act classification
- Discourse operators that transform IR at the pragmatics layer
- A discourse state type tracking conversational context across turns
- Speech act certification (a SpeechActWitness type binding utterance to intent classification)

This is the furthest-out capability in the migration plan. Prerequisites include:
- Text boundary contract (see [text_boundary reference](text_boundary.md)) — discourse operates on text IR, not raw surface text
- Entity identity mechanism (see [knowledge_graph reference](knowledge_graph.md)) — entity binding requires content-addressed referents
- Operator registry breadth — discourse operators need a registry that supports operator categories beyond State (S)

See also Import Group F (Text boundary) in the parity audit, which establishes the foundation that discourse builds on.
