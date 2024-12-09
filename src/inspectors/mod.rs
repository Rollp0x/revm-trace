//! EVM execution inspectors
//! 
//! This module provides two types of inspectors:
//! 
//! - `revm_inspectors`: Implementations for REVM's built-in inspectors
//!   with additional trait implementations for our system
//! 
//! - `tx_inspector`: Custom transaction inspector that provides detailed
//!   tracking of execution, including asset transfers, call hierarchy,
//!   and error propagation
//! 
//! Both inspector types implement the core traits needed for integration
//! with our EVM implementation.

pub mod revm_inspectors;
pub mod tx_inspector;

pub use revm_inspectors::*;
pub use tx_inspector::*;
