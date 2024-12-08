//! EVM instance creation and initialization
//! 
//! Provides functions for creating TraceEvm instances with:
//! - HTTP providers
//! - WebSocket providers
//! - Custom inspectors
//! - Default configuration

use super::TraceEvm;
use crate::errors::{EvmError, InitError};
use crate::traits::{TraceInspector, GetInspector};
use crate::types::*;
use crate::inspectors::NoOpInspector;

use revm::{
    Evm, inspector_handle_register,
    db::{WrapDatabaseRef, AlloyDB, in_memory_db::CacheDB},
};
use alloy::{
    eips::BlockId,
    network::Ethereum,
    pubsub::PubSubFrontend, 
    providers::{ProviderBuilder, Provider, RootProvider},
    transports::{Transport, ws::WsConnect},
};

/// Creates a TraceEvm instance with custom provider and inspector
/// 
/// # Arguments
/// * `provider` - Network provider implementation
/// * `chain_id` - Target chain identifier
/// * `inspector` - Custom inspector implementation
/// 
/// # Returns
/// * `Result<TraceEvm, InitError>` - Configured EVM instance or error
/// 
/// # Type Parameters
/// * `T` - Transport type
/// * `P` - Provider type
/// * `I` - Inspector type implementing required traits
fn create_evm_internal<'a, T, P, I>(
    provider: P,
    chain_id: u64,
    inspector: I,
) -> Result<TraceEvm<'a, T, P, I>, InitError> 
where
    T: Transport + Clone,
    P: Provider<T>,
    I: TraceInspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + 
       GetInspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>>,
{   
    // Initialize database with provider
    let alloy_db = AlloyDB::new(provider, BlockId::latest())
        .ok_or_else(|| InitError::Database("Failed to create AlloyDB...".into()))?;
    let cached_db = CacheDB::new(alloy_db);
    
    // Configure and build EVM
    let mut evm = Evm::builder()
        .with_ref_db(cached_db)
        .with_external_context(inspector)
        .append_handler_register(inspector_handle_register)
        .build();

    // Apply default settings
    let cfg = evm.cfg_mut();
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    cfg.limit_contract_code_size = None;
    cfg.disable_base_fee = true;
    cfg.chain_id = chain_id;
    evm.tx_mut().chain_id = Some(chain_id);
    
    Ok(TraceEvm(evm))
}

/// Creates a TraceEvm instance with HTTP provider and default inspector
/// 
/// # Arguments
/// * `rpc_url` - HTTP RPC endpoint URL
/// 
/// # Returns
/// * `Result<TraceEvm, EvmError>` - Configured EVM instance or error
pub async fn create_evm(
    rpc_url: &str,
) -> Result<TraceEvm<'_, HttpClient, HttpProvider, NoOpInspector>, EvmError> {
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|e| 
            InitError::InvalidRpcUrl(format!("Failed to parse RPC URL: {}", e))
        )?);
    let chain_id = provider.get_chain_id().await
        .map_err(|e| InitError::ChainId(format!("Failed to get chain ID: {}", e)))?;
    Ok(create_evm_internal(provider, chain_id, NoOpInspector)?)
}

/// Creates a TraceEvm instance with HTTP provider and custom inspector
/// 
/// # Arguments
/// * `rpc_url` - HTTP RPC endpoint URL
/// * `inspector` - Custom inspector implementation
/// 
/// # Returns
/// * `Result<TraceEvm, EvmError>` - Configured EVM instance or error
/// 
/// # Type Parameters
/// * `I` - Inspector type implementing required traits
pub async fn create_evm_with_inspector<'a, I>(
    rpc_url: &str,
    inspector: I,
) -> Result<TraceEvm<'a, HttpClient, HttpProvider, I>, EvmError> 
where
    I: 'a + TraceInspector<WrapDatabaseRef<CacheDB<AlloyDB<HttpClient, Ethereum, HttpProvider>>>> + 
       GetInspector<WrapDatabaseRef<CacheDB<AlloyDB<HttpClient, Ethereum, HttpProvider>>>>,
{
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse().map_err(|e| 
            InitError::InvalidRpcUrl(format!("Failed to parse RPC URL: {}", e))
        )?);
    let chain_id = provider.get_chain_id().await
        .map_err(|e| InitError::ChainId(format!("Failed to get chain ID: {}", e)))?;
    Ok(create_evm_internal(provider, chain_id, inspector)?)
}

/// Creates a TraceEvm instance with WebSocket provider and custom inspector
/// 
/// # Arguments
/// * `ws_url` - WebSocket endpoint URL
/// * `inspector` - Custom inspector implementation
/// 
/// # Returns
/// * `Result<TraceEvm, EvmError>` - Configured EVM instance or error
/// 
/// # Type Parameters
/// * `I` - Inspector type implementing required traits
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
    let chain_id = provider.get_chain_id().await
        .map_err(|e| InitError::ChainId(format!("Failed to get chain ID: {}", e)))?;
    Ok(create_evm_internal(provider, chain_id, inspector)?)
}