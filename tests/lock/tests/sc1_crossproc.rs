//! SC1-M1-SEARCH-DETERMINISM-CROSSPROC: cross-process determinism test
//! for search bundles.
//!
//! Spawns the `search_fixture` binary under 4 environment variants
//! and asserts all produce identical output.

use std::path::Path;
use std::process::Command;

fn binary_path() -> String {
    let mut path = std::env::current_exe()
        .expect("can resolve test binary path")
        .parent()
        .expect("binary dir exists")
        .parent()
        .expect("deps parent exists")
        .to_path_buf();
    path.push("search_fixture");
    path.to_string_lossy().to_string()
}

fn workspace_root() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests/ exists")
        .parent()
        .expect("workspace root exists")
        .to_string_lossy()
        .to_string()
}

fn run_variant(work_dir: &str, env_overrides: &[(&str, &str)]) -> String {
    let bin = binary_path();

    let mut command = Command::new(&bin);
    command.current_dir(work_dir);

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
        "search_fixture exited with {}: stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout is valid UTF-8")
}

// ACCEPTANCE: SC1-M1-SEARCH-DETERMINISM-CROSSPROC
#[test]
fn crossproc_determinism_four_env_variants() {
    let root = workspace_root();
    let baseline = run_variant(&root, &[]);

    // Sanity checks.
    assert!(
        baseline.contains("bundle_digest=sha256:"),
        "baseline output missing bundle_digest"
    );
    assert!(
        baseline.contains("search_graph_digest=sha256:"),
        "baseline output missing search_graph_digest"
    );
    assert!(
        baseline.contains("policy_digest=sha256:"),
        "baseline output missing policy_digest"
    );
    assert!(
        baseline.contains("termination_reason=goal_reached"),
        "baseline output missing termination_reason=goal_reached"
    );
    assert!(
        baseline.contains("artifact_count=8"),
        "baseline output missing artifact_count=8"
    );
    assert!(
        baseline.contains("search_graph_normative=true"),
        "baseline output missing search_graph_normative=true"
    );
    assert!(
        baseline.contains("mode=search"),
        "baseline output missing mode=search"
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
        "output differs with spurious env vars"
    );
}
