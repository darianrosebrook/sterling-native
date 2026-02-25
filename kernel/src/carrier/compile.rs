//! Compilation boundary: `compile(payload, schema_descriptor, registry_snapshot) -> ByteState`.
//!
//! This is a **new concept** in Sterling Native, not ported from v1.
//! (v1's `compiler.py` is a `ByteTrace` encoder — that maps to the `bytetrace` writer.)
//!
//! `compile()` transforms a domain payload into an initial `ByteStateV1`.
//! It is a pure function: identical inputs produce identical outputs.
//!
//! Policy does not affect compilation. Policy applies at the `apply`/harness layer.
//!
//! # M0 scope
//!
//! Types and signature only. Logic is M1 scope.

use crate::carrier::bytestate::{ByteStateV1, RegistrySnapshot, SchemaDescriptor};

/// A successful compilation result.
#[derive(Debug, Clone)]
pub struct CompilationResultV1 {
    /// The compiled initial state.
    pub state: ByteStateV1,
    /// Hash of the compilation request that produced this result.
    pub request_manifest_hash: String,
    /// Schema used for compilation.
    pub schema_descriptor: SchemaDescriptor,
    /// Registry used for compilation.
    pub registry_descriptor: RegistrySnapshot,
    /// Canonical JSON manifest recording every dependency hash.
    pub compilation_manifest: Vec<u8>,
}

/// Typed compilation failure. Fail-closed: no partial `ByteState` is produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilationFailure {
    /// Schema descriptor does not match expected schema.
    SchemaMismatch {
        detail: String,
        request_digest: String,
    },
    /// Registry snapshot epoch/hash mismatch.
    RegistryMismatch {
        detail: String,
        request_digest: String,
    },
    /// Payload references a concept not in the registry.
    UnknownConcept {
        detail: String,
        request_digest: String,
    },
    /// Payload violates a schema constraint.
    ConstraintViolation {
        detail: String,
        request_digest: String,
    },
}

/// Result type for compilation.
pub type CompileResult = Result<CompilationResultV1, CompilationFailure>;

/// Compile a domain payload into an initial `ByteState`.
///
/// Pure function: identical inputs produce identical output bytes.
///
/// # Arguments
///
/// * `payload_bytes` - Canonical JSON bytes of the domain payload.
/// * `schema_descriptor` - Identifies the `ByteState` schema to use.
/// * `registry_snapshot` - Identifies the `Code32` <-> `ConceptID` mapping epoch.
///
/// Policy is **not** an input. It applies at the apply/harness layer.
///
/// # Errors
///
/// Returns [`CompilationFailure`] on any mismatch. Fail-closed.
///
/// # Panics
///
/// M0 stub. Will panic until M1 implementation.
pub fn compile(
    _payload_bytes: &[u8],
    _schema_descriptor: &SchemaDescriptor,
    _registry_snapshot: &RegistrySnapshot,
) -> CompileResult {
    todo!("M1: implement compilation logic")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "M1")]
    fn compile_stub_panics() {
        let sd = SchemaDescriptor {
            id: "test".into(),
            version: "1.0".into(),
            hash: "sha256:000".into(),
        };
        let rs = RegistrySnapshot {
            epoch: "epoch-0".into(),
            hash: "sha256:000".into(),
        };
        let _ = compile(b"{}", &sd, &rs);
    }
}
