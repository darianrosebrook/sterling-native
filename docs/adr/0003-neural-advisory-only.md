# ADR 0003: Neural Components are Advisory Only (Enforced by API Shape)

Status: Accepted (Design target)
Date: 2026-02-23

## Decision

Neural components may parse, rank, compress, and realize; they may not mutate state, create operators, or bypass preconditions.
Enforcement is structural: the kernel exposes no mutating methods to the ML layer.

## Rationale

- Prevent “LLM agent with extra steps” failure mode.
- Preserve invariant: symbolic legality and contracts are authoritative.

## Consequences

- Scoring is separated from legality.
- All state mutation occurs only via governed operators.
