//! Policy snapshot: auditable declaration of the conditions under which a
//! bundle was produced.
//!
//! The runner derives a [`PolicySnapshotV1`] deterministically from world
//! metadata and runner-supplied policy parameters. Worlds do NOT declare
//! policy — that would violate the invariant "Worlds may not implement
//! policy enforcement."
//!
//! The policy snapshot is a normative artifact included in every bundle's
//! `digest_basis`, committing the bundle digest to the policy.

use crate::contract::{ProgramStep, WorldHarnessV1};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::HashDomain;

/// Domain prefix for policy snapshot hashing (harness-originated).
pub const DOMAIN_POLICY_SNAPSHOT: HashDomain = HashDomain::PolicySnapshot;

/// Default budget values for harness runs.
const DEFAULT_MAX_STEPS: usize = 10;
const DEFAULT_MAX_TRACE_BYTES: usize = 65_536;
const DEFAULT_MAX_ARTIFACT_BYTES_TOTAL: usize = 1_048_576;

/// Policy configuration that can override defaults.
///
/// The runner accepts this to allow tests to exercise budget/allowlist
/// enforcement without modifying world implementations.
#[derive(Debug, Clone, Default)]
pub struct PolicyConfig {
    /// Maximum total trace frames (including frame 0 sentinel).
    /// `None` uses `DEFAULT_MAX_STEPS`.
    pub max_steps: Option<usize>,
    /// Maximum trace binary size in bytes. `None` uses default.
    pub max_trace_bytes: Option<usize>,
    /// Maximum total artifact bytes across all artifacts. `None` uses default.
    pub max_artifact_bytes_total: Option<usize>,
    /// Operator allowlist override. `None` derives from the world's program.
    pub allowed_ops: Option<Vec<Code32>>,
}

/// In-memory policy snapshot for a harness run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySnapshotV1 {
    /// Canonical JSON bytes of the policy snapshot.
    pub bytes: Vec<u8>,
}

/// Error building a policy snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyBuildError {
    /// Canonical JSON serialization failed.
    CanonError { detail: String },
}

/// Error enforcing a policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyViolation {
    /// An operator in the program is not in the allowlist.
    AllowlistViolation {
        op_code_hex: String,
        step_index: usize,
    },
    /// The program's total frame count exceeds `max_steps`.
    StepBudgetExceeded { max_steps: usize, actual: usize },
    /// The trace binary exceeds `max_trace_bytes`.
    TraceByteBudgetExceeded { max_bytes: usize, actual: usize },
    /// Total artifact bytes exceed `max_artifact_bytes_total`.
    ArtifactByteBudgetExceeded { max_bytes: usize, actual: usize },
}

/// Build a [`PolicySnapshotV1`] from world metadata and optional config.
///
/// The policy is derived deterministically:
/// - `world_id` from the world
/// - `allowed_ops` from the world's program (deduplicated, sorted by hex)
///   or overridden by config
/// - `budgets` from config or defaults
/// - `determinism_contract` is always the same (no wall time, no env reads,
///   fixed epoch)
///
/// # Errors
///
/// Returns [`PolicyBuildError`] if canonical JSON serialization fails.
pub fn build_policy(
    world: &dyn WorldHarnessV1,
    config: &PolicyConfig,
) -> Result<PolicySnapshotV1, PolicyBuildError> {
    let world_id = world.world_id();
    let program = world.program();

    let allowed_ops = match &config.allowed_ops {
        Some(ops) => ops.clone(),
        None => derive_allowed_ops(&program),
    };

    let allowed_ops_json: Vec<serde_json::Value> = allowed_ops
        .iter()
        .map(|op| {
            serde_json::json!({
                "op_code_hex": hex::encode(op.to_le_bytes()),
            })
        })
        .collect();

    let max_steps = config.max_steps.unwrap_or(DEFAULT_MAX_STEPS);
    let max_trace_bytes = config.max_trace_bytes.unwrap_or(DEFAULT_MAX_TRACE_BYTES);
    let max_artifact_bytes_total = config
        .max_artifact_bytes_total
        .unwrap_or(DEFAULT_MAX_ARTIFACT_BYTES_TOTAL);

    let snapshot_value = serde_json::json!({
        "allowed_ops": allowed_ops_json,
        "budgets": {
            "max_artifact_bytes_total": max_artifact_bytes_total,
            "max_steps": max_steps,
            "max_trace_bytes": max_trace_bytes,
        },
        "determinism_contract": {
            "fixed_epoch": true,
            "no_env_reads": true,
            "no_wall_time": true,
        },
        "schema_version": "policy.v1",
        "world_id": world_id,
    });

    let bytes =
        canonical_json_bytes(&snapshot_value).map_err(|e| PolicyBuildError::CanonError {
            detail: format!("{e:?}"),
        })?;

    Ok(PolicySnapshotV1 { bytes })
}

/// Derive allowed ops from the program: deduplicate by Code32, sort by hex.
fn derive_allowed_ops(program: &[ProgramStep]) -> Vec<Code32> {
    use std::collections::BTreeSet;
    let mut seen: BTreeSet<[u8; 4]> = BTreeSet::new();
    for step in program {
        seen.insert(step.op_code.to_le_bytes());
    }
    seen.into_iter().map(Code32::from_le_bytes).collect()
}

