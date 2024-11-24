//! # REVM Transaction Simulator and Asset Tracer
//!
//! A library for simulating EVM transactions and tracking asset transfers across multiple
//! EVM-compatible chains. Provides comprehensive transaction analysis and asset tracking
//! capabilities using REVM.
//!
//! ## Core Features
//!
//! - **Transaction Simulation**
//!   - Full EVM execution environment
//!   - Support for multiple EVM-compatible chains
//!   - Historical block state access
//!   - Custom inspector integration
//!
//! - **Asset Tracking**
//!   - Native token transfers (ETH/BNB/MATIC)
//!   - ERC20 token transfers
//!   - Token metadata collection
//!   - Transfer chronological ordering
//!
//! - **Contract Analysis**
//!   - Proxy contract detection
//!   - Implementation address resolution
//!   - Call trace recording
//!   - Error tracking and analysis
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use revm_trace::{
//!     trace::trace_tx_assets,
//!     trace::TransactionTracer,
//!     evm::create_evm_instance_with_inspector
//! };
//! use alloy::primitives::{address, U256};
//!
//! async fn example() -> anyhow::Result<()> {
//!     // Initialize EVM with transaction tracer
//!     let mut evm = create_evm_instance_with_inspector(
//!         "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY",
//!         TransactionTracer::default(),
//!         Some(17_000_000), // Specific block number
//!     )?;
//!     
//!     // Setup transaction parameters
//!     let from = address!("dead00000000000000000000000000000000beef");
//!     let to = address!("dac17f958d2ee523a2206206994597c13d831ec7"); // USDT
//!     let value = U256::from(1000000000000000000u64); // 1 ETH
//!     let data = vec![]; // Empty calldata
//!     let native_token = "ETH"; // Use "BNB" for BSC, "MATIC" for Polygon
//!     
//!     // Simulate transaction and analyze results
//!     let result = trace_tx_assets(&mut evm, from, to, value, data, native_token).await;
//!     
//!     // Process results
//!     for transfer in &result.asset_transfers {
//!         let token_info = result.token_info.get(&transfer.token)
//!             .expect("Token info should exist");
//!         println!(
//!             "Transfer: {} {} from {} to {}",
//!             transfer.value, token_info.symbol,
//!             transfer.from, transfer.to
//!         );
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
//! - [`trace`]: Transaction tracing and asset tracking
//! - [`utils`]: Helper functions for token and proxy interactions
//!
//! ## Common Types
//!
//! - [`TraceResult`]: Comprehensive transaction analysis results
//! - [`TransferRecord`]: Individual asset transfer record
//! - [`TokenInfo`]: Token metadata (symbol and decimals)
//! - [`TransactionTracer`]: Core transaction tracking inspector
//!
//! ## Supported Networks
//!
//! Works with any EVM-compatible network:
//! - Ethereum (ETH)
//! - BNB Smart Chain (BNB)
//! - Polygon (MATIC)
//! - Avalanche (AVAX)
//! - And more...

pub mod evm;
pub mod trace;
pub mod utils;

// Re-export commonly used types and functions for convenience
pub use evm::{create_evm_instance, create_evm_instance_with_inspector};
pub use trace::types::{TokenInfo, TraceResult, TransferRecord, NATIVE_TOKEN_ADDRESS};
pub use trace::{inspector::TransactionTracer, trace_tx_assets};
