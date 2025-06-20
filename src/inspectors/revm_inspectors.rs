//! REVM built-in inspector implementations
//!
//! This module provides trait implementations for REVM's built-in inspectors,
//! making them compatible with our TraceInspector trait system.
//!
//! # Supported Inspectors
//! - `NoOpInspector`: No-operation inspector for basic execution
//! - `GasInspector`: Gas consumption tracking inspector
//! - `TracerEip3155`: EIP-3155 compliant execution tracing
//!
//! Each inspector implements the `Reset` and `TraceOutput` traits to provide
//! consistent behavior within the TraceEvm framework.

pub use revm::inspector::{
    NoOpInspector,
    inspectors::{GasInspector, TracerEip3155},
};
use revm::{
    handler::MainnetContext,
    database::{DatabaseRef,CacheDB},
};
use crate::traits::{Reset, TraceOutput,TraceInspector};

/// NoOpInspector implementations
/// 
/// Basic inspector that performs no operations during execution.
/// Useful for simple transaction execution without tracing overhead.
impl Reset for NoOpInspector {
    fn reset(&mut self) {}
}

impl TraceOutput for NoOpInspector {
    type Output = ();
    
    fn get_output(&self) -> Self::Output { 
        // No-op inspector produces no output
    }
}

/// GasInspector implementations
/// 
/// Inspector for tracking gas consumption during execution.
/// Gas state is automatically managed by the EVM in initialize_interp.
impl Reset for GasInspector {
    fn reset(&mut self) {}
}

impl TraceOutput for GasInspector {
    type Output = ();
    
    fn get_output(&self) -> Self::Output { 
        // Gas data is available through GasInspector's own methods
    }
}

/// TracerEip3155 implementations
/// 
/// Inspector that provides EIP-3155 compliant execution tracing.
/// Captures detailed step-by-step execution information.
impl Reset for TracerEip3155 {
    fn reset(&mut self) {
        self.clear();
    }
}

impl TraceOutput for TracerEip3155 {
    type Output = ();
    
    fn get_output(&self) -> Self::Output {
        // Trace data is available through TracerEip3155's own methods
    }
}


impl<DB> TraceInspector<MainnetContext<CacheDB<DB>>> for TracerEip3155
where
    DB: DatabaseRef,
{
    
}


impl<DB> TraceInspector<MainnetContext<CacheDB<DB>>> for NoOpInspector
where
    DB: DatabaseRef,
{
    
}