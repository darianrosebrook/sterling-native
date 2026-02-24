> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Theory Conformance Specification

**Version**: 2.1
**Status**: Active (Tests Partially Implemented)
**Author**: @darianrosebrook
**Last Updated**: February 17, 2026

---

## Overview

This document defines Sterling's theory conformance invariants (TC-1 through TC-11). These are **testable contracts** that prevent drift from Sterling's core theory and ensure the system "realizes the theory" rather than just "works on benchmarks."

---

## Invariant Summary

| ID     | Name                           | One-Liner                                                                 |
| ------ | ------------------------------ | ------------------------------------------------------------------------- |
| TC-1   | No-Regression Semantic Invariance | Hybrid configs never reduce correctly solved instances vs structural-only |
| TC-2   | IR-Only Inputs                 | Value heads consume only IR, StateGraph, SWM/decay - never raw text/CoT   |
| TC-3   | Bounded Student-Teacher Gap    | Student predictions stay within thresholds relative to teacher            |
| TC-4   | No CoT in Decision Loop        | Value functions are pure scorers, no text generation or conversation      |
| TC-5   | Latent is Advisory             | Latent reorders search but cannot override operator/goal semantics        |
| TC-6A  | Provenance Tracking            | All hypothesis nodes have auditable DERIVES_FROM edges in StateGraph      |
| TC-7A  | Hypothesis Influence Gate      | Hypotheses cannot affect ranking/pruning without tested predictions       |
| TC-8   | Invariance Checking            | Hypotheses must pass cross-example invariance before influencing search   |
| TC-9A  | Applicability Preservation     | Hypothesis influence only reorders candidates, never adds or removes      |
| TC-10  | Registered Interpreters        | All hypothesis interpreters must be registered with declared contracts    |
| TC-11  | Prior Lineage Replay Determinism | Prior artifact integrity is deterministically verifiable via hash chain |

---

## TC-1: No-Regression Semantic Invariance

### Definition

For a fixed canonical benchmark suite (WordNet v1, IR_NAV v0.2, Rush Hour v1):

1. Run Sterling with structural-only value function.
2. Run Sterling with structural + latent + student (full hybrid stack).
3. Assert:
   - **TC-1a**: Every instance successfully solved by structural-only is also solved by hybrid (no semantic regressions).
   - **TC-1b**: Any additional successes introduced by hybrid must match the same gold goal and verdict.

### Rationale

Adding latent/hybrid components should only **improve or maintain** performance, never cause previously-working cases to fail. This ensures latent is truly additive.

### What's Allowed

- Differences in path length
- Differences in node count
- Differences in budget usage
- Different operator sequences (as long as goal is reached)

### What's Forbidden

- Hybrid solving fewer instances than structural-only
- Hybrid reaching different verdicts on the same input
- Hybrid timing out on instances structural-only solves

### Test Implementation

```python
# tests/conformance/test_tc1_semantic_invariance.py

def test_tc1_no_regression():
    """TC-1: Hybrid never reduces solved instances vs structural-only."""
    benchmark = load_canonical_benchmark("wordnet_v1")

    # Run structural-only
    structural_results = run_with_config(benchmark, value_mode="structural")
    structural_solved = {r.instance_id for r in structural_results if r.success}

    # Run full hybrid
    hybrid_results = run_with_config(benchmark, value_mode="hybrid")
    hybrid_solved = {r.instance_id for r in hybrid_results if r.success}

    # TC-1a: No regressions
    regressions = structural_solved - hybrid_solved
    assert len(regressions) == 0, f"TC-1a violated: {regressions}"

    # TC-1b: Same verdicts where both succeed
    for instance_id in structural_solved & hybrid_solved:
        s_result = get_result(structural_results, instance_id)
        h_result = get_result(hybrid_results, instance_id)
        assert s_result.verdict == h_result.verdict, f"TC-1b violated: {instance_id}"
```

**Test File**: `tests/conformance/test_tc1_semantic_invariance.py`

---

## TC-2: IR-Only Inputs

### Definition

All value heads consume ONLY:

- IR tokens / IR_LATENT_V1 structures
- Features that are **pure functions** of (IR, StateGraph, SWM/Decay)

