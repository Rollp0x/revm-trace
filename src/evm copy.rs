//! Enhanced EVM implementation with tracing capabilities
//!
//! This module provides a wrapper around revm's EVM implementation with additional
//! features for transaction tracing, token transfer tracking, and call analysis.
//!
//! # Features
//! - HTTP and WebSocket provider support
//! - Transaction simulation and tracing
//! - Token transfer tracking
//! - Execution state management
//! - Customizable chain configuration
//!
//! The implementation focuses on providing a clean interface for transaction simulation
//! while maintaining detailed execution traces and transfer records.
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

use std::ops::{Deref, DerefMut};
pub use revm::primitives::ExecutionResult;
use revm::{
    Evm,inspector_handle_register,
    db::{
        WrapDatabaseRef, AlloyDB,
        in_memory_db::CacheDB,
    }
};
use anyhow::Result;
pub use revm::GetInspector;

use alloy::{
    eips::{BlockId,BlockNumberOrTag},
    primitives::U256,
    network::Ethereum,
    pubsub::PubSubFrontend, 
    providers::{ProviderBuilder,Provider, RootProvider},
    transports::{
        Transport,
        ws::WsConnect,
        http::{Client, Http},
    },
};
use crate::types::*;
use crate::traits::{Reset, TraceInspector,TraceOutput};
use crate::inspectors::NoOpInspector;

/// Type alias for HTTP client
type HttpClient = Http<Client>;

/// Type alias for HTTP provider
type HttpProvider = RootProvider<HttpClient>;

/// Type alias for EVM with transaction inspector
type InspectorEvm<'a, T, P,I> = Evm<'a, I, WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>>;

/// Type alias for database with inspector
type InspectorDB<T, P> = WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>;

use crate::errors::{EvmError, InitError, RuntimeError};

/// Enhanced EVM implementation with tracing capabilities
///
/// Provides functionality for transaction simulation with detailed tracing
/// of execution steps, token transfers, and state changes.
pub struct TraceEvm<'a, T, P,I>(InspectorEvm<'a, T, P,I>)
where
    T: Transport + Clone,
    P: Provider<T>;


/// Internal function to create TraceEvm instance with any provider type
fn create_evm_internal<'a,T,P,I>(
    provider: P,
    chain_id: u64,
    inspector: I,
) -> Result<TraceEvm<'a, T, P,I>, InitError> 
where
    T: Transport + Clone,
    P: Provider<T>,
    I: TraceInspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + 
       GetInspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>>,
{   
    // Initialize AlloyDB with the provider
    let alloy_db = AlloyDB::new(provider, BlockId::latest())
        .ok_or_else(|| InitError::Database(
            "Failed to create AlloyDB...".into()
        ))?;
    // Create cached database and inspector
    let cached_db = CacheDB::new(alloy_db);
    
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
    cfg.chain_id = chain_id;
    evm.tx_mut().chain_id = Some(chain_id);

    
    Ok(TraceEvm(evm))
}

pub async fn create_evm(
    rpc_url: &str,
) -> Result<TraceEvm<'_, HttpClient, HttpProvider,NoOpInspector>, EvmError> {
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|e| 
            InitError::InvalidRpcUrl(format!("Failed to parse RPC URL: {}", e))
        )?);
    let chain_id = provider.get_chain_id().await.map_err(|e| 
        InitError::ChainId(format!("Failed to get chain ID: {}", e))
    )?;
    Ok(create_evm_internal(provider, chain_id, NoOpInspector)?)
}

pub async fn create_evm_with_inspector<'a, I>(
    rpc_url: &str,
    inspector: I,
) -> Result<TraceEvm<'a, HttpClient, HttpProvider,I>, EvmError> 
where
    I:'a + TraceInspector<WrapDatabaseRef<CacheDB<AlloyDB<HttpClient, Ethereum, HttpProvider>>>> + 
       GetInspector<WrapDatabaseRef<CacheDB<AlloyDB<HttpClient, Ethereum, HttpProvider>>>>,
{
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|e| 
            InitError::InvalidRpcUrl(format!("Failed to parse RPC URL: {}", e))
        )?);
    let chain_id = provider.get_chain_id().await.map_err(|e| 
        InitError::ChainId(format!("Failed to get chain ID: {}", e))
    )?;
    Ok(create_evm_internal(provider, chain_id, inspector)?)
}


