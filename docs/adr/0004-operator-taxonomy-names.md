---
status: Accepted
authority: adr
date: 2026-02-24
---
# ADR 0004: Operator Taxonomy Names (Seek/Memorize/Perceive/Knowledge/Control)

## Decision

Sterling Native standardizes the S/M/P/K/C operator taxonomy names as:

| Code | Canonical Name (Sterling Native) | Legacy Name (v1) |
|------|----------------------------------|-------------------|
| **S** | Seek | Structural |
| **M** | Memorize | Meaning |
| **P** | Perceive | Pragmatic |
| **K** | Knowledge | Knowledge |
| **C** | Control | Control |

The single-letter codes (S/M/P/K/C) are unchanged.

## Rationale

Sterling v1 used "Structural / Meaning / Pragmatic / Knowledge / Control," which reflects the project's linguistics-first origin (PN verification, WordNet navigation, discourse analysis). These names describe what the operators touch in a linguistic domain.

Sterling Native is multi-world by design. The operator categories need to describe *reasoning intent* that transfers across worlds (Rome navigation, Mastermind, EscapeGame, Minecraft crafting, etc.), not the linguistic layers they happen to affect in one domain.

- **Seek** (S): operators that explore or navigate state space
- **Memorize** (M): operators that commit or consolidate meaning
- **Perceive** (P): operators that interpret context or update beliefs
- **Knowledge** (K): operators that query or extend world knowledge
- **Control** (C): operators that manage search flow

This is a label change, not a semantic change. The operator contracts, signatures, preconditions, effects, and registry mechanics are identical.

## Consequences

- `docs/canonical/glossary.md` uses the v2 names with a note about the v1 legacy names.
- v1 reference docs under `docs/reference/v1/` retain the original names unchanged.
- Any new canonical doc or contract in Sterling Native uses the v2 names.
- The single-letter codes remain S/M/P/K/C — no code changes needed.

## Alternatives considered

- Keep v1 names: rejected because "Structural/Meaning/Pragmatic" implies linguistic domain structure, which is misleading for non-linguistic worlds.
- Introduce new codes: rejected because S/M/P/K/C is deeply embedded and the codes themselves are fine.