/// Validate the program against the policy BEFORE execution.
///
/// Checks:
/// 1. Every `op_code` in the program is in the `allowed_ops` list.
/// 2. Total frame count (program steps + 1 sentinel) does not exceed `max_steps`.
///
/// # Errors
///
/// Returns [`PolicyViolation`] if any check fails.
pub fn enforce_pre_execution(
    world: &dyn WorldHarnessV1,
    config: &PolicyConfig,
) -> Result<(), PolicyViolation> {
    let program = world.program();

    // Derive allowed ops the same way build_policy does.
    let allowed_ops: std::collections::BTreeSet<[u8; 4]> = match &config.allowed_ops {
        Some(ops) => ops.iter().map(|op| op.to_le_bytes()).collect(),
        None => program.iter().map(|s| s.op_code.to_le_bytes()).collect(),
    };

    // Check allowlist.
    for (i, step) in program.iter().enumerate() {
        if !allowed_ops.contains(&step.op_code.to_le_bytes()) {
            return Err(PolicyViolation::AllowlistViolation {
                op_code_hex: hex::encode(step.op_code.to_le_bytes()),
                step_index: i,
            });
        }
    }

    // Check step budget (total frames = program.len() + 1 for sentinel).
    let total_frames = program.len() + 1;
    let max_steps = config.max_steps.unwrap_or(DEFAULT_MAX_STEPS);
    if total_frames > max_steps {
        return Err(PolicyViolation::StepBudgetExceeded {
            max_steps,
            actual: total_frames,
        });
    }

    Ok(())
}

/// Validate trace byte budget AFTER serialization.
///
/// # Errors
///
/// Returns [`PolicyViolation`] if trace bytes exceed the budget.
pub fn enforce_trace_bytes(
    trace_bytes: &[u8],
    config: &PolicyConfig,
) -> Result<(), PolicyViolation> {
    let max = config.max_trace_bytes.unwrap_or(DEFAULT_MAX_TRACE_BYTES);
    if trace_bytes.len() > max {
        return Err(PolicyViolation::TraceByteBudgetExceeded {
            max_bytes: max,
            actual: trace_bytes.len(),
        });
    }
    Ok(())
}

/// Validate total artifact byte budget.
///
/// # Errors
///
/// Returns [`PolicyViolation`] if total bytes exceed the budget.
pub fn enforce_artifact_bytes(
    total_bytes: usize,
    config: &PolicyConfig,
) -> Result<(), PolicyViolation> {
    let max = config
        .max_artifact_bytes_total
        .unwrap_or(DEFAULT_MAX_ARTIFACT_BYTES_TOTAL);
    if total_bytes > max {
        return Err(PolicyViolation::ArtifactByteBudgetExceeded {
            max_bytes: max,
            actual: total_bytes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::rome_mini::RomeMini;

    #[test]
    fn build_policy_rome_mini() {
        let policy = build_policy(&RomeMini, &PolicyConfig::default()).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&policy.bytes).unwrap();
        assert_eq!(json["schema_version"], "policy.v1");
        assert_eq!(json["world_id"], "rome_mini");
        assert!(json["allowed_ops"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn policy_bytes_are_canonical() {
        let policy = build_policy(&RomeMini, &PolicyConfig::default()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&policy.bytes).unwrap();
        let recanonicalized = canonical_json_bytes(&value).unwrap();
        assert_eq!(policy.bytes, recanonicalized);
    }

    #[test]
    fn policy_deterministic_n10() {
        let first = build_policy(&RomeMini, &PolicyConfig::default()).unwrap();
        for _ in 1..10 {
            let other = build_policy(&RomeMini, &PolicyConfig::default()).unwrap();
            assert_eq!(first.bytes, other.bytes);
        }
    }

    #[test]
    fn enforce_passes_default_config() {
        enforce_pre_execution(&RomeMini, &PolicyConfig::default()).unwrap();
    }

    #[test]
    fn enforce_fails_step_budget() {
        let config = PolicyConfig {
            max_steps: Some(1), // RomeMini has 2 frames
            ..PolicyConfig::default()
        };
        let err = enforce_pre_execution(&RomeMini, &config).unwrap_err();
        match err {
            PolicyViolation::StepBudgetExceeded {
                max_steps: 1,
                actual: 2,
            } => {}
            other => panic!("expected StepBudgetExceeded, got {other:?}"),
        }
    }

    #[test]
    fn enforce_fails_allowlist() {
        let config = PolicyConfig {
            allowed_ops: Some(vec![Code32::new(99, 99, 99)]), // not SET_SLOT
            ..PolicyConfig::default()
        };
        let err = enforce_pre_execution(&RomeMini, &config).unwrap_err();
        match err {
            PolicyViolation::AllowlistViolation { .. } => {}
            other => panic!("expected AllowlistViolation, got {other:?}"),
        }
    }

    #[test]
    fn enforce_trace_bytes_passes() {
        enforce_trace_bytes(&[0; 100], &PolicyConfig::default()).unwrap();
    }

    #[test]
    fn enforce_trace_bytes_fails() {
        let config = PolicyConfig {
            max_trace_bytes: Some(10),
            ..PolicyConfig::default()
        };
        let err = enforce_trace_bytes(&[0; 100], &config).unwrap_err();
        match err {
            PolicyViolation::TraceByteBudgetExceeded { .. } => {}
            other => panic!("expected TraceByteBudgetExceeded, got {other:?}"),
        }
    }
}
