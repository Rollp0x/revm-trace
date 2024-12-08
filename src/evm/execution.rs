//! Transaction execution functionality for TraceEvm
//! 
//! Provides methods for:
//! - Single transaction processing
//! - Batch transaction handling
//! - Execution result management

use crate::traits::{GetInspector, Reset, TraceOutput};
use crate::types::{InspectorDB, SimulationTx, SimulationBatch};
use crate::errors::{RuntimeError, EvmError};
use alloy::{
    providers::Provider,
    transports::Transport,
};
use revm::primitives::ExecutionResult;

use super::TraceEvm;

impl<'a, T, P, I> TraceEvm<'a, T, P, I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a + GetInspector<InspectorDB<T, P>>,
{
    /// Process a single transaction and collect results
    /// 
    /// # Steps
    /// 1. Reset inspector state
    /// 2. Configure transaction parameters
    /// 3. Execute transaction
    /// 4. Collect inspector output
    /// 
    /// # Arguments
    /// * `input` - Transaction parameters for simulation
    /// 
    /// # Returns
    /// * `Ok((ExecutionResult, I::Output))` - Execution result and inspector data
    /// * `Err(RuntimeError)` - If execution fails
    /// 
    /// # Type Parameters
    /// * `I: Reset + TraceOutput` - Inspector must support state reset and output collection
    fn process_transaction_internal(
        &mut self,
        input: SimulationTx
    ) -> Result<(ExecutionResult, I::Output), RuntimeError>
    where
        I: Reset + TraceOutput,
    {   
        // Reset inspector state before execution
        self.reset_inspector();

        // Configure transaction parameters
        let tx = self.tx_mut();
        tx.caller = input.caller;
        tx.transact_to = input.transact_to;
        tx.value = input.value;
        tx.data = input.data;
        
        // Execute transaction and handle errors
        let execution_result = self.transact_commit()
            .map_err(|e| RuntimeError::ExecutionFailed(e.to_string()))?;
        
        // Collect inspector output
        let inspector_output = self.get_inspector_output();
        
        Ok((execution_result, inspector_output))
    }

    /// Process multiple transactions in batch mode
    /// 
    /// # Arguments
    /// * `batch` - Batch configuration containing:
    ///   - Block environment settings
    ///   - List of transactions
    ///   - State persistence flag
    /// 
    /// # Returns
    /// * `Ok(Vec<(ExecutionResult, I::Output)>)` - Results for each transaction
    /// * `Err(EvmError)` - If batch processing fails
    /// 
    /// # Features
    /// - Configures block environment for all transactions
    /// - Optionally maintains state between transactions
    /// - Collects results for each transaction
    /// 
    /// # Type Parameters
    /// * `I: Reset + TraceOutput` - Inspector must support state reset and output collection
    pub fn process_transactions(
        &mut self,
        batch: SimulationBatch
    ) -> Result<Vec<(ExecutionResult, I::Output)>, EvmError>
    where
        I: Reset + TraceOutput,
    {   
        let SimulationBatch { block_env, transactions, is_stateful } = batch;
        let mut results = Vec::new();
        
        // Configure block environment for all transactions
        self.set_block_env(block_env);
        
        // Process each transaction
        for input in transactions {
            let exec_result = self.process_transaction_internal(input)?;
            results.push(exec_result);
            
            // Reset state for independent transactions
            if !is_stateful {
                self.reset_db().reset_inspector();
            }
        }
        
        Ok(results)
    }
}    