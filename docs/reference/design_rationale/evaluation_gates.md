---
authority: reference
status: advisory
---

# Evaluation Gates

**Advisory -- not normative.** This document describes proof obligations for
Sterling's research evaluation gates. Do not cite as canonical. See
[parity audit](../../architecture/v1_v2_parity_audit.md) for capability
status.

## What Are Evaluation Gates?

Evaluation gates are research targets that must be demonstrated, not
architectural invariants that block experimentation. They represent the
success criteria for Sterling as a research project.

- Measured regularly through benchmarks and experiments
- Trigger investigation when violated
- Do NOT block merge or deploy (unlike core constraints)

## Why Gates, Not Invariants?

Core constraints define what Sterling IS (architectural invariants). Evaluation
gates define what Sterling ACHIEVES (research targets). You can satisfy all
core constraints and still fail evaluation gates -- that means you have built
Sterling correctly but it does not work well enough yet. You cannot satisfy
evaluation gates while violating core constraints -- that would mean achieving
the goals by cheating.

## EVAL-01: Compute Parity

**Question:** Does Sterling use less compute than a transformer baseline for
the same task?

**Why it matters:** If Sterling uses more compute than running an LLM
directly, it is a more complex transformer proxy. The thesis is efficiency
through structure.

**Proof obligation:** For a fixed long-horizon task, measure token usage per
episode, number of transformer forward passes, latency, and task success rate.
Sterling must match or beat the baseline on quality with significantly fewer
tokens and calls.

**Failure response:** Investigation sprint to identify bottlenecks, then
optimization.

**Relation to success rubric:** EVAL-01 validates the core compute thesis.
The existing search engine (bounded best-first with budget caps) already
operates with zero transformer calls during the search loop. The gate
becomes meaningful when language I/O is wired and transformer calls can be
counted end-to-end.

## EVAL-02: Long-Horizon State

**Question:** Where does Sterling store information about what has been done
and what remains?

**Why it matters:** If long-horizon state lives in prompts or transcripts,
Sterling is just doing context-window management. The thesis is that state
should be structural.

**Proof obligation:** Audit where task state is stored. Verify that reasoning
does not require re-reading transcripts. Confirm episode summaries are used as
first-class objects.

**Failure response:** Architecture review to identify where structural state
is leaking into text.

**Relation to success rubric:** EVAL-02 validates the structural-state
thesis. The existing SearchGraphV1 and expansion event log already represent
state structurally. The gate becomes meaningful when multi-episode reasoning
is implemented.

## EVAL-03: Reasoning Substrate

**Question:** Is Sterling doing the reasoning, or is it just wrapping an LLM?

**Why it matters:** If the LLM makes all decisions and Sterling is
scaffolding, nothing new has been built.

**Proof obligation:** Count decisions made by symbolic search vs LLM. Verify
LLM is used only at I/O boundaries. Confirm state transitions come from
operators, not LLM generation.

**Failure response:** Roadmap correction to identify where LLM is being used
for reasoning.

**Relation to success rubric:** EVAL-03 validates the reasoning-substrate
thesis. The existing search loop makes all decisions via operator apply,
value scoring, and frontier management with zero LLM involvement. The gate
becomes meaningful when realization layers (language I/O, neural components)
are added.

## Measurement Cadence

| Gate | Frequency | Owner |
|------|-----------|-------|
| EVAL-01 | Per benchmark run | Evaluation scripts |
| EVAL-02 | Per architecture change | Code review |
| EVAL-03 | Per feature addition | Code review + tests |

## Relationship to Core Constraints

```
Core Constraints (hard, merge-blocking)
    |
    | satisfying constraints is necessary but not sufficient
    v
Evaluation Gates (soft, investigation-triggering)
    |
    | sustained failure triggers roadmap correction
    v
Success Rubric (project-level goals)
```

The gates are the bridge between "built correctly" and "works well enough."
Persistent failure at any gate should prompt a structured investigation, not
a workaround.