They MUST NOT:

- Read raw text prompts or free-form chain-of-thought
- Call external LLMs or other generative models during value computation
- Depend on stateful history outside the StateGraph / Episode structures

### Rationale

Sterling's value heads must operate over structured representations, not raw text. This ensures:

1. Reasoning is auditable at the IR level
2. No hidden "LLM magic" in the decision loop
3. Value computation is deterministic and reproducible

### Test Implementation

```python
# tests/conformance/test_tc2_ir_only_inputs.py

def test_tc2_ir_only_inputs():
    """TC-2: Value heads only consume IR-derived inputs."""
    value_head = load_value_head("hybrid")

    # Create state with IR
    state = create_test_state_with_ir()

    # Should work with IR
    score = value_head.evaluate(state)
    assert isinstance(score, float)

    # Negative test: raw text should fail
    state_with_text = create_state_with_raw_text()
    with pytest.raises(IROnlyViolation):
        value_head.evaluate(state_with_text)

def test_tc2_no_llm_calls():
    """TC-2: Value heads do not call external LLMs."""
    with mock.patch("openai.ChatCompletion.create") as mock_llm:
        value_head = load_value_head("hybrid")
        state = create_test_state()

        # Evaluate should not trigger LLM
        value_head.evaluate(state)

        assert mock_llm.call_count == 0, "TC-2 violated: LLM called during value computation"
```

**Test File**: `tests/conformance/test_tc2_ir_only_inputs.py`

---

## TC-3: Bounded Student-Teacher Gap

### Definition

For each domain with a latent teacher + feature student pair:

- Mean squared value error: `E[(V_teacher - V_student)^2] <= epsilon_value`
- Operator disagreement rate: `<= epsilon_op`
- Difficulty disagreement rate: `<= epsilon_diff`

### Baseline Thresholds (WordNet v1, v7.5 -> v8.5-distilled)

| Metric                    | Threshold | Current (v8.5) |
| ------------------------- | --------- | -------------- |
| `epsilon_value`           | 0.01      | < 0.01         |
| `epsilon_op`              | 0.05      | 0.0108         |
| `epsilon_diff`            | 0.05      | TBD            |

### Rationale

The student model is a distillation of the teacher. If the gap grows too large, the student is no longer a valid approximation and must be retrained.

### Threshold Updates

Thresholds are defined per domain and updated **only by explicit decision** documented in this spec. Any threshold change requires:

1. Justification for why the old threshold is inappropriate
2. Evidence that the new threshold maintains semantic alignment
3. Update to this document with the new values

### Test Implementation

```python
# tests/conformance/test_tc3_student_teacher_gap.py

THRESHOLDS = {
    "wordnet_v1": {
        "epsilon_value": 0.01,
        "epsilon_op": 0.05,
        "epsilon_diff": 0.05,
    }
}

def test_tc3_value_gap():
    """TC-3: Student-teacher value gap within threshold."""
    teacher = load_teacher_model("v7.5")
    student = load_student_model("v8.5-distilled")
    test_states = load_test_states("wordnet_v1")

    squared_errors = []
    for state in test_states:
        v_teacher = teacher.evaluate(state)
        v_student = student.evaluate(state)
        squared_errors.append((v_teacher - v_student) ** 2)

    mse = np.mean(squared_errors)
    threshold = THRESHOLDS["wordnet_v1"]["epsilon_value"]

    assert mse <= threshold, f"TC-3 violated: MSE {mse} > {threshold}"

def test_tc3_operator_disagreement():
    """TC-3: Operator prediction disagreement within threshold."""
    teacher = load_teacher_model("v7.5")
    student = load_student_model("v8.5-distilled")
    test_states = load_test_states("wordnet_v1")

    disagreements = 0
    for state in test_states:
        op_teacher = teacher.predict_operator(state)
        op_student = student.predict_operator(state)
        if op_teacher != op_student:
            disagreements += 1

    rate = disagreements / len(test_states)
    threshold = THRESHOLDS["wordnet_v1"]["epsilon_op"]

    assert rate <= threshold, f"TC-3 violated: disagreement {rate} > {threshold}"
```

**Test File**: `tests/conformance/test_tc3_student_teacher_gap.py`

