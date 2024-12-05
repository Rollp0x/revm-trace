//! Enhanced EVM implementation with tracing capabilities
//!
//! This module provides a wrapper around revm's EVM implementation with additional
//! features for transaction tracing, token transfer tracking, and call analysis.
//!
//! # Features
//! - HTTP and WebSocket provider support
//! - Transaction tracing and analysis
//! - Token transfer tracking
//! - Call stack recording
//! - Customizable chain configuration
//!
//! # Example
//! ```no_run
//! use revm_trace::evm::create_evm;
//! # use anyhow::Result;
//!
//! # async fn example() -> Result<()> {
//! let mut evm = create_evm(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-api-key",
//!     Some(1),  // chain_id
//!     None,     // native token config
//! )?;
//!
//! // Use EVM for transaction simulation and analysis
//! # Ok(())
//! # }
//! ```

use std::any::Any;
use std::ops::{Deref, DerefMut};
use revm::{
    Evm,inspector_handle_register,
    db::{
        WrapDatabaseRef, AlloyDB,
        in_memory_db::CacheDB,
    }
};

use alloy::{
    eips::{BlockId,BlockNumberOrTag},
    primitives::{U256,Log},
    network::Ethereum,
    providers::{ProviderBuilder,Provider, RootProvider},
    pubsub::PubSubFrontend,
    transports::{
        Transport,
        http::{Client, Http},
        ws::WsConnect
    },
};
use anyhow::Result;
use crate::types::*;
use crate::inspector::TxInspector;

/// Type alias for HTTP client
pub type HttpClient = Http<Client>;

/// Type alias for HTTP provider
pub type HttpProvider = RootProvider<HttpClient>;

/// Type alias for EVM with transaction inspector
pub type InspectorEvm<'a, T, P> = Evm<'a, TxInspector, WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>>;

/// Enhanced EVM implementation with tracing capabilities
pub struct TraceEvm<'a, T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    evm: InspectorEvm<'a, T, P>,
    chain_id: u64,
    native_token_config: Option<TokenConfig>,
}

/// Possible errors when creating or using TraceEvm
#[derive(Debug, thiserror::Error)]
pub enum TraceEvmError {
    #[error("Invalid RPC URL: {0}")]
    InvalidRpcUrl(String),
    
    #[error("Database initialization failed: {0}")]
    DatabaseError(String),

    #[error("Runtime initialization failed: {0}")]
    RuntimeError(String),

    #[error("Connection to WS failed: {0}")]
    WsConnectionError(String),
}

/// Creates a new TraceEvm instance with HTTP transport
///
/// # Arguments
/// * `rpc_url` - HTTP RPC endpoint URL
/// * `chain_id` - Optional chain ID (e.g., 1 for Ethereum mainnet)
/// * `native_token_config` - Optional configuration for native token
///
/// # Returns
/// * `Ok(TraceEvm)` - Configured EVM instance
/// * `Err(TraceEvmError)` - If creation fails
pub fn create_evm(
    rpc_url: &str,
    chain_id: Option<u64>,
    native_token_config: Option<TokenConfig>,
) -> Result<TraceEvm<HttpClient, HttpProvider>, TraceEvmError> {
    // Create provider from RPC URL
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|e| 
            TraceEvmError::InvalidRpcUrl(format!("Failed to parse RPC URL: {}", e))
        )?);

    create_evm_internal(provider, chain_id, native_token_config)
}

/// Creates a new TraceEvm instance with WebSocket transport
#[cfg(feature = "ws")]
pub async fn create_evm_ws<'a>(
    ws_url: &str,
    chain_id: Option<u64>,
    native_token_config: Option<TokenConfig>,
) -> Result<TraceEvm<'a, PubSubFrontend, RootProvider<PubSubFrontend>>, TraceEvmError> {
    // Create provider directly in async context
    let provider = ProviderBuilder::new()
        .on_ws(WsConnect::new(ws_url))
        .await
        .map_err(|e| TraceEvmError::WsConnectionError(format!("Failed to connect to WS: {}", e)))?;

    create_evm_internal(provider, chain_id, native_token_config)
}

