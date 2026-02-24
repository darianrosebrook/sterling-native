# Sterling Future State Vision: January 2027

**Date**: 2027-01-27 (Target)
**Baseline**: 2026-01-27 (Current)
**Author**: @darianrosebrook
**Status**: Planning Document

---

## Executive Summary

This document captures a hypothetical "fully realized" Sterling narrative and maps it against current implementation state. The goal is to provide a concrete roadmap from where we are (~76% realized) to the complete vision.

The core thesis: **Reliability comes from proof-carrying artifacts, deterministic replay, and promotion gates—not from persuasive generations.**

---

## The Five-Act Narrative

### Act 1: The Pivot from "Reasoning Quality" to "Mechanical Trust"

> The origin story isn't that the system started making fewer mistakes; it's that you stopped letting correctness be a property of text output. The early work (strict/fail-closed seams, typed artifact resolution, witness-first semantics, structural replay determinism) becomes the moment Sterling stopped being "an agent" and became "a governed machine."

**Current Realization: ~90%**

| Capability | Status | Evidence |
|------------|--------|----------|
| No free-form CoT in decision loop | COMPLETE | `model_gateway.py`, `SearchPhaseModelCallError`, TC-4 conformance tests |
| Strict/fail-closed seams | COMPLETE | `RunIntent.is_strict`, `GovernanceContext` construction guards |
| Typed artifact resolution | COMPLETE | `ArtifactRefV1`, key-index, hash-index with CAS primitives |
| Witness-first semantics | COMPLETE | `record_and_raise()` pattern, dual-hash (semantic + record_id) |
| Structural replay determinism | ~85% | Framework complete; real-world tests deferred |

**Remaining Work**:
- [ ] Create real-world determinism tests (serial vs. parallel vs. sequential execution)
- [ ] Verify determinism under memory pressure across multiple runs
- [ ] Document determinism guarantees with concrete reproduction instructions

---

### Act 2: Evidence Packets Became the Unit of Progress

> In the successful timeline, "evidence packet design" becomes the organizing primitive for engineering and for belief. Each capability claim is inseparable from the packet that demonstrates it.

**Current Realization: ~75%**

| Component | Status | Evidence |
|-----------|--------|----------|
| Fixed task world + dataset provenance | COMPLETE | `WorldCapabilities`, `ValidationFixtureProvider`, versioned benchmarks |
| Budgeted run configuration | PARTIAL | `ExternalRef` budget exists; run config not canonicalized into packets |
| Trace bundle with content-addressed closure | COMPLETE | `StateGraph` append-only, `artifact_closure_hash`, `provenance_chain.py` |
| Verification harness | ~70% | 4-phase verification in `h2_evidence_bundle.py`; Phase D (replay) optional |
| Promotion decision with witness trail | COMPLETE | `FenceWitness`, `PromotionResult`, `CertificationBlocker` |

**Remaining Work**:
- [ ] Define `EvidencePacketV1` as first-class concept unifying:
  - Task world specification
  - Dataset provenance with content hash
  - Run configuration (budget, timeouts, feature mode)
  - Trace bundle (StateGraph + semantic deltas)
  - Verification harness configuration
  - Promotion decision with witness
- [ ] Make nightly replay verification mandatory (not optional Phase D)
- [ ] Add manifest-driven episode dependency resolution
- [ ] Create `sterling packet verify <packet_id>` CLI command

**Target Artifact Schema**:
```python
@dataclass
class EvidencePacketV1:
    """First-class evidence packet for capability claims."""
    packet_id: str  # sha256 of canonical payload
    schema_id: str = "sterling.evidence_packet.v1"

    # What was tested
    world_id: str
    task_type: str
    dataset_ref: ArtifactRefV1

    # How it was run
    run_config: RunConfigV1
    governance_context_hash: str

    # What happened
    trace_bundle_ref: ArtifactRefV1
    semantic_deltas: List[str]  # refs to SemanticDeltaIR

    # Verification
    verification_level: VerificationLevel  # INTEGRITY, REPLAYED, REPLAYABLE
    verification_report_ref: ArtifactRefV1

    # Decision
    promotion_decision: Optional[PromotionResult]
    witnesses: List[str]  # refs to witness artifacts
```

---

### Act 3: Promotion Lanes Turned Learning into a Governed Supply Chain

