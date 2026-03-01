# Reference Docs (Advisory, Non-Authoritative)

These documents describe **proof obligations**, **design rationale**, **world design framing**, and **historical context** for Sterling Native. They are explicitly non-normative — never cite as canonical requirements.

Every file in this directory carries `authority: reference` front-matter.

## Why these exist

Sterling v1 (Python) produced a catalog of proof obligations that the v2 substrate must eventually host. These docs preserve the valuable framing — what must be proven, what design decisions remain open, what the acceptance criteria look like — without carrying forward v1 implementation details or version-specific vocabulary.

## Schema version vs architecture generation

`V1` suffixes on artifact types (ByteStateV1, SearchTapeV1, OperatorRegistryV1, etc.) are **schema version labels**, not architecture generation markers. They indicate the wire format version of that specific artifact. See [`docs/canonical/glossary.md`](../canonical/glossary.md) §Naming Conventions.

## External v1 baseline

For raw v1 source material, the Sterling (Python) repo at commit `9b9c0daa` is the pinned baseline. These in-repo docs are version-agnostic rewrites, not copies.

## Directory structure

```
capabilities/          Proof obligations v2 must eventually host
  memory.md            SWM, landmarks, decay
  text_boundary.md     Text IR, realization trust boundary
  induction.md         Operator induction pipeline
  governance.md        Certification campaigns, verdicts, tool transcripts
  discourse.md         Intent/speech act system
  knowledge_graph.md   KG contract, entity/relation model

world_design/          How to build worlds + unproven capability axes
  capability_axes.md   10 orthogonal axes with acceptance criteria
  world_catalog.md     Planned worlds, status, which axes each exercises
  promotion_gates.md   D0–D4 ladder, CPG-0 through CPG-8 gates

design_rationale/      Enduring design philosophy and framing
  absorption_pipeline.md  6-step capability absorption + differentiators
  operator_policy.md      Operators as policies, evaluation contract
  value_architecture.md   Composable value heads, hybrid combiner design space
  conformance.md          TC-1 through TC-11 theory conformance obligations
  evaluation_gates.md     EVAL-01/02/03 research evaluation gates
  search_complexity.md    Big-O analysis of SC-001 search implementation

historical/            Context only, no action items
  retrospective.md     v1 retrospective and lessons learned
  north_star.md        Original thesis (still valid)
```

## Relationship to canonical docs

Canonical docs (`docs/canonical/`) are binding. These reference docs are advisory. The [parity audit](../architecture/v1_v2_parity_audit.md) maps capabilities between the two and tracks proof obligations.
