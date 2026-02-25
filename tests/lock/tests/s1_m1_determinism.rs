//! S1-M1 Determinism and structural lock tests.
//!
//! - S1-M1-DETERMINISM-INPROC: N>=10 compile calls yield identical results.
//! - S1-M1-NO-PATH-IN-HASH: no paths/cwd/timestamps in hashed surfaces.
//! - S1-M1-ONE-CANONICALIZER: exactly one canonical JSON implementation.
//! - S1-M1-EQ-SEPARATION-LOCK: `identity_eq` vs `bitwise_eq` + digests.
//! - S1-M1-ORDERING-INVARIANCE: reordered JSON keys → identical output.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use sterling_kernel::carrier::bytestate::{ByteStateV1, SchemaDescriptor, SlotStatus};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::proof::hash::{canonical_hash, DOMAIN_EVIDENCE_PLANE, DOMAIN_IDENTITY_PLANE};

fn workspace_root() -> &'static Path {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("tests/ exists")
        .parent()
        .expect("workspace root exists")
}

fn rome_registry() -> RegistryV1 {
    RegistryV1::new(
        "epoch-0".into(),
        vec![
            (Code32::new(1, 0, 0), "rome:node:start".into()),
            (Code32::new(1, 0, 1), "rome:node:forum".into()),
            (Code32::new(1, 0, 2), "rome:node:colosseum".into()),
            (Code32::new(1, 1, 0), "rome:edge:road".into()),
        ],
    )
    .unwrap()
}

fn rome_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        id: "rome".into(),
        version: "1.0".into(),
        hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
    }
}

// --- S1-M1-DETERMINISM-INPROC ---

#[test]
fn determinism_inproc_n10() {
    let registry = rome_registry();
    let schema = rome_schema();
    let payload = r#"{"layer_count":2,"slot_count":4,"identity":[[1,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]],"status":[0,0,0,0,0,0,0,0]}"#;

    let first = compile(payload.as_bytes(), &schema, &registry).unwrap();
    for i in 1..=10 {
        let result = compile(payload.as_bytes(), &schema, &registry).unwrap();
        assert!(
            result.state.bitwise_eq(&first.state),
            "run {i}: state bytes differ"
        );
        assert_eq!(
            result.identity_digest, first.identity_digest,
            "run {i}: identity digest differs"
        );
        assert_eq!(
            result.evidence_digest, first.evidence_digest,
            "run {i}: evidence digest differs"
        );
        assert_eq!(
            result.compilation_manifest, first.compilation_manifest,
            "run {i}: manifest differs"
        );
    }
}

// --- S1-M1-NO-PATH-IN-HASH, S1-M2-NO-PATH-IN-HASH ---

#[test]
fn no_paths_in_hashed_surfaces() {
    let registry = rome_registry();
    let schema = rome_schema();
    let payload =
        r#"{"layer_count":1,"slot_count":2,"identity":[[1,0,0,0],[0,0,0,0]],"status":[0,0]}"#;

    let result = compile(payload.as_bytes(), &schema, &registry).unwrap();

    // Check manifest (the only "derived text" surface).
    let manifest_str = std::str::from_utf8(&result.compilation_manifest).unwrap();

    let suspicious_patterns = [
        "/Users/",
        "/home/",
        "/tmp/",
        "\\Users\\",
        "cwd",
        "hostname",
        "username",
        "timestamp",
        "time",
        "date",
    ];
    for pattern in suspicious_patterns {
        assert!(
            !manifest_str.contains(pattern),
            "manifest contains suspicious pattern: {pattern}"
        );
    }

    // Digests are just hex — can't contain paths, but check they're clean.
    assert!(result.identity_digest.as_str().starts_with("sha256:"));
    assert!(result.evidence_digest.as_str().starts_with("sha256:"));
}

// --- S1-M1-ONE-CANONICALIZER ---

#[test]
fn one_canonical_json_implementation() {
    let kernel_src = workspace_root().join("kernel").join("src");
    let mut canon_impls = Vec::new();
    scan_for_canon_impls(&kernel_src, &mut canon_impls);

    // Exactly one file should define canonical JSON bytes.
    let expected_file = "proof/canon.rs";
    let canon_files: Vec<&str> = canon_impls
        .iter()
        .map(|s| {
            s.strip_prefix(&kernel_src.to_string_lossy().to_string())
                .unwrap_or(s)
                .trim_start_matches('/')
        })
        .collect();

    assert_eq!(
        canon_files.len(),
        1,
        "expected exactly 1 canonical JSON implementation, found {}: {canon_files:?}",
        canon_files.len()
    );
    assert!(
        canon_files[0].ends_with(expected_file),
        "canonical JSON implementation should be in {expected_file}, found in {}",
        canon_files[0]
    );
}

