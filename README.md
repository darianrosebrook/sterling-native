# Sterling Native

Sterling Native is the ground-up rebuild of [Sterling v1](https://github.com/darianrosebrook/sterling): a **structured semantic reasoning engine** that replaces LLM chain-of-thought with formal graph search over governed state representations.

Transformers are treated as **codecs and advisors** — they may parse, rank, compress, or render, but they cannot create operators, bypass preconditions, or mutate state. The cognitive core is explicit state, typed operators, and auditable search with deterministic replay and evidence-carrying traces.

## Current capabilities

Sterling Native implements a complete **deterministic execution + search + evidence** system across two layers:

### Execution substrate (kernel)

- **Compilation boundary**: `compile(payload, schema_descriptor, registry) → ByteState` — a pure function; identical inputs produce identical bytes
- **Operator dispatch**: `apply()` produces deterministic state transitions via governed operators
- **Carrier replay**: `ByteTrace` (`.bst1`) gives frame-by-frame replay verification for compile→apply sequences, with O(1) divergence localization
- **Canonical hashing**: SHA-256 with domain-separated prefixes; single `canonical_hash()` and single `canonical_json_bytes()` — no second implementation

### Search substrate (search + harness)

- **Best-first frontier search** over compiled state space with deterministic expansion ordering, visited-set deduplication, and dead-end tracking
- **Search transcript**: `SearchGraphV1` (`search_graph.json`) records every expansion, candidate, outcome, and termination reason as a canonical JSON artifact
- **Search tape**: `SearchTapeV1` (`.stap`) is a binary hot-loop event log with chain-hash integrity — the minimal evidence recorder for the search inner loop
- **Tape→graph equivalence**: In Cert mode, rendering a parsed tape to `SearchGraphV1` must produce byte-identical canonical JSON to the direct `search_graph.json` artifact
- **Scoring**: `UniformScorer` (baseline) and `TableScorer` (injected per-candidate bonuses with digest binding)
- **Policy enforcement**: `SearchPolicyV1` controls dedup strategy, step budgets, pruning, and candidate limits; `PolicySnapshotV1` captures policy as an auditable artifact

### Evidence packaging (harness)

- **Artifact bundles**: `ArtifactBundleV1` with normative/observational split — bundle digest computed from normative projection only
- **Fail-closed verification**: `verify_bundle()` checks content hashes, manifest integrity, metadata bindings, scorer coherence, and (when present) tape chain hash + header bindings
- **Verification profiles**: Base (integrity + bindings when evidence present) vs Cert (requires tape presence, adds tape→graph equivalence)
- **Persistence**: `write_bundle_dir()` / `read_bundle_dir()` with content-hash verification at the read boundary; extra/missing/tampered files rejected

### Test worlds

- **RomeMini**: 1 layer, 2 slots, 1 operator — minimal carrier-level fixture
- **RomeMiniSearch**: 2 slots × 4 values — minimal search fixture
- **SlotLatticeSearch**: parameterized N×V world with 6 regime constructors (trap rules, goal profiles) — stress-tests search at scale (1000+ expansions)
- **TransactionalKvStore**: 2-layer write-once KV with marker-based transactions (stage/commit/rollback)

### Evidence

578 tests (lock + unit + integration), all passing. Cross-process determinism fixtures verify independent processes produce identical artifacts. CI runs on Linux + macOS.

## Quickstart

```bash
# Build and test
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check
```

## Core concepts

### Compilation boundary

All domain data enters through a single pure function:

```
compile(payload_bytes, schema_descriptor, registry) → ByteState
```

This boundary prevents runtime mutation, domain coupling, and semantic drift. Nothing bypasses it.

### Two evidence layers

Sterling Native produces two distinct evidence artifacts, each certifying a different layer:

1. **Carrier replay** — `ByteTrace` (`.bst1`): fixed-width frames proving deterministic compile→apply execution. Supports replay verification with exact byte-offset divergence localization.

2. **Search replay** — `SearchTapeV1` (`.stap`) + `SearchGraphV1` (`search_graph.json`): the tape is the hot-loop recorder with chain-hash integrity; the graph is the canonical transcript for analysis. Cert mode requires tape→graph byte equivalence.

Both coexist in a bundle. They certify different claims and are verified independently.

### Governance

Sterling is designed so that "almost correct" is not acceptable:

- Typed errors throughout; no partial bundles, no silent fallbacks
- Strict canonical JSON rules for all hashed artifacts
- Policy constraints captured as an auditable artifact and bound into the verification report
- Normative/observational split: bundle digests are computed from normative artifacts only; observational artifacts (like traces) are bound via hash commitments in the normative verification report
- Verification profiles (Base/Cert) control strictness; Cert is fail-closed on missing evidence

## Repository map

### Code

```
kernel/         Sealed carrier kernel: Code32, ByteState, ByteTrace, compile(), apply(),
                replay_verify(), canonical hash, canonical JSON
search/         Search engine: frontier, scorer, graph, policy, tape writer/reader/renderer
harness/        Orchestration: run_search(), bundles, verification, policy snapshots,
                bundle persistence, test worlds
tests/lock/     Lock tests + cross-process determinism fixtures
tests/fixtures/ Golden oracles (JSON, .bst1)
benchmarks/     Criterion micro/macro benchmarks + auditable report harness
schemas/        JSON schemas for canonical artifact formats
```

Dependency direction: `kernel ← search ← harness`. One-way, no cycles.

### Specs and governance

```
.caws/specs/    Tracked specs with milestone claims, falsifiers, and test pointers
```

### Documentation

```
docs/canonical/     Single-source-of-truth contracts and invariants (v2 vocabulary)
docs/architecture/  Target architecture, module map, success rubric
docs/policy/        Benchmarking, transfer, governance, versioning policies
docs/adr/           Architecture decision records
docs/specs/         Forward-looking capability primitives (P01–P22)
docs/reference/v1/  Carried-over v1 reference (non-authoritative, with supersession tracking)
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

The frontier for moving from "verified execution + search substrate" to "reasoning engine with domain breadth":

1. **World diversity** — truth-regime worlds: tool-safety (transactional), partial observability (belief discipline), stochastic (seed-bound certification)
2. **Operator variety** — richer operator catalog with legality enforcement across diverse domains
3. **Induction / learning** — promotable operator synthesis pipeline with regression-free certification
4. **ML integration** — advisory-only neural scoring/parsing to demonstrate "transformer demotion" by construction
5. **Performance gates** — quantify and certify hot-loop budgets under load

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). This repo is intentionally strict: change surfaces require disciplined review and typically require new tests and updated claims with test pointers.

## License

[Source-Available License](LICENSE) — Copyright (c) 2026 Darian Rosebrook
