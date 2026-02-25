//! Typed search errors.

/// Typed failure for search operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    /// A candidate's `op_code` was not in the runner-supplied registry.
    WorldContractViolation { detail: String },

    /// Expansion budget exceeded (`max_expansions`).
    ExpansionBudgetExceeded {
        expansions: u64,
        max_expansions: u64,
    },

    /// A reserved policy option was selected that is not supported in M1.
    UnsupportedPolicyMode { detail: String },

    /// Frontier exhausted without reaching a goal.
    FrontierExhausted,

    /// Depth budget exceeded (`max_depth`).
    DepthBudgetExceeded { max_depth: u32 },

    /// Kernel `apply()` failed during search.
    ApplyFailed { detail: String },
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorldContractViolation { detail } => {
                write!(f, "world contract violation: {detail}")
            }
            Self::ExpansionBudgetExceeded {
                expansions,
                max_expansions,
            } => write!(
                f,
                "expansion budget exceeded: {expansions}/{max_expansions}"
            ),
            Self::UnsupportedPolicyMode { detail } => {
                write!(f, "unsupported policy mode in M1: {detail}")
            }
            Self::FrontierExhausted => write!(f, "frontier exhausted without reaching goal"),
            Self::DepthBudgetExceeded { max_depth } => {
                write!(f, "depth budget exceeded: max_depth={max_depth}")
            }
            Self::ApplyFailed { detail } => write!(f, "apply failed: {detail}"),
        }
    }
}

impl std::error::Error for SearchError {}
