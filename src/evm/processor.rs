//! Transaction processing implementation for TraceEvm
//!
//! This module implements batch transaction processing with tracing capabilities.
//! It supports both stateful and stateless execution modes and provides detailed
//! inspector output for each transaction.

use crate::{
    traits::{TransactionTrace,TraceOutput,ResetDB},
    evm::{TraceEvm,builder::DefaultEvm},
    types::{SimulationBatch,SimulationTx}
};

use revm::{
    InspectCommitEvm,
    context::{ContextTr, TxEnv}, 
    context_interface::result::ExecutionResult, 
    database::{CacheDB, Database, DatabaseRef,DatabaseCommit}, 
    handler::MainnetContext, ExecuteEvm,
};
use crate::errors::{EvmError, RuntimeError};
use crate::traits::TraceInspector;


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
impl<DB, INSP> TraceEvm<DB, INSP> 
where
    DB: Database + DatabaseCommit,
    INSP: TraceInspector<MainnetContext<DB>> + Clone,
{
    fn trace_internal(
        &mut self,
        input: SimulationTx,
    ) -> Result<(ExecutionResult, INSP::Output), RuntimeError> {
        // 重置 inspector 状态
        self.reset_inspector();
        
        // 获取当前 nonce
        let nonce = self.db()
            .basic(input.caller)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to get account info: {}", e)))?
            .map(|acc| acc.nonce)
            .unwrap_or_default();
        
        // 构建交易环境
        let tx = TxEnv::builder()
            .caller(input.caller)
            .value(input.value)
            .data(input.data)
            .kind(input.transact_to)
            .nonce(nonce)
            .build_fill();
            
        let inspector = self.clone_inspector();

        let r = self.inspect_commit(tx, inspector)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Inspector execution failed: {}", e)))?;
            
        // 收集 inspector 输出
        let output = self.get_inspector_output();
        Ok((r, output))
    }
}

/// Implementation of TransactionProcessor trait for batch processing
impl<DB, INSP> TransactionTrace<MainnetContext<CacheDB<DB>>> for TraceEvm<CacheDB<DB>, INSP> 
where
    DB: DatabaseRef,
    INSP: TraceInspector<MainnetContext<CacheDB<DB>>> + Clone,
{
    type Inspector = INSP;

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
    fn trace_transactions(
            &mut self,
            batch: SimulationBatch
        ) -> Vec<Result<(ExecutionResult, <Self::Inspector as TraceOutput>::Output), EvmError>> 
    {
        
        let SimulationBatch {
            block_env,
            transactions,
            is_stateful,
        } = batch;
        
        // 1. Set block environment if provided
        if let Some(block_env) = block_env {
            self.set_block(block_env);
        }
        
        // 2. Reset database to clean state
        self.reset_db();
        
        let len = transactions.len();
        let mut results = Vec::with_capacity(len);

        // 3. Process each transaction in the batch
        for (index, input) in transactions.into_iter().enumerate() {
            let result = self.trace_internal(input)
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

impl DefaultEvm {
    /// Execute transactions and return only execution results (ignore inspector output)
    pub fn execute_batch(
        &mut self,
        batch: SimulationBatch,
    ) -> Vec<Result<ExecutionResult, EvmError>> {
        self.trace_transactions(batch)
            .into_iter()
            .map(|result| result.map(|(exec_result, _)| exec_result))
            .collect()
    }
}