---

## TC-4: No CoT in Decision Loop

### Definition

Value heads are stateless scorers. They:

- Do NOT generate text
- Do NOT maintain conversation state
- Do NOT call external LLMs
- Return only numeric scores

### Rationale

Sterling's cognitive architecture separates:

- **Value computation**: Pure scoring over IR (fast, deterministic)
- **Explanation generation**: Post-hoc text generation from episode traces (optional, separate)

Mixing these would violate the theory's separation of concerns.

### Test Implementation

```python
# tests/conformance/test_tc4_no_cot.py

def test_tc4_no_text_generation():
    """TC-4: Value heads do not generate text."""
    value_head = load_value_head("hybrid")
    state = create_test_state()

    result = value_head.evaluate(state)

    # Result must be numeric, not text
    assert isinstance(result, (int, float))
    assert not isinstance(result, str)

def test_tc4_stateless():
    """TC-4: Value heads are stateless."""
    value_head = load_value_head("hybrid")
    state = create_test_state()

    # Same state should give same score
    score1 = value_head.evaluate(state)
    score2 = value_head.evaluate(state)

    assert score1 == score2, "TC-4 violated: non-deterministic value"

def test_tc4_no_conversation_state():
    """TC-4: Value heads do not maintain conversation state."""
    value_head = load_value_head("hybrid")

    # Evaluate multiple states
    states = [create_test_state() for _ in range(5)]
    scores = [value_head.evaluate(s) for s in states]

    # Re-evaluate first state - should be same as before
    score_again = value_head.evaluate(states[0])
    assert scores[0] == score_again, "TC-4 violated: state leaked between evaluations"
```

**Test File**: `tests/conformance/test_tc4_no_cot.py`

---

## TC-5: Latent is Advisory

### Definition

Value predictions inform but do not override:

- StateGraph search ordering (latent affects priority, not applicability)
- Operator applicability (determined by OperatorRegistry preconditions)
- Goal satisfaction (determined by Task goal predicate)

### Rationale

The latent value head provides guidance for search efficiency, but the **symbolic/IR layer remains authoritative** for:

1. What operators can legally fire (preconditions)
2. Whether the goal is satisfied (task definition)
3. What the final verdict is (symbolic evaluation)

### Test Implementation

```python
# tests/conformance/test_tc5_latent_advisory.py

def test_tc5_operator_applicability():
    """TC-5: Latent cannot override operator preconditions."""
    registry = OperatorRegistry()
    state = create_state_without_predication()

    # APPLY_NEGATION requires has_predication
    assert not registry.is_applicable("APPLY_NEGATION", state)

    # Even with high latent value, operator should not be applicable
    state_with_latent = add_high_latent_value(state, "APPLY_NEGATION")
    assert not registry.is_applicable("APPLY_NEGATION", state_with_latent)

def test_tc5_goal_satisfaction():
    """TC-5: Latent cannot override goal satisfaction."""
    task = create_wordnet_navigation_task(target="mammal.n.01")

    # State not at goal
    state = create_state_at_synset("cat.n.01")
    assert not task.is_goal_satisfied(state)

    # Even with high latent "goal-like" score, goal should not be satisfied
    state_with_latent = add_high_goal_latent(state)
    assert not task.is_goal_satisfied(state_with_latent)

def test_tc5_search_ordering_only():
    """TC-5: Latent affects search ordering, not validity."""
    search = SterlingSearch(value_mode="hybrid")

    # Get applicable operators
    state = create_test_state()
    applicable = search.get_applicable_operators(state)

    # Get latent-ordered operators
    ordered = search.order_by_latent_value(applicable, state)

    # Same operators, possibly different order
    assert set(applicable) == set(ordered)
```

**Test File**: `tests/conformance/test_tc5_latent_advisory.py`

---

## TC-6A: Provenance Tracking (January 2026)

### Definition

All hypothesis nodes in the StateGraph must have auditable `DERIVES_FROM` edges that trace their origin. This enables governance auditors to verify the provenance of any hypothesis used in search.

### Requirements

- Every hypothesis node has at least one `DERIVES_FROM` edge
- All predictions have `PREDICTS` edges
- Test results have `EVALUATES_ON` edges
- Witnesses have `WITNESS_FOR` edges

