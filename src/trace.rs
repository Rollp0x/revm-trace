//! Transaction tracing implementation
//!
//! This module provides functionality for:
//! - Executing and tracing individual transactions
//! - Handling batch transaction simulations
//! - Collecting execution results and token transfers
//! - Managing multicall scenarios
//!
//! The tracing system supports both standalone transactions and batched executions,
//! with special handling for multicall scenarios.

use alloy::{
    primitives::hex,
    providers::Provider,
    transports::Transport,
};
use anyhow::Result;
use revm::primitives::ExecutionResult;
use std::collections::HashMap;
use crate::{
    evm::*,
    types::*,
    utils::{erc20_utils::*, error_utils::parse_custom_error},
};

/// Trait for transaction tracing implementations
pub trait Tracer {
    /// Execute a batch of transactions
    ///
    /// # Arguments
    /// * `batch` - Batch of transactions to simulate
    ///
    /// # Returns
    /// * Vector of trace results for each transaction
    fn trace_txs(&mut self, batch: SimulationBatch) -> Result<Vec<TraceResult>>;

    /// Execute a single transaction with the current EVM state
    ///
    /// # Arguments
    /// * `input` - Transaction input parameters
    /// * `block_env` - Block environment for the simulation
    ///
    /// # Returns
    /// * Trace result containing execution details
    fn trace_tx(&mut self, input: SimulationTx, block_env: BlockEnv) -> Result<TraceResult>;
}

impl<'a, T, P> TraceEvm<'a, T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    /// Internal method to execute a single transaction
    ///
    /// # Arguments
    /// * `input` - Transaction parameters
    /// * `block_env` - Block environment
    /// * `initial_index` - Index for multicall sub-transactions (None for top-level calls)
    ///
    /// # Returns
    /// * Trace result containing execution details and collected data
    fn trace_tx_inner(
        &mut self,
        input: SimulationTx,
        block_env: &BlockEnv,
        initial_index: Option<usize>
    ) -> Result<TraceResult> {
        // Set transaction parameters
        let tx = self.tx_mut();
        tx.caller = input.caller;
        tx.transact_to = input.transact_to;
        tx.value = input.value;
        tx.data = input.data;

        // Execute transaction and handle pre-execution errors
        let execution_result = match self.transact_commit() {
            Err(evm_error) => {
                return Ok(TraceResult {
                    block_env: block_env.clone(),
                    asset_transfers: vec![],
                    token_infos: HashMap::new(),
                    call_traces: vec![],
                    logs: vec![],
                    status: ExecutionStatus::Failed {
                        kind: FailureKind::PreExecution(format!("{:?}", evm_error)),
                        gas_used: 0,
                        output: None,
                    },
                });
            }
            Ok(result) => result,
        };

        // Collect execution data
        let transfers = self.get_token_transfers().unwrap_or_default();
        let logs = self.get_logs().unwrap_or_default();
        let mut traces = self.get_call_traces().unwrap_or_default();

        // Update trace addresses for multicall sub-transactions
        if let Some(index) = initial_index {
            for trace in &mut traces {
                trace.trace_address.insert(0, index);
                fn update_subtrace_address(trace: &mut CallTrace, index: usize) {
                    for subtrace in &mut trace.subtraces {
                        subtrace.trace_address.insert(0, index);
                        update_subtrace_address(subtrace, index);
                    }
                }
                update_subtrace_address(trace, index);
            }
        }

        // Build final execution status
        let status = self.build_execution_status(execution_result);

        // Reset inspector state before collecting token info to avoid mixing traces
        self.reset_inspector();
        let token_infos = self.collect_token_info(&transfers)?;
        Ok(TraceResult {
            block_env: block_env.clone(),
            asset_transfers: transfers,
            token_infos,
            call_traces: traces,
            logs,
            status,
        })
    }

    /// Prepares the EVM for a new transaction
    ///
    /// Resets the database, updates block environment, and clears the inspector
    fn prepare_tx(&mut self, block_env: &BlockEnv) {
        self.reset_db().set_block_env(block_env.clone()).reset_inspector();
    }

    /// Collects token information for all transfers
    ///
    /// Gathers symbol and decimals for both native and ERC20 tokens
    fn collect_token_info(&mut self, transfers: &[TokenTransfer]) -> Result<HashMap<Address, TokenConfig>> {
        let mut token_infos = HashMap::new();
        
        // Add native token information
        if let Some(config) = self.get_native_token_config() {
            token_infos.insert(NATIVE_TOKEN_ADDRESS, TokenConfig { 
                symbol: config.symbol.clone(), 
                decimals: config.decimals 
            });
        } else {
            let default_token = get_default_native_token(self.get_chain_id());
            token_infos.insert(NATIVE_TOKEN_ADDRESS, TokenConfig { 
                symbol: default_token.symbol.clone(), 
                decimals: default_token.decimals 
            });
        }
        
        // Add ERC20 token information
        for transfer in transfers {
            if !transfer.is_native_token() && !token_infos.contains_key(&transfer.token) {
                if let (Ok(symbol), Ok(decimals)) = (
                    get_token_symbol(self, transfer.token),
                    get_token_decimals(self, transfer.token)
                ) {
                    token_infos.insert(transfer.token, TokenConfig { symbol, decimals });
                }
            }
        }
        
        Ok(token_infos)
    }

    /// Builds execution status from EVM execution result
    ///
    /// Handles success, revert, and halt cases
    fn build_execution_status(&self, result: ExecutionResult) -> ExecutionStatus {
        match result {
            ExecutionResult::Success { gas_used, gas_refunded, output, .. } => {
                ExecutionStatus::Success {
                    gas_used,
                    gas_refunded,
                    output,
                }
            },
            ExecutionResult::Revert { gas_used, output } => {
                ExecutionStatus::Failed {
                    kind: FailureKind::Revert(
                        parse_custom_error(&output)
                            .unwrap_or_else(|| format!("Reverted: 0x{}", hex::encode(output.clone())))
                    ),
                    gas_used,
                    output: Some(output),
                }
            },
            ExecutionResult::Halt { reason, gas_used } => {
                ExecutionStatus::Failed {
                    kind: FailureKind::Halt(format!("{:?}", reason)),
                    gas_used,
                    output: None,
                }
            },
        }
    }
}

