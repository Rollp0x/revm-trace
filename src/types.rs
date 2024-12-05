//! Core types for EVM tracing and simulation
//!
//! This module defines the core data structures used throughout the tracing system:
//! - Token configurations and transfers
//! - Block and transaction environments
//! - Call traces and execution results
//! - Status tracking and error handling

use std::collections::HashMap;
pub use revm::interpreter::{CallScheme, CreateScheme, InstructionResult};
pub use alloy::primitives::{Address, U256, Bytes, Log, TxKind};
pub use revm::primitives::Output;
use serde::Serialize;

/// Default native token address (zero address)
pub const NATIVE_TOKEN_ADDRESS: Address = Address::ZERO;

/// Mapping of chain IDs to their native token configurations
pub type ChainConfigs = HashMap<u64, TokenConfig>;

/// Mapping of token addresses to their configurations
pub type TokenInfos = HashMap<Address, TokenConfig>;

/// Token configuration including symbol and decimals
#[derive(Debug, Clone,Serialize)]
pub struct TokenConfig {
    /// Token symbol (e.g., "ETH", "USDC")
    pub symbol: String,
    /// Number of decimal places
    pub decimals: u8,
}

/// Block environment parameters for transaction simulation
#[derive(Debug, Clone,Serialize)]
pub struct BlockEnv {
    /// Block number
    pub number: u64,
    /// Block timestamp (Unix timestamp)
    pub timestamp: u64,
}

/// EVM configuration parameters
#[derive(Debug, Clone)]
pub struct EvmConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Chain ID for the target network
    pub chain_id: u64,
    /// Optional chain-specific token configurations
    pub chain_configs: Option<ChainConfigs>,
}

/// Transaction parameters for simulation
#[derive(Debug, Clone)]
pub struct SimulationTx {
    /// Transaction sender
    pub caller: Address,
    /// Native token value to send
    pub value: U256,
    /// Transaction input data
    pub data: Bytes,
    /// Transaction target (address or contract creation)
    pub transact_to: TxKind,
}

/// Parameters for batch transaction simulation
#[derive(Debug, Clone)]
pub struct SimulationBatch {
    /// Block environment for the simulation
    pub block_env: BlockEnv,
    /// List of transactions to execute
    pub transactions: Vec<SimulationTx>,
    /// Whether transactions should be executed as multicall
    pub is_multicall: bool,
}

/// Record of a token transfer event
#[derive(Debug, Clone,Serialize)]
pub struct TokenTransfer {
    /// Token contract address (zero address for native token)
    pub token: Address,
    /// Sender address
    pub from: Address,
    /// Recipient address
    pub to: Address,
    /// Transfer amount
    pub value: U256,
}

impl TokenTransfer {
    /// Check if this transfer is for the native token
    pub fn is_native_token(&self) -> bool {
        self.token == NATIVE_TOKEN_ADDRESS
    }
}

/// Type of contract interaction
#[derive(Debug, Clone)]
pub enum CallType {
    /// Regular contract call
    Call,
    /// Contract creation
    Create,
}

/// Status of a contract call
#[derive(Debug, Clone,Serialize)]
pub enum CallStatus {
    /// Call completed successfully
    Success,
    /// Call reverted with reason
    Revert(String),
    /// Call halted due to error
    Halt(String),
    /// Fatal error occurred
    FatalError,
    /// Call is still in progress
    InProgress,
}

impl CallStatus {
    /// Check if the call was successful
    pub fn is_success(&self) -> bool {
        matches!(self, CallStatus::Success)
    }
}

/// Detailed trace of a contract call
#[derive(Debug, Clone,Serialize)]
pub struct CallTrace {
    /// Caller address
    pub from: Address,
    /// Target address
    pub to: Address,
    /// Native token value
    pub value: U256,
    /// Call input data
    pub input: Bytes,
    /// Call scheme if regular call
    pub call_scheme: Option<CallScheme>,
    /// Create scheme if contract creation
    pub create_scheme: Option<CreateScheme>,
    /// Gas used by this call
    pub gas_used: U256,
    /// Call output data
    pub output: Bytes,
    /// Call execution status
    pub status: CallStatus,
    /// Whether this call is the source of an error
    pub error_origin: bool,
    /// Nested calls made by this call
    pub subtraces: Vec<CallTrace>,
    /// Position in the call tree
    pub trace_address: Vec<usize>,
}

/// Overall execution status of a transaction
#[derive(Debug, Clone,Serialize)]
pub enum ExecutionStatus {
    /// Transaction executed successfully
    Success {
        /// Gas used by the transaction
        gas_used: u64,
        /// Gas refunded after execution
        gas_refunded: u64,
        /// Transaction output
        output: Output,
    },
    /// Transaction execution failed
    Failed {
        /// Type of failure
        kind: FailureKind,
        /// Gas used before failure
        gas_used: u64,
        /// Error output data if any
        output: Option<Bytes>,
    },
}

