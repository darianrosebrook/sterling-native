---
status: Accepted
authority: adr
date: 2026-03-01
context: "Sterling now has three truth regimes (carrier/search, tool safety, epistemic) with a fourth (stochastic) planned. Each regime introduces world-specific operators and derived artifacts with distinct verification needs. Without an explicit authority division, the responsibilities of kernel, world, and verifier blur — leading to either duplicated checks, missing checks, or semantic logic leaking into the wrong layer."
---
# ADR 0009: Truth-Regime Authority Division (Kernel / World / Verifier)

## Decision

Every truth-regime world obeys a three-way authority division. Each authority has a defined write surface and a defined *non-responsibility*.

### 1. Kernel: Bounds the Write Surface

The kernel (`kernel/`) owns:

- **Operator dispatch**: Registry lookup, arg-length validation, apply() execution.
- **Effect-kind enforcement**: Post-apply diff counting matches the declared `EffectKind` (e.g., `WritesGuess` requires exactly K identity diffs and K status diffs).
- **Write-once invariant**: Slot transitions are `Hole → Provisional` only. No rewrites, no downgrades.
- **Operator arg builders**: Canonical byte-packing for each operator's arguments.

The kernel does **not**:

- Validate truth-dependent semantics (e.g., whether a feedback value matches the hidden truth, whether a stochastic outcome matches the seed).
- Know what the "correct" value for a world-specific computation is.
- Reference derived artifacts or evidence obligations.

**Principle**: The kernel guarantees that operators *mechanically* do what their `EffectKind` says. It does not guarantee that the values written are *correct* for the world's truth regime.

### 2. World: Computes Truth-Dependent Values

The world (`harness/src/worlds/`) owns:

- **Candidate enumeration**: `enumerate_candidates()` returns the legal actions from a given state. This is where truth-dependent computation happens — the world knows the hidden truth and produces candidates whose arg bytes encode world-specific values.
- **Goal predicate**: `is_goal()` defines when the search terminates.
- **State layout**: Layer/slot structure, initial payload encoding, schema descriptor.
- **Evidence obligations**: The world declares which derived artifacts must accompany the bundle (via `FixtureDimensions.evidence_obligations`).
- **Derived artifact rendering**: World-specific rendering functions (e.g., `render_epistemic_transcript()`) that reconstruct derived artifacts from the tape.

The world does **not**:

- Verify its own outputs. It is the *producer*, not the *auditor*.
- Modify the kernel's dispatch or validation behavior.
- Access the bundle or verification pipeline directly.

**Principle**: The world is the sole authority for "what values should appear in the evidence" — but it does not vouch for its own correctness.

### 3. Verifier: Proves Correspondence via Replay

The verifier (`harness/src/bundle.rs`, `harness/src/witness.rs`) owns:

- **Structural integrity**: Digest binding, canonical hash verification, artifact presence checks.
- **Cert-mode equivalence**: Re-rendering derived artifacts from the tape and comparing against the bundle-shipped artifacts (e.g., tool transcript equivalence, epistemic transcript equivalence via replay).
- **Invariant checking**: Running `ReplayInvariantChecker` implementations during winning-path replay to verify truth-dependent semantic properties (e.g., "feedback values match hidden truth", "belief is monotonically non-increasing").
- **Obligation enforcement**: Verifying that declared `evidence_obligations` match the artifacts present and the tape behavior observed (belt-and-suspenders checks).

The verifier does **not**:

- Generate evidence. It only checks evidence that was already produced.
- Know the hidden truth directly. It reconstructs it from the bundle-shipped inputs (fixture payload, tape records) using the same `ReplayInvariantChecker` interface the world provides.
- Modify operator semantics or candidate enumeration.

**Principle**: The verifier proves that the evidence is *internally consistent* and *corresponds to what the declared truth regime requires* — without being the original producer.

## Rationale

### Why three authorities, not two?

A two-authority model (kernel + verifier, or kernel + world) collapses distinct failure modes:

- If the kernel validates truth-dependent semantics, it needs world-specific knowledge — violating the sealed-kernel principle and requiring kernel changes for every new truth regime.
- If the world self-verifies, there is no independent check — the producer vouches for its own output.
- If the verifier generates evidence, the verification is not independent — it becomes self-referential.

The three-way division ensures: **the kernel constrains the space, the world fills the space with truth-dependent values, and the verifier checks that the filling is correct.**

### Proven across truth regimes

This division has been validated across three distinct truth regimes:

| Regime | Kernel bounds | World computes | Verifier proves |
|---|---|---|---|
| **Tool safety** (TOOLSCRIPT-001) | `EffectKind::StagesOneSlot`, `CommitsTransaction`, `RollsBackTransaction` | Staging layer writes, commit/rollback sequencing | Tool transcript equivalence render |
| **Epistemic** (POBS-001) | `EffectKind::WritesGuess`, `WritesFeedback`, `DeclaresSolution` | Feedback values from hidden truth, belief convergence | Epistemic transcript equivalence via replay, feedback correctness invariant, belief monotonicity |
| **Carrier/search** (baseline) | `EffectKind::WritesOneSlotFromArgs` | Candidate values from goal structure | Compilation replay, tape→graph equivalence |

Each regime uses the same three interfaces (`apply()` + `EffectKind`, `enumerate_candidates()` + `ReplayInvariantChecker`, `verify_bundle_with_profile()` + belt-and-suspenders) without cross-layer bleeding.

### Extension pattern for new regimes

A new truth regime (e.g., stochastic) follows the established pattern:

1. **Kernel**: Add new `EffectKind` variant(s) and operator(s) to the registry. Additive per ADR 0008.
2. **World**: Implement `WorldHarnessV1` + `SearchWorldV1`. Declare evidence obligations. Provide a `ReplayInvariantChecker` implementation.
3. **Verifier**: Add obligation-gated verification step(s). Add belt-and-suspenders tape detection for the new operator category.

No existing authority's contract changes. The division scales additively.

## Consequences

- New truth-regime worlds must explicitly document which values are truth-dependent (world's responsibility) vs. mechanically enforced (kernel's responsibility) vs. independently verified (verifier's responsibility).
- The `ReplayInvariantChecker` trait is the canonical interface for world-to-verifier truth-dependent checking. Worlds that need Cert-mode semantic verification must implement it.
- The belt-and-suspenders pattern (tape operator detection → obligation presence check) must be extended for each new operator category to prevent silent obligation stripping.
- Kernel operator additions remain additive (ADR 0008) — new `EffectKind` variants do not require existing worlds to change.
- This ADR composes with ADR 0006 (certification authority boundary): within the Rust evidence layer, the three-way division governs who produces, who constrains, and who verifies. Python governance (ADR 0006) sits above all three, referencing Rust bundles by digest.
