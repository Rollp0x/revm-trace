//! Built-in REVM inspectors with trait implementations
//! 
//! This module re-exports REVM's built-in inspectors and implements
//! our custom traits (Reset and TraceOutput) for them to enable
//! their use in our EVM implementation.

pub use revm::inspectors::{
    CustomPrintTracer,
    GasInspector,
    NoOpInspector,
    TracerEip3155
};
use crate::traits::{Reset, TraceOutput};

// NoOpInspector implementations
/// Basic inspector that does nothing
impl Reset for NoOpInspector {
    fn reset(&mut self) {}
}

impl TraceOutput for NoOpInspector {
    type Output = ();
    fn get_output(&self) -> Self::Output { 
        // Do nothing
     }
}

// GasInspector implementations
/// Inspector for gas tracking
/// Note: Gas state is set in initialize_interp
impl Reset for GasInspector {
    fn reset(&mut self) {}
}

impl TraceOutput for GasInspector {
    type Output = ();
    fn get_output(&self) -> Self::Output { 
        // Do nothing
     }
}

// CustomPrintTracer implementations
/// Debug-focused inspector that prints execution details
impl Reset for CustomPrintTracer {
    fn reset(&mut self) {}
}

impl TraceOutput for CustomPrintTracer {
    type Output = ();
    fn get_output(&self) -> Self::Output { 
        // Do nothing
     }
}

// TracerEip3155 implementations
/// EIP-3155 compliant tracer
impl Reset for TracerEip3155 {
    /// Clears the internal trace buffer
    fn reset(&mut self) {
        self.clear();
    }
}

/// Implementation to satisfy trait bounds
/// 
/// While TracerEip3155 has its own output mechanism through the
/// underlying writer, we implement TraceOutput to maintain
/// consistency with our trait system.
impl TraceOutput for TracerEip3155 {
    type Output = ();
    fn get_output(&self) -> Self::Output {
        // Do nothing
    }
}