### Rationale

Without provenance tracking, hypotheses could appear "from nowhere" and influence search without accountability. TC-6A ensures every hypothesis has a documented origin.

### Test Implementation

```python
# tests/conformance/test_tc6a_provenance.py

def test_tc6a_derives_from_edges():
    """TC-6A: All hypothesis nodes have DERIVES_FROM edges."""
    graph = run_reasoning_with_hypotheses()

    hypothesis_nodes = [n for n in graph.nodes if n.node_type == "HYPOTHESIS"]
    for node in hypothesis_nodes:
        derives_from = graph.get_edges(target=node.id, edge_type="DERIVES_FROM")
        assert len(derives_from) > 0, f"TC-6A violated: {node.id} has no DERIVES_FROM"
```

**Test File**: `tests/conformance/test_tc6a_provenance.py`

---

## TC-7A: Hypothesis Influence Gate (January 2026)

### Definition

Hypotheses cannot affect ranking or pruning without first having tested predictions. A hypothesis must:

1. Generate at least one prediction
2. Have that prediction tested against demonstration states
3. Pass minimum confidence thresholds

Only then can it influence search ordering.

### Rationale

Untested hypotheses are speculative. Allowing them to influence search could lead to reinforcing bad patterns. TC-7A ensures hypotheses prove their worth before gaining influence.

### Enforcement

`HypothesisLifecycleController` tracks hypothesis state and blocks influence until predictions are tested:

```python
def get_influence_eligible_hypotheses(self) -> List[HypothesisIR]:
    """Only return hypotheses that have tested predictions."""
    return [h for h in self._hypotheses if h.has_tested_predictions()]
```

### Test Implementation

```python
# tests/conformance/test_tc7a_influence_gate.py

def test_tc7a_no_influence_without_prediction():
    """TC-7A: Untested hypotheses cannot influence search."""
    lifecycle = HypothesisLifecycleController()

    # Create hypothesis without testing
    hypothesis = lifecycle.propose_hypothesis(observation)

    # Should not be influence-eligible
    eligible = lifecycle.get_influence_eligible_hypotheses()
    assert hypothesis not in eligible, "TC-7A violated: untested hypothesis eligible"
```

**Test File**: `tests/conformance/test_tc7a_influence_gate.py`

---

## TC-8: Invariance Checking (January 2026)

### Definition

Hypotheses must pass cross-example invariance before influencing search. A hypothesis that holds on one example but fails on others is not robust enough for influence.

### Requirements

- `InvarianceChecker` validates cross-example consistency
- Hypotheses must hold on all test examples in the demonstration set
- Invariance failures block eligibility (cannot be worked around)

### Rationale

A hypothesis like "always prefer HYPERNYM" might work on one example by coincidence. TC-8 ensures hypotheses generalize before they can affect search.

### Test Implementation

```python
# tests/conformance/test_tc8_invariance.py

def test_tc8_cross_example_invariance():
    """TC-8: Hypotheses must pass cross-example invariance."""
    checker = InvarianceChecker()

    # Hypothesis that only works on some examples
    partial_hypothesis = create_partial_hypothesis()

    # Should fail invariance check
    result = checker.check_invariance(partial_hypothesis, demo_states)
    assert not result.passed, "TC-8 violated: partial hypothesis passed invariance"
```

**Test File**: `tests/conformance/test_tc8_invariance.py`

---

## TC-9A: Applicability Preservation (January 2026)

### Definition

Hypothesis influence can only reorder candidates, never add or remove them. The candidate set before and after influence application must have identical members (only scores/order may differ).

### Enforcement

`CandidateSetDiff` runtime witness validates TC-9A:

```python
@dataclass
class CandidateSetDiff:
    """Runtime witness for TC-9A compliance."""
    pre_candidates: FrozenSet[str]
    post_candidates: FrozenSet[str]

    @property
    def tc9a_compliant(self) -> bool:
        return self.pre_candidates == self.post_candidates
```

### Rationale

If hypotheses could add/remove candidates, they could effectively override operator applicability, violating the symbolic layer's authority. TC-9A ensures hypotheses only "advise" on ordering.

