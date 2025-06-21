use revm::{
    context::{Context, BlockEnv}, 
    database::CacheDB, 
    inspector::NoOpInspector,
    handler::{MainBuilder,MainContext,MainnetContext}, 
};
use alloy::{
    eips::BlockId, network::AnyNetwork, providers::{
        Provider, ProviderBuilder, WsConnect
    },
};
use crate::{
    types::AnyNetworkProvider,
    errors::{EvmError,InitError},
    TraceEvm,
    TraceInspector
};
use foundry_fork_db::{backend::SharedBackend, BlockchainDb, cache::BlockchainDbMeta};
use std::sync::Arc;

pub type DefaultEvm = TraceEvm<CacheDB<SharedBackend>, NoOpInspector>;
pub type InspectorEvm<INSP> = TraceEvm<CacheDB<SharedBackend>, INSP>;

pub async fn get_http_provider(rpc_url:&str) -> Result<AnyNetworkProvider, EvmError> {
    let provider = if rpc_url.starts_with("http") {
        let url = rpc_url.parse()
        .map_err(|_| InitError::InvalidRpcUrl("Failed to parse RPC URL".to_string()))?;
        ProviderBuilder::new().network::<AnyNetwork>().connect_http(url)
    } else {
        let ws_connect = WsConnect::new(rpc_url);
        ProviderBuilder::new_with_network::<AnyNetwork>().connect_ws(ws_connect).await
        .map_err(|_| InitError::InvalidRpcUrl("Failed to connect to WebSocket".to_string()))?
    };
    Ok(provider)
}

async fn create_evm_internal<INSP>(
    rpc_url:&str,
    tracer: INSP
) -> Result<TraceEvm<CacheDB<SharedBackend>, INSP>, EvmError> {
    let provider = get_http_provider(rpc_url).await?;
    let chain_id = provider.get_chain_id().await
        .map_err(|_| InitError::ChainIdFetchError("Failed to fetch chain ID".to_string()))?;
    let meta = BlockchainDbMeta::new(BlockEnv::default(), rpc_url.to_string());
    let blockchain_db = BlockchainDb::new(meta, None); // None = use in-memory cache
    // Spawn a dedicated backend thread for handling database operations
    // This provides thread-safe access to blockchain state
    let shared_backend = SharedBackend::spawn_backend_thread(
        Arc::new(provider),
        blockchain_db,
        Some(BlockId::latest()) // Pin to the latest block
    );
    let cache_db = CacheDB::new(shared_backend);
    let mut ctx = Context::mainnet().with_db(cache_db);
    let cfg = &mut ctx.cfg;
    cfg.chain_id = chain_id;
    cfg.disable_eip3607 = true;
    cfg.limit_contract_code_size = None;
    cfg.disable_block_gas_limit = true;
    cfg.disable_base_fee = true;
    let evm = ctx.build_mainnet_with_inspector(tracer);
    Ok(TraceEvm::new(evm))
}


/// Create an EVM instance with HTTP provider (no tracer)
/// 
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::create_evm;
/// let evm = create_evm("https://eth.llamarpc.com").await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_evm(
    rpc_url: &str,
) -> Result<DefaultEvm, EvmError> {
    create_evm_internal(rpc_url, NoOpInspector).await
}

/// Create an EVM instance with WebSocket provider (no tracer)
/// 
/// # Example  
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::create_evm_ws;
/// let evm = create_evm_ws("wss://eth.llamarpc.com").await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_evm_ws(
    rpc_url: &str,
) -> Result<DefaultEvm, EvmError> {
    create_evm_internal(rpc_url, NoOpInspector).await
}

/// Create an EVM instance with HTTP provider and custom tracer
/// 
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::create_evm_with_tracer;
/// use revm_trace::TxInspector;
/// let tracer = TxInspector::new();
/// let evm = create_evm_with_tracer("https://eth.llamarpc.com", tracer).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_evm_with_tracer<INSP>(
    rpc_url: &str,
    tracer: INSP,
) -> Result<InspectorEvm<INSP>, EvmError>
where
    INSP: TraceInspector<MainnetContext<CacheDB<SharedBackend>>> + Clone,
{
    create_evm_internal(rpc_url, tracer).await
}

/// Create an EVM instance with WebSocket provider and custom tracer
/// 
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::create_evm_ws_with_tracer;
/// use revm_trace::TxInspector;
/// let tracer = TxInspector::new();
/// let evm = create_evm_ws_with_tracer("wss://mainnet.gateway.tenderly.co", tracer).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_evm_ws_with_tracer<INSP>(
    rpc_url: &str,
    tracer: INSP
) -> Result<InspectorEvm<INSP>, EvmError>
where 
    INSP: TraceInspector<MainnetContext<CacheDB<SharedBackend>>> + Clone,
{   
    create_evm_internal(rpc_url, tracer).await
}