//! # EVM Builder Module
//!
//! This module provides a unified, type-safe builder pattern for creating EVM instances
//! with different database backends and inspector configurations.
//!
//! ## Architecture Overview
//!
//! The module implements a generic builder pattern that supports:
//! - **AlloyDB Backend**: Stateless, async-first database for RPC access
//! - **SharedBackend**: High-performance cached database with shared state management (feature = "foundry-fork")
//! - **Generic Inspector Support**: Any inspector implementing the `TraceInspector` trait
//! - **Type-Safe Construction**: Compile-time guarantees for backend-inspector compatibility
//!
//! ## Key Design Principles
//!
//! 1. **Separation of Concerns**: Database backend selection is separate from inspector choice
//! 2. **Type Safety**: Each backend has specialized implementations with appropriate constraints
//! 3. **Ergonomic API**: Builder pattern allows fluent, readable EVM construction
//! 4. **Error Handling**: Production-ready error propagation without panics
//! 5. **Generic Block Reset**: Unified `ResetBlock` trait for state management across backends
//!
//! ## Thread Safety Limitations & Multi-Threading Solutions
//!
//! **IMPORTANT**: The EVM instances created by this module are **NOT Send + Sync** due to
//! limitations in the underlying REVM Context structure. However, we provide solutions
//! for multi-threaded usage:
//!
//! ### EVM Thread Safety Status
//! - ✅ EVMs work perfectly within a single thread
//! - ✅ Multiple EVMs can be used sequentially on the same thread
//! - ❌ EVMs cannot be moved between threads (not `Send`)
//! - ❌ EVMs cannot be shared across threads (not `Sync`)
//! - ❌ EVMs cannot be used in `tokio::spawn` or similar multi-threaded contexts
//!
//! ### Multi-Threading Solution with SharedBackend
//!
//! While EVMs themselves are not thread-safe, `SharedBackend` **IS Send + Sync** and can
//! be safely shared across threads. 典型用法如下：
//!
//! 1. **Create SharedBackend**: Use `create_shared_backend()` on the main thread
//! 2. **Share Backend**: Clone the SharedBackend for each worker thread  
//! 3. **Per-Thread EVMs**: Each thread creates its own EVM using `create_evm_from_shared_backend()`
//! 4. **Shared Benefits**: All threads benefit from shared cache and RPC connection pool
//!
//!
//! The root cause of EVM thread safety issues appears to be in REVM's Context structure,
//! specifically in the error handling mechanisms that are not thread-safe.
//!
//! ## Usage Patterns
//!
//! ```rust
//! // ✅ Single-threaded usage - works perfectly
//! let evm = EvmBuilder::new_alloy("https://eth.llamarpc.com")
//!     .with_block_number(18_000_000)
//!     .with_tracer(TxInspector::new())
//!     .build()
//!     .await?;
//!
//! // ✅ Multiple EVMs on same thread - works perfectly
//! let evm1 = create_evm("https://eth.llamarpc.com").await?;
//! let evm2 = create_evm("https://eth.llamarpc.com").await?;
//!
//! // ✅ Multi-threaded usage with SharedBackend - works perfectly
//! let shared_backend = create_shared_backend("https://eth.llamarpc.com", None).await?;
//!
//! let handles: Vec<_> = (0..4).map(|i| {
//!     let backend = shared_backend.clone();
//!     tokio::spawn(async move {
//!         let tracer = TxInspector::new();
//!         let evm = create_evm_from_shared_backend(backend, tracer).await?;
//!         // Process transactions on this thread
//!         process_transactions(evm, i).await
//!     })
//! }).collect();
//!
//! // ❌ Direct EVM multi-threading - will not compile
//! // let handle = tokio::spawn(async move {
//! //     evm.execute_transaction(tx).await
//! // });
//! ```
//!
//! ## Future Improvements
//!
//! To enable true multi-threading support, the following would be needed:
//! - Upstream changes to REVM's Context structure to be Send + Sync
//! - Alternative error handling that doesn't break thread safety
//! - Wrapper types that can safely cross thread boundaries
use crate::{
    errors::{EvmError, InitError},
    types::{AllDBType, AnyNetworkProvider},
    MyWrapDatabaseAsync, TraceEvm, TraceInspector,
};
use alloy::{
    eips::{BlockId, BlockNumberOrTag},
    network::{AnyNetwork, BlockResponse},
    providers::{Provider, ProviderBuilder, WsConnect},
};
use revm::{
    context::Context,
    database::{AlloyDB, CacheDB, DatabaseRef},
    handler::{MainBuilder, MainContext, MainnetContext},
    inspector::NoOpInspector
};


