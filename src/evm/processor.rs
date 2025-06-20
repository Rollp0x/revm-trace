//! Transaction processing implementation for TraceEvm
//!
//! This module implements batch transaction processing with tracing capabilities.
//! It supports both stateful and stateless execution modes and provides detailed
//! inspector output for each transaction.

use crate::{
    traits::TransactionProcessor,
    utils::block_utils::create_block_env,
    evm::TraceEvm,
    types::{SimulationBatch,SimulationTx}
};

use revm::{
    InspectCommitEvm,InspectEvm,
    context::{ContextTr, TxEnv}, 
    context_interface::result::ExecutionResult, 
    database::{CacheDB, Database, DatabaseRef}, 
    handler::MainnetContext, ExecuteCommitEvm, ExecuteEvm,
};
use crate::errors::{EvmError, RuntimeError};
use crate::traits::TraceInspector;

impl<DB, INSP> TraceEvm<CacheDB<DB>, INSP>
where
    DB: DatabaseRef,
    INSP: TraceInspector<MainnetContext<CacheDB<DB>>> + Clone,
{
    /// Process a single transaction with tracing
    ///
    /// Internal method that handles the execution of a single transaction,
    /// including inspector reset, transaction execution, and output collection.
    ///
    /// # Arguments
    /// * `input` - Transaction parameters and data
    ///
    /// # Returns
    /// * `Ok((ExecutionResult, Output))` - Execution result and inspector output
    /// * `Err(RuntimeError)` - If transaction execution fails
    ///
    /// # Implementation Details
    /// 1. Resets inspector state before execution
    /// 2. Builds transaction environment from input
    /// 3. Executes transaction with commit
    /// 4. Collects inspector output
    fn process_transaction_internal(
        &mut self,
        input: SimulationTx,
    ) -> Result<(ExecutionResult, INSP::Output), RuntimeError> {
        // Reset inspector state before execution
        self.reset_inspector();
        
        // Get current nonce from account state
        let nonce = self.db()
            .basic(input.caller)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to get account info: {}", e)))?
            .map(|acc| acc.nonce)
            .unwrap_or_default();
        
        // Build transaction environment from simulation input
        let tx = TxEnv::builder()
            .caller(input.caller)
            .value(input.value)
            .data(input.data)
            .kind(input.transact_to)
            .nonce(nonce)  // Use actual nonce from account state
            .build_fill();
        let inspector = self.clone_inspector();

        let r = self.inspect_commit(tx,inspector)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Inspector execution failed: {}", e)))?;
        // Collect inspector output after execution
        let output = self.get_inspector_output();
        Ok((r, output))
    }
}

/// Implementation of TransactionProcessor trait for batch processing
impl<DB, INSP> TransactionProcessor for TraceEvm<CacheDB<DB>, INSP> 
where
    DB: DatabaseRef,
    INSP: TraceInspector<MainnetContext<CacheDB<DB>>> + Clone,
{
    type InspectorOutput = INSP::Output;

    /// Process a batch of transactions with optional block context
    ///
    /// Executes multiple transactions in sequence, with support for both
    /// stateful (persistent state between transactions) and stateless
    /// (isolated transactions) execution modes.
    ///
    /// # Arguments
    /// * `batch` - Batch containing block parameters, transactions, and execution mode
    ///
    /// # Returns
    /// Vector of results, one for each transaction in the batch
    ///
    /// # Execution Modes
    /// - **Stateful** (`is_stateful = true`): State persists between transactions
    /// - **Stateless** (`is_stateful = false`): Database resets between transactions
    ///
    /// # Implementation Details
    /// 1. Sets block environment if provided
    /// 2. Resets database to clean state
    /// 3. Processes each transaction in sequence
    /// 4. Manages state persistence based on `is_stateful` flag
    /// 5. Resets inspector after batch completion
    fn process_transactions(
        &mut self,
        batch: SimulationBatch,
    ) -> Vec<Result<(ExecutionResult, Self::InspectorOutput), EvmError>> {
        let SimulationBatch {
            block_params,
            transactions,
            is_stateful,
        } = batch;
        
        // 1. Set block environment if provided
        if let Some(block) = block_params {
            let block = create_block_env(
                block.number,
                block.timestamp,
                None,
                None
            );
            self.set_block(block);
        }
        
        // 2. Reset database to clean state
        self.reset_db();
        
        let len = transactions.len();
        let mut results = Vec::with_capacity(len);

        // 3. Process each transaction in the batch
        for (index, input) in transactions.into_iter().enumerate() {
            let result = self.process_transaction_internal(input)
                .map_err(EvmError::Runtime);
            results.push(result);
            
            // Reset database between transactions if stateless mode
            if index != len - 1 && !is_stateful {
                self.reset_db();
            }
        }
        
        // 4. Clean up inspector state after batch completion
        self.reset_inspector();
        results
    }
}