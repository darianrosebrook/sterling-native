> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

# Module Interdependencies v1.1

**Status**: Canonical specification — sufficient to understand and reconstruct `core/` package structure.
**Scope**: Import graph, layer boundaries, circular dependency prevention, bootstrap order.
**Layer**: Cross-cutting (all layers)
**Version**: 1.1 (corrected from 1.0 — missing standalone files and modules)

---

## §1 Purpose

Sterling's `core/` package contains ~40 modules and ~666 Python files. This document maps the dependency graph between modules, defines layer boundaries that prevent circular imports, and specifies the initialization order required for correct bootstrap. Understanding these relationships is essential for rebuilding the system from scratch — even with perfect per-module documentation, incorrect wiring produces import cycles and initialization failures.

---

## §2 Module Inventory

### §2.1 Modules by Size

| Module | Files | Layer | Role |
|--------|-------|-------|------|
| `induction/` | 120 | 3 (Induction) | Operator hypothesis lifecycle, promotion, synthesis |
| `reasoning/` | 64 | 0 (Reasoning) | Search, state graph, episode management |
| `linguistics/` | 47 | 2 (Language) | IR v0, operators, parser trust, myelin sheath |
| `memory/` | 45 | 1 (Memory) | Claims, schemas, packets, verification, certification |
| `benchmarks/` | 42 | 4 (Testing) | Fence tests, K6 safety checks |
| `proofs/` | 39 | 1 (Proofs) | Evidence bundles, provenance, certificates, replay |
| `operators/` | 37 | 1 (Operators) | Registry, signatures, gated execution, promotion |
| `value/` | 33 | 0 (Value) | Value function components, hybrid composition |
| `worlds/` | 20 | 2 (Worlds) | Domain adapters, discourse, WordNet, PN |
| `carrier/` | 17 | 1 (Carrier) | ByteState, Code32, ByteTrace, compiler |
| `oracles/` | 18 | 3 (Oracles) | Dialogue oracle, code oracle, scoring |
| `governance/` | 13 | 1 (Governance) | Run intent, gate verdicts, failure witnesses |
| `domains/` | 12 | 2 (Domains) | Domain specs, capability descriptors, registration |
| `kernels/` | 10 | 2 (Kernels) | PN, WordNet, gridworld, mastermind kernels |
| `text/` | 10 | 2 (Text) | Text IR, parser, realizer, pipeline |
| `safeguards/` | 10 | 1 (Safeguards) | Certifying boundary, model gateway, invariance |
| `recursion/` | 10 | 0 (Reasoning) | StateGraph walk, recursion budget |
| `td/` | 10 | 4 (Docs) | Technical decision records |
| `contracts/` | 10 | 0 (Contracts) | Goal specs, governance status, invariants |
| `labels/` | 9 | 0 (Foundation) | Type IDs for entities, operations, relations |
| `ir/` | 5 | 0 (IR) | Semantic delta IR |
| `discourse/` | 7 | 2 (Discourse) | Dialogue state, phases, intent satisfaction |
| `intent/` | 7 | 2 (Intent) | Intent types, classification, value head |
| `canonicalization/` | 7 | 0 (Foundation) | JSON canonicalization, hash normalization |
| `environment/` | 6 | 4 (Config) | Environment configuration |
| `kg/` | 5 | 1 (KG) | Knowledge graph types, registry |
| `optimization/` | 5 | 0 (Foundation) | Caching, quantization, hashing |
| `engine/` | 5 | 3 (Engine) | Run orchestration, result assembly |
| `tasks/` | 5 | 0 (Foundation) | Task/goal specifications |
| `pn/` | 3 | 2 (PN) | Predicate nominal utilities |
| `hashing/` | 2 | 0 (Foundation) | Hash contracts |
| `verification/` | 2 | 0 (Foundation) | Hash utilities |
| `realization/` | 2 | 2 (Realization) | Realization spec, mask slots |
| `certification/` | 2 | 1 (Certification) | Episode hash versioning |
| `util/` | 3 | 0 (Foundation) | General utilities |
| `reconstruction/` | 2 | 3 (Recovery) | State reconstruction |
| `pseudocode/` | 7 | 2 (Language) | Pseudocode IR types, Python lowerer |
| `diagnostics/` | 1 | 0 (Foundation) | Diagnostic utilities |

### §2.2 Standalone Files in `core/`

