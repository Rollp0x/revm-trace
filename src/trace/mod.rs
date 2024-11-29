//! Transaction tracing and asset transfer tracking
//!
//! This module provides functionality for:
//! - Tracing EVM transaction execution and call hierarchy
//! - Tracking native token and ERC20 token transfers
//! - Collecting transaction logs and events
//! - Recording detailed execution traces and error information
//! - Supporting multiple EVM-compatible chains (Ethereum, BSC, etc.)
//!
//! # Core Features
//! - **Asset Tracking**: Monitor both native and ERC20 token transfers
//! - **Log Collection**: Capture all emitted events during execution
//! - **Call Tracing**: Record complete call hierarchy with results
//! - **Token Metadata**: Collect token symbols and decimals
//! - **Chain Support**: Work with any EVM-compatible blockchain
//!
//! # Example Usage
//! ```no_run
//! use revm_trace::{create_evm_instance_with_tracer, trace_tx_assets};
//! use alloy::primitives::{address, U256};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Initialize EVM with tracer
//! let mut evm = create_evm_instance_with_tracer(
//!     "https://eth-mainnet.example.com",
//!     Some(1)  // Ethereum mainnet
//! )?;
//!
//! // Setup transaction parameters
//! let from = address!("dead00000000000000000000000000000000beef");
//! let to = address!("cafe00000000000000000000000000000000face");
//! let value = U256::from(1000000000000000000u64); // 1 ETH
//! let data = vec![]; // Empty calldata for simple transfer
//!
//! // Execute and trace the transaction
//! let result = trace_tx_assets(&mut evm, from, to, value, data, "ETH").await;
//!
//! // Process results
//! for transfer in &result.asset_transfers {
//!     let token_info = result.token_info.get(&transfer.token)
//!         .expect("Token info should exist");
//!     println!("Transfer: {} {} from {} to {}",
//!         transfer.value, token_info.symbol, transfer.from, transfer.to);
//! }
//!
//! // Access transaction logs
//! for log in &result.logs {
//!     println!("Log from {}: {:?}", log.address, log);
//! }
//!
//! // Check for errors
//! if let Some(error) = result.error {
//!     println!("Transaction failed: {}", error);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Chain Support
//! Supports all major EVM-compatible chains:
//! - Ethereum (`"ETH"`)
//! - BNB Smart Chain (`"BNB"`)
//! - Polygon (`"MATIC"`)
//! - Arbitrum (`"ETH"`)
//! - Optimism (`"ETH"`)
//! - And more...

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

/// Simulates transaction execution and tracks all asset transfers, logs, and call traces
///
/// # Arguments
/// * `evm` - Configured EVM instance with optional inspector
/// * `from` - Transaction sender address
/// * `to` - Transaction recipient address
/// * `value` - Native token value to send (in wei)
/// * `data` - Transaction calldata (function selector and parameters)
/// * `native_token_symbol` - Native token symbol (e.g., "ETH", "BNB", "MATIC")
///
/// # Returns
/// A `TraceResult` containing:
/// * `asset_transfers` - List of all token transfers
/// * `token_info` - Token metadata (symbols and decimals)
/// * `traces` - Complete call hierarchy
/// * `logs` - All emitted transaction logs
/// * `error` - Error information if transaction failed
///
/// # Features
/// - Records all native and ERC20 token transfers
/// - Captures all transaction logs and events
/// - Tracks complete call hierarchy
/// - Collects token metadata
/// - Handles transaction errors
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
        let mut result = TraceResult::new(vec![], HashMap::new(), vec![], vec![], native_token_symbol);
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
    let logs = if let Some(inspector) =
        (&evm.context.external as &dyn Any).downcast_ref::<TransactionTracer>()
    {
        inspector.logs.clone()
    } else {
        Vec::new()
    };

    TraceResult::new(transfers, token_info, traces, logs,native_token_symbol)
}