// ========================= Type Aliases =========================

/// Default EVM instance using AlloyDB backend with no-op inspector
///
/// This is the most basic EVM configuration, suitable for simple execution
/// scenarios where tracing is not required.
pub type DefaultEvm = TraceEvm<CacheDB<AllDBType>, NoOpInspector>;

/// EVM instance using AlloyDB backend with custom inspector
///
/// Generic over inspector type `INSP`, allowing any inspector that implements
/// the required traits for AlloyDB backend.
pub type InspectorEvm<INSP> = TraceEvm<CacheDB<AllDBType>, INSP>;



// ========================= Provider Creation =========================

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
///
/// # Design Notes
///
/// This function abstracts away the complexity of provider creation, allowing
/// users to simply provide a URL string. The automatic detection makes the API
/// more ergonomic while supporting both HTTP and WebSocket protocols.
pub async fn get_provider(rpc_url: &str) -> Result<AnyNetworkProvider, EvmError> {
    let provider = if rpc_url.starts_with("http") || rpc_url.starts_with("https") {
        // HTTP/HTTPS provider creation
        let url = rpc_url
            .parse()
            .map_err(|_| InitError::InvalidRpcUrl("Failed to parse RPC URL".to_string()))?;
        ProviderBuilder::new()
            .network::<AnyNetwork>()
            .connect_http(url)
    } else if rpc_url.starts_with("ws") || rpc_url.starts_with("wss") {
        // WebSocket provider creation
        let ws_connect = WsConnect::new(rpc_url);
        ProviderBuilder::new_with_network::<AnyNetwork>()
            .connect_ws(ws_connect)
            .await
            .map_err(|_| InitError::InvalidRpcUrl("Failed to connect to WebSocket".to_string()))?
    } else {
        // Unsupported protocol scheme
        return Err(EvmError::Init(InitError::InvalidRpcUrl(
            "Unsupported RPC URL scheme".to_string(),
        )));
    };
    Ok(provider)
}

/// Internal function to fetch block information from the blockchain
///
/// Retrieves essential block data needed for EVM initialization:
/// - Chain ID for network identification
/// - Block number (either specified or latest)
/// - Block timestamp for EVM context
///
/// # Arguments
/// - `provider`: Blockchain provider for RPC calls
/// - `block_number`: Optional specific block number (uses latest if None)
///
/// # Returns
/// - `Ok((chain_id, block_number, timestamp))`: Essential block data
/// - `Err(InitError)`: Failed to fetch required blockchain data
///
/// # Design Notes
///
/// This function centralizes all blockchain data fetching, ensuring consistent
/// error handling and reducing code duplication across backend implementations.
pub async fn get_block<P: Provider<AnyNetwork>>(
    provider: &P,
    block_number: Option<u64>,
) -> Result<(u64, u64, u64), InitError> {
    // Fetch chain ID for network identification
    let chain_id = provider
        .get_chain_id()
        .await
        .map_err(|_| InitError::BlockFetchError("Failed to fetch chain ID".to_string()))?;

    // Determine block number (use latest if not specified)
    let block_number = if let Some(number) = block_number {
        number
    } else {
        let number = provider.get_block_number().await.map_err(|_| {
            InitError::BlockFetchError("Failed to fetch latest block number".to_string())
        })?;
        number
    };

    // Fetch block information for timestamp
    let block_info = provider
        .get_block_by_number(BlockNumberOrTag::Number(block_number))
        .await
        .map_err(|_| InitError::BlockFetchError("Failed to fetch block".to_string()))?
        .ok_or_else(|| InitError::BlockNotFound("Block not found".to_string()))?;
    let timestamp = block_info.header().timestamp;

    Ok((chain_id, block_number, timestamp))
}

// ========================= Core Builder Structure =========================

