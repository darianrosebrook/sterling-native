# Primitive Inventory

Generated: 2026-02-22
Scope: all `test-scenarios/`, `core/capsules/`, `core/proofs/`, `core/operators/`

Machine-generated from capsule specs, builders, and schema definitions.
See `primitive-inventory.ndjson` for structured data.

---

## 1. Capsule Identities

| capsule_id | contract_version | owner_surface | builder | status | gate |
|-----------|------------------|---------------|---------|--------|------|
| `p01_plan_trace` | `p01_plan_trace@1.0` | `core/capsules/p01_spec.py` | `build_p01_capsule_spec` | promoted | — |
| `p22` | `p22@0.1` | `test-scenarios/perceptual-substrate-demo` | `build_p22_capsule_spec` | partial (D1) | CPG-0..6 |
| `p22.di` | `p22.di@1.0` | `test-scenarios/induction-synthesis-demo` | `build_p22_di_capsule_spec` | promoted | CPG-0..8 |
| `p14.ps` | `p14.ps@1.0` | `test-scenarios/induction-synthesis-demo` | `build_p14_ps_capsule_spec` | promoted | CPG-0..8 |
| `p23.di2` | `p23.di2@1.0` | `test-scenarios/dips-02-internalization` | not yet implemented | spec_only | — |
| `p15.ps2` | `p15.ps2@1.0` | `test-scenarios/dips-02-internalization` | not yet implemented | spec_only | — |
| `p24.tr2` | `p24.tr2@1.0` | `test-scenarios/dips-02-internalization` | not yet implemented | spec_only | — |
| `p06` | `p06@1.0` | `test-scenarios/swm-io-demo` | `build_p06_capsule_spec` | promoted | CPG-0..8 |
| `p25.lo` | `p25.lo@1.0` | `test-scenarios/ling-ops-demo` | `build_p25_lo_capsule_spec` | promoted | CPG-0..8 |
| `p26.vrf2` | `p26.vrf2@1.0` | `test-scenarios/dips-02-internalization` | not yet implemented | spec_only | — |
| `p27.ea` | `p27.ea@1.0` | `test-scenarios/graphing-calc-demo` | `build_p27_ea_capsule_spec` | promoted | CPG-0..8 |

**Collision check**: No duplicate `(capsule_id, contract_version)` pairs.

---

## 2. Operator Library Namespaces

### Predicate semantics (`pred.*`)

Reserved by: `PredicateLibraryV1` (DIPS-02, spec only)

| semantics_id | op_name | status |
|-------------|---------|--------|
| `pred.le.int` | `le` | spec_only |
| `pred.eq.int` | `eq` | spec_only |
| `pred.lt.int` | `lt` | spec_only |
| `pred.and.bool` | `and` | spec_only |
| `pred.or.bool` | `or` | spec_only |
| `pred.not.bool` | `not` | spec_only |
| `pred.in_range.int` | `in_range` | spec_only |
| `pred.add.int` | `add` | spec_only |
| `pred.mul.int` | `mul` | spec_only |

### Delta semantics (`delta.*`)

Reserved by: `DeltaLibraryV1` (DIPS-02, spec only)

| semantics_id | op_name | status |
|-------------|---------|--------|
| `delta.assign.int` | `assign` | spec_only |
| `delta.add_const.int` | `add_const` | spec_only |
| `delta.sub_const.int` | `sub_const` | spec_only |

### Core operator semantics (`sterling.effect.*`)

Reserved by: `core/operators/effects.py` (implemented)

| effect_id | status |
|-----------|--------|
| `sterling.effect.syntax_modified.v1` | implemented |
| `sterling.effect.semantics_modified.v1` | implemented |
| `sterling.effect.polarity_flipped.v1` | implemented |
| `sterling.effect.sense_resolved.v1` | implemented |
| `sterling.effect.entity_activated.v1` | implemented |

### Operator categories

Reserved by: `core/operators/registry_types.py` (implemented)

- `S` (Structural), `M` (Meaning), `P` (Pragmatic), `K` (Knowledge), `C` (Control)

### P22 operator families

Reserved by: `primitives_proposed/p22/formal_spec_p22.py`

- `S22`, `K22`, `P22`, `C22`

**Collision check**: No namespace overlaps between `pred.*`, `delta.*`, `sterling.effect.*`, and operator categories.

---

## 3. Failure Codes

Source: `core/proofs/shared/failures.py` — `FailureCode` enum

