//! Inspector implementations for transaction tracing
//!
//! This module provides various inspector implementations that can be used
//! with the TraceEvm for collecting detailed execution information.
//!
//! # Available Inspectors
//! - `revm_inspectors`: Implementations for built-in REVM inspectors
//! - `tx_inspector`: Custom transaction inspector with comprehensive tracing

pub mod revm_inspectors;
pub mod tx_inspector;
pub mod test_inspector;