/// Generic EVM builder supporting multiple database backends and inspectors
///
/// The `EvmBuilder` uses the builder pattern to provide a fluent, type-safe API
/// for constructing EVM instances. The generic design allows it to work with
/// different database backends (AlloyDB, SharedBackend) and inspector types.
///
/// # Type Parameters
/// - `DB`: Database backend type (must implement `DatabaseRef`)
/// - `INSP`: Inspector type (defaults to `NoOpInspector`)
///
/// # Design Philosophy
///
/// The builder pattern separates configuration from construction, allowing:
/// - **Incremental Configuration**: Set options one at a time
/// - **Type Safety**: Backend-specific constraints enforced at compile time
/// - **Flexibility**: Support for different backends without code duplication
/// - **Ergonomics**: Fluent API that reads naturally
///
/// # Usage Flow
///
/// 1. Create builder with backend-specific constructor (`new_alloy`, `new_shared`)
/// 2. Configure options with chainable methods (`with_block_number`, `with_tracer`)
/// 3. Build final EVM instance with `build()` method
pub struct EvmBuilder<DB: DatabaseRef, INSP = NoOpInspector> {
    /// RPC endpoint URL for blockchain connectivity
    rpc_url: String,
    /// Optional specific block number (uses latest if None)
    block_number: Option<u64>,
    /// Inspector instance for transaction tracing
    inspector: INSP,
    /// Phantom data to track database type at compile time
    _marker: std::marker::PhantomData<DB>,
}

// ========================= Backend-Specific Constructors =========================

/// AlloyDB-specific constructor implementations
///
/// These implementations are specialized for the AlloyDB backend type,
/// ensuring type safety and proper initialization.
impl EvmBuilder<AllDBType, NoOpInspector> {
    /// Creates a new EVM builder configured for AlloyDB backend
    ///
    /// AlloyDB is optimized for:
    /// - **Stateless Operations**: No persistent state between calls
    /// - **Async-First Design**: Built for async/await patterns
    /// - **Multi-Threading**: Safe for concurrent usage
    /// - **Direct RPC Access**: Minimal caching for latest data
    ///
    /// # Arguments
    /// - `url`: RPC endpoint URL (HTTP/HTTPS or WS/WSS)
    ///
    /// # Returns
    /// A new builder instance ready for further configuration
    ///
    /// # Example
    /// ```rust
    /// let builder = EvmBuilder::new_alloy("https://eth.llamarpc.com");
    /// ```
    pub fn new_alloy(url: &str) -> Self {
        Self {
            rpc_url: url.to_string(),
            block_number: None,
            inspector: NoOpInspector,
            _marker: std::marker::PhantomData,
        }
    }
}

// ========================= Generic Configuration Methods =========================

/// Generic builder methods available for all database backends
///
/// These methods provide a consistent API regardless of the chosen backend,
/// allowing users to configure the EVM without backend-specific knowledge.
impl<DB: DatabaseRef, INSP> EvmBuilder<DB, INSP> {
    /// Sets the specific block number for EVM initialization
    ///
    /// By default, the EVM will use the latest block. This method allows
    /// pinning to a specific historical block for:
    /// - **Historical Analysis**: Analyzing past blockchain state
    /// - **Reproducible Results**: Ensuring consistent execution context
    /// - **Testing**: Using known block states for predictable tests
    ///
    /// # Arguments
    /// - `block_number`: Specific block number to use
    ///
    /// # Returns
    /// Updated builder instance with block number set
    ///
    /// # Example
    /// ```rust
    /// let builder = EvmBuilder::new_alloy("https://eth.llamarpc.com")
    ///     .with_block_number(18_000_000);
    /// ```
    pub fn with_block_number(self, block_number: u64) -> Self {
        EvmBuilder {
            rpc_url: self.rpc_url,
            block_number: Some(block_number),
            inspector: self.inspector,
            _marker: std::marker::PhantomData,
        }
    }

