//! World harness contract: the minimal trait a world must implement.
//!
//! Worlds provide domain-specific fixture encoding, schema descriptors,
//! registry snapshots, and operator programs. Worlds may NOT implement
//! hashing, trace writing, replay verification, or policy enforcement
//! (SPINE-001 invariant: those are kernel/runner concerns).

use sterling_kernel::carrier::bytestate::SchemaDescriptor;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::registry::RegistryV1;

/// A single program instruction: `op_code` + serialized `op_args` bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramStep {
    /// The operator code (4 bytes as `Code32`).
    pub op_code: Code32,
    /// Serialized operator arguments (e.g., 12 bytes for `SET_SLOT`).
    pub op_args: Vec<u8>,
}

/// Fixture dimensions for a world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureDimensions {
    /// Number of layers in the `ByteState`.
    pub layer_count: usize,
    /// Number of slots per layer.
    pub slot_count: usize,
    /// Fixed number of operator argument slots per frame.
    pub arg_slot_count: usize,
    /// Evidence obligations declared by this world.
    ///
    /// Each obligation is a named, versioned contract (e.g., `"tool_transcript_v1"`)
    /// that the verifier maps to specific verification steps. Defaults to empty
    /// for worlds that don't require additional evidence artifacts.
    pub evidence_obligations: Vec<String>,
}

/// Typed failure for world harness operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldHarnessError {
    /// Fixture encoding failed.
    EncodeFailure { detail: String },
}

/// The contract a world must implement to be run by the harness runner.
///
/// A world provides:
/// - A unique identifier
/// - Fixture dimensions (layer/slot/arg counts)
/// - A domain payload (canonical JSON bytes) for compilation
/// - A schema descriptor and registry for the domain
/// - A program (sequence of operator steps to apply)
///
/// A world does NOT provide:
/// - Hashing, trace writing, or replay verification (kernel's job)
/// - Policy enforcement (future milestone)
/// - State decoding or human-readable summaries (future V2 trait)
pub trait WorldHarnessV1 {
    /// Unique world identifier (e.g., `"rome_mini"`).
    fn world_id(&self) -> &str;

    /// Fixture dimensions.
    fn dimensions(&self) -> FixtureDimensions;

    /// Encode the domain payload as canonical JSON bytes for `compile()`.
    ///
    /// The returned bytes must already be in canonical JSON form
    /// (the runner will verify this).
    ///
    /// # Errors
    ///
    /// Returns [`WorldHarnessError::EncodeFailure`] if encoding fails.
    fn encode_payload(&self) -> Result<Vec<u8>, WorldHarnessError>;

    /// The schema descriptor for this world.
    fn schema_descriptor(&self) -> SchemaDescriptor;

    /// The registry for this world.
    ///
    /// # Errors
    ///
    /// Returns [`WorldHarnessError::EncodeFailure`] if construction fails.
    fn registry(&self) -> Result<RegistryV1, WorldHarnessError>;

    /// The program: sequence of operator steps to apply after compilation.
    fn program(&self) -> Vec<ProgramStep>;
}