| Code | Used by capsules | Detail types |
|------|-----------------|-------------|
| `SCHEMA_MISMATCH` | p22.di | `DICompilationFailureV1` |
| `REGISTRY_MISMATCH` | p22.di | `DICompilationFailureV1` |
| `UNKNOWN_CONCEPT` | — | — |
| `CONSTRAINT_VIOLATION` | p15.ps2, p24.tr2 | `PSVerificationFailureV1` (spec) |
| `EVIDENCE_INVALID` | p22.di, p23.di2 | `DIEvidenceInvalidV1` |
| `EVIDENCE_INSUFFICIENT` | p22.di, p23.di2 | `DIEvidenceInsufficientV1` |
| `PROBE_SUITE_MISSING` | p22.di | — |
| `PROBE_FAILED` | p22.di | `DIProbeFailureV1` |
| `DRIFT_DETECTED` | p22.di | `DIDriftDetectedV1` |
| `BUDGET_EXHAUSTED` | p14.ps, p15.ps2, p24.tr2 | `PSBudgetExhaustedV1` |
| `ILLEGAL_OPERATOR_REF` | p14.ps, p15.ps2 | `PSIllegalOperatorRefV1` |
| `UNRESOLVED_UNKNOWNS` | p14.ps, p15.ps2, p23.di2 | `PSUnresolvedUnknownsV1` |
| `INTERNAL_INVARIANT_BROKEN` | all | `PSInternalInvariantBrokenV1` |

DIPS-02 failure detail types (spec only, not yet implemented):
- `DI2EvidenceInvalidV1`, `DI2EvidenceInsufficientV1`, `DI2UnresolvedUnknownsV1`, `DI2IllegalOperatorV1`, `DI2InternalInvariantBrokenV1`
- `PS2BudgetExhaustedV1`, `PS2IllegalOperatorRefV1`, `PS2ConstraintViolationV1`, `PS2UnresolvedUnknownsV1`, `PS2InternalInvariantBrokenV1`
- `TR2SchemaMismatchV1`, `TR2SurfaceCheckFailedV1`, `TR2AlignmentRefusedV1`, `TR2UnknownConceptV1`, `TR2ConstraintViolationV1`, `TR2BudgetExhaustedV1`, `TR2InternalInvariantBrokenV1`

**Note**: DIPS-02 reuses `FailureCode` enum values from DIPS-01 but introduces capsule-specific detail types. No new failure codes are needed.

---

## 4. Policy Artifacts

| Policy type | Source | Status | Used by |
|-------------|--------|--------|---------|
| `InductionPolicyV1` | `core/proofs/induction/policies.py` | implemented | p22.di |
| `SynthesisPolicyV1` | `core/proofs/synthesis/policies.py` | implemented | p14.ps |
| `DI2PolicyV1` | spec only (DIPS-02) | spec_only | p23.di2 |
| `ExpansionPolicyV1` | spec only (DIPS-02) | spec_only | p15.ps2 |
| `ScoringPolicyV1` | spec only (DIPS-02) | spec_only | p15.ps2 |
| `TieBreakPolicyV1` | spec only (DIPS-02) | spec_only | p15.ps2 |
| `AlignmentPolicyV1` | spec only (DIPS-02) | spec_only | p24.tr2 |
| `VerificationPolicyV1` | spec only (DIPS-02) | spec_only | p26.vrf2 |

**Potential collision zone**: `SynthesisPolicyV1` (DIPS-01) vs `ExpansionPolicyV1 + ScoringPolicyV1 + TieBreakPolicyV1` (DIPS-02). DIPS-02 decomposes DIPS-01's monolithic policy into first-class artifacts. No name collision (different names), but semantic overlap exists by design.

---

## 5. Bundle Formats

| Format | Source | Index schema | Closure policy | Status |
|--------|--------|-------------|----------------|--------|
| DIPS-01 bundle | `test-scenarios/induction-synthesis-demo/verify_bundle.py` | `bundle/index.json` | allowlist per schema_id | implemented |
| DIPS-02 bundle | spec only (VRF2 capsule spec) | `bundle/index.json` | exhaustive schema table | spec_only |
| Verification bundle v1 | `core/proofs/verification_bundle.py` | `manifest.json` | fixed set | implemented |
| H2 evidence bundle | `core/proofs/h2_evidence_bundle.py` | `h2_evidence_bundle_manifest.v1.json` | fixed set | implemented |

**Potential collision zone**: DIPS-01 and DIPS-02 bundles share the `bundle/index.json` convention but have different closure policies. DIPS-02's VRF2 must handle both (or explicitly scope to DIPS-02 artifacts only). The verification_bundle_v1 and h2_evidence_bundle use completely different formats.

---

## 6. Scenario-Local Types (Not Yet on Promotion Path)

### ling-ops-demo (promoted — `p25.lo@1.0`, CPG-0..8 complete)

