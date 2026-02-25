//! S1-M2-DETERMINISM-CROSSPROC: cross-process determinism test.
//!
//! Spawns the `trace_fixture` binary under >=3 environment variants
//! and asserts that all produce identical output. This proves that
//! trace serialization, hashing, and replay are not influenced by
//! process-level state (cwd, locale, env vars).
//!
//! Also verifies that Native's `.bst1` bytes and payload hash match
//! the v1 Python oracle fixture.

use std::path::Path;
use std::process::Command;

/// Resolve the path to the compiled `trace_fixture` binary.
///
/// NOTE: If CI expands to Windows, this needs `.exe` suffix handling
/// (e.g., `path.set_extension("exe")` on `cfg!(target_os = "windows")`).
/// Windows is not a current target.
fn binary_path() -> String {
    let mut path = std::env::current_exe()
        .expect("can resolve test binary path")
        .parent()
        .expect("binary dir exists")
        .parent()
        .expect("deps parent exists")
        .to_path_buf();
    path.push("trace_fixture");
    path.to_string_lossy().to_string()
}

/// Resolve the workspace root.
fn workspace_root() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests/ exists")
        .parent()
        .expect("workspace root exists")
        .to_string_lossy()
        .to_string()
}

/// Run the binary with the given cwd and environment overrides.
/// Returns stdout as a string.
fn run_variant(work_dir: &str, env_overrides: &[(&str, &str)]) -> String {
    let bin = binary_path();

    let mut command = Command::new(&bin);
    command.current_dir(work_dir);

    // Clear locale-related env to establish baseline, then apply overrides.
    command
        .env_remove("LC_ALL")
        .env_remove("LC_COLLATE")
        .env_remove("LANG")
        .env_remove("LANGUAGE");

    for &(key, val) in env_overrides {
        command.env(key, val);
    }

    let output = command.output().unwrap_or_else(|e| {
        panic!("failed to spawn {bin} (work_dir={work_dir}, overrides={env_overrides:?}): {e}")
    });

    assert!(
        output.status.success(),
        "trace_fixture exited with {}: stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout is valid UTF-8")
}

// ---------------------------------------------------------------------------
// Cross-process determinism
// ---------------------------------------------------------------------------

#[test]
fn crossproc_determinism_three_env_variants() {
    let root = workspace_root();
    let baseline = run_variant(&root, &[]);

    // Sanity: output should contain expected fields.
    assert!(
        baseline.contains("bst1_hex="),
        "baseline output missing bst1_hex"
    );
    assert!(
        baseline.contains("payload_hash=sha256:"),
        "baseline output missing payload_hash"
    );
    assert!(
        baseline.contains("replay_verdict=Match"),
        "baseline output missing replay_verdict=Match"
    );

    // Variant 2: different cwd.
    let alt_cwd = if cfg!(target_os = "windows") {
        "C:\\"
    } else {
        "/tmp"
    };
    let variant_cwd = run_variant(alt_cwd, &[]);
    assert_eq!(
        baseline, variant_cwd,
        "output differs when cwd changes from {root} to {alt_cwd}"
    );

    // Variant 3: different locale env.
    let variant_locale = run_variant(&root, &[("LC_ALL", "C"), ("LANG", "C")]);
    assert_eq!(
        baseline, variant_locale,
        "output differs when LC_ALL=C LANG=C"
    );

    // Variant 4: spurious env vars.
    let variant_noise = run_variant(
        &root,
        &[
            ("STERLING_NOISE", "should_not_matter"),
            ("TZ", "America/New_York"),
            ("HOME", "/nonexistent"),
        ],
    );
    assert_eq!(
        baseline, variant_noise,
        "output differs with spurious env vars (STERLING_NOISE, TZ, HOME)"
    );
}

// ---------------------------------------------------------------------------
// V1 oracle parity
// ---------------------------------------------------------------------------

#[test]
fn crossproc_output_matches_v1_oracle() {
    // NOTE: This test compares full .bst1 wire bytes (including envelope).
    // That means envelope serialization (field order, formatting) is part of
    // the wire-compat contract. Changing envelope layout will break this test
    // even though the payload hash surface is unaffected. This is intentional:
    // M2 commits to full wire stability, not just payload stability.
    let root = workspace_root();
    let output = run_variant(&root, &[]);

    // Load v1-generated golden fixture.
    let fixture_path = Path::new(&root).join("tests/fixtures/m2_golden_trace.json");
    let fixture_contents = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", fixture_path.display()));
    let fixture: serde_json::Value =
        serde_json::from_str(&fixture_contents).expect("fixture is valid JSON");

    let expected_bst1_hex = fixture["bst1_hex"].as_str().unwrap();
    let expected_payload_hash = fixture["payload_hash"].as_str().unwrap();

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        lines.len(),
        5,
        "expected 5 output lines, got {}",
        lines.len()
    );

    // Line 0: bst1_hex — byte-for-byte match with v1
    assert_eq!(
        lines[0],
        format!("bst1_hex={expected_bst1_hex}"),
        "Native .bst1 bytes differ from v1 Python oracle"
    );

    // Line 1: payload_hash — digest match with v1
    assert_eq!(
        lines[1],
        format!("payload_hash={expected_payload_hash}"),
        "Native payload hash differs from v1 Python oracle"
    );

    // Lines 2-3: step chain (Native-only, no v1 oracle — just verify present)
    assert!(
        lines[2].starts_with("step_chain_0=sha256:"),
        "step_chain_0 missing or malformed"
    );
    assert!(
        lines[3].starts_with("step_chain_final=sha256:"),
        "step_chain_final missing or malformed"
    );

    // Line 4: replay verdict
    assert_eq!(
        lines[4], "replay_verdict=Match",
        "replay verification failed"
    );
}

// ---------------------------------------------------------------------------
// V1 oracle: direct .bst1 file read
// ---------------------------------------------------------------------------

#[test]
fn v1_bst1_file_parseable_by_native_reader() {
    let root = workspace_root();
    let bst1_path = Path::new(&root).join("tests/fixtures/m2_golden_trace.bst1");
    let bst1_bytes = std::fs::read(&bst1_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", bst1_path.display()));

    // Native reader must parse the v1-generated .bst1 without error.
    let trace = sterling_kernel::carrier::trace_reader::bytes_to_trace(&bst1_bytes)
        .unwrap_or_else(|e| panic!("Native reader rejected v1 .bst1: {e:?}"));

    // Payload hash must match v1's value.
    let ph = sterling_kernel::proof::trace_hash::payload_hash(&trace).unwrap();
    assert_eq!(
        ph.as_str(),
        "sha256:da06d8cc3476cefb662351cea3c1ea21d7ffa7e0a3f11590fa6367501e41a091",
        "Native payload hash of v1-generated .bst1 differs from v1's own hash"
    );

    // Re-serialize must produce identical bytes (round-trip).
    let rebytes = sterling_kernel::carrier::trace_writer::trace_to_bytes(&trace).unwrap();
    assert_eq!(
        bst1_bytes, rebytes,
        "Native re-serialization of v1-generated .bst1 differs from original"
    );
}
