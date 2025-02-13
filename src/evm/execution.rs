//! Transaction execution functionality for TraceEvm
//! 
//! Provides methods for:
//! - Single transaction processing
//! - Batch transaction handling
//! - Execution result management
//!
//! # Standard Execution Flow
//! 
//! The execution process follows a standardized flow for all inspector types:
//! 
//! 1. **Transaction Preparation**
//!    - Reset inspector state
//!    - Configure transaction parameters (caller, target, value, data)
//!    - Set up execution environment (block context, chain settings)
//! 
//! 2. **Execution Phase**
//!    - Execute transaction in EVM
//!    - Collect execution results (success/revert/error)
//!    - Gather inspector data (traces, transfers, logs)
//! 
//! 3. **State Management**
//!    - Reset database state (if non-stateful)
//!    - Maintain execution context between transactions
//!    - Handle state persistence based on configuration
//! 
//! 4. **Result Collection**
//!    - Process execution outcome (success/failure)
//!    - Convert error types if necessary (Runtime -> Evm)
//!    - Package results with inspector data for analysis
//!
//! # Batch Processing
//! 
//! When processing multiple transactions:
//! - All transactions are executed regardless of previous failures
//! - Each transaction maintains its own execution context
//! - Database state can be preserved between transactions if needed
//! - Inspector state is reset after all transactions complete
//! 
//! # Error Handling
//! 
//! The system distinguishes between two types of errors:
//! - `RuntimeError`: Execution-level errors (revert, out of gas, etc.)
//! - `EvmError`: System-level errors (invalid transaction, node failure, etc.)
//!
//! Error conversion follows a standard path:
//! 1. EVM errors -> RuntimeError (in process_transaction_internal)
//! 2. RuntimeError -> EvmError (in process_transactions)

use crate::traits::{GetInspector, Reset, TraceOutput, TransactionProcessor};
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
    /// Executes a single transaction with full inspection and result collection.
    /// This internal method handles the core execution logic and error conversion.
    /// 
    /// # Steps
    /// 1. Reset inspector state for clean execution
    /// 2. Configure transaction parameters (caller, target, value, data)
    /// 3. Execute transaction in EVM context
    /// 4. Collect and package execution results
    /// 
    /// # Arguments
    /// * `input` - Transaction parameters for simulation
    /// 
    /// # Returns
    /// * `Ok((ExecutionResult, I::Output))` - Successful execution with:
    ///   - ExecutionResult: EVM execution outcome
    ///   - I::Output: Collected inspector data
    /// * `Err(RuntimeError)` - Execution failed with runtime error
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
} 


impl<'a, T, P, I> TransactionProcessor for TraceEvm<'a, T, P, I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a + GetInspector<InspectorDB<T, P>> + Reset + TraceOutput,
{
    type InspectorOutput = I::Output;

    /// Process a batch of transactions following the standard execution flow
    /// 
    /// Executes multiple transactions while managing state and collecting results.
    /// Follows the standardized execution flow defined in the trait documentation.
    /// 
    /// # Processing Steps
    /// 1. **Preparation**
    ///    - Configure block environment for all transactions
    ///    - Initialize result collection
    /// 
    /// 2. **Execution**
    ///    - Process each transaction independently
    ///    - Convert runtime errors to system errors
    ///    - Maintain state based on configuration
    /// 
    /// 3. **State Management**
    ///    - Reset database if non-stateful
    ///    - Final inspector reset after completion
    /// 
    /// # Arguments
    /// * `batch` - Contains:
    ///   - Block environment settings
    ///   - Transaction list
    ///   - State persistence flag
    /// 
    /// # Returns
    /// * Vector of results where each entry contains:
    ///   - ExecutionResult: Transaction execution outcome
    ///   - InspectorOutput: Collected trace data
    ///   - Or EvmError if processing failed
    fn process_transactions(
        &mut self,
        batch: SimulationBatch
    ) -> Vec<Result<(ExecutionResult, Self::InspectorOutput), EvmError>> {
        let SimulationBatch { block_env, transactions, is_stateful } = batch;
        let mut results = Vec::with_capacity(transactions.len());
        
        // 1. Preparation
        self.set_block_env(block_env);
        self.reset_db();
        
        // 2. Execution
        let len = transactions.len();
        for (index,input) in transactions.into_iter().enumerate() {
            let result = self.process_transaction_internal(input)
                .map_err(EvmError::Runtime);
            results.push(result);
            
            if index !=len -1 &&  !is_stateful {
                self.reset_db();
            }
        }
        
        // 3. State Management
        self.reset_inspector();
        
        results
    }
}