> A year later, the story will highlight that you built a pipeline where new behaviors don't "appear," they get inducted. The artifact store + lane mechanics function like a CI/CD system for cognition: certify → promote → freeze → replay.

**Current Realization: ~85%**

| Capability | Status | Evidence |
|------------|--------|----------|
| Operators with typed contracts | COMPLETE | `OperatorSignature`, versioned precondition/effect IDs |
| Measurable discriminative value (K1) | COMPLETE | `k1_metrics.py`, EscapeGame certification reference |
| Datasets as production dependencies | ~80% | Versioned benchmarks; no manifest-driven resolution |
| Strictness at construction | COMPLETE | `GovernanceContext.__init__` fail-closed guards |
| CI/CD for cognition | COMPLETE | `PromotionLane` → `ShadowStore` → `CertifiedStore` pipeline |
| Improvement without trust degradation | COMPLETE | `PriorInfluenceIR`, registry hash, universe hash snapshots |

**Remaining Work**:
- [ ] Create operator bill-of-materials view (`OperatorManifestV1`)
- [ ] Complete K7 induced operator synthesis pipeline
- [ ] Add dataset dependency manifest with transitive resolution
- [ ] Implement "promotion audit log" queryable by time range

**Target: Operator Manifest**:
```python
@dataclass
class OperatorManifestV1:
    """Bill of materials for operators in a certified bundle."""
    manifest_id: str
    schema_id: str = "sterling.operator_manifest.v1"

    # Production operators
    production_operators: List[OperatorEntry]

    # Induced operators (with full provenance)
    induced_operators: List[InducedOperatorEntry]

    # Dependency graph
    operator_dependencies: Dict[str, List[str]]

    # Certification chain
    certificate_refs: List[str]

    @dataclass
    class OperatorEntry:
        operator_id: str
        signature_hash: str
        world_id: str
        k1_discriminativity: Optional[float]

    @dataclass
    class InducedOperatorEntry(OperatorEntry):
        source_hypothesis_ref: str
        promotion_certificate_ref: str
        evidence_packet_refs: List[str]
```

---

### Act 4: The Second Iteration is About Scaling Trust, Not Adding Features

> If you already iterated "once more," that iteration likely isn't about new worlds first. It's about eliminating friction and footguns in the governance loop.

**Current Realization: ~60%**

| Capability | Status | Evidence |
|------------|--------|----------|
| Better canonicalization | IN PROGRESS | `CANONICALIZATION_VERSION`, cluster key normalization, evidence sorting |
| Transition-level semantic replay | PARTIAL | Gate S (claim-field) defined; semantic checks scaffolded |
| Runtime vs. governance separation | COMPLETE | `kg_ref` pattern, `StateNode` vs `SearchNode`, universe snapshots |
| Witness ergonomics (review in minutes) | NOT STARTED | Witnesses exist but no human-facing tooling |

**Remaining Work**:
- [ ] Implement transition-level semantic replay (beyond structural determinism)
- [ ] Create witness review CLI: `sterling witness show <witness_id>`
- [ ] Create promotion review CLI: `sterling promotion review <certificate_id>`
- [ ] Add "why was this promoted/rejected?" query tool
- [ ] Implement float quantization audit (detect precision-related nondeterminism)
- [ ] Create mode provenance population in `InstrumentedSearchResult`
- [ ] Build parallel/sequential emission audit parity tests

**Target: Witness Review Tooling**:
```bash
# Show a specific witness
$ sterling witness show wit_abc123
Witness: wit_abc123
Type: GovernanceFailureWitness
Gate: promotion_regression_fence
Verdict: FAIL
Semantic Hash: sha256:def456...

Failure Details:
  - Code: REGRESSION_DETECTED
  - Operator: pn.normalize_copula.v2
  - Baseline Score: 0.847
  - Current Score: 0.812
  - Delta: -0.035 (threshold: 0.02)

Evidence Chain:
  1. Baseline run: run_20260115_abc
  2. Current run: run_20260127_def
  3. Comparison artifact: cmp_ghi789

# Review a promotion decision
$ sterling promotion review cert_xyz789
Certificate: cert_xyz789
Operator: wordnet.hypernym_hop.v3
Decision: PROMOTED
Tier: PROVISIONAL → PRODUCTION

Evidence Summary:
  - K1 Discriminativity: 0.892 (threshold: 0.70)
  - Regression Fence: PASSED
  - Fixture Coverage: 47/47 (100%)

Witnesses (3):
  1. fence_execution_wit_001 [EXECUTED]
  2. k1_validation_wit_002 [PASSED]
  3. fixture_coverage_wit_003 [COMPLETE]

View full evidence packet: sterling packet show pkt_uvw456
```

