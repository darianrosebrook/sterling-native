---
status: Active
authority: policy
scope: "All documents in docs/ except docs/reference/v1/"
---
# Document Authority Policy

---

## Authority Levels

Every document outside `docs/reference/v1/` must declare an `Authority:` field in its front-matter. The field indicates what kind of truth the document represents.

| Authority | Meaning | Enforcement | Location |
|-----------|---------|-------------|----------|
| `canonical` | Binding contract, invariant, or definition. Edits require version bump and review. | CI-enforced: no v1 stage taxonomy, no legacy operator labels, no broken links. | `docs/canonical/` |
| `policy` | Process rule or eligibility gate. Binding on workflow, not on runtime behavior. | Review-enforced. | `docs/policy/` |
| `adr` | Architecture decision record. Binding until superseded by a later ADR. | Immutable once accepted (new ADR to change). | `docs/adr/` |
| `architecture` | Design target or planning artifact. Informative, not binding. | None — readers must not cite as requirement. | `docs/architecture/` |
| `reference` | Historical material. Non-authoritative for Sterling Native. | None — explicitly non-normative. | `docs/reference/v1/` |
| `ephemeral` | Temporal work (roadmaps, session notes, sprint plans). Becomes stale. | Gitignored — not committed to repo. | `docs/ephemeral/` |

## Rules

1. **Every `.md` file** in `docs/` (except `docs/reference/v1/` and README/index files) must have an `Authority:` field in its front-matter.
2. **CI lint** checks that the authority field is present and matches the file's directory:
   - Files in `docs/canonical/` must declare `Authority: canonical`
   - Files in `docs/policy/` must declare `Authority: policy`
   - Files in `docs/adr/` must declare `Authority: adr`
   - Files in `docs/architecture/` must declare `Authority: architecture`
3. **`docs/reference/v1/`** is exempt — those files use v1 conventions and are non-normative by definition.
4. **Templates** (`docs/templates/`) are exempt — they are structural scaffolds, not assertions.
5. **README/index files** are exempt — they are navigational, not authoritative.

## Why This Exists

Sterling v1's docs tree collapsed multiple authority regimes (canonical contracts, working plans, session reviews, generated audits, historical archive) into one browsing surface. Readers had no reliable way to know what was binding vs contextual vs stale. Phase-specific work plans lived alongside canonical contracts and got cited as precedent, then quietly became requirements.

This policy makes the authority boundary enforceable rather than advisory.
