use revm::{
    context::{BlockEnv, Context}, context_interface::ContextTr, database::{CacheDB, Database}, handler::{MainBuilder,MainContext,MainnetContext}, inspector::NoOpInspector, ExecuteEvm 
};
use alloy::{
    eips::{BlockId,BlockNumberOrTag}, network::AnyNetwork, providers::{
        Provider, ProviderBuilder, WsConnect
    }
};
use crate::{
    ResetDB,
    types::AnyNetworkProvider,
    errors::{EvmError,InitError},
    TraceEvm,
    TraceInspector,utils::block_utils::get_block_env,
};
use foundry_fork_db::{backend::SharedBackend, BlockchainDb, cache::BlockchainDbMeta};
use std::sync::Arc;

pub type DefaultEvm = TraceEvm<CacheDB<SharedBackend>, NoOpInspector>;
pub type InspectorEvm<INSP> = TraceEvm<CacheDB<SharedBackend>, INSP>;

/// Internal function to create a provider with automatic protocol detection
/// 
/// Automatically detects the connection type based on the URL scheme and creates
/// the appropriate provider:
/// - URLs starting with `http://` or `https://` → HTTP provider
/// - URLs starting with `ws://` or `wss://` → WebSocket provider
/// - Other URL schemes → Error
/// 
/// # Arguments
/// - `rpc_url`: RPC endpoint URL with protocol scheme
/// 
/// # Returns
/// - `Ok(AnyNetworkProvider)`: Successfully created provider
/// - `Err(EvmError)`: Failed to parse URL, unsupported scheme, or connection failure
/// 
/// # Error Cases
/// - Invalid URL format
/// - Unsupported protocol scheme
/// - Network connection failure
/// - WebSocket handshake failure
async fn get_provider(rpc_url:&str) -> Result<AnyNetworkProvider, EvmError> {
    let provider = if rpc_url.starts_with("http") || rpc_url.starts_with("https") {
        let url = rpc_url.parse()
        .map_err(|_| InitError::InvalidRpcUrl("Failed to parse RPC URL".to_string()))?;
        ProviderBuilder::new().network::<AnyNetwork>().connect_http(url)
    } else if rpc_url.starts_with("ws") || rpc_url.starts_with("wss") {
        let ws_connect = WsConnect::new(rpc_url);
        ProviderBuilder::new_with_network::<AnyNetwork>().connect_ws(ws_connect).await
        .map_err(|_| InitError::InvalidRpcUrl("Failed to connect to WebSocket".to_string()))?
    } else {
        return Err(EvmError::Init(InitError::InvalidRpcUrl(
            "Unsupported RPC URL scheme".to_string(),
        )));
    };
    Ok(provider)
}

/// Internal function to create EVM instances with any inspector type
/// 
/// This is the core implementation used by both `create_evm` and `create_evm_with_tracer`.
/// It handles the complete EVM setup process:
/// 
/// 1. **Provider Creation**: Automatic HTTP/WebSocket detection and connection
/// 2. **Chain Configuration**: Fetches chain ID and configures EVM settings
/// 3. **Database Setup**: Creates thread-safe blockchain database with caching
/// 4. **EVM Construction**: Builds mainnet-compatible EVM with the provided inspector
/// 
/// # EVM Configuration
/// - Disables EIP-3607 (account existence check)
/// - Removes contract code size limits
/// - Disables block gas limits for simulation
/// - Disables base fee checks
/// - Pins to latest block for consistent state
/// 
/// # Type Parameters
/// - `INSP`: Inspector type for transaction analysis
/// 
/// # Arguments
/// - `rpc_url`: RPC endpoint URL (HTTP or WebSocket)
/// - `tracer`: Inspector instance for transaction monitoring
async fn create_evm_internal<INSP>(
    rpc_url:&str,
    tracer: INSP
) -> Result<TraceEvm<CacheDB<SharedBackend>, INSP>, EvmError> {
    let provider = get_provider(rpc_url).await?;
    let chain_id = provider.get_chain_id().await
        .map_err(|_| InitError::ChainIdFetchError("Failed to fetch chain ID".to_string()))?;
    let meta = BlockchainDbMeta::new(BlockEnv::default(), rpc_url.to_string());
    let blockchain_db = BlockchainDb::new(meta, None); // None = use in-memory cache
    // Spawn a dedicated backend thread for handling database operations
    // This provides thread-safe access to blockchain state
    // let block_env = get_block_env(19416560, 1600000000);
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
    // cfg.disable_balance_check = true;
    let evm = ctx.build_mainnet_with_inspector(tracer);
    Ok(TraceEvm::new(evm))
}


