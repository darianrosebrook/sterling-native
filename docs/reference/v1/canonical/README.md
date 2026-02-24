# v1 Canonical Contracts (Quarantine)

**Status**: Non-authoritative for Sterling Native

These files are the canonical contract definitions from Sterling v1. They are imported here as reference material for review and selective promotion into `docs/canonical/`.

## Promotion criteria

A v1 contract is promoted to `docs/canonical/` (normative for Sterling Native) only if it satisfies all of:

1. **Aligns with the v2 spine**: compilation boundary is authoritative (`compile(...) → ByteState`), no runtime substrate mutation mid-episode.
2. **Uses v2 governance taxonomy**: DEV / CERTIFIED only (no old run-intent modes like EXPLORATORY, PROMOTION, REPLAY).
3. **No parallel implementations**: does not introduce alternative schemas or implementations that violate INV-CORE-12.
4. **Clean trace authority**: ByteTrace is the canonical persisted trace artifact; StateGraph and other views are derived.
5. **Has version metadata and change policy header**: and the team is willing to enforce it going forward.

## What's here

These 29 files cover:

- State model, operator registry, and operator induction contracts
- Knowledge graph, SWM, and linguistic IR contracts
- Hashing, proof/evidence, and governance certification contracts
- Schema registry, compilation boundary, and conformance definitions
- Value function, world adapter protocol, and evaluation gates
- Architecture layers, module interdependencies, and north star

## Do not reference these as normative

If you need a contract definition for Sterling Native, check `docs/canonical/` first. If it's not there yet, the v1 version here may be a useful starting point — but it needs the promotion review before it becomes authoritative.
