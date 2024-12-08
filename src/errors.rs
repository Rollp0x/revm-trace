//! Error types for EVM tracing and simulation
//! 
//! This module defines a comprehensive error handling system that covers:
//! - EVM initialization errors
//! - Runtime execution errors
//! - Token-related errors
//! - Error conversion and propagation

use thiserror::Error;

/// Top-level error type for the EVM tracing system
/// 
/// Encompasses all possible errors that can occur during EVM operations,
/// providing a unified error handling interface for users.
#[derive(Debug, Error)]
pub enum EvmError {
    /// Errors occurring during EVM initialization
    #[error("Failed to initialize EVM: {0}")]
    Init(#[from] InitError),

    /// Errors occurring during transaction execution
    #[error("Error during execution: {0}")]
    Runtime(#[from] RuntimeError),

    /// Errors related to token operations
    #[error("Token error: {0}")]
    Token(#[from] TokenError),
}

/// Initialization-specific errors
/// 
/// These errors occur during the setup phase of the EVM,
/// typically related to network connectivity and configuration.
#[derive(Debug, Error)]
pub enum InitError {
    /// Invalid or malformed RPC URL
    #[error("Invalid RPC URL: {0}")]
    InvalidRpcUrl(String),
    
    /// Database setup or connection errors
    #[error("Database initialization failed: {0}")]
    Database(String),
    
    /// WebSocket connection establishment errors
    #[error("WebSocket connection failed: {0}")]
    WsConnection(String),
    
    /// Chain ID retrieval or validation errors
    #[error("Failed to get chain ID: {0}")]
    ChainId(String),
}

/// Runtime execution errors
/// 
/// These errors occur during actual transaction execution,
/// including gas issues, reverts, and state access problems.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// General transaction execution failures
    #[error("Transaction execution failed: {0}")]
    ExecutionFailed(String),
    
    /// Errors accessing account information
    #[error("Account access error: {0}")]
    AccountAccess(String),

    /// Errors accessing storage slots
    #[error("Slot access error: {0}")]
    SlotAccess(String),
    
    /// Transaction ran out of gas
    #[error("Out of gas")]
    OutOfGas,
    
    /// Transaction explicitly reverted
    #[error("Reverted: {0}")]
    Revert(String),
}

/// Token-specific errors
/// 
/// These errors occur during ERC20 token operations,
/// including symbol and decimals queries, and general token calls.
#[derive(Debug, Error)]
pub enum TokenError {
    /// Failed to decode token symbol
    /// 
    /// # Fields
    /// * `address` - Token contract address
    /// * `reason` - Detailed error message
    #[error("Failed to decode token symbol for {address}: {reason}")]
    SymbolDecode {
        address: String,
        reason: String,
    },

    /// Failed to decode token decimals
    /// 
    /// # Fields
    /// * `address` - Token contract address
    /// * `reason` - Detailed error message
    #[error("Failed to decode token decimals for {address}: {reason}")]
    DecimalsDecode {
        address: String,
        reason: String,
    },

    /// General token query failures
    /// 
    /// # Fields
    /// * `address` - Token contract address
    /// * `reason` - Detailed error message
    #[error("Failed to query token {address}: {reason}")]
    QueryFailed {
        address: String,
        reason: String,
    },

    /// Token call reverted
    /// 
    /// # Fields
    /// * `address` - Token contract address
    #[error("Token call reverted for {address}")]
    CallReverted {
        address: String,
    },
} 