//! S1-M0 Acceptance Test: Build-graph isolation.
//!
//! Verifies that nothing under `kernel/` references v1 implementation code.
//! Per ADR 0005: "v1 is a test oracle, not a dependency."

use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Forbidden patterns in non-comment Rust source lines.
const FORBIDDEN_PATTERNS: &[&str] = &["reference::v1", "v1_impl", "docs/reference/v1"];

/// Scan all `.rs` files under a directory for forbidden patterns.
fn scan_rs_files_for_v1_refs(dir: &Path) -> Vec<(String, usize, String)> {
    let mut violations = Vec::new();
    walk(dir, &mut violations);
    violations
}

fn walk(dir: &Path, violations: &mut Vec<(String, usize, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, violations);
        } else if path.extension().is_some_and(|e| e == "rs") {
            check_file(&path, violations);
        }
    }
}

fn check_file(path: &Path, violations: &mut Vec<(String, usize, String)>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // Skip comments.
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        for pattern in FORBIDDEN_PATTERNS {
            if trimmed.contains(pattern) {
                violations.push((path.display().to_string(), line_no + 1, line.to_string()));
            }
        }
    }
}

/// Resolve the workspace root from `CARGO_MANIFEST_DIR` of the lock-tests crate.
fn workspace_root() -> &'static Path {
    // lock-tests lives at tests/lock/, so workspace root is ../..
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("tests/ exists")
        .parent()
        .expect("workspace root exists")
}

#[test]
fn kernel_source_has_no_v1_references() {
    let kernel_src = workspace_root().join("kernel").join("src");

    let violations = scan_rs_files_for_v1_refs(&kernel_src);

    if !violations.is_empty() {
        let mut msg = String::from("v1 references found in kernel source:\n");
        for (file, line, content) in &violations {
            let _ = writeln!(msg, "  {file}:{line}: {content}");
        }
        panic!("{msg}");
    }
}

#[test]
fn kernel_cargo_toml_has_no_v1_dependencies() {
    let cargo_toml = workspace_root().join("kernel").join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).expect("kernel/Cargo.toml must exist");

    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        assert!(
            !trimmed.contains("v1_impl") && !trimmed.contains("reference/v1"),
            "kernel/Cargo.toml line {}: references v1 code: {trimmed}",
            line_no + 1
        );
    }
}
