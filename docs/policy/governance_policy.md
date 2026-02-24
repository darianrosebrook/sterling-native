---
status: "Draft (Sterling Native)"
authority: policy
scope: "Run modes, eligibility, and artifact semantics"
---
# Governance Policy

## Two run modes

### DEV
DEV is artifact-complete but gate-non-blocking:
- the run may continue after failures
- the run MUST emit a GateVerdictBundle and witness trail
- the run MUST NOT emit any “Certified” stamp or promotion-eligible proposal

### CERTIFIED
CERTIFIED is fail-closed:
- missing fixtures, missing trace obligations, or gate failures stop execution
- CERTIFIED is the only mode eligible to produce promotion-grade artifacts

## Gate semantics

- Gates are obligations that must be evaluated and recorded.
- A missing gate verdict is a failure in CERTIFIED mode.
- “Non-blocking” is not a concept; only DEV can proceed past failures.

## Artifact semantics

- All artifacts MUST record: engine version, epoch ID, schema descriptor, registry snapshot, and policy snapshot.
- ByteTrace is the canonical trace artifact for replay verification.
- Derived views (graphs, reports) are non-authoritative renderings of canonical artifacts.

## Waivers

Waivers are typed, explicit, and recorded into the policy snapshot.
Any run requiring waivers is ineligible for promotion unless the waiver is explicitly allowed by the promotion target.