---

### Act 5: The Punchline—Sterling Changed How Work Gets Done

> The payoff isn't only that Sterling solves tasks. It's that the organization around it stops being dependent on personal heroics and starts being dependent on a process that produces truth you can ship.

**Current Realization: ~70%**

| Capability | Status | Evidence |
|------------|--------|----------|
| Language as ingress/egress, not authority | COMPLETE | LLM only at I/O boundaries, `model_gateway.py` |
| Correctness in mechanics, not model | ~90% | 10 core constraints + 12 governance realizations |
| Promotion is mechanically enforceable | COMPLETE | `TD12MSCertificate`, `PromotionGate`, `CertifiedOperatorStore` |
| Every claim has replayable packet | PARTIAL | Claims map to code + tests; not all have self-contained bundles |

**Remaining Work**:
- [ ] Create claim → packet index (which packet proves which claim?)
- [ ] Build "capability registry" mapping README claims to evidence packets
- [ ] Implement "truth table" for all claimed capabilities
- [ ] Document operational runbook for "debating packets, not outputs"

**Target: Capability Registry**:
```yaml
# docs/evidence/capability_registry.yaml
schema: sterling.capability_registry.v1
capabilities:
  - id: CAP-001
    claim: "29x-600x faster than GPT-4 on WordNet navigation"
    evidence_packet: pkt_wordnet_perf_001
    verification_level: REPLAYED
    last_verified: 2026-01-27

  - id: CAP-002
    claim: "No free-form chain-of-thought in decision loop"
    evidence_packet: pkt_cot_constraint_001
    verification_level: REPLAYABLE
    last_verified: 2026-01-27
    negative_control: test_model_call_blocked_during_search

  - id: CAP-003
    claim: "Promotion is cryptographically bound"
    evidence_packet: pkt_td12_cert_001
    verification_level: REPLAYED
    last_verified: 2026-01-27
```

---

## Gap Summary

| Act | Current | Target | Gap |
|-----|---------|--------|-----|
| Act 1: Mechanical Trust | 90% | 100% | Real-world determinism tests |
| Act 2: Evidence Packets | 75% | 100% | Unified packet concept, mandatory replay |
| Act 3: Governed Supply Chain | 85% | 100% | Operator manifest, K7 synthesis |
| Act 4: Scaling Trust | 60% | 100% | Semantic replay, review tooling |
| Act 5: Changed Work | 70% | 100% | Claim registry, process adoption |
| **Overall** | **76%** | **100%** | **~6-12 weeks focused work** |

---

## Implementation Phases

### Phase 1: Diagnostic Hardening (2-4 weeks)
**Goal**: Complete Act 1, foundation for everything else

- [ ] Real-world determinism test suite
  - Serial execution baseline
  - Parallel execution comparison
  - Sequential (different order) comparison
  - Memory pressure stress tests
- [ ] Mode provenance in `InstrumentedSearchResult`
- [ ] Parallel/sequential emission audit parity tests
- [ ] Float quantization precision audit

**Exit Criteria**: All determinism tests pass; no flaky governance tests

### Phase 2: Evidence Packet Unification (3-5 weeks)
**Goal**: Complete Act 2, make packets the unit of progress

- [ ] Define `EvidencePacketV1` schema
- [ ] Implement packet builder in `core/proofs/evidence_packet.py`
- [ ] Create packet verification CLI
- [ ] Make Phase D (replay) mandatory for promotion
- [ ] Add episode dependency manifest

**Exit Criteria**: Every promotion produces a self-contained evidence packet

### Phase 3: Operator Supply Chain (3-4 weeks)
**Goal**: Complete Act 3, operators have full provenance

- [ ] Implement `OperatorManifestV1`
- [ ] Complete K7 induced operator synthesis
- [ ] Add dataset dependency resolution
- [ ] Create promotion audit log

