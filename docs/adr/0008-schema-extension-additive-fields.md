---
status: Accepted
authority: adr
date: 2026-02-28
context: "SearchGraphV1, SearchTapeV1, and verification_report.json are canonical surfaces with sorted-key canonical JSON. Adding new binding fields (e.g., operator_set_digest for the operator registry MVP) requires a defined extension mechanism. Open decision #7 in the parity audit."
---
# ADR 0008: Schema Extension via Additive Fields

## Decision

Canonical JSON schemas (tape headers, graph metadata, verification reports) extend via **additive fields within a schema version**. Breaking changes require a **version bump**.

### Rules

1. **Additive changes** (new optional or required fields) are permitted within a schema version. Canonical JSON sorting makes this safe — adding a key preserves sort order of existing keys, so existing hashes of the *old* schema's canonical bytes are not recomputable from the *new* schema's bytes, but the reverse is also true: the new schema does not invalidate old bundles (old bundles simply lack the new field).

2. **Version-gated validation**: Each `schema_version` value (e.g., `"search_tape.v1"`, `"verification_report.v1"`) defines which fields are required and which are optional. The verifier checks field requirements based on the declared schema version. A new required field means a new schema version.

3. **No "extension block" or sub-record map**: Extension blocks add indirection complexity. Since canonical JSON already handles arbitrary sorted keys, adding fields directly is simpler and equally safe.

4. **Breaking changes require version bumps**: Removing a field, changing a field's semantics, or changing a field's type is a breaking change requiring a new schema version (e.g., `"search_tape.v2"`). Old bundles remain verifiable under their declared version.

5. **Digest computation is always over the full canonical bytes**: The bundle digest, tape chain hash, and all binding digests are computed over whatever canonical JSON bytes the schema version produces. Adding a field changes the canonical bytes (and therefore the digest) for new bundles — this is correct and expected.

### Concrete application to operator registry

Adding `operator_set_digest` to tape headers and graph metadata:
- The field is added to `MetadataBindings` in Rust.
- Tape header and graph metadata builders include it in canonical JSON.
- `schema_version` stays `"search_tape.v1"` if the field is optional (pre-registry bundles lack it).
- If the field becomes required (fail-closed on absence), bump to `"search_tape.v1.1"` or `"search_tape.v2"`.
- Verification: if `operator_set_digest` is present, verify it binds correctly. If absent, pass (Base profile for pre-registry bundles).

## Rationale

- Canonical JSON with sorted keys is inherently additive-safe — new keys sort in deterministically.
- Extension blocks (a map of domain-separated sub-records) were considered but rejected: they add indirection, require defining sub-record canonicalization rules, and don't actually prevent version churn — you still need to version the extension block schema.
- Strict-version-bumps-only was considered but rejected for additive changes: it would force a tape/graph schema bump just to add `operator_set_digest`, `tool_transcript_digest`, etc. — fields that are purely additive and don't change existing semantics.

## Consequences

- New binding fields (operator_set_digest, future tool transcript digests, stochastic witness fields) can be added without schema version changes, as long as they are initially optional.
- Making a previously optional field required is a minor version bump.
- Verifiers must be forward-compatible: unknown fields in canonical JSON are ignored (not rejected).
- Old bundles (pre-field-addition) remain verifiable — the verifier simply skips checks for absent optional fields.
