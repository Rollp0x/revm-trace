//! Utility functions for Ethereum smart contract analysis
//!
//! This module provides specialized utilities for analyzing and interacting with
//! different types of Ethereum smart contracts.
//!
//! # Modules
//!
//! - [`erc20_utils`]: ERC20 token interaction utilities
//!   - Metadata retrieval (symbol, decimals)
//!   - Parsing ERC20 Transfer events
//!
//! - [`proxy_utils`]: Proxy contract analysis
//!   - Implementation contract resolution
//!   - Common proxy pattern detection (EIP-1967, OpenZeppelin)
//!   - Storage slot analysis
//!
//! - [`error_utils`]: Smart contract error handling
//!   - Custom error parsing
//!   - Revert reason extraction
//!   - Solidity panic code interpretation
//!
//! # Example
//!
//! ```no_run
//! use revm_trace::{
//!     evm::create_evm_with_inspector,
//!     utils::{erc20_utils, proxy_utils},
//!     inspectors::TxInspector,
//! };
//! use alloy::primitives::address;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Initialize EVM with inspector
//! let mut evm = create_evm_with_inspector(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-api-key",
//!     TxInspector::new(),
//! ).await?;
//!
//! // Check if contract is a proxy
//! let contract = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
//! if let Some(impl_addr) = proxy_utils::resolve_implementation(&mut evm, contract)? {
//!     // Get token metadata if it's an ERC20
//!     if let Ok(symbol) = erc20_utils::get_symbol(&mut evm, contract) {
//!         println!("Token {} implementation: {}", symbol, impl_addr);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

/// ERC20 token interaction utilities
pub mod erc20_utils;

/// Error parsing utilities
pub mod error_utils;

/// Proxy contract analysis utilities
pub mod proxy_utils;

/// Balance calculation utilities
pub mod balance_utils;

/// Multicall utilities for batch contract calls
pub mod multicall_utils;