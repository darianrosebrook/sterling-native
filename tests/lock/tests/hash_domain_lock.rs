//! Hash domain governance lock tests.
//!
//! Proves:
//! 1. Canonical domain set has expected count (catches forgotten additions to ALL)
//! 2. All domain byte strings are unique (prevents domain collision)
//! 3. All domains are null-terminated (wire format invariant)
//! 4. All domains follow `STERLING::*::V1\0` naming convention
//! 5. No raw `STERLING::` domain literals in production source outside `hash_domain.rs`
//! 6. No `deny_unknown_fields` in production source (ADR 0008 extensibility)

use std::collections::BTreeSet;
use sterling_kernel::proof::hash::HashDomain;

// ---------------------------------------------------------------------------
// 1. Canonical set count
// ---------------------------------------------------------------------------

/// ACCEPTANCE: HASH-001-LOCK
#[test]
fn hash_domain_canonical_set_count() {
    assert_eq!(
        HashDomain::ALL.len(),
        21,
        "expected 21 domain variants; if you added a new domain, update this count"
    );
}

// ---------------------------------------------------------------------------
// 2. All unique bytes
// ---------------------------------------------------------------------------

/// ACCEPTANCE: HASH-001-LOCK
#[test]
fn hash_domain_all_unique_bytes() {
    let mut seen = BTreeSet::new();
    for domain in HashDomain::ALL {
        assert!(
            seen.insert(domain.as_bytes()),
            "duplicate domain bytes: {domain}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. All null-terminated
// ---------------------------------------------------------------------------

/// ACCEPTANCE: HASH-001-LOCK
#[test]
fn hash_domain_all_null_terminated() {
    for domain in HashDomain::ALL {
        assert!(
            domain.as_bytes().ends_with(&[0]),
            "{domain} is not null-terminated"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Naming convention
// ---------------------------------------------------------------------------

/// ACCEPTANCE: HASH-001-LOCK
#[test]
fn hash_domain_all_follow_naming_convention() {
    for domain in HashDomain::ALL {
        let bytes = domain.as_bytes();
        assert!(
            bytes.starts_with(b"STERLING::"),
            "{domain} does not start with STERLING::"
        );
        assert!(
            bytes.ends_with(b"::V1\0"),
            "{domain} does not end with ::V1\\0"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. No raw STERLING:: domain literals in production source
// ---------------------------------------------------------------------------

/// Scan kernel/, search/, harness/ source for `b"STERLING::` literals.
/// The only file allowed to contain them is `hash_domain.rs`.
///
/// ACCEPTANCE: HASH-001-LOCK
#[test]
fn no_raw_domain_literals_outside_authority() {
    let production_dirs = [
        concat!(env!("CARGO_MANIFEST_DIR"), "/../kernel/src"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/../search/src"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/../harness/src"),
    ];

    let pattern = "b\"STERLING::";
    let authority_file = "hash_domain.rs";
    let mut violations = Vec::new();

    for dir in &production_dirs {
        scan_dir_for_pattern(dir, pattern, authority_file, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "raw STERLING:: domain literals found outside {authority_file}:\n{}",
        violations.join("\n")
    );
}

fn scan_dir_for_pattern(
    dir: &str,
    pattern: &str,
    authority_file: &str,
    violations: &mut Vec<String>,
) {
    let dir_path = std::path::Path::new(dir);
    if !dir_path.exists() {
        return;
    }
    for entry in walkdir(dir_path) {
        let path = entry.as_path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if path
            .file_name()
            .and_then(|n| n.to_str())
            == Some(authority_file)
        {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };

        // Skip #[cfg(test)] module blocks via brace-depth tracking.
        let mut brace_depth: usize = 0;
        let mut skip_depth: Option<usize> = None;
        let mut cfg_test_pending = false;

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.contains("#[cfg(test)]") {
                cfg_test_pending = true;
                continue;
            }

            let opens = line.chars().filter(|&c| c == '{').count();
            let closes = line.chars().filter(|&c| c == '}').count();

            if cfg_test_pending && opens > 0 {
                skip_depth = Some(brace_depth);
                cfg_test_pending = false;
            }

            brace_depth = brace_depth.saturating_add(opens);
            brace_depth = brace_depth.saturating_sub(closes);

            if let Some(depth) = skip_depth {
                if brace_depth <= depth {
                    skip_depth = None;
                }
                continue;
            }

            // Skip comment lines.
            if trimmed.starts_with("//") {
                continue;
            }

            if trimmed.contains(pattern) {
                violations.push(format!("  {}:{}: {}", path.display(), i + 1, trimmed));
            }
        }
    }
}

/// Simple recursive directory walker (avoids adding walkdir dependency).
fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else {
                results.push(path);
            }
        }
    }
    results
}

// ---------------------------------------------------------------------------
// 6. No deny_unknown_fields in production source
// ---------------------------------------------------------------------------

/// `#[serde(deny_unknown_fields)]` breaks schema extensibility (ADR 0008).
/// Scan kernel/, search/, harness/ production code for it.
///
/// ACCEPTANCE: HASH-001-DUF
#[test]
fn no_deny_unknown_fields_in_production_code() {
    let production_dirs = [
        concat!(env!("CARGO_MANIFEST_DIR"), "/../kernel/src"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/../search/src"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/../harness/src"),
    ];

    let pattern = "deny_unknown_fields";
    let mut violations = Vec::new();

    for dir in &production_dirs {
        scan_dir_for_pattern(dir, pattern, "", &mut violations);
    }

    assert!(
        violations.is_empty(),
        "deny_unknown_fields found in production code (breaks ADR 0008 extensibility):\n{}",
        violations.join("\n")
    );
}
