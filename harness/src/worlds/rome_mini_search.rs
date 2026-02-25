//! `RomeMiniSearch`: search-capable extension of `RomeMini`.
//!
//! Implements `SearchWorldV1` by enumerating `SET_SLOT` candidates for
//! each slot using the first K concept identities from the runner-supplied
//! registry (K = min(registry size, 4)).
//!
//! Goal: slot 0 in layer 0 has identity `Code32::new(1, 0, 1)` (the
//! "forum" concept).

use sterling_kernel::carrier::bytestate::ByteStateV1;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply::{set_slot_args, OP_SET_SLOT};

use sterling_search::contract::SearchWorldV1;
use sterling_search::node::{candidate_canonical_hash, CandidateActionV1};

use crate::contract::{FixtureDimensions, ProgramStep, WorldHarnessError, WorldHarnessV1};
use sterling_kernel::carrier::bytestate::SchemaDescriptor;
use sterling_kernel::proof::canon::canonical_json_bytes;

/// Search-capable Rome Mini world.
///
/// Extends `RomeMini` with candidate enumeration and goal detection.
pub struct RomeMiniSearch;

/// The goal identity value: `Code32::new(1, 0, 1)` ("rome:node:forum").
const GOAL_VALUE: Code32 = Code32::new(1, 0, 1);

/// Number of slots in the world.
const SLOT_COUNT: usize = 2;

/// Maximum candidate values per slot from registry.
const MAX_VALUES_PER_SLOT: usize = 4;

impl WorldHarnessV1 for RomeMiniSearch {
    #[allow(clippy::unnecessary_literal_bound)]
    fn world_id(&self) -> &str {
        "rome_mini_search"
    }

    fn dimensions(&self) -> FixtureDimensions {
        FixtureDimensions {
            layer_count: 1,
            slot_count: SLOT_COUNT,
            arg_slot_count: 3, // SET_SLOT takes 3 arg slots (layer, slot, value)
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, WorldHarnessError> {
        let payload = serde_json::json!({
            "identity": [[0, 0, 0, 0], [0, 0, 0, 0]],
            "layer_count": 1,
            "slot_count": SLOT_COUNT,
            "status": [0, 0],
        });
        canonical_json_bytes(&payload).map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("canonical JSON error: {e:?}"),
        })
    }

    fn schema_descriptor(&self) -> SchemaDescriptor {
        SchemaDescriptor {
            id: "rome".into(),
            version: "1.0".into(),
            hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        }
    }

    fn registry(&self) -> Result<RegistryV1, WorldHarnessError> {
        RegistryV1::new(
            "epoch-0".into(),
            vec![
                (Code32::new(1, 0, 0), "rome:node:start".into()),
                (Code32::new(1, 0, 1), "rome:node:forum".into()),
                (Code32::new(1, 0, 2), "rome:node:colosseum".into()),
                (Code32::new(1, 1, 0), "rome:edge:road".into()),
                (OP_SET_SLOT, "rome:op:set_slot".into()),
            ],
        )
        .map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("registry construction error: {e:?}"),
        })
    }

    fn program(&self) -> Vec<ProgramStep> {
        // Linear program: single SET_SLOT to reach goal directly.
        vec![ProgramStep {
            op_code: OP_SET_SLOT,
            op_args: set_slot_args(0, 0, GOAL_VALUE),
        }]
    }
}

impl SearchWorldV1 for RomeMiniSearch {
    #[allow(clippy::unnecessary_literal_bound)]
    fn world_id(&self) -> &str {
        "rome_mini_search"
    }

    fn enumerate_candidates(
        &self,
        _state: &ByteStateV1,
        registry: &RegistryV1,
    ) -> Vec<CandidateActionV1> {
        // Known concept values that can be assigned to slots.
        // These are the identity codes (not operator codes) from the registry.
        // Sorted by le bytes for deterministic enumeration.
        // Filtered to those present in the runner-supplied registry (INV-SC-08).
        let known_values: [Code32; 4] = [
            Code32::new(1, 0, 0),
            Code32::new(1, 0, 1),
            Code32::new(1, 0, 2),
            Code32::new(1, 1, 0),
        ];

        let values: Vec<Code32> = known_values
            .iter()
            .filter(|c| registry.contains(c))
            .take(MAX_VALUES_PER_SLOT)
            .copied()
            .collect();

        let mut candidates = Vec::new();
        for slot in 0..SLOT_COUNT {
            for &value in &values {
                #[allow(clippy::cast_possible_truncation)]
                let op_args = set_slot_args(0, slot as u32, value);
                let canonical_hash = candidate_canonical_hash(OP_SET_SLOT, &op_args);
                candidates.push(CandidateActionV1 {
                    op_code: OP_SET_SLOT,
                    op_args,
                    canonical_hash,
                });
            }
        }

        candidates
    }

    fn is_goal(&self, state: &ByteStateV1) -> bool {
        // Goal: slot 0 identity == GOAL_VALUE bytes.
        let identity = state.identity_bytes();
        // Each slot is 4 bytes. Slot 0 starts at offset 0.
        if identity.len() < 4 {
            return false;
        }
        identity[..4] == GOAL_VALUE.to_le_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerate_candidates_uses_registry() {
        let registry = RomeMiniSearch.registry().unwrap();
        let state = ByteStateV1::new(1, 2);
        let candidates = RomeMiniSearch.enumerate_candidates(&state, &registry);
        // 2 slots × 4 values = 8 candidates
        assert_eq!(candidates.len(), 8);
        // All use SET_SLOT
        for c in &candidates {
            assert_eq!(c.op_code, OP_SET_SLOT);
        }
    }

    #[test]
    fn initial_state_is_not_goal() {
        let state = ByteStateV1::new(1, 2);
        assert!(!RomeMiniSearch.is_goal(&state));
    }

    #[test]
    fn goal_state_detected() {
        let state = ByteStateV1::new(1, 2);
        // Apply SET_SLOT(0, 0, GOAL_VALUE) to reach goal
        let (goal_state, _) = sterling_kernel::operators::apply::apply(
            &state,
            OP_SET_SLOT,
            &set_slot_args(0, 0, GOAL_VALUE),
        )
        .unwrap();
        assert!(RomeMiniSearch.is_goal(&goal_state));
    }

    #[test]
    fn enumeration_is_deterministic() {
        let registry = RomeMiniSearch.registry().unwrap();
        let state = ByteStateV1::new(1, 2);
        let c1 = RomeMiniSearch.enumerate_candidates(&state, &registry);
        let c2 = RomeMiniSearch.enumerate_candidates(&state, &registry);
        assert_eq!(c1, c2);
    }
}
