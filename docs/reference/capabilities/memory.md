---
authority: reference
status: advisory
date: 2026-03-01
capability: memory
parity_audit_sections: "E1, E2, E3"
---

# Memory Substrate

**Advisory — not normative.** This document describes proof obligations for future v2 work. Do not cite as canonical. See [parity audit](../../architecture/v1_v2_parity_audit.md) for capability status.

## Overview

Memory in Sterling encompasses episode identity, state summarization, decay dynamics, and governed lifecycle transitions. The sterling Python repo defined three overlapping memory models: a four-status Semantic Working Memory contract, a three-tier myelin-sheath corridor promotion system, and a decay hierarchy with budget-driven eviction. This reference captures the proof obligations those designs impose on the native substrate, without prescribing which model carries forward.

## Key Concepts

### Four-Status Model

Memory entries move through a lifecycle: **Committed**, **Shadow**, **Weak**, and **Frontier**. Committed entries are durable and verified. Shadow entries are provisionally retained but not yet promoted. Weak entries are subject to eviction under budget pressure. Frontier entries are speculative and unverified.

The planning boundary separates Committed+Shadow (available for deterministic planning) from Weak+Frontier (available only for heuristic advisory). This boundary must be mechanically enforced — a planner must never read Frontier state as if it were Committed.

### MeaningStateDigest Chain

Each memory state transition produces a cryptographic digest binding the prior state to the new episode. The chain structure is: `state_n → episode_evidence → state_{n+1}`. This ensures memory mutations are replay-linkable — given the chain, any intermediate state can be reconstructed and verified.

In the native substrate, this maps naturally to the content-addressing and chain-hash patterns already proven for SearchTapeV1 (step chain digests) and ByteTraceV1 (frame-by-frame replay). The memory chain would use `canonical_hash(HashDomain, &[u8])` with a dedicated memory domain separator.

### Certified Corridors (Myelin Sheath)

Frequently-traversed reasoning paths can be promoted to certified corridors — pre-verified fast paths that skip redundant search. Corridor promotion is itself a governed operation: it requires evidence that the path was certified under a specific policy snapshot, and that the corridor's preconditions still hold.

This is analogous to the operator promotion lifecycle described in the [induction reference](induction.md) (Shadow → Provisional → Production) and should share the same governance surface once implemented.

### Three-Tier Decay Hierarchy

Memory entries are organized into tiers by durability:

- **Anchors**: Long-lived structural knowledge. High retention score. Evicted only under extreme budget pressure.
- **Episodic summaries**: Compressed episode records. Medium retention. Subject to compaction.
- **Scratch**: Transient working state. Low retention. Evicted first under budget pressure.

Decay is budget-driven: a memory budget (analogous to SearchPolicyV1's `max_candidates_per_node`) determines how many entries survive each compaction pass. The assignment of entries to tiers and the computation of retention scores are governed operations with auditable evidence.

### Compaction as Witnessed Operator

Compaction (merging or evicting memory entries) must be a registered operator with a signature, preconditions, and effect contract — not silent garbage collection. Each compaction produces a witness artifact recording what was evicted, what was merged, and the resulting state digest.

This parallels the existing `apply()` contract: operators declare their effects via `EffectKind`, and post-apply validation confirms the declared effects match actual state changes. Memory compaction would declare its effect kind (e.g., `EvictsEntriesByBudget`) and produce a verifiable diff.

## Design Decisions (Open)

| Decision | Options | Constraint |
|----------|---------|------------|
| Which memory model | Four-status SWM, three-tier myelin, hybrid, or new design | Must support content-addressed entries and governed transitions |
| Memory as world or as harness extension | Memory could be a SearchWorldV1 impl or a harness-level service | If world: existing search/verification pipeline applies. If harness: new verification surface needed |
| Compaction operator category | State (S), Knowledge (K), or new Memory (M) category | Must fit OperatorRegistryV1 taxonomy |
| Decay policy location | In PolicySnapshotV1 or separate MemoryPolicyV1 | PolicySnapshotV1 already carries budget knobs; extending it may be simpler |
| Corridor promotion gate | Reuse PromotionGateV1 from induction or define CorridorGateV1 | Shared gate shape reduces API surface |

## Proof Obligations for v2

1. **Content-addressed entries.** Every memory entry has a ContentHash computed via `canonical_hash`. No entry exists without an address.

2. **Replay-linked chain.** Memory state transitions form a hash chain. Given the chain and the initial state, any intermediate state is reconstructable and verifiable.

3. **Governed mutations.** Every memory update (write, compact, evict, promote) is a registered operator in OperatorRegistryV1. No memory state changes without an operator invocation recorded in the execution trace.

4. **Planning boundary enforcement.** The projection of memory available to deterministic planning excludes entries below the planning boundary. This is enforced by API shape (distinct view types), not by convention.

5. **Budget-driven eviction with witness.** Compaction produces a witness artifact recording the eviction decisions, the budget that triggered them, and the resulting state digest. The witness is content-addressed and included in the evidence bundle.

6. **Corridor certification evidence.** A promoted corridor includes the policy snapshot under which it was certified, the evidence bundle(s) demonstrating its validity, and a content hash binding the corridor definition to its evidence.

7. **No silent state loss.** Every entry that existed at state_n is either present at state_{n+1} or accounted for in a compaction witness. The system must not silently drop entries.

## Parity Audit Reference

This document covers capabilities **E1** (Semantic Working Memory), **E2** (Landmarks + compression), and **E3** (Decay / activation dynamics) from the [parity audit](../../architecture/v1_v2_parity_audit.md).

All three are currently **Not started** in the native substrate. The parity audit identifies the following import obligations:

- Episode identity and durable summaries (landmarks or equivalent)
- Governed memory updates (operators or explicit post-pass artifacts)
- Content-addressed, bundle-linked memory artifacts

See also Import Group E (Memory MVP) in the parity audit for the strategic context.