### Test Implementation

```python
# tests/conformance/test_tc9a_applicability.py

def test_tc9a_ordering_only():
    """TC-9A: Hypothesis influence only reorders, never adds/removes."""
    candidates_before = get_applicable_operators(state)

    # Apply hypothesis influence
    candidates_after = apply_hypothesis_influence(candidates_before, hypothesis)

    # Same set, possibly different order
    assert set(candidates_before) == set(candidates_after), "TC-9A violated"
```

**Test File**: `tests/conformance/test_tc9a_applicability.py`

---

## TC-10: Registered Interpreters (January 2026)

### Definition

All hypothesis interpreters must be registered with declared contracts. Ad-hoc interpretation of hypotheses is forbidden.

### Requirements

- Interpreters registered via `InterpreterRegistry`
- Each interpreter declares its input/output contracts
- Unregistered interpreters raise `UnregisteredInterpreterError`

### Rationale

Without registration, hypothesis interpretation becomes a "black box" that could violate other constraints. TC-10 ensures all interpretation is auditable.

### Test Implementation

```python
# tests/conformance/test_tc10_registered_interpreters.py

def test_tc10_unregistered_interpreter_blocked():
    """TC-10: Unregistered interpreters cannot be used."""
    registry = InterpreterRegistry()

    # Attempting to use unregistered interpreter should fail
    with pytest.raises(UnregisteredInterpreterError):
        registry.interpret(hypothesis, interpreter_id="unregistered")
```

**Test File**: `tests/conformance/test_tc10_registered_interpreters.py`

---

## TC-11: Prior Lineage Replay Determinism (February 2026)

### Definition

Prior artifacts (value-head weights, feature configs, calibration tables) must be deterministically verifiable via a hash chain. Given the same prior artifact store, any verifier can independently confirm:

1. **Integrity**: The artifact bytes match their declared content hash.
2. **Lineage**: Each artifact's `parent_hash` references a valid predecessor, forming an unbroken chain back to the genesis artifact.
3. **Replay**: Re-running the prior-production pipeline from the same inputs produces the same output hash.

### Requirements

- `PriorVerifierV0` validates artifact integrity and lineage
- Two-phase write protocol: stage artifact, verify hash, then commit
- Genesis artifacts have `parent_hash = None` (chain root)
- Hash algorithm: SHA-256 over canonical byte representation

### Rationale

Without lineage verification, a value head could silently change between training runs, breaking reproducibility guarantees. TC-11 ensures that any deployed prior can be traced back to its training provenance and independently verified.

### Enforcement

`PriorVerifierV0` checks artifact integrity at load time:

```python
class PriorVerifierV0:
    """Verify prior artifact integrity and lineage."""

    def verify(self, artifact_id: str) -> VerificationResult:
        artifact = self._store.load(artifact_id)
        # 1. Content hash matches declared hash
        actual_hash = sha256(artifact.bytes)
        if actual_hash != artifact.content_hash:
            return VerificationResult(valid=False, reason="content_hash_mismatch")
        # 2. Parent chain is valid (if not genesis)
        if artifact.parent_hash is not None:
            parent = self._store.load_by_hash(artifact.parent_hash)
            if parent is None:
                return VerificationResult(valid=False, reason="broken_lineage")
        return VerificationResult(valid=True)
```

### Test Implementation

```python
# tests/conformance/test_tc11v0_prior_lineage.py

def test_tc11_content_integrity():
    """TC-11: Artifact content matches declared hash."""
    store = create_test_artifact_store()
    artifact = store.stage_and_commit(content=b"test_weights", parent_hash=None)
    result = PriorVerifierV0(store).verify(artifact.id)
    assert result.valid, "TC-11 violated: content hash mismatch"

def test_tc11_lineage_chain():
    """TC-11: Artifact lineage chain is unbroken."""
    store = create_test_artifact_store()
    genesis = store.stage_and_commit(content=b"v1_weights", parent_hash=None)
    child = store.stage_and_commit(content=b"v2_weights", parent_hash=genesis.content_hash)
    result = PriorVerifierV0(store).verify(child.id)
    assert result.valid, "TC-11 violated: broken lineage chain"

def test_tc11_broken_lineage_detected():
    """TC-11: Broken lineage is detected and rejected."""
    store = create_test_artifact_store()
    # Artifact references non-existent parent
    artifact = store.stage_and_commit(content=b"orphan", parent_hash="deadbeef")
    result = PriorVerifierV0(store).verify(artifact.id)
    assert not result.valid, "TC-11 violated: broken lineage not detected"

def test_tc11_replay_determinism():
    """TC-11: Same inputs produce same artifact hash."""
    store = create_test_artifact_store()
    a1 = store.stage_and_commit(content=b"deterministic_weights", parent_hash=None)
    a2 = store.stage_and_commit(content=b"deterministic_weights", parent_hash=None)
    assert a1.content_hash == a2.content_hash, "TC-11 violated: non-deterministic hash"
```

