//! Transaction processing implementation for TraceEvm
//!
//! This module implements batch transaction processing with tracing capabilities.
//! It supports both stateful and stateless execution modes and provides detailed
//! inspector output for each transaction.

use std::collections::HashMap;

use crate::{
    evm::TraceEvm,
    traits::{ResetDB, TraceOutput, TransactionTrace, StorageDiff},
    types::{SimulationBatch, SimulationTx, SlotChange},
};

use crate::errors::{EvmError, RuntimeError};
use crate::traits::TraceInspector;
use revm::{
    context::{ContextTr, TxEnv},
    context_interface::result::ExecutionResult,
    database::{CacheDB, Database, DatabaseCommit, DatabaseRef},
    handler::MainnetContext,
    ExecuteEvm, InspectEvm,
};

impl<DB, INSP> TraceEvm<DB, INSP>
where
    DB: Database + DatabaseCommit,
    INSP: TraceInspector<MainnetContext<DB>>,
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
    /// 2. Fetches current nonce from account state
    /// 3. Builds transaction environment from input parameters
    /// 4. Executes transaction with inspector and commits changes
    /// 5. Collects and returns inspector output
    ///
    /// # Note
    /// This method is internal and should not be called directly.
    /// Use `trace_transactions` or `execute_batch` instead.
    fn trace_internal(
        &mut self,
        input: SimulationTx,
        is_stateful: bool,
    ) -> Result<(ExecutionResult, StorageDiff, INSP::Output), RuntimeError> {
        // Reset inspector state before processing
        self.reset_inspector();

        // Fetch current nonce for the transaction sender
        let nonce = self
            .db()
            .basic(input.caller)
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to get account info: {e}")))?
            .map(|acc| acc.nonce)
            .unwrap_or_default();
        let chain_id = self.cfg.chain_id;
        // Build transaction environment
        let tx = TxEnv::builder()
            .caller(input.caller)
            .value(input.value)
            .data(input.data)
            .kind(input.transact_to)
            .nonce(nonce)
            .chain_id(Some(chain_id))
            .build_fill();

        // Set transaction and execute with current inspector, committing changes
        self.set_tx(tx);
        let result = self.inspect_replay().map_err(|e| {
            RuntimeError::ExecutionFailed(format!("Inspector execution failed: {e}"))
        })?;
        let state = result.state;
        let result = result.result;
        let mut diffs = HashMap::new();
        for (address, account) in state.iter() {
            for (slot, value) in account.storage.iter() {
                if value.original_value != value.present_value {
                    // Store slot changes for diff output
                    diffs.entry(*address).or_insert_with(Vec::new).push(SlotChange {
                        address: *address,
                        slot: *slot,
                        old_value: value.original_value,
                        new_value: value.present_value,
                    });
                }
            }
        }
        if is_stateful {
            self.db().commit(state)
        } else {
            self.inspector.reset_slot_cache();
        }
        // Collect inspector output
        let output = self.get_inspector_output();
        Ok((result, diffs, output))
    }
}

/// Implementation of TransactionTrace trait for batch processing
impl<DB, INSP> TransactionTrace<MainnetContext<CacheDB<DB>>> for TraceEvm<CacheDB<DB>, INSP>
where
    DB: DatabaseRef,
    INSP: TraceInspector<MainnetContext<CacheDB<DB>>>,
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
    /// Vector of results, one for each transaction in the batch.
    /// Each result contains the execution result and inspector output.
    ///
    /// # Execution Modes
    /// - **Stateful** (`is_stateful = true`): State persists between transactions
    /// - **Stateless** (`is_stateful = false`): Database resets between transactions
    ///
    /// # Implementation Details
    /// 1. Sets block environment if provided in batch parameters
    /// 2. Resets database to clean state before processing
    /// 3. Processes each transaction in sequence using `trace_internal`
    /// 4. Manages state persistence based on `is_stateful` flag
    /// 5. Resets inspector state after batch completion
    ///
    /// # Example
    /// ```no_run
    /// # use revm_trace::*;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tracer = TxInspector::new();
    /// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", tracer).await?;
    /// let batch = SimulationBatch {
    ///     transactions: vec![/* transactions */],
    ///     is_stateful: true,
    /// };
    /// let results = evm.trace_transactions(batch);
    /// # Ok(())
    /// # }
    /// ```
    fn trace_transactions(
        &mut self,
        batch: SimulationBatch,
    ) -> Vec<Result<(ExecutionResult, StorageDiff, <Self::Inspector as TraceOutput>::Output), EvmError>> {
        let SimulationBatch {
            transactions,
            is_stateful,
        } = batch;

        // 2. Reset database to clean state
        self.reset_db();
        // reset inspector slot cache
        self.inspector.reset_slot_cache();

        let len = transactions.len();
        let mut results = Vec::with_capacity(len);

        // 3. Process each transaction in the batch
        for input in transactions.into_iter() {
            let result = self.trace_internal(input, is_stateful).map_err(EvmError::Runtime);
            results.push(result);
        }

        // 4. Clean up inspector state after batch completion
        self.reset_inspector();

        // 5. Reset transaction environment to prevent interference with other uses
        self.set_tx(Default::default());
        // Note: We don't reset_db here because EVM state can be preserved for other scenarios,
        // such as querying ERC20 token balances

        results
    }
}

impl<DB, INSP> TraceEvm<CacheDB<DB>, INSP>
where
    DB: DatabaseRef,
    INSP: TraceInspector<MainnetContext<CacheDB<DB>>>,
{
    /// Execute a batch of transactions and return only execution results
    ///
    /// This is a convenience method for users who only need transaction execution
    /// results without inspector output. It internally uses `trace_transactions`
    /// but discards the inspector output (which is `()` for `NoOpInspector`).
    ///
    /// # Arguments
    /// * `batch` - Batch of transactions to execute
    ///
    /// # Returns
    /// Vector of execution results, one for each transaction in the batch
    ///
    /// # Example
    /// ```no_run
    /// # use revm_trace::*;
    /// use revm_trace::errors::EvmError;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut evm = create_evm("https://eth.llamarpc.com").await?;
    /// let batch = SimulationBatch {
    ///     transactions: vec![/* transactions */],
    ///     is_stateful: false,
    /// };
    /// let results = evm.execute_batch(batch);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_batch(
        &mut self,
        batch: SimulationBatch,
    ) -> Vec<Result<ExecutionResult, EvmError>> {
        self.trace_transactions(batch)
            .into_iter()
            .map(|result| result.map(|(exec_result, _  , _)| exec_result))
            .collect()
    }
}