impl<'a, T, P> Tracer for TraceEvm<'a, T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    fn trace_txs(&mut self, batch: SimulationBatch) -> Result<Vec<TraceResult>> {
        let SimulationBatch { block_env, transactions, is_multicall } = batch;
        self.reset_db().set_block_env(block_env.clone());

        if !is_multicall {
            // Handle independent transactions
            let mut results = Vec::with_capacity(transactions.len());
            for tx in transactions {
                self.reset_inspector();
                let result = self.trace_tx_inner(tx, &block_env, None)?;
                results.push(result);
            }
            return Ok(results);
        }

        // Handle multicall scenario
        let mut combined_transfers = Vec::new();
        let mut combined_token_infos = HashMap::new();
        let mut combined_traces = Vec::new();
        let mut combined_logs = Vec::new();
        let mut sum_gas_used = 0;
        let mut sum_gas_refunded = 0;

        for (index, tx) in transactions.into_iter().enumerate() {
            self.reset_inspector();
            let result = self.trace_tx_inner(tx, &block_env, Some(index))?;
            let is_success = result.is_success();
            let output_bytes = result.get_output_bytes();
            let status = result.status.clone();

            // Combine results
            sum_gas_used += result.get_gas_used();
            sum_gas_refunded += result.get_gas_refunded();
            combined_transfers.extend(result.asset_transfers);
            combined_token_infos.extend(result.token_infos);
            combined_traces.extend(result.call_traces);
            combined_logs.extend(result.logs);

            // Stop execution on first error in multicall
            if !is_success {
                return Ok(vec![TraceResult {
                    block_env: block_env.clone(),
                    asset_transfers: combined_transfers,
                    token_infos: combined_token_infos,
                    call_traces: combined_traces,
                    logs: combined_logs,
                    status: ExecutionStatus::Failed {
                        kind: match status {
                            ExecutionStatus::Failed { kind, .. } => kind,
                            _ => unreachable!(),
                        },
                        gas_used: sum_gas_used,
                        output: output_bytes,
                    },
                }]);
            }
        }

        Ok(vec![TraceResult {
            block_env,
            asset_transfers: combined_transfers,
            token_infos: combined_token_infos,
            call_traces: combined_traces,
            logs: combined_logs,
            status: ExecutionStatus::Success {
                gas_used: sum_gas_used,
                gas_refunded: sum_gas_refunded,
                output: Output::Call(Bytes::new()),
            },
        }])
    }

    fn trace_tx(&mut self, input: SimulationTx, block_env: BlockEnv) -> Result<TraceResult> {
        self.prepare_tx(&block_env);
        self.trace_tx_inner(input, &block_env, None)
    }
}

