//! Transaction tracing and asset transfer tracking
//!
//! This module provides functionality for:
//! - Tracing EVM transaction execution and call hierarchy
//! - Tracking native token and ERC20 token transfers
//! - Collecting token metadata (symbols and decimals)
//! - Recording detailed execution traces and error information
//! - Supporting multiple EVM-compatible chains (Ethereum, BSC, etc.)

pub mod inspector;
pub mod types;

use crate::utils::erc20_utils::{get_token_decimals, get_token_symbol};
use alloy::{network::Network, primitives::Address, providers::Provider, transports::Transport};
use inspector::TransactionTracer;
use revm::{
    db::{AlloyDB, WrapDatabaseRef}, primitives::{TxKind, U256}, Evm, Inspector
};
use std::any::Any;
use std::collections::HashMap;
use types::{ExecutionError, TokenInfo, TraceResult, TransferRecord};

/// Simulates transaction execution and tracks all asset transfers and call traces
///
/// This function provides comprehensive transaction tracing capabilities by:
/// - Recording all native token and ERC20 token transfers
/// - Capturing complete call hierarchy with execution results
/// - Collecting token metadata (symbols and decimals)
/// - Tracking transaction errors and custom revert messages
/// - Supporting multiple EVM-compatible blockchains
///
/// # Arguments
/// * `evm` - Configured EVM instance with optional inspector
/// * `from` - Transaction sender address
/// * `to` - Transaction recipient address
/// * `value` - Native token value to send (in wei)
/// * `data` - Transaction calldata (function selector and parameters)
/// * `native_token_symbol` - Native token symbol of the blockchain (e.g., "ETH" for Ethereum, "BNB" for BSC)
///
/// # Returns
/// Returns a `TraceResult` containing:
/// * `asset_transfers` - Chronologically ordered list of all asset transfers
/// * `token_info` - Metadata for all involved tokens (including native token)
/// * `traces` - Complete call hierarchy with execution details
/// * `error` - Detailed error information if the transaction failed
///
/// # Example
/// ```no_run
/// use revm_trace::{
///     evm::create_evm_instance_with_inspector,
///     trace::{trace_tx_assets, TransactionTracer},
/// };
/// use alloy::primitives::{address, U256};
///
/// # async fn example() -> anyhow::Result<()> {
/// // Initialize EVM for Ethereum mainnet
/// let mut evm = create_evm_instance_with_inspector(
///     "https://eth-mainnet.example.com",
///     TransactionTracer::default(),
///     None
/// )?;
///
/// // Setup transaction parameters
/// let from = address!("dead00000000000000000000000000000000beef");
/// let to = address!("cafe00000000000000000000000000000000face");
/// let value = U256::from(1000000000000000000u64); // 1 native token
/// let data = vec![]; // Empty calldata for simple transfer
///
/// // Choose native token based on the chain
/// let native_token_symbol = "ETH";  // Use appropriate symbol for each chain:
///                                   // "ETH" for Ethereum
///                                   // "BNB" for BSC
///                                   // "MATIC" for Polygon
///
/// // Execute and trace the transaction
/// let result = trace_tx_assets(&mut evm, from, to, value, data, native_token_symbol).await;
///
/// // Process trace results
/// for transfer in &result.asset_transfers {
///     let token_info = result.token_info.get(&transfer.token)
///         .expect("Token info should exist");
///     println!("Transfer: {} {} from {} to {}",
///         transfer.value, token_info.symbol, transfer.from, transfer.to);
/// }
///
/// // Check for errors
/// if let Some(error) = result.error {
///     println!("Transaction failed: {}", error);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Notes
/// - Using `TransactionTracer` enables comprehensive transfer and call tracking
/// - Other inspectors will only record top-level native token transfers
/// - Token metadata is collected regardless of transaction success
/// - Execution errors are included in the result rather than thrown
/// - Supports all EVM-compatible chains (Ethereum, BSC, Polygon, etc.)
pub async fn trace_tx_assets<'a, T, N, P, I>(
    evm: &mut Evm<'a, I, WrapDatabaseRef<AlloyDB<T, N, P>>>,
    from: Address,
    to: Address,
    value: U256,
    data: Vec<u8>,
    native_token_symbol: &str,
) -> TraceResult
where
    T: Transport + Clone,
    N: Network,
    P: Provider<T, N>,
    I: Inspector<WrapDatabaseRef<AlloyDB<T, N, P>>> + Default + Any,
{
    let mut transfers = Vec::new();
    let mut token_info = HashMap::new();

    let tx = evm.tx_mut();
    tx.caller = from;
    tx.transact_to = TxKind::Call(to);
    tx.value = value;
    tx.data = data.into();

    // Execute transaction and return error of pre-execution
    let execution_result = evm.transact();
    if let Err(evm_error) = execution_result {
        let mut result = TraceResult::new(vec![], HashMap::new(), vec![], native_token_symbol);
        result.error = Some(ExecutionError::Custom(format!("Pre-execution error: {}", evm_error)));
        return result;
    }

    // Get transfers from inspector if available
    if let Some(inspector) = (&evm.context.external as &dyn Any).downcast_ref::<TransactionTracer>()
    {
        transfers = inspector.transfers.clone();
    } else {
        // For other inspectors, only record top-level native token transfer
        if value > U256::ZERO {
            transfers.push(TransferRecord::new_native_token(from, to, value));
        }
    }

    // Collect token information even if transaction failed
    for transfer in &transfers {
        if !transfer.is_native_token() && !token_info.contains_key(&transfer.token) {
            if let Ok(symbol) = get_token_symbol(evm, transfer.token) {
                if let Ok(decimals) = get_token_decimals(evm, transfer.token) {
                    token_info.insert(transfer.token, TokenInfo { symbol, decimals });
                }
            }
        }
    }

    // Get call traces
    let traces = if let Some(inspector) =
        (&evm.context.external as &dyn Any).downcast_ref::<TransactionTracer>()
    {
        inspector.traces.clone()
    } else {
        Vec::new()
    };

    TraceResult::new(transfers, token_info, traces, native_token_symbol)
}