fn scan_for_canon_impls(dir: &Path, results: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_for_canon_impls(&path, results);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            // Look for functions that produce canonical JSON bytes.
            // The canonical implementation uses "canonical_json_bytes" as its name.
            // If another file defines a similar function, this test catches it.
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                    continue;
                }
                if trimmed.contains("fn canonical_json_bytes")
                    || trimmed.contains("fn canonicalize_json")
                    || trimmed.contains("fn json_canonical")
                {
                    results.push(path.display().to_string());
                    break;
                }
            }
        }
    }
}

// --- S1-M1-ORDERING-INVARIANCE ---

#[test]
fn ordering_invariance_compile() {
    let registry = rome_registry();
    let schema = rome_schema();

    // Three different key orderings.
    let payloads = [
        r#"{"layer_count":1,"slot_count":2,"identity":[[1,0,0,0],[0,0,0,0]],"status":[0,0]}"#,
        r#"{"status":[0,0],"identity":[[1,0,0,0],[0,0,0,0]],"slot_count":2,"layer_count":1}"#,
        r#"{"identity":[[1,0,0,0],[0,0,0,0]],"layer_count":1,"status":[0,0],"slot_count":2}"#,
    ];

    let first = compile(payloads[0].as_bytes(), &schema, &registry).unwrap();
    for (i, payload) in payloads.iter().enumerate().skip(1) {
        let result = compile(payload.as_bytes(), &schema, &registry).unwrap();
        assert!(
            result.state.bitwise_eq(&first.state),
            "ordering {i}: state differs"
        );
        assert_eq!(
            result.identity_digest, first.identity_digest,
            "ordering {i}: identity digest differs"
        );
        assert_eq!(
            result.evidence_digest, first.evidence_digest,
            "ordering {i}: evidence digest differs"
        );
    }
}

// --- S1-M1-EQ-SEPARATION-LOCK ---

#[test]
fn eq_separation_digests_differ_on_status_change() {
    let mut a = ByteStateV1::new(2, 4);
    let mut b = ByteStateV1::new(2, 4);

    a.set_identity(0, 0, Code32::new(1, 0, 0));
    b.set_identity(0, 0, Code32::new(1, 0, 0));

    a.set_status(0, 0, SlotStatus::Certified);
    b.set_status(0, 0, SlotStatus::Shadow);

    // identity_eq true.
    assert!(a.identity_eq(&b));
    // bitwise_eq false.
    assert!(!a.bitwise_eq(&b));
    // Identity digests: same (status ignored).
    let id_a = canonical_hash(DOMAIN_IDENTITY_PLANE, &a.identity_bytes());
    let id_b = canonical_hash(DOMAIN_IDENTITY_PLANE, &b.identity_bytes());
    assert_eq!(id_a, id_b, "identity digests should match");
    // Evidence digests: different (status included).
    let ev_a = canonical_hash(DOMAIN_EVIDENCE_PLANE, &a.evidence_bytes());
    let ev_b = canonical_hash(DOMAIN_EVIDENCE_PLANE, &b.evidence_bytes());
    assert_ne!(ev_a, ev_b, "evidence digests should differ");
}

// --- S1-M1-NO-V1-DEPS (extends S1-M0) ---

#[test]
fn kernel_source_has_no_v1_references_m1() {
    // Same logic as s1_m0_no_v1_deps.rs but explicitly for M1 scope.
    let kernel_src = workspace_root().join("kernel").join("src");
    let forbidden = ["reference::v1", "v1_impl", "docs/reference/v1"];

    let mut violations = Vec::new();
    walk_for_v1(&kernel_src, &forbidden, &mut violations);

    if !violations.is_empty() {
        let mut msg = String::from("v1 references found in kernel source (M1 check):\n");
        for (file, line, content) in &violations {
            let _ = writeln!(msg, "  {file}:{line}: {content}");
        }
        panic!("{msg}");
    }
}

fn walk_for_v1(dir: &Path, forbidden: &[&str], violations: &mut Vec<(String, usize, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_for_v1(&path, forbidden, violations);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            for (line_no, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                {
                    continue;
                }
                for pattern in forbidden {
                    if trimmed.contains(pattern) {
                        violations.push((
                            path.display().to_string(),
                            line_no + 1,
                            line.to_string(),
                        ));
                    }
                }
            }
        }
    }
}