| File | Role | Dependencies |
|------|------|-------------|
| `state_model.py` | Three-tier state hierarchy (UtteranceState, WorldState, StateNode) | Only `core.kg.registry` (TYPE_CHECKING) |
| `search_health.py` | O(1) search health accumulator | None (stdlib only) |
| `exceptions.py` | Sterling exception hierarchy | None |
| `features.py` | Feature computation | `core.optimization` |
| `features_grouped.py` | Grouped feature computation (507 LOC) | `core.features` |
| `operator_masking.py` | Operator mask builder | Self-referential |
| `ir_serialization.py` | IR token serialization | None |
| `ir_extraction.py` | IR feature extraction (303 LOC) | `core.ir` |
| `id_registry.py` | ID registry and tracking (732 LOC) | Foundation types |
| `external_ref.py` | External reference handling (205 LOC) | Foundation types |
| `simple_kg.py` | Lightweight KG implementation (219 LOC) | `core.kg` |
| `logging_config.py` | Logging configuration (271 LOC) | None (stdlib only) |
| `profiling.py` | Performance profiling utilities (367 LOC) | None (stdlib only) |
| `tasks.py` | Legacy task definitions (395 LOC) | Foundation types |

---

## §3 Layer Architecture

### §3.1 Layer Definitions

```
Layer 0 — Foundation
    state_model, contracts, labels, ir, canonicalization,
    hashing, verification, optimization, tasks, util,
    search_health, exceptions, features, ir_serialization

Layer 1 — Infrastructure
    operators, memory, governance, proofs, carrier,
    kg, certification, safeguards

Layer 2 — Domains & Language
    worlds, text, linguistics, discourse, intent,
    domains, kernels, pn, realization

Layer 3 — Integration
    reasoning, value, induction, oracles, engine,
    recursion, reconstruction

Layer 4 — Testing & Config
    benchmarks, td, environment
```

### §3.2 Layer Rule

**INV-M1**: A module at Layer N may import from Layer N or any Layer < N. Imports from Layer > N must use `TYPE_CHECKING` guards or lazy imports.

This rule is not perfectly enforced in practice — 232 files use `TYPE_CHECKING` to break cycles, and `importlib`-based lazy loading appears in `reasoning/`, `tasks/`, and `benchmarks/`.

### §3.3 Rationale

- **Layer 0** defines data structures and protocols with no behavioral dependencies
- **Layer 1** builds operational machinery (registries, hash verification, governance gates) on Layer 0 types
- **Layer 2** implements domain-specific logic using Layer 1 infrastructure
- **Layer 3** orchestrates Layer 2 components into search, evaluation, and induction pipelines
- **Layer 4** exists outside the production dependency graph

---

## §4 Import Graph

### §4.1 Import Frequency (top modules as import targets)

| Target Module | Import Count | Primary Importers |
|--------------|-------------|-------------------|
| `induction` | 482 | Self (internal), `reasoning`, `operators`, `proofs`, `engine` |
| `operators` | 303 | `reasoning`, `induction`, `engine`, `text`, `governance` |
| `reasoning` | 197 | Self (internal), `induction`, `engine`, `text`, `value` |
| `memory` | 192 | Self (internal), `proofs`, `reasoning`, `induction` |
| `state_model` | 173 | Nearly all modules (universal data type) |
| `linguistics` | 161 | Self (internal), `text`, `domains`, `safeguards` |
| `worlds` | 118 | `reasoning`, `engine`, `governance`, `kernels`, `induction` |
| `canonicalization` | 83 | `memory`, `proofs`, `domains`, `engine`, `reasoning` |
| `proofs` | 70 | Self (internal), `induction`, `memory` |
| `governance` | 65 | `engine`, `induction`, `reasoning`, `operators` |
| `kernels` | 63 | Self (internal), `reasoning`, `value` |
| `oracles` | 59 | Self (internal), `reasoning`, `kernels` |
| `contracts` | 53 | `reasoning`, `text`, `engine`, `ir` |
| `verification` | 48 | `carrier`, `proofs`, `domains`, `governance` |
| `carrier` | 46 | Self (internal), `proofs` |
| `text` | 42 | `linguistics`, `safeguards`, `text` (internal) |
| `value` | 39 | `reasoning`, `intent`, `value` (internal) |
| `domains` | 39 | Self (internal), `governance`, `engine` |
| `intent` | 38 | `reasoning`, `discourse`, `value`, `intent` (internal) |
| `ir` | 34 | `reasoning`, `induction`, `contracts` |
| `discourse` | 28 | `oracles`, `reasoning`, `worlds` |
| `labels` | 27 | `discourse`, `domains` |
| `safeguards` | 25 | `text`, `reasoning`, `safeguards` (internal) |
| `kg` | 25 | `kg` (internal), `kernels`, `state_model` (TYPE_CHECKING) |

### §4.2 Critical Dependency Paths

**Search pipeline** (the hot path):
```
engine → reasoning → state_model
                   → operators → state_model
                   → value → state_model
                   → worlds → state_model
```

**Induction pipeline**:
```
engine → induction → operators → state_model
                   → memory → canonicalization
                   → proofs → verification
                   → reasoning (episode loader)
```

