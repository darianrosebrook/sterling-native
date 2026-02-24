# Roadmap: Sterling Native Spine

**Status:** Draft
**Date:** 2026-02-24
**Principle:** Every milestone produces a runnable thin-slice that exercises the compilation boundary end-to-end, emits ByteTrace, and can replay-verify. If a milestone doesn't tighten the spine, it's a distraction.

---

## Roadmap 0: Reuse Posture (one-time decision)

**Policy:** v1 is a test oracle, not a dependency.

### A) Allowed (safe reuse)

- Specs and docs (done — `docs/reference/v1/`)
- Goldens, fixtures, and evidence bundles as test vectors
- Minimal reference implementations used only to generate/validate vectors (kept quarantined)

### B) Allowed but quarantined (strangler-fig style)

- A v1 reference implementation may exist under `reference/v1_impl/` (or a separate repo/submodule) solely to:
  - Generate canonical test vectors (ByteState, ByteTrace, hashes)
  - Act as a differential oracle during early development
- Never linked into the v2 runtime. Never imported by v2 kernel packages.

### C) Not allowed

- Copying v1 core modules into v2 "because it works"
- Any v1 import path reachable from the v2 kernel build graph
- "Temporary" code that bypasses `compile(...) → ByteState` or writes trace in ad hoc formats

---

## Roadmap 1: The Spine (M0–M3)

### M0 — Kernel contract surface and repo layout

**Goal:** Make the architecture enforceable by structure.

**Deliverables:**
- [ ] v2 kernel package boundary with minimal API:
  - `compile(payload_bytes, schema_descriptor, registry_snapshot, policy_snapshot) → ByteState`
  - `apply(byte_state, operator_id, inputs, policy_snapshot) → (byte_state', trace_row)`
  - `replay_verify(trace_bundle) → verdict`
- [ ] Canonical artifact schema directory:
  - `schemas/` for trace bundle, policy snapshot, schema descriptor, registry snapshot
- [ ] Canonical lint CI check:
  - `docs/canonical/*` contains no v1 stage taxonomy, no old RunIntent modes, no dead links
  - `docs/reference/v1/*` is allowed to contain anything

**Acceptance:**
- [ ] Exactly one place in the codebase defines canonical hashing and serialization
- [ ] Kernel build graph does not depend on `docs/reference/v1/*` or any v1 implementation code

**Non-goals:**
- No algorithms, no search, no operators beyond type stubs
- No world adapters

**File targets:**
- `kernel/carrier/__init__.py` (or Rust equivalent)
- `kernel/proof/__init__.py`
- `schemas/trace_bundle.schema.json`
- `schemas/policy_snapshot.schema.json`
- `schemas/schema_descriptor.schema.json`
- `schemas/registry_snapshot.schema.json`
- CI config for canonical lint

---

### M1 — ByteState/Code32 + canonical hashing

**Goal:** Implement the carrier as a sealed substrate with deterministic hashing.

**Deliverables:**
- [ ] Code32 registry snapshot format + deterministic allocation rules
- [ ] ByteStateV1 layout + encode/decode
- [ ] Canonical hashing module (blake3 or equivalent) with domain-separated commits:
  - schema_descriptor hash
  - registry_snapshot hash
  - payload hash
  - ByteState bytes hash
- [ ] Lock tests:
  - Same inputs → same bytes → same digest
  - Endianness and layout invariants
  - Hash domain separation strings are fixed and versioned

**Acceptance:**
- [ ] `compile(...)` produces a ByteState with a stable digest across machines (within declared environment constraints)
- [ ] A "golden byte" test exists for at least one minimal payload

**Non-goals:**
- No trace writing yet
- No operator application
- No search

**File targets:**
- `kernel/carrier/code32.py` (or `.rs`)
- `kernel/carrier/bytestate.py`
- `kernel/carrier/hashing.py`
- `kernel/carrier/compiler.py`
- `tests/lock/test_golden_bytes.py`
- `tests/lock/test_hash_stability.py`

---

### M2 — ByteTrace writer + replay verifier

**Goal:** Make replay verification real before search exists.

**Deliverables:**
- [ ] ByteTrace format + append-only writer
- [ ] Replay verifier that checks:
  - Canonical hashing of each step
  - Commit index monotonicity (no holes)
  - Schema/registry/policy snapshot consistency across the episode
- [ ] Minimal operator application primitive:
  - Apply a trivial operator that changes one field deterministically
  - Log to ByteTrace
  - Replay matches bit-identical outputs

**Acceptance:**
- [ ] A single-step episode replays exactly
- [ ] Divergence localization can point to the first differing step

**Non-goals:**
- No search engine
- No multi-step episodes beyond what's needed for replay proof
- No world harness

**File targets:**
- `kernel/carrier/bytetrace.py`
- `kernel/proof/replay_verifier.py`
- `kernel/operators/apply.py` (minimal)
- `tests/lock/test_single_step_replay.py`
- `tests/lock/test_divergence_localization.py`

---

### M3 — Unified World Harness "hello world"

**Goal:** Prove the harness contract without building the full engine.

**Deliverables:**
- [ ] World harness contract:
  - Fixtures → payload_bytes
  - Compiler/decompiler hooks
  - Deterministic verifier for correctness
  - Negative control hooks (even if stubbed)
- [ ] One minimal world ("Corridor" / "Rome path" / "toy graph"):
  - Deterministic transitions
  - Small operator catalog as data (not code branching)
