//! Sterling Search: deterministic best-first search with auditable graph artifact.
//!
//! This crate provides the search layer for Sterling v2. It depends only on
//! `sterling_kernel` — it does NOT depend on `sterling_harness`.
//!
//! # Crate dependency graph
//!
//! ```text
//! sterling_kernel  ←  sterling_search  ←  sterling_harness
//! (pure carrier)      (frontier, nodes)    (bundles, runner, worlds)
//! ```
//!
//! # Key types
//!
//! - [`SearchNodeV1`] — immutable state node with deterministic ordering
//! - [`CandidateActionV1`] — a candidate operator application
//! - [`SearchGraphV1`] — expansion-event audit log (normative bundle artifact)
//! - [`SearchPolicyV1`] — search budget and dedup configuration
//! - [`ValueScorer`] — trait for candidate scoring (integer scores in Cert mode)
//! - [`SearchWorldV1`] — trait for worlds that support search

#![forbid(unsafe_code)]

pub mod contract;
pub mod error;
pub mod frontier;
pub mod graph;
pub mod node;
pub mod policy;
pub mod scorer;
pub mod search;
