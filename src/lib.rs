//! # REVM Transaction Simulator and Asset Tracer
//!
//! A library for simulating EVM transactions and tracking asset transfers, logs, and events
//! across multiple EVM-compatible chains.
//!
//! ## Core Features
//!
//! - **Transaction Simulation**
//!   - Full EVM execution environment
//!   - Support for multiple EVM-compatible chains
//!   - Historical block state access
//!   - Custom inspector integration
//!   - Gas-free simulation mode
//!
//! - **Asset and Event Tracking**
//!   - Native token transfers (ETH/BNB/MATIC)
//!   - ERC20 token transfers
//!   - Transaction logs and events
//!   - Token metadata collection
//!   - Chronological ordering
//!
//! - **Execution Analysis**
//!   - Complete call traces
//!   - Event log collection
//!   - Proxy contract detection
//!   - Error tracking and analysis
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use revm_trace::{
//!     trace::trace_tx_assets,
//!     create_evm_instance_with_tracer,
//!     TransactionTracer,
//! };
//! use alloy::primitives::{address, U256};
//!
//! async fn example() -> anyhow::Result<()> {
//!     // Initialize EVM with transaction tracer
//!     let mut evm = create_evm_instance_with_tracer(
//!         "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY",
//!         Some(1)  // Chain ID for Ethereum mainnet
//!     )?;
//!     
//!     // Setup transaction parameters
//!     let from = address!("dead00000000000000000000000000000000beef");
//!     let to = address!("dac17f958d2ee523a2206206994597c13d831ec7"); // USDT
//!     let value = U256::from(1000000000000000000u64); // 1 ETH
//!     let data = vec![]; // Empty calldata
//!     
//!     // Simulate transaction and analyze results
//!     let result = trace_tx_assets(&mut evm, from, to, value, data, "ETH").await;
//!     
//!     // Process transfers
//!     for transfer in result.asset_transfers() {
//!         let token_info = result.token_info.get(&transfer.token)
//!             .expect("Token info should exist");
//!         println!(
//!             "Transfer: {} {} from {} to {}",
//!             transfer.value, token_info.symbol,
//!             transfer.from, transfer.to
//!         );
//!     }
//!     
//!     // Process logs
//!     for log in result.logs {
//!         println!("Log from {}: {:?}", log.address, log);
//!     }
//!     
//!     // Check for errors
//!     if let Some(error) = result.error {
//!         println!("Transaction failed: {}", error);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Module Structure
//!
//! - [`evm`]: EVM configuration and initialization
//! - [`trace`]: Transaction tracing and event tracking
//! - [`utils`]: Helper functions for contract interactions
//! - [`traits`]: Core traits for inspector and block configuration
//!
//! ## Core Types
//!
//! - [`TraceResult`]: Complete transaction analysis results
//! - [`TransferRecord`]: Individual asset transfer records
//! - [`TokenInfo`]: Token metadata
//! - [`TransactionTracer`]: Transaction tracking inspector
//! - [`ExecutionError`]: Structured error information
//!
//! ## Supported Networks
//!
//! Compatible with all EVM-based networks:
//! - Ethereum (ETH)
//! - BNB Smart Chain (BNB)
//! - Polygon (MATIC)
//! - Arbitrum (ETH)
//! - Optimism (ETH)
//! - And more...

pub mod evm;
pub mod trace;
pub mod utils;
pub mod traits;

// Re-export commonly used types and functions
pub use evm::{create_evm_instance, create_evm_instance_with_tracer};
pub use trace::{
    inspector::TransactionTracer,
    trace_tx_assets,
    types::{
        TokenInfo, TraceResult, TransferRecord, ExecutionError,
        CallTrace, NATIVE_TOKEN_ADDRESS,
    },
};
pub use traits::{GetTransactionTracer, Reset, BlockEnvConfig};