- [ ] Harness emits:
  - Inputs bundle
  - ByteTrace
  - Verification bundle (replay + gate verdict)
  - Eligibility status (everything is DEV initially)

**Acceptance:**
- [ ] One end-to-end harness run produces a fully self-contained artifact bundle
- [ ] Re-run produces identical ByteTrace digest

**Non-goals:**
- No induction
- No memory compaction
- No cross-domain planning
- No CERTIFIED mode yet (all DEV)

**File targets:**
- `harness/contract.py` (world harness protocol)
- `harness/verifier.py`
- `worlds/corridor/` (or `worlds/rome/`)
- `worlds/corridor/fixtures/`
- `tests/integration/test_harness_e2e.py`
- `tests/lock/test_harness_determinism.py`

---

## Roadmap 2: Navigation and Governance (M4–M6)

### M4 — Single search engine

**Goal:** One engine implementation that can be measured, not duplicated.

**Deliverables:**
- [ ] Best-first/A* skeleton over ByteState nodes
- [ ] Frontier structure + dedup by digest
- [ ] Budget policy plumbing (step budget, expansion budget)
- [ ] Instrumentation hooks that emit metrics without a second code path

**Acceptance:**
- [ ] Search produces a valid ByteTrace for multi-step episodes
- [ ] Instrumentation does not change canonical bytes (or is explicitly part of trace format)

**Non-goals:**
- No parallel search strategies
- No neural heuristics yet
- No operator induction

**File targets:**
- `kernel/search/engine.py`
- `kernel/search/frontier.py`
- `kernel/search/budget.py`
- `tests/integration/test_multi_step_search.py`

---

### M5 — CERTIFIED mode + negative controls

**Goal:** Certification semantics exist as enforcement, not documentation.

**Deliverables:**
- [ ] DEV vs CERTIFIED policy snapshot enforcement
- [ ] Mandatory negative controls for the first world:
  - Oracle-leak attempt must fail
  - Trace omission must fail
  - Illegal operator application must fail
- [ ] Eligibility gating for benchmark/certification (without publishing numbers)

**Acceptance:**
- [ ] CERTIFIED runs fail closed on missing obligations
- [ ] DEV runs record failures but cannot produce promotion-eligible artifacts

**Non-goals:**
- No operator induction pipeline
- No cross-domain transfer yet

**File targets:**
- `kernel/proof/policy.py` (DEV/CERTIFIED enforcement)
- `kernel/proof/eligibility.py`
- `tests/negative/test_oracle_leak.py`
- `tests/negative/test_trace_omission.py`
- `tests/negative/test_illegal_operator.py`

---

### M6 — Transfer pack scaffolding (two worlds, same claim)

**Goal:** Enforce that capability transfer is real.

**Deliverables:**
- [ ] Transfer Pack format (claims + falsifiers + fixtures + budgets)
- [ ] A second minimal world with a different truth regime
- [ ] One shared claim verified in both worlds under CERTIFIED

**Acceptance:**
- [ ] Transfer matrix exists with two passing cells and at least one negative control

**Non-goals:**
- No more than two worlds
- No induction
- No memory tiering

**File targets:**
- `kernel/proof/transfer_pack.py`
- `worlds/mastermind_lite/` (or equivalent second world)
- `tests/integration/test_transfer_claim.py`

---

## Roadmap 3: Reuse Decision by Evidence (M7)

### M7 — "Pull from v1?" checkpoint

**Goal:** Decide what (if anything) to port or rewrite, with evidence.

**Criteria to allow porting (expected to be rare):**
- [ ] The v1 code can be isolated as a single-purpose module behind the v2 kernel API
- [ ] Its behavior is fully covered by v2 lock tests and goldens
- [ ] It does not import the old dependency web (no worlds/induction cross-imports)
- [ ] It does not add parallel implementations (INV-CORE-12)

**What usually happens in practice:**
- v1 stays as a test oracle and performance comparator
- Kernel surfaces are rewritten fresh because they're sharply specified and much smaller than the old sprawl

**Deliverables:**
- [ ] Written decision record (ADR) for each module considered for porting
- [ ] Differential test suite comparing v1 oracle output against v2 kernel output
- [ ] Import boundary proof: v2 kernel build graph has zero v1 dependencies

---

## What not to do in the first pass

- Do not port any v1 "reasoning" or "induction" packages
- Do not reintroduce Stage taxonomies (Stage K/M, etc.) into v2 canonical docs
- Do not add more than 1–2 worlds before transfer is proven
- Do not implement memory tiering beyond what the trace needs to represent; memory governance comes after replay and certification are real

---

## Target repo layout

```
kernel/                     # Authoritative runtime (Rust eventually)
  carrier/                  # Code32, ByteState, ByteTrace, hashing, compile
  search/                   # Engine, frontier, budgets
  proof/                    # Replay verifier, gate verdict, eligibility
  operators/                # Typed signatures, registry, apply
harness/                    # World harness contract, fixtures, verifiers
worlds/                     # World configs + adapters (no proof plumbing)
ml/                         # Advisory only (score, parse, realize; no mutation)
schemas/                    # Canonical artifact schemas (JSON Schema)
reference/v1_impl/          # Optional quarantined oracle (never linked to kernel)
docs/                       # Already cleanly separated
tests/
  lock/                     # Golden bytes, hash stability, replay determinism
  negative/                 # Negative controls (oracle leak, trace omission, etc.)
  integration/              # End-to-end harness, multi-step search, transfer
```
