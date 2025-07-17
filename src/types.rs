use crate::MyWrapDatabaseAsync;
use alloy::{
    network::AnyNetwork,
    primitives::{fixed_bytes, Address, Bytes, FixedBytes, Log, TxKind, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, RootProvider,
    },
};
pub use revm::{
    context::BlockEnv,
    database::AlloyDB,
    interpreter::{CallScheme, CreateScheme},
};
use serde::Serialize;

pub const ERC20_TRANSFER_EVENT_SIGNATURE: FixedBytes<32> =
    fixed_bytes!("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
pub const ERC1155_TRANSFER_BATCH_EVENT_SIGNATURE: FixedBytes<32> =
    fixed_bytes!("0x4a39dc06d4c0dbc64b70af90fd698a233a518aa5d07e595d983b8c0526c8f7fb");
pub const ERC1155_TRANSFER_SINGLE_EVENT_SIGNATURE: FixedBytes<32> =
    fixed_bytes!("0xc3d58168c5ae7397731d063d5bbf3d657854427343f4c083240f7aacaa2d0f62");

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

pub type AllDBType = MyWrapDatabaseAsync<AlloyDB<AnyNetwork, AnyNetworkProvider>>;

#[derive(Debug, Clone, Serialize)]
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

/// Type of token transfer (supports future extensibility)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum TokenType {
    Native,
    ERC20,
    ERC721,
    ERC1155,
    // More token types can be added in future
}

/// Record of a token transfer event
///
/// Captures all relevant information about a token transfer,
/// supporting both native (ETH) and ERC20/ERC721/ERC1155 tokens.
/// - For ERC20: `value` is the transfer amount, `id` is None.
/// - For ERC721: `value` is the tokenId, `id` is Some(tokenId).
/// - For ERC1155: `value` is the transfer amount, `id` is Some(tokenId).
/// - For native token: `value` is the amount, `id` is None.
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
    /// Type of token being transferred
    pub token_type: TokenType,
    /// ERC721/1155 id (Some for ERC721/ERC1155, None for ERC20/Native)
    pub id: Option<U256>,
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
#[derive(Debug, Clone, Serialize, Default)]
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
#[derive(Debug, Clone, Serialize, Default)]
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

impl TokenTransfer {
    /// Parses a token transfer log and returns a vector of TokenTransfer objects
    pub fn get_token_transfers(log: &Log) -> Vec<TokenTransfer> {
        let mut results = vec![];
        // erc20/erc721 transfer
        if log.topics()[0] == ERC20_TRANSFER_EVENT_SIGNATURE {
            if log.topics().len() == 3 {
                let from = Address::from_slice(&log.topics()[1].as_slice()[12..]);
                let to = Address::from_slice(&log.topics()[2].as_slice()[12..]);
                let data = &log.data.data;
                let amount = U256::from_be_slice(data);
                if !amount.is_zero() {
                    results.push(TokenTransfer {
                        token: log.address,
                        from,
                        to: Some(to),
                        value: amount,
                        token_type: TokenType::ERC20,
                        id: None,
                    });
                }
            } else if log.topics().len() == 4 {
                let from = Address::from_slice(&log.topics()[1].as_slice()[12..]);
                let to = Address::from_slice(&log.topics()[2].as_slice()[12..]);
                let id = U256::from_be_slice(log.topics()[3].as_slice());
                let amount = U256::from(1);
                results.push(TokenTransfer {
                    token: log.address,
                    from,
                    to: Some(to),
                    value: amount,
                    token_type: TokenType::ERC721,
                    id: Some(id),
                });
            }
        } else if log.topics()[0] == ERC1155_TRANSFER_BATCH_EVENT_SIGNATURE
            && log.topics().len() == 4
        {
            let data = &log.data.data;
            if data.len() >= 96 {
                let from = Address::from_slice(&log.topics()[2].as_slice()[12..]);
                let to = Address::from_slice(&log.topics()[3].as_slice()[12..]);
                let ids_len = U256::from_be_slice(&data[64..96]).to::<usize>();
                let mut ids = Vec::with_capacity(ids_len);
                let mut offset = 96;
                for _ in 0..ids_len {
                    ids.push(U256::from_be_slice(&data[offset..offset + 32]));
                    offset += 32;
                }
                // 解析 values
                let values_len = U256::from_be_slice(&data[offset..offset + 32]).to::<usize>();
                offset += 32;
                let mut values = Vec::with_capacity(values_len);
                for _ in 0..values_len {
                    values.push(U256::from_be_slice(&data[offset..offset + 32]));
                    offset += 32;
                }
                // 匹配 ids 和 values
                for (id, value) in ids.into_iter().zip(values.into_iter()) {
                    results.push(TokenTransfer {
                        token: log.address,
                        from,
                        to: Some(to),
                        value,
                        token_type: TokenType::ERC1155,
                        id: Some(id),
                    });
                }
            }
        } else if log.topics()[0] == ERC1155_TRANSFER_SINGLE_EVENT_SIGNATURE
            && log.topics().len() == 4
        {
            let data = &log.data.data;
            if data.len() >= 64 {
                let from = Address::from_slice(&log.topics()[2].as_slice()[12..]);
                let to = Address::from_slice(&log.topics()[3].as_slice()[12..]);
                let id = U256::from_be_slice(&data[..32]);
                let value = U256::from_be_slice(&data[32..64]);
                results.push(TokenTransfer {
                    token: log.address,
                    from,
                    to: Some(to),
                    value,
                    token_type: TokenType::ERC1155,
                    id: Some(id),
                });
            }
        }
        results
    }
}