**Test File**: `tests/conformance/test_tc11v0_prior_lineage.py`

---

## Governance Mode Conformance (January 2026)

### RunIntent Validation

All conformance tests must pass in both DEV and CERTIFYING modes:

```python
@pytest.mark.parametrize("run_intent", [RunIntent.DEV, RunIntent.CERTIFYING])
def test_tc1_with_run_intent(run_intent):
    """TC-1 must hold regardless of governance mode."""
    with governance_context(run_intent=run_intent):
        test_tc1_no_regression()
```

### Fail-Closed Conformance

In CERTIFYING mode, conformance tests verify fail-closed behavior:

```python
def test_certifying_mode_fail_closed():
    """Certifying mode must fail-closed on missing artifacts."""
    with governance_context(run_intent=RunIntent.CERTIFYING):
        # Missing artifact_store should raise
        with pytest.raises(ValueError, match="artifact_store is required"):
            PromotionLane(shadow_store, artifact_store=None, run_intent="certifying")
```

---

## Relationship to Other Invariants

TC-4 and TC-5 codify:

- Invariants I21-I23 from `docs/semantic_syntactic_intent_invariants.md`
- Key Invariants (2-4) from `docs/v7_5_latent_line_architecture.md`

---

## Test Status

| Test File                              | Tests | Status                        |
| -------------------------------------- | ----- | ----------------------------- |
| `test_tc1_semantic_invariance.py`      | 14    | Implemented                   |
| `test_tc2_ir_only_inputs.py`           | 11    | Implemented                   |
| `test_tc3_student_teacher_gap.py`      | 11    | Implemented                   |
| `test_tc4_no_cot.py`                   | 12    | Implemented                   |
| `test_tc5_latent_advisory.py`          | 12    | Implemented                   |
| `test_tc6a_provenance.py`              | —     | Not yet implemented           |
| `test_tc7a_influence_gate.py`          | —     | Not yet implemented           |
| `test_tc8_invariance.py`               | —     | Not yet implemented           |
| `test_tc9a_applicability.py`           | —     | Not yet implemented           |
| `test_tc10_registered_interpreters.py` | —     | Not yet implemented           |
| `test_tc11v0_prior_lineage.py`         | 7     | Implemented                   |
| **Total (implemented)**                | 67    |                               |

---

## Enforcement

### Pre-Commit

Conformance tests run as part of the standard test suite:

```bash
pytest tests/conformance/ -v
```

### CI/CD

All conformance tests must pass before merge to main.

### Monitoring

Student-teacher gap metrics are tracked over time:

```bash
python scripts/monitor_tc3_gap.py --domain wordnet_v1 --output metrics/tc3_gap.json
```

---

## Related Documents

- `../roadmaps/core_roadmap.md` - Overall roadmap
- `docs/v7_5_latent_line_architecture.md` - Latent line details
- `docs/v7_5_v8_5_comparison.md` - Student-teacher comparison
- `docs/value/hybrid_value_function.md` - Hybrid value architecture
- `docs/guides/study_materials/10_governance_and_lifecycle_deep_dive.md` - Governance deep dive
- `docs/guides/hypothesis_lifecycle_integration.md` - Hypothesis integration guide
- `core/governance/run_intent.py` - RunIntent enum implementation
- `core/induction/promotion_lane.py` - FenceWitness and fail-closed implementation

---

**Author**: @darianrosebrook
**Last Updated**: February 15, 2026
