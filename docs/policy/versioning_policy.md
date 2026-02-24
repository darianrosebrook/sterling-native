---
status: "Draft (Sterling Native)"
authority: policy
scope: "schemas, registries, epochs, and compatibility"
---
# Versioning and Epoch Policy

## Definitions

- Engine version: semantic version of the kernel implementation.
- Schema descriptor: (schema_id, schema_version, schema_hash)
- Registry snapshot: (registry_epoch, registry_hash)
- Epoch: a period during which schema + registry are frozen for compilation and replay.

## Rules

1) Compilation is parameterized by (payload, schema_descriptor, registry_snapshot).
2) Schema and registry do not change mid-episode.
3) Epoch transitions occur between episodes.
4) Any change that alters canonical bytes requires a version/epoch change and updated goldens.

## Compatibility

- Old artifact schemas may be supported via a single canonical upgrade path.
- If an artifact cannot be upgraded deterministically, it is treated as incompatible and fails closed in CERTIFIED mode.
