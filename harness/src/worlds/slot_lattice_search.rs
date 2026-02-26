//! `SlotLatticeSearch`: parameterized search world for stress-testing.
//!
//! Represents state as N active slots (within a fixed `MAX_SLOTS` layout)
//! with values in `{UNSET, CONCEPT_VALUES[0..V]}`. A candidate is "set one
//! UNSET slot to a nonzero value." Because any UNSET slot can be set next,
//! the same final assignment is reachable via many orderings — this naturally
//! generates duplicate states for visited-set suppression.
//!
//! Configurable trap rules and goal profiles allow each "regime" to
//! deterministically force a specific stress axis (truncation, duplicates,
//! dead ends, budget, frontier pressure) without changing the search loop.
//!
//! # Kernel boundary
//!
//! This world is entirely a consumer of existing kernel primitives
//! (`ByteStateV1`, `Code32`, `SET_SLOT` via `apply()`). No kernel changes.

use sterling_kernel::carrier::bytestate::{ByteStateV1, SchemaDescriptor};
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply::{set_slot_args, OP_SET_SLOT};
use sterling_kernel::proof::canon::canonical_json_bytes;

use sterling_search::contract::SearchWorldV1;
use sterling_search::node::CandidateActionV1;

use crate::contract::{FixtureDimensions, ProgramStep, WorldHarnessError, WorldHarnessV1};

/// Fixed layout size for the `ByteStateV1` slot dimension.
/// All regimes share this layout; `active_slots` controls semantics.
const MAX_SLOTS: usize = 10;

/// Shared concept values — same 4 codes used by `RomeMiniSearch`.
/// Sorted by LE bytes for deterministic enumeration.
const CONCEPT_VALUES: [Code32; 4] = [
    Code32::new(1, 0, 0), // "start"
    Code32::new(1, 0, 1), // "forum"
    Code32::new(1, 0, 2), // "colosseum"
    Code32::new(1, 1, 0), // "road"
];

/// PADDING bytes for UNSET slot detection.
const PADDING_BYTES: [u8; 4] = [0, 0, 0, 0];

/// Normative schema basis bytes for the slot lattice schema artifact.
///
/// This is the canonical JSON representation of the schema identity.
/// The hash is computed as `canonical_hash(DOMAIN_HARNESS_FIXTURE, SCHEMA_BASIS_BYTES)`.
/// Config-independent: same hash for all regimes regardless of `active_slots` or values.
///
/// Changing this constant is a **schema version bump** — existing evidence bundles
/// will have a different schema hash and cannot be compared against new ones.
const SCHEMA_BASIS_BYTES: &[u8] =
    br#"{"domain_id":"slot_lattice","schema_version":"slot_lattice_schema.v1","version":"1.0"}"#;

/// Compute the stable schema hash for the slot lattice schema artifact.
fn slot_lattice_schema_hash() -> String {
    let hash = sterling_kernel::proof::hash::canonical_hash(
        crate::bundle::DOMAIN_HARNESS_FIXTURE,
        SCHEMA_BASIS_BYTES,
    );
    hash.as_str().to_string()
}

/// Trap rule: controls deterministic exhaustive dead-end generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrapRule {
    /// No trap states — pure assignment lattice.
    None,
    /// Trapped when `slot[0] == CONCEPT_VALUES[v_index]`.
    /// `v_index` is 0-based into `CONCEPT_VALUES`.
    Slot0Eq(u8),
}

/// Goal profile: controls when `is_goal()` returns true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalProfile {
    /// Goal when all active slots are non-PADDING.
    AllNonzero,
    /// Intentionally unreachable goal (forces search to exhaust budget/frontier).
    Never,
    /// Goal when all active slots non-PADDING AND slot 0 is NOT `CONCEPT_VALUES[v_index]`.
    /// Used with `TrapRule::Slot0Eq(v_index)` so that trap branches are dead ends
    /// but non-trap branches can still reach the goal.
    AllNonzeroExceptSlot0Eq(u8),
}

/// Configuration for a slot lattice world instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotLatticeConfig {
    /// Number of active slots (must be <= `MAX_SLOTS`).
    pub active_slots: u8,
    /// Number of values per slot (must be 1..=4, indexing into `CONCEPT_VALUES`).
    pub values_per_slot: u8,
    /// Trap rule for exhaustive dead-end generation.
    pub trap_rule: TrapRule,
    /// Goal profile.
    pub goal_profile: GoalProfile,
}

/// Parameterized search world for stress-testing the search loop.
///
/// Construct via regime constructors in [`super::slot_lattice_regimes`],
/// not directly.
pub struct SlotLatticeSearch {
    config: SlotLatticeConfig,
    world_id: String,
}

