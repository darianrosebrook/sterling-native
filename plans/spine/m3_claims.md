# M3 Claim Catalog

> What Sterling Native is allowed to say after M3 is complete.
> Each claim has a falsifier: a concrete way to disprove it.

## M3-CLAIM-001: Bundle determinism (in-process)

**Statement**: `run(RomeMini)` is a pure function; N=10 consecutive in-process runs produce identical bundle digests, artifact content bytes, manifest bytes, and digest_basis bytes.

**Scope**: This claim covers the `RomeMini` world (1 layer, 2 slots, 3 arg slots, one `SET_SLOT` operation). The determinism property should hold for any `WorldHarnessV1` implementation that returns deterministic values, but only `RomeMini` is locked by tests.

**Falsifier**: Call `run(&RomeMini)` 10 times. If any run produces a different `bundle.digest`, different artifact content bytes for any artifact, or different manifest/digest_basis bytes, the claim is false.

**Required artifacts**: `bundle_digest_deterministic_n10`, `all_artifact_bytes_deterministic_n10`, `manifest_bytes_deterministic_n10` in `s1_m3_determinism.rs`.

## M3-CLAIM-002: Cross-process determinism

**Statement**: The `harness_fixture` binary produces identical output under 4 environment variants: baseline (workspace root, no overrides), different cwd (`/tmp`), different locale (`LC_ALL=C LANG=C`), and spurious env vars (`STERLING_NOISE`, `TZ`, `HOME`).

**Falsifier**: Spawn `harness_fixture` under the 4 variants. If any output line (bundle_digest, fixture_hash, trace_payload_hash, verification_verdict, artifact_count) differs between variants, the claim is false.

**Required artifacts**: `crossproc_determinism_four_env_variants` in `s1_m3_crossproc.rs`, `harness_fixture` binary at `tests/lock/src/bin/harness_fixture.rs`.

## M3-CLAIM-003: Normative/observational isolation

**Statement**: Mutating `trace.bst1` envelope bytes changes the `trace.bst1` content hash and therefore the manifest (which lists all artifacts with content hashes), but leaves the bundle digest unchanged. The bundle digest is computed from `digest_basis`, which includes only normative artifact hashes. `trace.bst1` is observational. The normative `verification_report.json` declares the trace's payload-level commitments (`payload_hash`, `step_chain_digest`). `verify_bundle()` mechanically enforces this binding by recomputing these values from `trace.bst1` and comparing to the report, enabling detection of trace/report inconsistency.

**What `verify_bundle()` checks** (the bundle verification algorithm):
1. Each artifact's `content_hash` matches recomputed `canonical_hash(DOMAIN_BUNDLE_ARTIFACT, content)`.
2. `manifest` bytes match the canonical JSON projection recomputed from all artifacts (not just stored bytes).
3. `digest_basis` bytes match the canonical JSON projection recomputed from normative artifacts only.
4. `digest` matches `canonical_hash(DOMAIN_BUNDLE_DIGEST, digest_basis)`.
5. `manifest`, `digest_basis`, and normative JSON artifacts are in canonical JSON form.
6. If `trace.bst1` and `verification_report.json` both exist: `payload_hash` and `step_chain_digest` recomputed from `trace.bst1` match the values declared in the report.

**Falsifier** (envelope mutation): Run `RomeMini`, mutate bytes in the `trace.bst1` envelope region, rebuild the bundle. If the bundle digest changes, the normative/observational separation is broken. If the manifest does NOT change, auditability is broken.

**Falsifier** (trace/report binding): Run `RomeMini`, mutate trace body bytes, recompute the trace artifact's content hash and update the manifest (keeping verification_report unchanged). Call `verify_bundle()`. If it does not return `PayloadHashMismatch` or `StepChainMismatch`, the binding is broken.

**Required artifacts**: `bundle_digest_ignores_observational_envelope_mutation` in `s1_m3_determinism.rs`, `normative_observational_classification` in `s1_m3_harness.rs`, `verify_bundle_passes_clean_bundle` in `s1_m3_harness.rs`, `verify_bundle_detects_trace_report_payload_hash_mismatch` in `s1_m3_harness.rs`.

## M3-CLAIM-004: Replay scope declared

**Statement**: The verification report contains `planes_verified: ["identity", "status"]`, declaring exactly which planes were checked by `replay_verify()`. This prevents replay verification from silently overstating coverage.

**Scope**: Adding `"evidence"` to `planes_verified` is a claim expansion and must be accompanied by a new lock test and claim catalog bump.

**Falsifier**: Parse `verification_report.json` from `run(RomeMini)`. If `planes_verified` is absent, empty, or contains planes other than `["identity", "status"]`, the claim is false.

**Required artifacts**: `verification_report_declares_planes_verified` in `s1_m3_harness.rs`.

---

## Admissibility

A claim is admissible only when:
1. All required artifacts exist and pass in CI.
2. The harness crate does not import `sha2` — all hashing routes through kernel's `canonical_hash`.
3. No manual steps are required beyond `cargo test --workspace`.
4. Claims apply only to the `RomeMini` world and the implemented operator set (sentinels + `SET_SLOT`).
5. `verify_bundle()` mechanically enforces the normative/observational boundary by recomputing derived projections (manifest, digest_basis) from artifacts and verifying trace/report payload commitments. The binding is not automatic — it requires calling `verify_bundle()`.
