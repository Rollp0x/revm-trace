//! Trait implementations for TxInspector
//! 
//! This module implements the core traits required for the transaction inspector:
//! 
//! # Traits
//! 
//! - `Reset`: Manages inspector state between transactions
//!   - Clears all internal collections
//!   - Prepares inspector for new transaction
//! 
//! - `TraceOutput`: Converts collected data into final output format
//!   - Aggregates all execution data
//!   - Formats results for external consumption
//! 
//! These implementations enable the inspector to:
//! - Maintain clean state between transactions
//! - Provide standardized output format
//! - Integrate with the broader tracing system
use crate::traits::{Reset, TraceOutput};
use crate::inspectors::tx_inspector::TxTraceOutput;
use crate::inspectors::tx_inspector::TxInspector;

impl Reset for TxInspector {
    /// Resets all internal state for processing a new transaction
    /// 
    /// Clears all collections:
    /// - Transfer records
    /// - Call traces
    /// - Event logs
    /// - Call and address stacks
    /// - Pending creation transfers
    fn reset(&mut self) {
        self.call_traces = Vec::new();
        self.call_stack = Vec::new();
        self.transfers = Vec::new();
        self.logs = Vec::new();
        self.address_stack = Vec::new();
        self.pending_create_transfers = Vec::new();
    }
}

impl TraceOutput for TxInspector {
    type Output = TxTraceOutput;
    
    /// Generates the final trace output from collected execution data
    /// 
    /// Returns a TxTraceOutput containing:
    /// - All asset transfers
    /// - Complete call tree
    /// - All event logs
    /// - Error location if any
    fn get_output(&self) -> Self::Output {
        TxTraceOutput {
            asset_transfers: self.transfers.clone(),
            call_trace: self.call_traces.first().cloned(),
            logs: self.logs.clone(),
            error_trace_address: self.get_error_trace_address(),
        }
    }
}