/// Internal function to create TraceEvm instance with any provider type
fn create_evm_internal<'a,T,P>(
    provider: P,
    chain_id: Option<u64>,
    native_token_config: Option<TokenConfig>,
) -> Result<TraceEvm<'a, T, P>, TraceEvmError> 
where
    T: Transport + Clone,
    P: Provider<T>,
{
    // Initialize AlloyDB with the provider
    let alloy_db = AlloyDB::new(provider, BlockId::latest())
        .ok_or_else(|| TraceEvmError::DatabaseError(
            "Failed to create AlloyDB...".into()
        ))?;

    // Create cached database and inspector
    let cached_db = CacheDB::new(alloy_db);
    let inspector = TxInspector::new();
    
    // Build EVM with custom configuration
    let mut evm = Evm::builder()
        .with_ref_db(cached_db)
        .with_external_context(inspector)
        .append_handler_register(inspector_handle_register)
        .build();

    // Configure EVM settings
    let cfg = evm.cfg_mut();
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    cfg.limit_contract_code_size = None;
    cfg.disable_base_fee = true;

    // Set chain ID if provided
    if let Some(chain_id) = chain_id {
        cfg.chain_id = chain_id;
        evm.tx_mut().chain_id = Some(chain_id);
    }
    
    Ok(TraceEvm { 
        evm, 
        native_token_config,
        chain_id: chain_id.unwrap_or(0)
    })
}

impl<'a, T, P> Deref for TraceEvm<'a, T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    type Target = InspectorEvm<'a, T, P>;

    fn deref(&self) -> &Self::Target {
        &self.evm
    }
}

impl<'a, T, P> DerefMut for TraceEvm<'a, T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.evm
    }
}

impl<'a, T, P> TraceEvm<'a, T, P>
where
    T: Transport + Clone,
    P: Provider<T>,
{
    /// Resets the transaction inspector to its initial state
    pub fn reset_inspector(&mut self) {
        if let Some(inspector) = (&mut self.evm.context.external as &mut dyn Any).downcast_mut::<TxInspector>() {
            inspector.reset();
        }
    }

    /// Resets the database cache while preserving the underlying provider
    ///
    /// Returns self for method chaining
    pub fn reset_db(&mut self) -> &mut Self {
        // Reset CacheDB state
        let cached_db = &mut self.db_mut().0;
        cached_db.accounts.clear();
        cached_db.contracts.clear();
        cached_db.logs = Vec::new();
        cached_db.block_hashes.clear();
        // No need to reset alloy db as it's stateless
        self
    }

    /// Sets the block environment parameters
    ///
    /// # Arguments
    /// * `block_env` - Block environment configuration
    ///
    /// Returns self for method chaining
    pub fn set_block_env(&mut self, block_env: BlockEnv) -> &mut Self {
        // Set block number
        self.block_mut().number = U256::from(block_env.number);
        // Set block timestamp
        self.block_mut().timestamp = U256::from(block_env.timestamp);
        // Update block number in alloy db
        self.db_mut().0.db.set_block_number(BlockId::Number(BlockNumberOrTag::Number(block_env.number)));
        self
    }

    /// Sets the block number for the current environment
    pub fn set_block_number(&mut self, block_number: u64) -> &mut Self {
        self.block_mut().number = U256::from(block_number);
        self.db_mut().0.db.set_block_number(BlockId::Number(BlockNumberOrTag::Number(block_number)));
        self
    }

    /// Sets the block timestamp for the current environment
    pub fn set_block_timestamp(&mut self, timestamp: u64) -> &mut Self {
        self.block_mut().timestamp = U256::from(timestamp);
        self
    }

    /// Returns a reference to the transaction inspector if available
    pub fn get_inspector(&self) -> Option<&TxInspector> {
        (&self.evm.context.external as &dyn Any).downcast_ref::<TxInspector>()
    }

    /// Returns the list of token transfers recorded during execution
    pub fn get_token_transfers(&mut self) -> Option<Vec<TokenTransfer>> {
        if let Some(inspector) = self.get_inspector() {
            Some(inspector.get_transfers().clone())
        } else {
            None
        }
    }

    /// Returns the call traces recorded during execution
    pub fn get_call_traces(&mut self) -> Option<Vec<CallTrace>> {
        if let Some(inspector) = self.get_inspector() {
            Some(inspector.get_traces().clone())
        } else {
            None
        }
    }

    /// Returns the logs generated during execution
    pub fn get_logs(&mut self) -> Option<Vec<Log>> {
        if let Some(inspector) = self.get_inspector() {
            Some(inspector.get_logs().clone())
        } else {
            None
        }
    }

    /// Returns the native token configuration if set
    pub fn get_native_token_config(&self) -> Option<&TokenConfig> {
        self.native_token_config.as_ref()
    }

    /// Returns the chain ID used by this EVM instance
    pub fn get_chain_id(&self) -> u64 {
        self.chain_id
    }
}