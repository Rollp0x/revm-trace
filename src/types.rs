//! Core types for EVM tracing and simulation
//!
//! This module defines the core data structures used throughout the tracing system:
//! - Token configurations and transfers
//! - Block and transaction environments
//! - Call traces and execution results
//! - Status tracking and error handling

/// Re-exports from revm and alloy for user convenience
pub use revm::interpreter::{CallScheme, CreateScheme, InstructionResult};
pub use alloy::primitives::{Address, U256, Bytes, Log, TxKind};
pub use revm::primitives::Output;
use alloy::{
    network::Ethereum,
    providers::{Provider, RootProvider},
    transports::{Transport, http::{Client, Http}},
    pubsub::PubSubFrontend,
};
use revm::{
    Evm,
    db::{WrapDatabaseRef, AlloyDB, in_memory_db::CacheDB},
};

// Provider types
pub type HttpClient = Http<Client>;
pub type HttpProvider = RootProvider<HttpClient>;

// Database types
pub type AlloyDBType<T, P> = AlloyDB<T, Ethereum, P>;
pub type CacheDBType<T, P> = CacheDB<AlloyDBType<T, P>>;
pub type InspectorDB<T, P> = WrapDatabaseRef<CacheDBType<T, P>>;

// EVM types
pub type InspectorEvm<'a, T, P, I> = Evm<'a, I, InspectorDB<T, P>>;

// ... other existing types ...
use serde::Serialize;

/// Default native token (ETH) address - the zero address
pub const NATIVE_TOKEN_ADDRESS: Address = Address::ZERO;

/// Token configuration information
/// 
/// Stores essential information about a token including its symbol
/// and decimal places for proper value formatting.
#[derive(Debug, Clone, Serialize, Default)]
pub struct TokenInfo {
    /// Token symbol (e.g., "ETH", "USDC")
    pub symbol: String,
    /// Number of decimal places for value formatting
    pub decimals: u8,
}

/// Block environment for transaction simulation
/// 
/// Contains the necessary block context parameters required
/// for accurate transaction simulation.
#[derive(Debug, Clone, Serialize)]
pub struct BlockEnv {
    /// Block number for the simulation context
    pub number: u64,
    /// Block timestamp in Unix format
    pub timestamp: u64,
}

/// Transaction parameters for simulation
/// 
/// Defines all necessary parameters to simulate a transaction
/// in the EVM environment.
#[derive(Debug, Clone)]
pub struct SimulationTx {
    /// Address initiating the transaction
    pub caller: Address,
    /// Amount of native token (ETH) to send
    pub value: U256,
    /// Transaction calldata
    pub data: Bytes,
    /// Transaction target (address for calls, None for creation)
    pub transact_to: TxKind,
}

/// Batch transaction simulation parameters
/// 
/// Allows execution of multiple transactions in sequence with
/// configurable state handling between transactions.
#[derive(Debug, Clone)]
pub struct SimulationBatch {
    /// Block environment for all transactions in the batch
    pub block_env: BlockEnv,
    /// Sequence of transactions to execute
    pub transactions: Vec<SimulationTx>,
    /// Whether to preserve state between transactions
    /// 
    /// Set to true when:
    /// - Deploying then interacting with contracts
    /// - Executing dependent transactions
    /// - State changes should affect subsequent transactions
    /// 
    /// Set to false when:
    /// - Simulating independent scenarios
    /// - Comparing different outcomes from same starting state
    pub is_stateful: bool,
}

/// Record of a token transfer event
/// 
/// Captures all relevant information about a token transfer,
/// supporting both native (ETH) and ERC20 tokens.
#[derive(Debug, Clone, Serialize)]
pub struct TokenTransfer {
    /// Token address (NATIVE_TOKEN_ADDRESS for ETH)
    pub token: Address,
    /// Transfer sender
    pub from: Address,
    /// Transfer recipient (None if contract creation failed)
    pub to: Option<Address>,
    /// Transfer amount in token's smallest unit
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
#[derive(Debug, Clone,Serialize,Default)]
pub enum CallStatus {
    /// Call completed successfully
    #[default]
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
#[derive(Debug, Clone,Serialize,Default)]
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



