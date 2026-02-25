//! S1-M1-DETERMINISM-CROSSPROC: cross-process determinism test.
//!
//! Spawns the `compile_fixture` binary under >=3 environment variants
//! and asserts that all produce identical output. This proves that
//! compilation results are not influenced by process-level state
//! (cwd, locale, env vars, iteration order).

use std::path::Path;
use std::process::Command;

/// Resolve the absolute path to the golden fixture.
fn fixture_path() -> String {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests/ exists")
        .parent()
        .expect("workspace root exists");
    workspace_root
        .join("tests/fixtures/rome_2x4_golden.json")
        .to_string_lossy()
        .to_string()
}

/// Resolve the path to the compiled binary.
///
/// `cargo test` puts test binaries in `target/debug/` (or the profile dir).
/// The `compile_fixture` binary lives alongside them.
fn binary_path() -> String {
    let mut path = std::env::current_exe()
        .expect("can resolve test binary path")
        .parent()
        .expect("binary dir exists")
        .parent()
        .expect("deps parent exists")
        .to_path_buf();
    path.push("compile_fixture");
    path.to_string_lossy().to_string()
}

/// Run the binary with the given cwd and environment overrides.
/// Returns stdout as a string.
fn run_variant(work_dir: &str, env_overrides: &[(&str, &str)]) -> String {
    let bin = binary_path();
    let fixture = fixture_path();

    let mut command = Command::new(&bin);
    command.arg(&fixture).current_dir(work_dir);

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
        "compile_fixture exited with {}: stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout is valid UTF-8")
}

// --- S1-M1-DETERMINISM-CROSSPROC ---

#[test]
fn crossproc_determinism_three_env_variants() {
    // Variant 1: baseline — cwd is workspace root, no locale overrides.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let baseline = run_variant(&workspace_root, &[]);

    // Sanity: output should contain the expected digests.
    assert!(
        baseline.contains("identity_digest=sha256:"),
        "baseline output missing identity_digest"
    );
    assert!(
        baseline.contains("evidence_digest=sha256:"),
        "baseline output missing evidence_digest"
    );

    // Variant 2: different cwd (/ or /tmp).
    let alt_cwd = if cfg!(target_os = "windows") {
        "C:\\"
    } else {
        "/tmp"
    };
    let variant_cwd = run_variant(alt_cwd, &[]);
    assert_eq!(
        baseline, variant_cwd,
        "output differs when cwd changes from {workspace_root} to {alt_cwd}"
    );

    // Variant 3: different locale env.
    let variant_locale = run_variant(&workspace_root, &[("LC_ALL", "C"), ("LANG", "C")]);
    assert_eq!(
        baseline, variant_locale,
        "output differs when LC_ALL=C LANG=C"
    );

    // Variant 4: spurious env vars that should not affect output.
    let variant_noise = run_variant(
        &workspace_root,
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

#[test]
fn crossproc_output_matches_golden_fixture() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let output = run_variant(&workspace_root.to_string_lossy(), &[]);

    // Load golden fixture for expected values.
    let fixture_contents =
        std::fs::read_to_string(workspace_root.join("tests/fixtures/rome_2x4_golden.json"))
            .unwrap();
    let fixture: serde_json::Value = serde_json::from_str(&fixture_contents).unwrap();

    let expected_identity = fixture["identity_digest"].as_str().unwrap();
    let expected_evidence = fixture["evidence_digest"].as_str().unwrap();
    let expected_evidence_hex = fixture["evidence_bytes_hex"].as_str().unwrap();

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "expected 3 output lines, got {}",
        lines.len()
    );
    assert_eq!(
        lines[0],
        format!("identity_digest={expected_identity}"),
        "identity digest mismatch"
    );
    assert_eq!(
        lines[1],
        format!("evidence_digest={expected_evidence}"),
        "evidence digest mismatch"
    );
    assert_eq!(
        lines[2],
        format!("evidence_bytes_hex={expected_evidence_hex}"),
        "evidence bytes mismatch"
    );
}
