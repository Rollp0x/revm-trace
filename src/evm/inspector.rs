//! Inspector management for TraceEvm
//!
//! This module provides inspector-specific functionality for the TraceEvm,
//! allowing access to trace data and inspector state management.

use revm::database::Database;
use crate::traits::{TraceOutput,Reset};
use crate::evm::TraceEvm;

impl<DB,INSP> TraceEvm<DB,INSP> 
where
    DB: Database,
    INSP: TraceOutput + Reset + Clone,
{
    /// Retrieve the current output from the inspector
    ///
    /// Returns the accumulated trace data or analysis results from the inspector.
    /// The exact type and content depends on the specific inspector implementation.
    ///
    /// # Examples
    /// ```no_run
    /// use revm_trace::{create_evm_with_tracer, TxInspector};
    /// 
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let trace_inspector = TxInspector::new();
    /// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", trace_inspector).await?;
    /// 
    /// // After processing transactions...
    /// let trace_output = evm.get_inspector_output();
    /// println!("Collected trace data: {:?}", trace_output);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Returns
    /// The inspector's output data (traces, logs, analysis results, etc.)
    pub fn get_inspector_output(&self) -> INSP::Output {
        self.inspector.get_output()
    }

    /// Reset the inspector to its initial state
    ///
    /// Clears any accumulated trace data, logs, or internal state in the inspector.
    /// This should be called before processing a new transaction or batch to ensure
    /// clean state isolation between transactions.
    ///
    /// # Examples
    /// ```no_run
    /// use revm_trace::{create_evm_with_tracer, TxInspector};
    /// 
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let trace_inspector = TxInspector::new();
    /// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", trace_inspector).await?;
    /// 
    /// // Process first transaction...
    /// // let result1 = evm.process_transaction(tx1).await?;
    /// 
    /// // Reset before processing next transaction
    /// evm.reset_inspector();
    /// 
    /// // Process second transaction with clean state...
    /// // let result2 = evm.process_transaction(tx2).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Note
    /// This method is automatically called internally before each transaction
    /// in batch processing to ensure proper state isolation.
    pub fn reset_inspector(&mut self) {
        self.inspector.reset();
    }

    /// Clone the inspector instance for trace analysis
    ///
    /// Creates a fresh copy of the current inspector. This is primarily used
    /// internally by the tracing system when calling `inspect_commit` functions,
    /// which require ownership of an inspector instance.
    ///
    /// # Design Rationale
    /// 
    /// In revm 24+, the new inspector architecture requires passing inspector
    /// instances by value to `inspect_commit` functions. This means:
    /// 
    /// - Each transaction trace needs its own inspector instance
    /// - The inspector must be cloned before being passed to `inspect_commit`
    /// - This ensures proper isolation between transaction traces
    /// 
    /// # Examples
    /// ```no_run
    /// use revm_trace::{create_evm_with_tracer, TxInspector};
    /// 
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let trace_inspector = TxInspector::new();
    /// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", trace_inspector).await?;
    /// 
    /// // Clone inspector for external analysis
    /// let inspector_copy = evm.clone_inspector();
    /// 
    /// // The original inspector remains unchanged
    /// // while the copy can be used independently
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Returns
    /// A fresh copy of the inspector with the same configuration but
    /// independent state (reset to initial state).
    ///
    /// # Note
    /// This method is called internally during transaction processing to
    /// provide fresh inspector instances to `inspect_commit` functions.
    /// The `Clone` trait requirement ensures this operation is always available.
    pub fn clone_inspector(&self) -> INSP {
        self.inspector.clone()
    }
}