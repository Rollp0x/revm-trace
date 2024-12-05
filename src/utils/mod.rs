//! Utility functions for Ethereum smart contract analysis
//!
//! This module provides specialized utilities for analyzing and interacting with
//! different types of Ethereum smart contracts:
//!
//! # Modules
//!
//! - [`erc20_utils`]: ERC20 token interaction utilities
//!   - Token balance queries
//!   - Metadata retrieval (symbol, decimals)
//!   - Standard ERC20 function calls
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
//! use revm_trace::utils::{erc20_utils, proxy_utils};
//! # use alloy::primitives::address;
//! # async fn example() -> anyhow::Result<()> {
//! # let mut evm = todo!();
//!
//! // Check if contract is a proxy
//! let contract = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
//! if let Some(impl_addr) = proxy_utils::get_implement(&mut evm, contract).await? {
//!     // Get token metadata if it's an ERC20
//!     let symbol = erc20_utils::get_token_symbol(&mut evm, contract)?;
//!     println!("Token {} implementation: {}", symbol, impl_addr);
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