/// Types of execution failures
#[derive(Debug, Clone,Serialize)]
pub enum FailureKind {
    /// Errors before execution starts
    PreExecution(String),
    /// Transaction reverted with reason
    Revert(String),
    /// Execution halted due to error
    Halt(String),
}

/// Complete result of a transaction simulation
#[derive(Debug, Clone,Serialize)]
pub struct TraceResult {
    /// Block environment used
    pub block_env: BlockEnv,
    /// All asset transfers during execution
    pub asset_transfers: Vec<TokenTransfer>,
    /// Token metadata collected
    pub token_infos: TokenInfos,
    /// Execution call traces
    pub call_traces: Vec<CallTrace>,
    /// Emitted event logs
    pub logs: Vec<Log>,
    /// Final execution status
    pub status: ExecutionStatus,
}

impl TraceResult {
    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success { .. })
    }

    /// Get error message if execution failed
    pub fn get_error_message(&self) -> Option<String> {
        match &self.status {
            ExecutionStatus::Success { .. } => None,
            ExecutionStatus::Failed { kind, .. } => Some(match kind {
                FailureKind::PreExecution(evm_error) => format!("Pre-execution error: {}", evm_error),
                FailureKind::Revert(reason) => format!("Reverted: {}", reason),
                FailureKind::Halt(reason) => format!("Halted: {:?}", reason),
            }),
        }
    }

    /// Find the last error source in the call trace
    ///
    /// Performs a depth-first search through the call tree to find
    /// the most recent unhandled error
    pub fn find_error_trace(&self) -> Option<&CallTrace> {
        fn find_error_recursive(trace: &CallTrace) -> Option<&CallTrace> {
            let mut last_error = None;
            for subtrace in &trace.subtraces {
                if !subtrace.status.is_success() {
                    if let Some(error) = find_error_recursive(subtrace) {
                        last_error = Some(error);
                    }
                }
            }

            if trace.error_origin {
                Some(trace)
            } else {
                last_error
            }
        }

        let mut last_error = None;
        for trace in &self.call_traces {
            if !trace.status.is_success() {
                if let Some(error) = find_error_recursive(trace) {
                    last_error = Some(error);
                }
            }
        }
        last_error
    }

    /// Get total gas used
    pub fn get_gas_used(&self) -> u64 {
        match &self.status {
            ExecutionStatus::Success { gas_used, .. } => *gas_used,
            ExecutionStatus::Failed { gas_used, .. } => *gas_used,
        }
    }

    /// Get gas refunded (zero if failed)
    pub fn get_gas_refunded(&self) -> u64 {
        match &self.status {
            ExecutionStatus::Success { gas_refunded, .. } => *gas_refunded,
            ExecutionStatus::Failed { .. } => 0,
        }
    }

    /// Get output bytes if any
    pub fn get_output_bytes(&self) -> Option<Bytes> {
        match &self.status {
            ExecutionStatus::Success { output, .. } => Some(output.data().clone()),
            ExecutionStatus::Failed { output, .. } => output.clone(),
        }
    }
}

/// Get default native token configuration for known chains
pub fn get_default_native_token(chain_id: u64) -> TokenConfig {
    match chain_id {
        1 => TokenConfig { symbol: "ETH".into(), decimals: 18 },
        5 => TokenConfig { symbol: "GOERLI_ETH".into(), decimals: 18 },
        10 => TokenConfig { symbol: "OPT_ETH".into(), decimals: 18 },
        56 => TokenConfig { symbol: "BNB".into(), decimals: 18 },
        137 => TokenConfig { symbol: "MATIC".into(), decimals: 18 },
        42161 => TokenConfig { symbol: "ARB_ETH".into(), decimals: 18 },
        200901 => TokenConfig { symbol: "BTC".into(), decimals: 18 },
        // Default to ETH configuration for unknown chains
        _ => TokenConfig { symbol: "ETH".into(), decimals: 18 },
    }
}
#[derive(Debug,Serialize,Clone)]
pub enum TransactionStatus {
    /// Transaction succeeded completely without any errors
    Success,
    /// Transaction succeeded overall but contains internal errors
    PartialSuccess,
    /// Transaction failed
    Failed {
        /// Main error message of the transaction
        error: String,
        /// Original error source message if available
        origin_error: Option<String>,
    },
}

impl TraceResult {
    /// Get the execution status of the transaction
    pub fn execution_status(&self) -> TransactionStatus {
        match &self.status {
            ExecutionStatus::Failed { kind, .. } => {
                // Get main error message
                let error = format!("{:?}", kind);
                
                // Get original error message if exists
                let origin_error = self.find_error_trace()
                    .map(|trace| format!("{:?}", trace.status));

                TransactionStatus::Failed {
                    error,
                    origin_error,
                }
            }
            ExecutionStatus::Success { .. } => {
                let has_internal_errors = self.call_traces.iter()
                    .any(|trace| trace.error_origin || 
                         trace.subtraces.iter().any(|t| t.error_origin));
                
                if has_internal_errors {
                    TransactionStatus::PartialSuccess
                } else {
                    TransactionStatus::Success
                }
            }
        }
    }
}