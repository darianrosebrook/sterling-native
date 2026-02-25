//! Search world contract trait.

use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::registry::RegistryV1;

use crate::node::CandidateActionV1;

/// Trait for worlds that support search.
///
/// Extends `WorldHarnessV1` (which provides linear program execution) with
/// candidate enumeration and goal detection for frontier-managed search.
///
/// # Contract
///
/// - `enumerate_candidates` must use the registry snapshot provided by the
///   runner; it must NOT call `self.registry()` internally (INV-SC-08).
/// - All candidates must have `op_code` values that exist in the provided
///   registry (INV-SC-02).
/// - Enumeration must be deterministic: same `(state, registry)` → same
///   candidates in the same order.
pub trait SearchWorldV1 {
    /// Unique world identifier (must match `WorldHarnessV1::world_id()`).
    fn world_id(&self) -> &str;

    /// Enumerate all legal candidate actions from the given state.
    ///
    /// The returned candidates must be deterministically ordered and use only
    /// operator codes present in the provided `registry`.
    fn enumerate_candidates(
        &self,
        state: &ByteStateV1,
        registry: &RegistryV1,
    ) -> Vec<CandidateActionV1>;

    /// Test whether the given state satisfies the world's goal.
    fn is_goal(&self, state: &ByteStateV1) -> bool;
}