| Type | Module Path | Frozen |
|------|-------------|--------|
| `StageStateV1` | `core.linguistics.mta.stage_state` | yes |
| `EditLogV1` | `core.linguistics.mta.edit_log` | yes |
| `PlanResultV1` | `core.linguistics.mta.planner` | yes |

**Collision status**: Resolved via module-level namespacing. Types live under `core.linguistics.mta.*`; `core/proofs/` has zero imports from this namespace.

### graphing-calc-demo (promoted — `p27.ea@1.0`, CPG-0..8 complete)

| Type | File | Frozen |
|------|------|--------|
| `ExpressionStateV1` | `expression_graph.py` | no |
| `CompareFunctionsV1` | `operators.py` | no |
| `MinimizeCounterexampleV1` | `operators.py` | no |
| `ExpressionCommitV1` | `proof_artifacts.py` | no |
| `ProofBundleV1` | `proof_artifacts.py` | no |
| `ClaimBundleV1` | `proof_artifacts.py` | no |
| `PlotDataArtifactV1` | `plot_data.py` | no |

**Collision risk**: `ProofBundleV1`, `ClaimBundleV1` are very generic names. `StepWitnessV1` (shared name with core) is not used here but exists in phonology.

### structured-patch-demo (D4 transfer evidence for `p01_plan_trace@1.0`)

| Type | File | Frozen |
|------|------|--------|
| `PatchObligation` | `patch_engine.py` | yes |
| `PatchSpec` | `patch_engine.py` | yes |
| `PatchState` | `patch_engine.py` | yes |

**Role**: D4 transfer rig for the already-promoted P01 capsule. Proves `core/capsules/p01/` is domain-agnostic by running it on a JSON document patch domain (alongside lemma-grammar-demo on a natural language domain). Not promoting a new primitive — this is permanent transfer evidence. See `D4_TRANSFER_VALIDATION.md` for the full writeup.

### phonology-g2p-demo (runnable, lower priority)

| Type | File | Frozen |
|------|------|--------|
| `OperatorContractV1` | `operators.py` | yes |
| `PhonologyStepWitnessV1` | `operators.py` | yes |
| `TokenRealizationV1` | `proof_artifacts.py` | yes |
| `DisambiguationWitnessV1` | `proof_artifacts.py` | yes |
| `MeterScanWitnessV1` | `proof_artifacts.py` | yes |
| `PhonologyBundleV1` | `proof_artifacts.py` | yes |

**Collision status**: Resolved. Renamed from `StepWitnessV1` to `PhonologyStepWitnessV1` to avoid triple-definition collision with `core/proofs/step_witness.py:StepWitnessV1` and `core/proofs/synthesis/witnesses.py:StepWitnessV1`. Wire schema prefix (`STERLING::G2P_STEP_WITNESS::V1\0`) is unchanged.

---

## 7. Known Collision Risks (Prioritized)

### ~~HIGH~~ RESOLVED: `StepWitnessV1` triple definition

1. `core/proofs/step_witness.py` — general-purpose step witness (`StepWitnessV1`)
2. `core/proofs/synthesis/witnesses.py` — PS synthesis step witness (`StepWitnessV1`)
3. `test-scenarios/phonology-g2p-demo/operators.py` — **renamed to `PhonologyStepWitnessV1`**

The phonology-local type was renamed. The two core definitions (items 1 and 2) remain — they share a name but live in separate modules with distinct import paths. If core ever needs to unify them, that's a separate refactor.

### MEDIUM: Generic type names in scenario-local code

- `ProofBundleV1`, `ClaimBundleV1` (graphing-calc) — too generic for core promotion
- `StageStateV1` (ling-ops) — too generic; will collide if another demo has "stages"
- `OperatorContractV1` (phonology) — collides conceptually with `core/operators/evidence_contract.py:OperatorContractV1`

### LOW: Bundle format divergence

- Three different bundle conventions exist (DIPS-01/02, verification_bundle_v1, h2_evidence_bundle)
- No name collisions but semantic divergence in closure policies

### LOW: Effect namespace growth

- `sterling.effect.*` (5 effects) is currently small but will grow with each promoted demo
- No collision mechanism exists yet (no registry, no dedup check)

---

## 8. Recommendations

1. ~~**Resolve `StepWitnessV1` collision**~~ Done — renamed to `PhonologyStepWitnessV1`
2. **Namespace scenario-local types** before promoting ling-ops or graphing-calc (prefix with capsule domain or use module-level namespacing)
3. **Add collision guard test** (`tests/proofs/test_primitive_collision_guard.py`) that fails on duplicate capsule_id, duplicate semantics_id, and duplicate type names across promotion-path modules
4. **Standardize bundle format** or explicitly version/scope bundle conventions so DIPS-02 VRF2 doesn't accidentally try to verify a verification_bundle_v1
