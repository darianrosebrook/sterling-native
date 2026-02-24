---
status: "Draft (Sterling Native)"
authority: policy
scope: Transfer claims across worlds / truth regimes
---
# Domain Transfer Scenario Policy

## Purpose

Domain transfer tests prove Sterling Native is a reasoning substrate with stable semantics, not a set of domain-specific solvers.

Transfer succeeds when the same claim catalog is satisfied across multiple worlds:
- without world-specific routing hacks,
- under the same governance mode and budget assumptions,
- producing comparable artifact bundles and replay guarantees.

## Transfer unit: capability

Sterling transfers capabilities, not implementations.

A capability is defined by:
- operator family contracts (signatures, preconditions, effects, write-sets)
- claim catalog (what must be true)
- falsifiers/negative controls (how it must fail when cheating)
- required evidence surfaces (artifacts and trace obligations)
- acceptable variability (what may differ by world: ontologies, observation schemas)

Worlds must not redefine capability meaning. Worlds may only map local ontologies into the capability interface.

## Transfer matrix

For any capability promoted as “Sterling-owned semantics”, define a matrix:

- Rows: worlds (W1..Wn)
- Columns: claims (C1..Cm)
- Cells: pass/fail/ineligible with artifact references

Minimum requirements:
- pass in CERTIFIED mode in at least 2 worlds
- include at least 1 negative control per capability
- include at least 1 non-linguistic world for non-linguistic claims

## Claim structure

Each claim MUST specify:
- Claim ID (stable, versioned)
- Preconditions
- Success criterion (verifier)
- Failure family (typed refusal vs incorrect vs incomplete trace)
- Required evidence surfaces (trace fields, witness types, tool transcripts)
- Falsifier suite

## Negative controls (mandatory)

Each transferred capability requires at least one negative control:
- oracle leakage control
- shortcut control (no phrase routing / special casing)
- drift control (hash/IR/loop drift triggers failure)

## World adapter constraints (anti “transfer by glue code”)

World adapters must not:
- mutate kernel state outside operators
- perform domain selection as control flow (must be MetaPlan steps)
- bypass canonical hashing/IR
- emit non-canonical traces or omit required obligations

Any exception must be a typed waiver, and the run becomes ineligible for promotion-grade transfer evidence.

## Transfer Pack

A Transfer Pack is a standardized bundle runnable by the Unified World Harness:

- fixtures/ (payloads per world)
- verifiers/ (world-level correctness verifiers)
- claims/ (shared claim catalog)
- falsifiers/ (negative controls)
- budgets/ (standard budget profiles)
- expected/ (goldens, where appropriate)

## Promotion gating for transfer

A capability cannot be promoted as “core” unless:
- it passes CERTIFIED in 2+ worlds
- it passes negative controls in 1+ world
- it produces eligible artifacts (replay verified, trace complete)
- it does not depend on world-specific patches that change semantics