    /// Replaces the inspector with a custom implementation
    ///
    /// This method enables the builder to switch from the default `NoOpInspector`
    /// to a custom inspector that implements transaction tracing, analysis, or
    /// other monitoring capabilities.
    ///
    /// # Type Parameters
    /// - `NewInsp`: New inspector type (must implement `TraceInspector`)
    ///
    /// # Arguments
    /// - `inspector`: Custom inspector instance
    ///
    /// # Returns
    /// New builder instance with the specified inspector type
    ///
    /// # Type Safety
    ///
    /// The inspector must be compatible with the chosen database backend,
    /// enforced through trait bounds at compile time.
    ///
    /// # Example
    /// ```rust
    /// let builder = EvmBuilder::new_alloy("https://eth.llamarpc.com")
    ///     .with_tracer(TxInspector::new());
    /// ```
    pub fn with_tracer<NewInsp>(self, inspector: NewInsp) -> EvmBuilder<DB, NewInsp>
    where
        NewInsp: TraceInspector<MainnetContext<CacheDB<DB>>>,
    {
        EvmBuilder {
            rpc_url: self.rpc_url,
            block_number: self.block_number,
            inspector,
            _marker: std::marker::PhantomData,
        }
    }
}

// ========================= Backend-Specific Build Implementations =========================

/// AlloyDB-specific build implementation
///
/// This specialized implementation handles the unique requirements of AlloyDB,
/// including async wrapper creation and stateless database configuration.
impl<INSP> EvmBuilder<AllDBType, INSP> {
    /// Builds an EVM instance using AlloyDB backend
    ///
    /// This async method performs the complete EVM initialization process:
    /// 1. **Provider Setup**: Creates RPC provider with protocol detection
    /// 2. **Blockchain Data**: Fetches chain ID, block number, and timestamp
    /// 3. **Database Creation**: Initializes AlloyDB with async wrapper
    /// 4. **EVM Context**: Configures mainnet context with proper settings
    /// 5. **Inspector Integration**: Builds EVM with the specified inspector
    ///
    /// # Returns
    /// - `Ok(TraceEvm)`: Fully configured EVM instance ready for execution
    /// - `Err(EvmError)`: Configuration or network error occurred
    ///
    /// # AlloyDB Configuration
    ///
    /// - **Async Wrapper**: Required for sync Database trait compatibility
    /// - **Multi-Thread Runtime**: Needs suitable tokio runtime context
    /// - **Direct RPC**: Minimal caching for real-time blockchain access
    /// - **Stateless Design**: No persistent state between operations
    ///
    /// # EVM Configuration
    ///
    /// The following mainnet-compatible settings are applied:
    /// - `disable_eip3607`: Allows transactions from zero-address
    /// - `limit_contract_code_size`: Removes contract size limits
    /// - `disable_block_gas_limit`: Removes gas limit restrictions
    /// - `disable_base_fee`: Disables EIP-1559 base fee requirements
    ///
    /// # Error Handling
    ///
    /// All potential failures are properly handled and propagated:
    /// - Network connectivity issues
    /// - Invalid block numbers
    /// - Runtime availability problems
    /// - Provider creation failures
    pub async fn build(self) -> Result<TraceEvm<CacheDB<AllDBType>, INSP>, EvmError>
    where
        INSP: TraceInspector<MainnetContext<CacheDB<AllDBType>>>,
    {
        // Destructure builder to extract configuration
        let EvmBuilder {
            rpc_url,
            block_number,
            inspector,
            _marker,
        } = self;

        // Step 1: Create provider with automatic protocol detection
        let provider = get_provider(&rpc_url).await?;

        // Step 2: Fetch essential blockchain data
        let (chain_id, block_number, timestamp) = get_block(&provider, block_number).await?;

        // Step 3: Create AlloyDB instance
        let block_id = BlockId::Number(BlockNumberOrTag::Number(block_number));
        let alloy_db = AlloyDB::new(provider, block_id);

        // Step 4: Wrap AlloyDB for sync compatibility
        // Note: This requires a suitable tokio runtime to be available
        let wrap_db = MyWrapDatabaseAsync::new(alloy_db).ok_or_else(|| {
            EvmError::Init(InitError::DatabaseError(
                "Failed to create wrapped database: no suitable tokio runtime available"
                    .to_string(),
            ))
        })?;

        // Step 5: Create cache layer on top of wrapped database
        let cache_db = CacheDB::new(wrap_db);

        // Step 6: Create and configure EVM context
        let mut ctx = Context::mainnet().with_db(cache_db);

        // Network configuration
        ctx.cfg.chain_id = chain_id;

        // Disable restrictions for simulation environment
        ctx.cfg.disable_eip3607 = true; // Allow zero-address transactions
        ctx.cfg.limit_contract_code_size = None; // Remove contract size limits
        ctx.cfg.disable_block_gas_limit = true; // Remove gas limit restrictions
        ctx.cfg.disable_base_fee = true; // Disable EIP-1559 base fee

        // Block environment configuration
        ctx.block.number = block_number;
        ctx.block.timestamp = timestamp;

        // Step 7: Build final EVM instance with inspector
        let evm = ctx.build_mainnet_with_inspector(inspector);
        Ok(TraceEvm::new(evm))
    }
}


