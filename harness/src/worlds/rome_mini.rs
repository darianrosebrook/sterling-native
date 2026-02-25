//! `RomeMini`: minimal world for M3 harness testing.
//!
//! Reuses the exact dimensions and operator from M2's canonical test trace:
//! 1 layer, 2 slots, 3 arg slots, one `SET_SLOT` operation.
//!
//! The initial state is all `PADDING`/`Hole` (matching `ByteStateV1::new(1, 2)`).
//! The program applies `SET_SLOT(0, 0, Code32::new(1, 1, 5))`.

use crate::contract::{FixtureDimensions, ProgramStep, WorldHarnessError, WorldHarnessV1};
use sterling_kernel::carrier::bytestate::SchemaDescriptor;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;
use sterling_kernel::operators::apply::{set_slot_args, OP_SET_SLOT, SET_SLOT_ARG_COUNT};
use sterling_kernel::proof::canon::canonical_json_bytes;

/// Minimal world for M3 harness testing.
pub struct RomeMini;

impl WorldHarnessV1 for RomeMini {
    #[allow(clippy::unnecessary_literal_bound)]
    fn world_id(&self) -> &str {
        "rome_mini"
    }

    fn dimensions(&self) -> FixtureDimensions {
        FixtureDimensions {
            layer_count: 1,
            slot_count: 2,
            arg_slot_count: SET_SLOT_ARG_COUNT,
        }
    }

    fn encode_payload(&self) -> Result<Vec<u8>, WorldHarnessError> {
        // All PADDING identities, all Hole statuses — matches ByteStateV1::new(1, 2).
        let payload = serde_json::json!({
            "identity": [[0, 0, 0, 0], [0, 0, 0, 0]],
            "layer_count": 1,
            "slot_count": 2,
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
            ],
        )
        .map_err(|e| WorldHarnessError::EncodeFailure {
            detail: format!("registry construction error: {e:?}"),
        })
    }

    fn program(&self) -> Vec<ProgramStep> {
        vec![ProgramStep {
            op_code: OP_SET_SLOT,
            op_args: set_slot_args(0, 0, Code32::new(1, 1, 5)),
        }]
    }
}
