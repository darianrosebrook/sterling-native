//! Sterling Harness: world-level orchestration for the kernel.
//!
//! The harness runs a world through the kernel's proof pipeline
//! (`compile` → `apply` → `trace_to_bytes` → `replay_verify`)
//! and packages the result as a self-contained artifact bundle.
//!
//! The harness does NOT implement proof logic — it delegates to the kernel.
//! Worlds provide domain data only; the harness owns orchestration.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]

pub mod bundle;
pub mod bundle_dir;
pub mod contract;
pub mod policy;
pub mod runner;
pub mod worlds;