impl SlotLatticeSearch {
    /// Construct a new slot lattice world from config.
    ///
    /// # Panics
    ///
    /// Panics if `active_slots > MAX_SLOTS`, `values_per_slot` not in `1..=4`,
    /// or trap/goal value indices are out of range for `CONCEPT_VALUES`.
    #[must_use]
    pub fn new(config: SlotLatticeConfig) -> Self {
        assert!(
            config.active_slots as usize <= MAX_SLOTS,
            "active_slots {} exceeds MAX_SLOTS {}",
            config.active_slots,
            MAX_SLOTS,
        );
        assert!(
            config.values_per_slot >= 1 && config.values_per_slot <= 4,
            "values_per_slot must be 1..=4, got {}",
            config.values_per_slot,
        );
        if let TrapRule::Slot0Eq(v) = &config.trap_rule {
            assert!(
                (*v as usize) < CONCEPT_VALUES.len(),
                "TrapRule::Slot0Eq index {} out of range for CONCEPT_VALUES (len {})",
                v,
                CONCEPT_VALUES.len(),
            );
        }
        if let GoalProfile::AllNonzeroExceptSlot0Eq(v) = &config.goal_profile {
            assert!(
                (*v as usize) < CONCEPT_VALUES.len(),
                "GoalProfile::AllNonzeroExceptSlot0Eq index {} out of range for CONCEPT_VALUES (len {})",
                v,
                CONCEPT_VALUES.len(),
            );
        }

        let trap_str = match &config.trap_rule {
            TrapRule::None => "none".to_string(),
            TrapRule::Slot0Eq(v) => format!("slot0eq{v}"),
        };
        let goal_str = match &config.goal_profile {
            GoalProfile::AllNonzero => "all_nonzero".to_string(),
            GoalProfile::Never => "never".to_string(),
            GoalProfile::AllNonzeroExceptSlot0Eq(v) => format!("all_nonzero_except_slot0eq{v}"),
        };

        let world_id = format!(
            "slot_lattice:v1:n{}:v{}:trap_{}:goal_{}",
            config.active_slots, config.values_per_slot, trap_str, goal_str,
        );

        Self { config, world_id }
    }

    /// Read the config (for test assertions).
    #[must_use]
    pub fn config(&self) -> &SlotLatticeConfig {
        &self.config
    }

    /// Check if a state is trapped (enumerate returns empty).
    fn is_trap(&self, state: &ByteStateV1) -> bool {
        match &self.config.trap_rule {
            TrapRule::None => false,
            TrapRule::Slot0Eq(v_index) => {
                let identity = state.identity_bytes();
                if identity.len() < 4 {
                    return false;
                }
                let slot0 = &identity[..4];
                let target = CONCEPT_VALUES[*v_index as usize].to_le_bytes();
                slot0 == target
            }
        }
    }

    /// Check if all active slots are non-PADDING.
    fn all_active_nonzero(&self, state: &ByteStateV1) -> bool {
        let identity = state.identity_bytes();
        for i in 0..self.config.active_slots as usize {
            let start = i * 4;
            if start + 4 > identity.len() {
                return false;
            }
            if identity[start..start + 4] == PADDING_BYTES {
                return false;
            }
        }
        true
    }

    /// Read slot 0's identity bytes.
    fn slot0_value(state: &ByteStateV1) -> [u8; 4] {
        let identity = state.identity_bytes();
        if identity.len() < 4 {
            return PADDING_BYTES;
        }
        [identity[0], identity[1], identity[2], identity[3]]
    }
}

impl WorldHarnessV1 for SlotLatticeSearch {
    fn world_id(&self) -> &str {
        &self.world_id
    }

    fn dimensions(&self) -> FixtureDimensions {
        FixtureDimensions {
            layer_count: 1,
            slot_count: MAX_SLOTS,
            arg_slot_count: 3, // SET_SLOT takes 3 arg slots
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, WorldHarnessError> {
        let zeros = vec![vec![0u32; 4]; MAX_SLOTS];
        let status_zeros = vec![0u8; MAX_SLOTS];

        // active_slots is world-local config (held in SlotLatticeConfig), not part
        // of the compiled payload surface. Only compiler-recognized fields here.
        let payload = serde_json::json!({
            "identity": zeros,
            "layer_count": 1,
            "slot_count": MAX_SLOTS,
            "status": status_zeros,
        });

        canonical_json_bytes(&payload).map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("canonical JSON error: {e:?}"),
        })
    }

    fn schema_descriptor(&self) -> SchemaDescriptor {
        SchemaDescriptor {
            id: "slot_lattice".into(),
            version: "1.0".into(),
            hash: slot_lattice_schema_hash(),
        }
    }

    fn registry(&self) -> Result<RegistryV1, WorldHarnessError> {
        RegistryV1::new(
            "epoch-0".into(),
            vec![
                (Code32::new(1, 0, 0), "lattice:value:start".into()),
                (Code32::new(1, 0, 1), "lattice:value:forum".into()),
                (Code32::new(1, 0, 2), "lattice:value:colosseum".into()),
                (Code32::new(1, 1, 0), "lattice:value:road".into()),
                (OP_SET_SLOT, "lattice:op:set_slot".into()),
            ],
        )
        .map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("registry construction error: {e:?}"),
        })
    }

    fn program(&self) -> Vec<ProgramStep> {
        // Search worlds return empty program — program is non-semantic in search mode.
        // Codebook hash basis will contain an empty operators array.
        vec![]
    }
}

