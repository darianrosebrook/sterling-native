//! Carrier module: `ByteState`, `Code32`, `ByteTrace`, and the compilation boundary.
//!
//! This is the foundational layer. No other kernel module is imported here.

pub mod bytestate;
pub mod bytetrace;
pub mod code32;
pub mod compile;
pub mod registry;
pub mod trace_reader;
pub mod trace_writer;