/// Create an EVM instance (no tracer)
/// 
/// Supports both HTTP and WebSocket RPC endpoints. The connection type is
/// automatically detected based on the URL scheme:
/// - `http://` or `https://` → HTTP provider
/// - `ws://` or `wss://` → WebSocket provider
/// 
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::create_evm;
/// 
/// // HTTP provider
/// let evm_http = create_evm("https://eth.llamarpc.com").await?;
/// 
/// // WebSocket provider
/// let evm_ws = create_evm("wss://mainnet.gateway.tenderly.co").await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_evm(
    rpc_url: &str,
) -> Result<DefaultEvm, EvmError> {
    create_evm_internal(rpc_url, NoOpInspector).await
}



/// Create an EVM instance with custom tracer or inspector
/// 
/// Supports both HTTP and WebSocket RPC endpoints. The connection type is
/// automatically detected based on the URL scheme:
/// - `http://` or `https://` → HTTP provider  
/// - `ws://` or `wss://` → WebSocket provider
/// 
/// This function accepts any inspector that implements the required traits:
/// - `TraceInspector<MainnetContext<CacheDB<SharedBackend>>>`: Core inspector functionality
/// - `Clone`: Required for EVM construction
/// 
/// You can use:
/// - **Built-in inspectors**: `TxInspector` for transaction tracing
/// - **REVM inspectors**: Any inspector from the REVM ecosystem
/// - **Custom inspectors**: Your own implementations for specialized analysis
/// 
/// # Type Parameters
/// - `INSP`: The inspector type that will be used for transaction analysis
/// 
/// # Arguments
/// - `rpc_url`: RPC endpoint URL (HTTP or WebSocket)
/// - `tracer`: Inspector instance for transaction analysis
/// 
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::{create_evm_with_tracer, TxInspector};
/// 
/// // Using built-in TxInspector with HTTP
/// let tx_inspector = TxInspector::new();
/// let evm_http = create_evm_with_tracer("https://eth.llamarpc.com", tx_inspector).await?;
/// 
/// // Using TxInspector with WebSocket (same inspector type)
/// let tx_inspector = TxInspector::new();  
/// let evm_ws = create_evm_with_tracer("wss://mainnet.gateway.tenderly.co", tx_inspector).await?;
/// 
/// // Custom inspector example (pseudo-code)
/// // struct MyCustomInspector { /* ... */ }
/// // impl TraceInspector<MainnetContext<CacheDB<SharedBackend>>> for MyCustomInspector { /* ... */ }
/// // let custom_inspector = MyCustomInspector::new();
/// // let evm_custom = create_evm_with_tracer("https://rpc-url", custom_inspector).await?;
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

impl<INSP> TraceEvm<CacheDB<SharedBackend>, INSP> {

    pub fn set_db_block(
        &mut self,
        block_env: BlockEnv,
    ) -> Result<(), EvmError>
    where
        INSP: TraceInspector<MainnetContext<CacheDB<SharedBackend>>>,
    {   
        let block_id = BlockId::Number(BlockNumberOrTag::Number(block_env.number));
        
        // Clear both cache layers and set the pinned block in a single scope
        {
            let cache_db = &mut self.0.ctx.db().db;
            
            // Clear SharedBackend's internal cache by accessing its MemDb
            let mem_db = cache_db.data();
            mem_db.clear();
            
            // Set the backend's pinned block
            cache_db.set_pinned_block(block_id)
                .map_err(|e| EvmError::Init(InitError::DatabaseError(format!("Failed to set pinned block: {}", e))))?;
        }
        
        // Clear CacheDB layer cache
        self.reset_db();
        
        // Set the EVM block context
        self.set_block(block_env);
        
        Ok(())
    }
}