// ========================= Convenience Functions =========================

/// Creates a basic EVM instance using AlloyDB backend with no tracing
///
/// This is the simplest way to create an EVM instance for basic execution
/// scenarios where transaction tracing is not required. The EVM will use
/// the latest block from the specified RPC endpoint.
///
/// # Arguments
/// - `rpc_url`: RPC endpoint URL (HTTP/HTTPS or WS/WSS)
///
/// # Returns
/// - `Ok(DefaultEvm)`: Ready-to-use EVM instance with AlloyDB backend
/// - `Err(EvmError)`: Failed to create EVM due to network or configuration issues
///
/// # Configuration
///
/// The EVM is configured with:
/// - **AlloyDB Backend**: Direct RPC access with minimal caching
/// - **NoOpInspector**: No transaction tracing or monitoring
/// - **Latest Block**: Uses the most recent block from the blockchain
/// - **Mainnet Configuration**: Standard Ethereum mainnet settings
///
/// # Use Cases
///
/// - Simple transaction execution
/// - Basic contract calls
/// - Quick prototyping and testing
/// - Scenarios where performance is prioritized over observability
///
/// # Example
///
/// ```rust
/// let evm = create_evm("https://eth.llamarpc.com").await?;
/// let result = evm.execute_transaction(tx)?;
/// ```
pub async fn create_evm(rpc_url: &str) -> Result<DefaultEvm, EvmError> {
    let evm_builder = EvmBuilder::<AllDBType, NoOpInspector>::new_alloy(rpc_url);
    evm_builder.build().await
}

/// Creates an EVM instance using AlloyDB backend with custom inspector
///
/// This function provides a convenient way to create an EVM instance with
/// transaction tracing capabilities. The custom inspector allows monitoring
/// transaction execution, collecting traces, and implementing custom logic.
///
/// # Type Parameters
/// - `INSP`: Inspector type implementing TraceInspector for AlloyDB contexts
///
/// # Arguments
/// - `rpc_url`: RPC endpoint URL (HTTP/HTTPS or WS/WSS)
/// - `tracer`: Custom inspector instance for transaction monitoring
///
/// # Returns
/// - `Ok(InspectorEvm<INSP>)`: EVM instance with tracing capabilities
/// - `Err(EvmError)`: Failed to create EVM due to network or configuration issues
///
/// # Configuration
///
/// The EVM is configured with:
/// - **AlloyDB Backend**: Direct RPC access for real-time data
/// - **Custom Inspector**: User-provided tracing and monitoring
/// - **Latest Block**: Uses the most recent block from the blockchain
/// - **Mainnet Configuration**: Standard Ethereum mainnet settings
///
/// # Use Cases
///
/// - Transaction tracing and analysis
/// - Custom monitoring and logging
/// - Debug information collection
/// - Performance profiling
/// - Security analysis
///
/// # Example
///
/// ```rust
/// let tracer = TxInspector::new();
/// let evm = create_evm_with_tracer("https://eth.llamarpc.com", tracer).await?;
/// let result = evm.execute_transaction(tx)?;
/// let traces = result.inspector().get_traces();
/// ```
pub async fn create_evm_with_tracer<INSP>(
    rpc_url: &str,
    tracer: INSP,
) -> Result<InspectorEvm<INSP>, EvmError>
where
    INSP: TraceInspector<MainnetContext<CacheDB<AllDBType>>>,
{
    // Use the internal builder pattern to create EVM with custom tracer
    let evm_builder =
        EvmBuilder::<AllDBType, NoOpInspector>::new_alloy(rpc_url).with_tracer(tracer);
    evm_builder.build().await
}


#[cfg(feature = "foundry-fork")]
pub mod fork_db;