impl SearchWorldV1 for SlotLatticeSearch {
    fn world_id(&self) -> &str {
        &self.world_id
    }

    fn enumerate_candidates(
        &self,
        state: &ByteStateV1,
        registry: &RegistryV1,
    ) -> Vec<CandidateActionV1> {
        // Trap check: if trapped, return empty (forces exhaustive dead end).
        if self.is_trap(state) {
            return Vec::new();
        }

        let identity = state.identity_bytes();
        let v_count = self.config.values_per_slot as usize;

        let mut candidates = Vec::new();

        // Slot ascending, value ascending — deterministic enumeration order.
        for slot in 0..self.config.active_slots as usize {
            let start = slot * 4;
            if start + 4 > identity.len() {
                continue;
            }
            // Only enumerate for UNSET slots.
            if identity[start..start + 4] != PADDING_BYTES {
                continue;
            }

            for &value in CONCEPT_VALUES.iter().take(v_count) {
                // INV-SC-02: all candidate op_codes must be in the registry.
                if !registry.contains(&OP_SET_SLOT) {
                    continue;
                }
                // INV-SC-08: use runner-supplied registry, not self.registry().
                if !registry.contains(&value) {
                    continue;
                }

                #[allow(clippy::cast_possible_truncation)]
                let op_args = set_slot_args(0, slot as u32, value);
                candidates.push(CandidateActionV1::new(OP_SET_SLOT, op_args));
            }
        }

        candidates
    }

    fn is_goal(&self, state: &ByteStateV1) -> bool {
        match &self.config.goal_profile {
            GoalProfile::AllNonzero => self.all_active_nonzero(state),
            GoalProfile::Never => false,
            GoalProfile::AllNonzeroExceptSlot0Eq(v_index) => {
                if !self.all_active_nonzero(state) {
                    return false;
                }
                let slot0 = Self::slot0_value(state);
                let excluded = CONCEPT_VALUES[*v_index as usize].to_le_bytes();
                slot0 != excluded
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SlotLatticeConfig {
        SlotLatticeConfig {
            active_slots: 3,
            values_per_slot: 2,
            trap_rule: TrapRule::None,
            goal_profile: GoalProfile::AllNonzero,
        }
    }

    fn test_world() -> SlotLatticeSearch {
        SlotLatticeSearch::new(test_config())
    }

    #[test]
    fn enumerate_candidates_uses_registry() {
        let world = test_world();
        let registry = world.registry().unwrap();
        let state = ByteStateV1::new(1, MAX_SLOTS);
        let candidates = world.enumerate_candidates(&state, &registry);
        // 3 active slots × 2 values = 6 candidates
        assert_eq!(candidates.len(), 6);
        for c in &candidates {
            assert_eq!(c.op_code, OP_SET_SLOT);
        }
    }

    #[test]
    fn initial_state_is_not_goal() {
        let world = test_world();
        let state = ByteStateV1::new(1, MAX_SLOTS);
        assert!(!world.is_goal(&state));
    }

    #[test]
    fn goal_state_detected() {
        let world = test_world();
        let mut state = ByteStateV1::new(1, MAX_SLOTS);
        // Fill all 3 active slots.
        for i in 0..3u32 {
            let (new_state, _) = sterling_kernel::operators::apply::apply(
                &state,
                OP_SET_SLOT,
                &set_slot_args(0, i, CONCEPT_VALUES[0]),
            )
            .unwrap();
            state = new_state;
        }
        assert!(world.is_goal(&state));
    }

    #[test]
    fn enumeration_is_deterministic() {
        let world = test_world();
        let registry = world.registry().unwrap();
        let state = ByteStateV1::new(1, MAX_SLOTS);
        let c1 = world.enumerate_candidates(&state, &registry);
        let c2 = world.enumerate_candidates(&state, &registry);
        assert_eq!(c1, c2);
    }

    #[test]
    fn trap_state_returns_empty_candidates() {
        let config = SlotLatticeConfig {
            active_slots: 3,
            values_per_slot: 2,
            trap_rule: TrapRule::Slot0Eq(0), // trap when slot0 == CONCEPT_VALUES[0]
            goal_profile: GoalProfile::AllNonzero,
        };
        let world = SlotLatticeSearch::new(config);
        let registry = world.registry().unwrap();

        // Set slot 0 to the trap value.
        let state = ByteStateV1::new(1, MAX_SLOTS);
        let (trapped_state, _) = sterling_kernel::operators::apply::apply(
            &state,
            OP_SET_SLOT,
            &set_slot_args(0, 0, CONCEPT_VALUES[0]),
        )
        .unwrap();

        let candidates = world.enumerate_candidates(&trapped_state, &registry);
        assert!(
            candidates.is_empty(),
            "trapped state should have zero candidates"
        );
    }
}