**Text processing**:
```
engine → text → linguistics → state_model
             → worlds (discourse adapter)
             → safeguards → linguistics
```

### §4.3 Hub Modules

Three modules serve as import hubs with extremely high fan-in:

1. **`state_model.py`** (173 imports): Defines the universal data types (`UtteranceState`, `WorldState`, `StateNode`) used by nearly every module. Has near-zero outgoing dependencies — only a `TYPE_CHECKING` import of `core.kg.registry.KGRef`.

2. **`operators/registry`** (subset of 303): The operator signature and registry types are imported across reasoning, induction, governance, text, and value modules.

3. **`canonicalization/json`** (~83 total for module): `canonical_json_dumps` and `canonical_json_bytes` are used by memory, proofs, domains, engine, and reasoning for deterministic serialization.

---

## §5 Circular Dependency Prevention

### §5.1 TYPE_CHECKING Pattern

Sterling uses Python's `TYPE_CHECKING` guard extensively (232 files) to break potential import cycles. The pattern:

```python
from __future__ import annotations
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from core.kg.registry import KGRef  # Only used in type annotations
```

This allows type annotations to reference types from higher layers without creating runtime import dependencies.

### §5.2 Known Cycle-Breaking Points

| Cycle Risk | Resolution |
|-----------|------------|
| `state_model` ↔ `kg` | `state_model.py` imports `KGRef` under `TYPE_CHECKING` only |
| `reasoning` ↔ `induction` | `reasoning` imports lifecycle controller lazily in expansion loop |
| `reasoning` ↔ `operators` | Some imports use TYPE_CHECKING; `GovernanceRuntimeProtocol` is a Protocol |
| `text` ↔ `linguistics` | `text.pipeline` imports `linguistics.intake` at call sites |
| `induction` ↔ `operators` | `induction` imports from `operators.registry`; `operators.promotion_service` imports from `induction` |
| `memory` ↔ `proofs` | `memory.certification` and `proofs.td12_ms_integration` cross-reference; TYPE_CHECKING used |

### §5.3 Lazy Import Pattern

Three modules use `importlib` for deferred loading:

```python
# core/reasoning/__init__.py
import importlib
module = importlib.import_module(module_name)  # Loads submodules on demand

# core/reasoning/loop/__init__.py
import importlib
module = importlib.import_module(module_name)  # Deferred loop component loading

# core/tasks/__init__.py
import importlib.util
spec = importlib.util.spec_from_file_location(...)  # Legacy tasks module
```

---

## §6 Bootstrap Order

### §6.1 Initialization Sequence

To build a functioning Sterling instance from a cold start:

```
Phase 1: Foundation types (no inter-module deps)
    core.exceptions
    core.labels
    core.hashing
    core.verification
    core.canonicalization
    core.optimization
    core.ir_serialization
    core.state_model
    core.search_health

Phase 2: Infrastructure (depends on Phase 1)
    core.ir
    core.contracts
    core.kg
    core.carrier
    core.governance
    core.certification

Phase 3: Registries and memory (depends on Phase 2)
    core.operators  (registry, signatures)
    core.memory     (schemas, claim registry)

Phase 4: Domain layer (depends on Phase 3)
    core.text
    core.linguistics
    core.discourse
    core.intent
    core.pn
    core.realization
    core.worlds
    core.domains
    core.kernels

Phase 5: Orchestration (depends on Phase 4)
    core.value
    core.safeguards
    core.reasoning
    core.induction
    core.oracles
    core.recursion

Phase 6: Top-level (depends on Phase 5)
    core.engine
    core.tasks
    core.benchmarks
```

### §6.2 Bootstrap Invariant

**INV-M2**: Each phase may only import from the current phase or earlier phases. Violations must use TYPE_CHECKING or lazy imports.

### §6.3 Engine Assembly

The `core/engine/` module is the top-level orchestrator. Its `run_result.py` and main entry points lazily import from nearly every other module:

```
engine imports:
    → state_model, contracts, canonicalization     (Phase 1-2)
    → operators, governance                         (Phase 3)
    → worlds, discourse                            (Phase 4)
    → reasoning, induction, value, safeguards      (Phase 5)
```

Most of these imports are deferred (inside functions or under TYPE_CHECKING) to avoid pulling the entire dependency graph at module load time.

---

## §7 Module Coupling Metrics

### §7.1 Afferent Coupling (Ce — who depends on me)

High afferent coupling = many dependents = hard to change.

| Module | Dependents | Stability |
|--------|-----------|-----------|
| `state_model` | ~30 modules | Very high (frozen API) |
| `operators/registry` | ~15 modules | High (K6-A sealed) |
| `canonicalization` | ~12 modules | High (determinism contract) |
| `verification/hash_utils` | ~10 modules | High (hash contract) |
| `contracts` | ~8 modules | Medium-high |