pub async fn create_evm_ws<'a, I>(
    ws_url: &str,
    inspector: I,   
) -> Result<TraceEvm<'a, PubSubFrontend, RootProvider<PubSubFrontend>, I>, EvmError> 
where
    I: 'a + 
       TraceInspector<WrapDatabaseRef<CacheDB<AlloyDB<PubSubFrontend, Ethereum, RootProvider<PubSubFrontend>>>>> + 
       GetInspector<WrapDatabaseRef<CacheDB<AlloyDB<PubSubFrontend, Ethereum, RootProvider<PubSubFrontend>>>>>,
{
    let provider = ProviderBuilder::new()
        .on_ws(WsConnect::new(ws_url))
        .await
        .map_err(|e| InitError::WsConnection(format!("Failed to connect to WS: {}", e)))?;
    let chain_id = provider.get_chain_id().await.map_err(|e| 
        InitError::ChainId(format!("Failed to get chain ID: {}", e))
    )?;
    Ok(create_evm_internal(provider, chain_id, inspector)?)
}


impl<'a, T, P,I> Deref for TraceEvm<'a, T, P,I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a,
{
    type Target = InspectorEvm<'a, T, P,I>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T, P,I> DerefMut for TraceEvm<'a, T, P,I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T, P,I> TraceEvm<'a, T, P,I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a + GetInspector<InspectorDB<T, P>>,
{

    /// 重置 inspector 状态
    pub fn reset_inspector(&mut self) -> &mut Self
    where
        I: Reset,
    {
        // 直接通过 trait object 调用
        self.0.context.external.reset();
        self
    }

    /// 获取 inspector 的输出结果
    pub fn get_inspector_output(&mut self) -> I::Output
    where
        I: TraceOutput,
    {
        // 直接通过 trait object 获取输出
        self.0.context.external.get_output()
    }

    /// 处理单个交易并获取结果
    fn process_transaction_internal(&mut self, input: SimulationTx) -> Result<(ExecutionResult, I::Output),RuntimeError>
    where
        I: Reset + TraceOutput,
    {   
        // 重置 inspector 状态
        self.reset_inspector();
        // Set transaction parameters
        let tx = self.tx_mut();
        tx.caller = input.caller;
        tx.transact_to = input.transact_to;
        tx.value = input.value;
        tx.data = input.data;
        
        // Execute transaction
        let execution_result = self.transact_commit().map_err(|e| RuntimeError::ExecutionFailed(e.to_string()))?;  // 这里的错误是真正的执行错误
        
        // 获取 inspector 输出
        let inspector_output = self.get_inspector_output();
        
        // 返回执行结果和 inspector 输出
        Ok((execution_result, inspector_output))
    }

    /// 批量处理交易
    pub fn process_transactions(&mut self, batch: SimulationBatch) -> Result<Vec<(ExecutionResult,I::Output)>,EvmError>
    where
        I: Reset + TraceOutput,
    {   
        let SimulationBatch { block_env, transactions, is_stateful } = batch;
        let mut results = Vec::new();
        self.set_block_env(block_env);
        
        for input in transactions {
            let exec_result = self.process_transaction_internal(input)?;
            results.push(exec_result);
            
            // For independent transactions, reset state after each tx
            if !is_stateful {
                self.reset_db().reset_inspector();
            }
        }
        
        Ok(results)
    }

    /// Resets the database cache while preserving the underlying provider
    ///
    /// This is automatically called after each independent transaction in batch processing,
    /// but can be manually called if needed for custom implementations.
    ///
    /// Returns self for method chaining
    pub fn reset_db(&mut self) -> &mut Self {
        // Reset CacheDB state
        let cached_db = &mut self.db_mut().0;
        cached_db.accounts.clear();
        cached_db.contracts.clear();
        cached_db.logs = Vec::new();
        cached_db.block_hashes.clear();
        self
    }

    /// Sets the block environment parameters
    ///
    /// Updates block number, timestamp, and database block reference.
    ///
    /// # Arguments
    /// * `block_env` - Block environment configuration
    ///
    /// Returns self for method chaining
    pub fn set_block_env(&mut self, block_env: BlockEnv) -> &mut Self {
        self.block_mut().number = U256::from(block_env.number);
        self.block_mut().timestamp = U256::from(block_env.timestamp);
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

}