**Exit Criteria**: `sterling manifest show` displays complete operator provenance

### Phase 4: Human-Facing Tooling (4-6 weeks)
**Goal**: Complete Act 4, witnesses are reviewable

- [ ] `sterling witness show` CLI
- [ ] `sterling promotion review` CLI
- [ ] `sterling packet verify` CLI
- [ ] Web dashboard for operator provenance (optional)

**Exit Criteria**: Non-engineer can review promotion decision in <5 minutes

### Phase 5: Process Adoption (Ongoing)
**Goal**: Complete Act 5, team operates on packets

- [ ] Create capability registry
- [ ] Document "debating packets" workflow
- [ ] Train team on new tools
- [ ] Establish packet review as PR requirement

**Exit Criteria**: New capabilities require evidence packet before merge

---

## Success Metrics

### Quantitative
- 100% of promotion decisions have evidence packets
- Witness review time < 5 minutes for standard cases
- 0 governance test flakiness over 30-day window
- 100% claim coverage in capability registry

### Qualitative
- "What's the evidence?" is the default question
- Promotions are reviewed via packet, not ad-hoc testing
- Regressions are caught by fences, not users
- New team members can verify claims independently

---

## Tradeoffs Acknowledged

Even in the "thesis achieved" future, these tradeoffs remain explicit:

1. **Cost**: Building auditable systems costs more up front. This is deliberate.

2. **Scope**: Some classes of "creative" or open-ended work remain outside the strict certification path. The boundary is explicit, not hidden.

3. **Complexity**: The governance system adds cognitive overhead. The bet is that this overhead is paid back in trust and debuggability.

4. **Speed**: Mandatory verification slows individual changes. The bet is that it speeds overall progress by preventing regressions.

---

## References

- `docs/evidence/sterling_capabilities_evidence_packet.md`: Current evidence packet
- `docs/reference/governance/STERLING_REALIZATIONS_JAN_2026.md`: 12 governance realizations
- `docs/reference/governance/governance-hardening-skein-v1.md`: Governance hardening spec
- `CLAUDE.md`: Development guidance and architecture overview

---

## Appendix: The Full Narrative

For reference, here is the complete hypothetical narrative this document works toward:

> **Act 1**: The origin story isn't that the system started making fewer mistakes; it's that you stopped letting correctness be a property of text output. The early work (strict/fail-closed seams, typed artifact resolution, witness-first semantics, structural replay determinism) becomes the moment Sterling stopped being "an agent" and became "a governed machine." The thesis that survives in hindsight is: reliability comes from proof-carrying artifacts, deterministic replay, and promotion gates—not from persuasive generations.

> **Act 2**: In the successful timeline, "evidence packet design" becomes the organizing primitive for engineering and for belief. Each capability claim ("Sterling can do X") is inseparable from the packet that demonstrates it: a fixed task world + dataset provenance, a budgeted run configuration, a trace bundle with content-addressed closure, a verification harness, and a promotion decision with an explicit witness trail. The team's day-to-day shifts: instead of debating outputs, you debate packets. That changes culture.

> **Act 3**: A year later, the story will highlight that you built a pipeline where new behaviors don't "appear," they get inducted. Operators are introduced with typed contracts and measurable discriminative value. Datasets are treated like production dependencies: versioned, content-addressed, and promotion-gated. "Strictness at construction" becomes a design law. The artifact store + lane mechanics function like a CI/CD system for cognition: certify → promote → freeze → replay. The key narrative beat is that Sterling starts improving without becoming less trustworthy.

> **Act 4**: If you already iterated "once more," that iteration likely isn't about new worlds first. It's about eliminating friction and footguns in the governance loop: better canonicalization, more precise transition replay, clear separation of runtime state vs governance facts, stronger witness ergonomics. This is where Sterling becomes legible to other engineers: it reads like an audited system, not a research artifact.

> **Act 5**: The payoff isn't only that Sterling solves tasks. It's that the organization around it stops being dependent on personal heroics and starts being dependent on a process that produces truth you can ship. You'd likely say: "We treated language as ingress/egress, not authority." "We moved correctness from the model to the mechanics." "We made promotion a mechanically enforceable event." "We built a system where every meaningful claim has a replayable packet behind it."
