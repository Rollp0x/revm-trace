//! Utility functions for EVM interaction and analysis
//!
//! This module provides various utility functions for:
//! - ERC20 token interactions (balance checks, metadata retrieval)
//! - Proxy contract detection and analysis
//! - Common blockchain operations and data handling
//!
//! ## Modules
//!
//! - [`erc20_utils`]: Utilities for interacting with ERC20 tokens
//!   - Reading token balances
//!   - Getting token metadata (symbol, decimals)
//!   - Handling token transfers
//!
//! - [`proxy_utils`]: Utilities for working with proxy contracts
//!   - Detecting proxy patterns
//!   - Resolving implementation addresses
//!   - Handling proxy-specific operations

/// ERC20 token interaction utilities
pub mod erc20_utils;

/// Proxy contract analysis utilities
pub mod proxy_utils;
