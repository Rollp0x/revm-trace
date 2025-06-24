//! Utility modules for EVM operations and blockchain interactions
//!
//! This module collection provides essential utilities for:
//! - **ERC20 tokens**: Balance queries and metadata retrieval
//! - **Account balances**: Native token balance queries
//! - **Error handling**: Transaction error parsing and analysis
//! - **Proxy contracts**: Implementation resolution and detection
//! - **Multicall operations**: Batch contract call execution

pub mod erc20_utils;
pub mod balance_utils;
pub mod error_utils;
pub mod proxy_utils;
pub mod multicall_utils;