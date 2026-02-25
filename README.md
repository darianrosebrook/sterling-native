# Sterling Native

Sterling Native is the ground-up rebuild of [Sterling v1](https://github.com/darianrosebrook/sterling): a **structured semantic reasoning engine** that replaces LLM chain-of-thought with formal graph search over governed state representations.

Transformers are treated as **codecs and advisors** — they may parse, rank, compress, or render, but they cannot create operators, bypass preconditions, or mutate state. The cognitive core is explicit state, typed operators, and auditable search with deterministic replay and evidence-carrying traces.

## Status

Sterling Native currently implements the **carrier and evidence substrate** (the architectural spine):

| Layer | Status | What it provides |
|-------|--------|-----------------|
| Kernel | Implemented | Canonical state (ByteState), traces (ByteTrace/.bst1), hashing (SHA-256 with domain prefixes), deterministic replay, operator dispatch |
| Harness | Implemented | Bundle creation, fail-closed verification, tamper-evident persistence, policy snapshots, pipeline orchestration |
| Lock tests | Implemented | Cross-process determinism fixtures, golden oracles, divergence detection |
| Governance | Implemented | Spec-anchored claims with test pointers, claim pointer linting, acceptance ID anchoring |

**Not yet ported from v1:**

- Multi-operator catalog (v1 had 28 operators across 5 categories; v2 has a minimal set)
- Real multi-world workloads (v2 has one fixture world; v1 had 10 domains)
- Frontier-based graph search (branching execution, backtracking, scoring)
- Value function and scoring surfaces
- Memory, landmarks, and learned heuristics

## Quickstart

```bash
# Build and test (240+ tests across 3 crates)
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check
```

When the harness runs a world, it produces an **artifact bundle** containing:

- A canonical trace (`.bst1`) — fixed-width frames where the bytes computed during reasoning *are* the evidence
- Canonical JSON artifacts (compilation manifest, verification report, policy snapshot)
- A manifest listing every artifact with its content hash
- A digest basis defining what is normative (included in the bundle digest)
- A bundle digest for integrity verification

Bundles can be persisted to disk and read back **fail-closed** — missing, extra, or tampered files are rejected at the read boundary.

## Core concepts

### Compilation boundary

All domain data enters through a single pure function:

```
compile(payload_bytes, schema_descriptor, registry) → ByteState
```

This boundary prevents runtime mutation, domain coupling, and semantic drift. Nothing bypasses it.

### State and evidence carriers

- **Code32** — 32-bit semantic atoms with structured domain/kind/local allocation
- **ByteStateV1** — a fixed-layout two-plane tensor (identity + status) of semantic codes
- **ByteTrace (.bst1)** — fixed-width trace frames; the bytes computed during reasoning are the evidence artifact, not a serialization of something else
- **Replay verification** — deterministic re-execution that localizes divergence to an exact byte offset (O(1) divergence localization)

### Governance

Sterling is designed so that "almost correct" is not acceptable:

- Typed errors throughout; no partial bundles, no silent fallbacks
- Strict canonical JSON rules for all hashed artifacts
- Policy constraints captured as an auditable artifact and bound into the verification report
- Normative/observational split: bundle digests are computed from normative artifacts only; observational artifacts (like traces) are bound via hash commitments in the normative verification report

## Repository map

### Code

```
kernel/         Sealed carrier kernel: state, trace, canonical hash, replay, operators
harness/        Orchestration: bundle creation, verification, policy, persistence
tests/lock/     Lock tests + cross-process fixtures enforcing determinism
tests/fixtures/ Golden oracles (JSON, .bst1)
schemas/        JSON schemas for all canonical artifact formats
```

### Specs and governance

```
.caws/specs/    Tracked specs with milestone claims, falsifiers, and test pointers
```

### Documentation

```
docs/canonical/     Single-source-of-truth contracts and invariants
docs/architecture/  Target architecture, module map, success rubric
docs/policy/        Benchmarking, transfer, governance, versioning policies
docs/adr/           Architecture decision records
docs/specs/         Forward-looking capability primitives (P01–P22)
docs/reference/v1/  Carried-over v1 reference (non-authoritative)
docs/templates/     Templates for transfer packs, benchmarks, claim catalogs
```

When docs and code disagree, the tracked spec and lock tests are authoritative.

## Proof model

Claims in Sterling Native are only admissible if they have:

- Declared falsifiers (what would disprove the claim)
- Concrete test pointers (grep-findable, executable)
- Deterministic verification surfaces (reproducible across processes)

This is enforced mechanically: claim pointer resolution is linted, acceptance IDs are anchored to code, and cross-process fixture binaries verify that independent processes produce identical artifacts.

## What's next

The leverage points for moving from "verified substrate" to "reasoning engine":

1. **Operator catalog** — port real operators with signature enforcement beyond the minimal set
2. **A second world** — stress the world contract (`WorldHarnessV1`) with non-trivial programs
3. **Graph search** — frontier-based search with a branching audit structure
4. **Scoring and memory** — value heads, SWM, landmarks

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). This repo is intentionally strict: change surfaces require disciplined review and typically require new tests and updated claims with test pointers.

## License

[Source-Available License](LICENSE) — Copyright (c) 2026 Darian Rosebrook
