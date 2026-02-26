//! Typed search errors.
//!
//! `SearchError` represents pre-flight failures only. Runtime terminations
//! (including contract violations, panics, and budget exhaustion) are expressed
//! via [`crate::graph::TerminationReasonV1`] and always produce a
//! `SearchGraphV1` audit trail.

/// Typed failure for pre-flight search validation.
///
/// These errors are returned before search execution begins. No `SearchGraphV1`
/// is produced because no search steps were taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    /// A reserved policy option was selected that is not supported in M1.
    UnsupportedPolicyMode { detail: String },
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPolicyMode { detail } => {
                write!(f, "unsupported policy mode in M1: {detail}")
            }
        }
    }
}

impl std::error::Error for SearchError {}
