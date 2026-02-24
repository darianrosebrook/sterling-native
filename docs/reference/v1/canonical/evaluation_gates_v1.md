> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Evaluation Gates v1

**This is a canonical definition. Do not edit without a version bump or CANONICAL-CHANGE PR label.**

---

## What Are Evaluation Gates?

Evaluation gates are **research targets that must be demonstrated**, not architectural invariants that block experimentation.

They are:
- **Measured regularly** through benchmarks and experiments
- **Trigger investigation** when violated
- **Do NOT block merge/deploy** like Core Constraints do

## Why Gates, Not Invariants?

Invariants (INV-CORE-xx) are hard constraints that block merge/deploy. You can't ship code that violates them.

Evaluation gates are different: they represent the **success criteria** for Sterling as a research project. You can't require "already won" as a precondition for experimentation.

Example: "Compute Parity" means Sterling should match/beat transformer baselines. But we can't block all development until we've already achieved that - we need to iterate toward it.

## The Three Evaluation Gates

| ID | Gate | What It Measures | Success Criterion | Failure Response |
|----|------|------------------|-------------------|------------------|
| EVAL-01 | Compute Parity | Sterling's efficiency vs transformers | Match/beat transformer baselines in compute per task | Investigation + optimization sprint |
| EVAL-02 | Long-Horizon State | Where state lives | All long-horizon state in KG + summaries, not prompts/transcripts | Architecture review |
| EVAL-03 | Reasoning Substrate | What does the reasoning | Sterling demonstrably replaces LLM reasoning, not wraps it | Roadmap correction |

## EVAL-01: Compute Parity

**Question**: Does Sterling use less compute than a transformer baseline for the same task?

**Why it matters**: If Sterling uses MORE compute than just running an LLM, we've built a more complex transformer proxy. The whole point is efficiency through structure.

**How to measure**:
- Token usage per episode
- Number of transformer forward passes
- Latency
- Accuracy / task success

**Success looks like**: Same or better accuracy with significantly fewer tokens + calls.

**Failure response**: Investigation sprint to identify bottlenecks, then optimization.

## EVAL-02: Long-Horizon State

**Question**: Where does Sterling store information about "what have we done / what's left"?

**Why it matters**: If long-horizon state lives in prompts/transcripts, we're just doing context-window management. Sterling's thesis is that state should be structural.

**How to measure**:
- Audit where task state is stored
- Check if reasoning requires re-reading transcripts
- Verify episode summaries are used as first-class objects

**Success looks like**: All long-horizon tracking passes through episode summaries and path algebra, not text logs.

**Failure response**: Architecture review to identify where structural state is leaking into text.

## EVAL-03: Reasoning Substrate

**Question**: Is Sterling doing the reasoning, or is it just wrapping an LLM?

**Why it matters**: If the LLM is still making all the decisions and Sterling is just scaffolding, we haven't built anything new.

**How to measure**:
- Count decisions made by symbolic search vs LLM
- Verify LLM is only used at I/O boundaries
- Check that state transitions come from operators, not LLM generation

**Success looks like**: Sterling's path algebra + value functions make the decisions; LLM only parses input and renders output.

**Failure response**: Roadmap correction to identify where LLM is being used for reasoning.

## Measurement Cadence

| Gate | Measurement Frequency | Owner |
|------|----------------------|-------|
| EVAL-01 | Per benchmark run | Evaluation scripts |
| EVAL-02 | Per architecture change | Code review |
| EVAL-03 | Per feature addition | Code review + tests |

## Relationship to Core Constraints

Evaluation gates and core constraints work together:

- **Core Constraints** (INV-CORE-xx): Define what Sterling IS (architectural invariants)
- **Evaluation Gates** (EVAL-xx): Define what Sterling ACHIEVES (research targets)

You can satisfy all core constraints and still fail evaluation gates. That means you've built Sterling correctly but it doesn't work well enough yet.

You cannot satisfy evaluation gates while violating core constraints. That would mean you've achieved the goals by cheating.

## Gate Status Tracking

Gate status should be tracked in:
- Benchmark reports
- Release notes
- Roadmap documents

Format:
```
EVAL-01: PASS/FAIL/PARTIAL - [date] - [evidence link]
EVAL-02: PASS/FAIL/PARTIAL - [date] - [evidence link]
EVAL-03: PASS/FAIL/PARTIAL - [date] - [evidence link]
```
