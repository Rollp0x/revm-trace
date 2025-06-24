use alloy::{
    network::AnyNetwork,
    providers::{
        RootProvider, 
        Identity,
        fillers::{FillProvider, JoinFill, GasFiller, BlobGasFiller, NonceFiller, ChainIdFiller},
    },
    primitives::{U256, Address, Bytes, TxKind},
};
use serde::Serialize;
pub use revm::{
    database::AlloyDB,
    interpreter::{CallScheme, CreateScheme},
    context::BlockEnv
};
use crate::MyWrapDatabaseAsync;

// ========================= Provider Type Definitions =========================
//
// These type aliases create a layered provider system using alloy's filler pattern.
// Fillers automatically populate transaction fields during execution.

/// Base filler layer that handles nonce and chain ID management
/// 
/// Combines:
/// - `NonceFiller`: Automatically sets transaction nonce from account state
/// - `ChainIdFiller`: Automatically sets chain ID from provider
type BaseFiller = JoinFill<NonceFiller, ChainIdFiller>;

/// Blob filler layer that adds EIP-4844 blob gas management
/// 
/// Extends BaseFiller with:
/// - `BlobGasFiller`: Handles blob gas pricing for blob transactions (EIP-4844)
type BlobFiller = JoinFill<BlobGasFiller, BaseFiller>;

/// Gas filler layer that adds general gas management
/// 
/// Extends BlobFiller with:
/// - `GasFiller`: Automatically estimates and sets gas limit and gas price
type GasFillers = JoinFill<GasFiller, BlobFiller>;

/// Complete filler stack with identity layer
/// 
/// Adds the identity filler on top of all gas management layers:
/// - `Identity`: Pass-through filler that preserves existing values
/// - Provides a complete transaction filling pipeline
type AllFillers = JoinFill<Identity, GasFillers>;

/// HTTP provider with automatic transaction filling
/// 
/// A fully configured HTTP provider that:
/// - Uses `AnyNetwork` for maximum blockchain compatibility
/// - Automatically fills transaction fields using `AllFillers`
/// - Provides type-safe access to Ethereum JSON-RPC methods
/// 
/// This is the primary provider type for single-threaded operations.
pub type AnyNetworkProvider = FillProvider<AllFillers, RootProvider<AnyNetwork>, AnyNetwork>;

pub type ArcAnyNetworkProvider = std::sync::Arc<AnyNetworkProvider>;

pub const NATIVE_TOKEN_ADDRESS: Address = Address::ZERO;

pub type AllDBType = MyWrapDatabaseAsync<AlloyDB<AnyNetwork,AnyNetworkProvider>>;


#[derive(Debug, Clone, Serialize, Default)]
pub struct TokenInfo {
    pub name: String,
    /// Token symbol (e.g., "ETH", "USDC")
    pub symbol: String,
    /// Number of decimal places for value formatting
    pub decimals: u8,
    /// Total supply of the token
    pub total_supply: U256,
}

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
