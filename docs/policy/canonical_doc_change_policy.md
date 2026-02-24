---
status: “Draft (Sterling Native)”
authority: policy
scope: “docs/canonical/* and other canonical surfaces”
---
# Canonical Document Change Policy

## Goal

Prevent semantic drift by enforcing single-source-of-truth ownership for core contracts.

## Canonical docs

Canonical docs are:
- normative: they define contracts and invariants
- versioned: edits require explicit version bumps
- gate-checked: CI can fail if changes are inconsistent

Files under `docs/canonical/` MUST include:
- Version
- Date
- Status
- Change policy note: “Do not edit without a version bump”

## Compatibility

Versioned data formats may coexist (e.g., artifact schemas), but MUST be handled by a single canonical upgrade/decode pathway.
Parallel “v2” implementations of the same concept are forbidden (INV-CORE-12).
