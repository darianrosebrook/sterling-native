> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**
>
> **Superseded in v2**: `docs/canonical/core_constraints.md` (INV-CORE-01 through INV-CORE-11, identical content promoted to v2 canonical).

# Core Constraints v1

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

---

These are **architectural invariants** that block merge/deploy if violated. They are enforced by CI, code review, and runtime checks where applicable.

## The 11 Core Constraints

| ID | Constraint | Description |
|----|------------|-------------|
| INV-CORE-01 | No Free-Form CoT | No generative LLM chain-of-thought in the decision loop. All mid-episode state transitions must go: UtteranceState/KG -> PathAlgebraEngine -> new UtteranceState/KG. The LLM is not allowed to generate new steps or state transitions directly. |
| INV-CORE-02 | Explicit State | All task state in UtteranceState + KG, not transformer KV cache. The transformer's internal cache is irrelevant to long-term state. Every piece of information considered is an explicit node or edge in the graph or a field in the state. |
| INV-CORE-03 | Structural Memory | Episode summaries + path algebra for long-horizon, not transcript prompts. If you need to know "what have we done so far / what's left?", you inspect episode summary nodes, their attached latents, and the path history - not a text transcript in a prompt. |
| INV-CORE-04 | No Phrase Routing | No phrase dictionary or regex-based routing; all routing via scored search. There is no brittle prompt-based or regex-based router that sends queries to different subsystems. All world or domain routing is handled by the core search. |
| INV-CORE-05 | Computed Bridges | Cross-domain bridges computed at runtime, not static lookup tables. When bridging between domains, the connections are computed at runtime via learned landmark embeddings and operator signatures. Any initial mappings are only used for training bootstrap. |
| INV-CORE-06 | Contract Signatures | Landmark/operator signatures are typed contracts, not learned embeddings. Each reasoning operator or landmark is defined by a signature (preconditions, effects, type) that acts as a contract. The embeddings are learned separately and never override the signature's logic. |
| INV-CORE-07 | Explicit Bridge Costs | Domain transitions carry explicit costs with hysteresis. Moving between domains carries an explicit cost in the value function. A small hysteresis is applied so the engine only shifts domain when the expected value gain outweighs the bridge cost by a threshold. |
| INV-CORE-08 | No Hidden Routers | All routing decisions auditable via StateGraph; no secret bypasses. There are no hidden conditional behaviors that circumvent the search process. Every decision to apply an operator or switch worlds must come from the search mechanism itself and is logged in the StateGraph. |
| INV-CORE-09 | Oracle Separation | No future/oracle knowledge in inference inputs; only in training signals. Features that rely on ground truth or future knowledge are disallowed in the model's inputs during live inference. They can be used as training signals, but runtime inputs are strictly checked to include only admissible features. |
| INV-CORE-10 | Value Target Contract | Canonical value targets versioned and hash-verified. Every learned value function is trained to a well-defined target formula. These contracts are versioned and their hash is recorded, so the system can detect if a model's training deviates from the agreed definition. |
| INV-CORE-11 | Sealed External Interface | External tools cannot mutate internal state except via governed operators with declared write-sets. External integrations cannot directly alter the internal reasoning state or knowledge graph. All internal state changes must occur via the governed operators and search process. |

## Enforcement

These constraints are enforced through:

1. **CI checks**: Automated validation that code changes don't violate constraints
2. **Runtime assertions**: Code-level enforcement where applicable (e.g., TraceAuditor for INV-CORE-08)
3. **Code review**: Human verification that architectural changes respect invariants
4. **Documentation alignment**: All docs must reference these constraints consistently

## Constraint Categories

**Search Integrity** (INV-CORE-01, 04, 08): Ensure the search process is the sole decision-maker

**State Explicitness** (INV-CORE-02, 03): All state is inspectable and structured

**Bridge Governance** (INV-CORE-05, 06, 07): Cross-domain transitions are principled

**Learning Integrity** (INV-CORE-09, 10): Training and inference are properly separated

**External Boundary** (INV-CORE-11): The system boundary is sealed and governed
