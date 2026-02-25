//! Sterling Kernel: the deterministic core of Sterling Native.
//!
//! # API Surface
//!
//! The kernel exposes exactly three entry points:
//!
//! - [`carrier::compile::compile`] -- compile a domain payload into `ByteState`
//! - [`operators::apply::apply`] -- apply an operator to `ByteState`, producing a new state + step record
//! - [`proof::replay::replay_verify`] -- verify a trace bundle by deterministic replay
//!
//! # Module Dependency Direction
//!
//! `carrier` ← `operators` ← `proof`
//!
//! One-way only. No cycles. `proof` depends on `operators` and `carrier`.
//! `operators` depends on `carrier`. `carrier` depends on nothing internal.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]

pub mod carrier;
pub mod operators;
pub mod proof;
