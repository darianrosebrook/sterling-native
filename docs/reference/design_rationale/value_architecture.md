---
authority: reference
status: advisory
---

# Value Architecture

**Advisory -- not normative.** This document describes design rationale for
Sterling's composable value function architecture. Do not cite as canonical.
See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability
status.

## HybridValueFunction Architecture

The value function estimates quality of search states to guide reasoning
search. The architecture is composable: individual component heads each
produce a score, and the hybrid function combines them with configurable
weights. This supports ablation studies, progressive capability addition, and
domain-specific tuning.

### Seven Composable Heads

| Head | Name | Signal |
|------|------|--------|
| 1 | Structural | Degree normalization, depth progress, goal distance |
| 2 | Memory | SWM-based novelty, recency, visit penalty |
| 3 | Task | Task-specific reward shaping |
| 4 | Latent | Learned embedding similarity (advisory only) |
| 5 | DialogueOperator | Discourse-aware operator scoring |
| 6 | MDL Parsimony | Minimum description length penalty for complex solutions |
| 7 | PragmaticsPrior | Gricean maxims and discourse conventions |

Composition formula: active weights are normalized at evaluation time. Only
enabled components participate. Scores are in [0, 1] by convention.

### Key Invariants

- **V-1:** All component evaluate() methods return values in [0, 1].
- **V-2:** HybridValueFunction normalizes weights -- only active components
  participate.
- **V-3:** With latent disabled, behavior is identical to pre-latent
  baseline.
- **V-4:** *(Gap — not documented in original v1 source; reserved for future
  assignment.)*
- **V-5:** Dialogue scoring fallback is never silent -- always records
  reason.

## 39-Dimensional Operator Feature Vector

Operator signatures are encoded as fixed-dimension feature vectors for
comparison and scoring:

| Feature Group | Dimensions | Content |
|---------------|-----------|---------|
| Category | 5 | S, M, P, K, C |
| Scope | 3 | utterance, discourse, world |
| Layer (reads + writes) | 5+5 | syntax, semantics, semiotics, pragmatics, latent |
| Preconditions | 13 | From operator registry |
| Effects | 5 | From operator registry |
| Label constraints | 3 | From operator registry |
| **Total** | **39** | |

Provides cosine similarity, operator similarity, and nearest-neighbor lookup
for operator comparison.

## ConfigurableValueModel

Alternative scoring model with explicit formula:

```
combined = value_weight * value_score
         - heuristic_weight * heuristic_distance
         - penalties
```

Ablation toggles (enable_value, enable_heuristic) allow systematic
component-by-component evaluation. Loop penalty, depth penalty, and dead-end
penalty are configurable.

## MDL Component

Minimum description length parsimony penalty penalizes overly complex solutions:

```
MDL_total = L_struct + L_params + L_ex
```

- L_struct: Structure cost (program length, key count, nesting depth)
- L_params: Parameter cost (free variables, disjunctions, optional clauses)
- L_ex: Exception cost (failures weighted by episode weights)

Lower MDL yields higher score (reward simpler explanations).

## Design Space for Future Work

The existing codebase already provides two concrete ValueScorer
implementations:

- **UniformScorer:** All candidates receive equal score. This is the
  mandatory baseline.
- **TableScorer:** Per-action scoring via content-addressed lookup tables
  with integer-only bonuses.

The design question for future work is whether to port the full seven-head
component system or redesign around the ValueScorer trait:

| Approach | Advantage | Risk |
|----------|-----------|------|
| Port component system | Preserves ablation toggles, proven composition formula | Complexity; seven heads may not all apply to new domains |
| Redesign around ValueScorer | Simpler interface, already integrated with search | Loses component-level ablation and MDL/pragmatics heads |
| Hybrid: ValueScorer wrapping components | Best of both; components produce table entries | Requires careful interface design |

The proof obligation for any approach: the identifiability gate (from operator
policy) must demonstrate that the learned scorer outperforms UniformScorer on
held-out instances before deployment.
