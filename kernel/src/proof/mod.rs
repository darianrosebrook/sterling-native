//! Proof module: canonical hashing, replay verification, certificates.
//!
//! Depends on `carrier` and `operators`. Nothing depends on `proof` within the kernel.

pub mod canon;
pub mod hash;
pub mod replay;
