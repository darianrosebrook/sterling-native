> **NOTE: Quarantined v1 canonical contract. Non-authoritative for Sterling Native. See [promotion criteria](README.md).**

Text Boundary Canonical Index
=============================

Purpose
-------
Single source of truth for text-boundary contracts. Boundary module docstrings
must reference only the canonical items listed below (or this index), not
experimental or advisory docs.

Identity Model (Authoritative)
------------------------------
Layer A (authoritative identity):
- schema_id, schema_version, domain-separated content_hash, size

Layer B (retrieval/storage, non-authoritative):
- storage backend, path/URI, encoding/compression, chunking

Layer C (governance lineage, non-authoritative identity but required for audit):
- pipeline_id, certificate_id, run_intent, request_digest, config_digest,
  completeness declarations

Hash-Critical Boundary Artifacts
--------------------------------
These are the only artifacts required to be hash-stable for certification.
All other telemetry is non-hash-critical and must not gate cert runs.
- TextPacket / TextIntakeIR
- RunResult (refs + digests only)
- ProofBundle / TraceBundle
- TextRealizationIR

Canonical Contracts (Authoritative)
-----------------------------------
| Contract | Location | Version | Status | Notes |
| --- | --- | --- | --- | --- |
| Text I/O Contract v1 | docs/canonical/text_io_contract_v1.md | 1.0 | Active | Defines intake trust boundary + invariants. |
| Text Realization Contract v0 | docs/specifications/text-semantic-ir/realization-ir-text-contract.md | 0.x | Authoritative (location provisional) | Canonical until relocated into docs/canonical/. |
| Schema Registry (Draft) | docs/canonical/schema_registry.md | 0.1 | Draft | Planned source of canonical schema identities. |

Non-Canonical References (Advisory Only)
----------------------------------------
- docs/specifications/text-semantic-ir/semantic-text-contract.md
  - Realization-oriented notes; misnamed, rename pending.
- docs/internal/experiments/diffusion/SWM-IO.md
  - Experimental SWM ingestion notes; do not cite in code docstrings.
- docs/internal/experiments/diffusion/SWM-Operator.md
  - Experimental operator notes; do not cite in code docstrings.

Update Policy
-------------
- Any change to a canonical contract requires:
  - Version bump in the contract,
  - Update to this index,
  - CI check passing for boundary docstring references.