### §7.2 Efferent Coupling (Ca — who I depend on)

High efferent coupling = many dependencies = fragile.

| Module | Dependencies | Fragility |
|--------|-------------|-----------|
| `engine` | ~15 modules | Very high (orchestrator) |
| `induction` | ~12 modules | High (cross-cutting) |
| `reasoning` | ~12 modules | High (integrates everything) |
| `proofs` | ~8 modules | Medium-high |
| `memory` | ~6 modules | Medium |

### §7.3 Self-Contained Modules

These modules have minimal or zero external `core/` dependencies:

| Module | External `core/` Imports |
|--------|------------------------|
| `search_health.py` | 0 (stdlib only) |
| `exceptions.py` | 0 |
| `hashing/` | 0 (self-referential only) |
| `labels/` | 0 |
| `kg/` | Self-referential only |
| `carrier/` | `verification/hash_utils`, `proofs/benchmark_artifacts` |

---

## §8 Package Boundaries

### §8.1 Lane Ownership

Sterling's CAWS configuration defines lane boundaries for multi-agent development:

| Lane | Modules | Crossing Risk |
|------|---------|---------------|
| **Reasoning** | `reasoning/`, `state_model.py`, `value/`, `search_health.py` | Low — mostly consumes from other lanes |
| **Carrier/Cert** | `carrier/`, `proofs/`, `hashing/`, `certification/` | Low — self-contained with clear contracts |
| **Operators** | `operators/`, `induction/`, `memory/`, `governance/` | High — `induction` has 482 import references |
| **Domains** | `worlds/`, `domains/`, `linguistics/`, `text/` | Medium — cross-references with operators |

### §8.2 Cross-Lane Dependencies

The most significant cross-lane dependencies:

| From Lane | To Lane | Through |
|-----------|---------|---------|
| Reasoning → Operators | `operators.registry`, `operators.universe` | Operator signatures and application |
| Reasoning → Domains | `worlds.base`, `worlds.discourse` | World adapters |
| Operators → Reasoning | `reasoning.episode_loader`, `reasoning.state_graph` | Episode data, graph types |
| Domains → Operators | `operators.registry` | Operator category types |

---

## §9 Invariants

1. **M-1**: Layer N modules may import from Layer ≤ N. Higher-layer imports require TYPE_CHECKING or lazy loading.
2. **M-2**: Bootstrap phases are sequential — each phase may only import from current or earlier phases.
3. **M-3**: `state_model.py` has near-zero outgoing dependencies (only TYPE_CHECKING imports). It is the universal data layer.
4. **M-4**: `search_health.py`, `exceptions.py`, and `hashing/` have zero `core/` dependencies.
5. **M-5**: `engine/` is the top-level orchestrator and must not be imported by lower layers.
6. **M-6**: Cross-lane modifications require explicit scope declaration in CAWS feature specs.

---

## §10 Related Documents

- [State Model Contract](state_model_contract_v1.md) — The universal data types at the center of the import graph
- [Operator Registry Contract](operator_registry_contract_v1.md) — The second-highest import hub
- [Hashing Contracts](hashing_contracts_v1.md) — Foundation-layer hash contracts
- [Governance Certification Contract](governance_certification_contract_v1.md) — Governance gate infrastructure

---

## §11 Source File Index

| File/Module | Defines |
|-------------|---------|
| `core/state_model.py` | UtteranceState, WorldState, StateNode (universal data types) |
| `core/search_health.py` | SearchHealthAccumulator (zero-dependency) |
| `core/exceptions.py` | Sterling exception hierarchy (zero-dependency) |
| `core/features.py` | Feature computation utilities |
| `core/operator_masking.py` | OperatorMaskBuilder |
| `core/ir_serialization.py` | IR token serialization |
| `core/__init__.py` | Package metadata only |
| `core/reasoning/__init__.py` | Lazy submodule loading via importlib |
| `core/tasks/__init__.py` | Legacy module loading via importlib.util |
| `core/id_registry.py` | ID registry and tracking |
| `core/external_ref.py` | External reference handling |
| `core/features_grouped.py` | Grouped feature computation |
| `core/ir_extraction.py` | IR feature extraction |
| `core/simple_kg.py` | Lightweight KG implementation |
| `core/logging_config.py` | Logging configuration |
| `core/profiling.py` | Performance profiling utilities |
| `core/tasks.py` | Legacy task definitions |

---

## Changelog

### v1.1 (2026-02-17)
- **§2.1**: Added missing modules: `pseudocode/` (7 files), `diagnostics/` (1 file)
- **§2.2**: Added 8 missing standalone files: `features_grouped.py`, `ir_extraction.py`, `id_registry.py`, `external_ref.py`, `simple_kg.py`, `logging_config.py`, `profiling.py`, `tasks.py`
- **§11**: Updated source file index with missing standalone files
