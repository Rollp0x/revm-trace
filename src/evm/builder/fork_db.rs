//! SharedBackend (foundry-fork-db) support for high-performance, concurrent EVM simulation
//!
//! When the `foundry-fork` feature is enabled, this module provides EVM builders and helpers
//! based on foundry-fork-db's `SharedBackend`. This backend enables:
//! - High-performance, thread-safe state caching
//! - Efficient multi-threaded EVM simulation (each thread can create its own EVM instance)
//! - Shared RPC connection pool and memory-efficient state management
//!
//! Use this when you need to run many EVM simulations in parallel, such as for MEV, batch analysis,
//! or large-scale blockchain data processing.

use std::sync::Arc;
pub use foundry_fork_db::SharedBackend;
use foundry_fork_db::{cache::BlockchainDbMeta, BlockchainDb};

use crate::{
    errors::EvmError,
    TraceEvm, TraceInspector,
};
use alloy::{
    eips::{BlockId, BlockNumberOrTag},
    network::AnyNetwork,
    providers::Provider,
};
use revm::{
    context::{Context, BlockEnv},
    database::CacheDB,
    handler::{MainBuilder, MainContext, MainnetContext},
    inspector::NoOpInspector
};
use super::{get_provider, get_block, EvmBuilder};

/// EVM instance using SharedBackend with no-op inspector
///
/// Optimized for high-performance scenarios where caching and shared state
/// management are important.
pub type SharedEvm = TraceEvm<CacheDB<SharedBackend>, NoOpInspector>;

/// EVM instance using SharedBackend with custom inspector
///
/// Combines high-performance caching with custom tracing capabilities.
pub type InspectorSharedEvm<INSP> = TraceEvm<CacheDB<SharedBackend>, INSP>;


/// SharedBackend-specific constructor implementations
///
/// These implementations are specialized for the SharedBackend type,
/// optimized for high-performance caching scenarios.
impl EvmBuilder<SharedBackend, NoOpInspector> {
    /// Creates a new EVM builder configured for SharedBackend
    ///
    /// SharedBackend is optimized for:
    /// - **High-Performance Caching**: Intelligent state caching
    /// - **Shared State Management**: Efficient memory usage
    /// - **Batch Operations**: Optimized for processing multiple transactions
    /// - **Cache Coherency**: Consistent state across operations
    ///
    /// # Arguments
    /// - `url`: RPC endpoint URL (HTTP/HTTPS or WS/WSS)
    ///
    /// # Returns
    /// A new builder instance ready for further configuration
    ///
    /// # Example
    /// ```rust
    /// let builder = EvmBuilder::new_shared("https://eth.llamarpc.com");
    /// ```
    pub fn new_shared(url: &str) -> Self {
        Self {
            rpc_url: url.to_string(),
            block_number: None,
            inspector: NoOpInspector,
            _marker: std::marker::PhantomData,
        }
    }
}


/// SharedBackend-specific build implementation
///
/// This specialized implementation handles the unique requirements of SharedBackend,
/// including cache management and shared state coordination.
impl<INSP> EvmBuilder<SharedBackend, INSP> {
    /// Builds an EVM instance using SharedBackend
    ///
    /// This async method performs the complete EVM initialization process:
    /// 1. **Provider Setup**: Creates RPC provider with protocol detection
    /// 2. **Blockchain Data**: Fetches chain ID, block number, and timestamp
    /// 3. **Cache Database**: Initializes SharedBackend with metadata
    /// 4. **Backend Thread**: Spawns background thread for RPC operations
    /// 5. **EVM Context**: Configures mainnet context with proper settings
    /// 6. **Inspector Integration**: Builds EVM with the specified inspector
    ///
    /// # Returns
    /// - `Ok(TraceEvm)`: Fully configured EVM instance ready for execution
    /// - `Err(EvmError)`: Configuration or network error occurred
    ///
    /// # SharedBackend Configuration
    ///
    /// - **Background Thread**: Dedicated thread for RPC operations
    /// - **Intelligent Caching**: Automatic state caching and management
    /// - **Block Pinning**: Locks to specific block for consistent state
    /// - **Memory Optimization**: Efficient shared memory usage
    ///
    /// # Performance Characteristics
    ///
    /// SharedBackend is optimized for:
    /// - **Batch Processing**: Multiple transactions with state reuse
    /// - **Cache Efficiency**: Smart caching reduces RPC calls
    /// - **Memory Management**: Shared state reduces memory footprint
    /// - **Consistency**: Pinned blocks ensure consistent execution context
    ///
    /// # Error Handling
    ///
    /// All potential failures are properly handled and propagated:
    /// - Network connectivity issues
    /// - Invalid block numbers
    /// - Backend thread creation failures
    /// - Provider initialization problems
    pub async fn build(self) -> Result<TraceEvm<CacheDB<SharedBackend>, INSP>, EvmError>
    where
        INSP: TraceInspector<MainnetContext<CacheDB<SharedBackend>>>,
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
        let block_id = BlockId::Number(BlockNumberOrTag::Number(block_number));

        // Step 3: Create block environment for metadata
        let block_env = BlockEnv {
            number: block_number,
            timestamp,
            ..BlockEnv::default()
        };

        // Step 4: Initialize blockchain database with metadata
        let meta = BlockchainDbMeta::new(block_env, rpc_url);
        let blockchain_db = BlockchainDb::new(meta, None); // None = use in-memory cache

        // Step 5: Create SharedBackend with background thread
        // The Arc<provider> allows shared access across threads
        // The pinned block ensures consistent state for all operations
        let shared_backend = SharedBackend::spawn_backend_thread(
            Arc::new(provider),
            blockchain_db,
            Some(block_id), // Pin to the preset block for consistency
        );

        // Step 6: Create cache layer on top of SharedBackend
        let cache_db = CacheDB::new(shared_backend);

        // Step 7: Create and configure EVM context
        let mut ctx = Context::mainnet().with_db(cache_db);
        let cfg = &mut ctx.cfg;

        // Network configuration
        cfg.chain_id = chain_id;

        // Disable restrictions for simulation environment
        cfg.disable_eip3607 = true; // Allow zero-address transactions
        cfg.limit_contract_code_size = None; // Remove contract size limits
        cfg.disable_block_gas_limit = true; // Remove gas limit restrictions
        cfg.disable_base_fee = true; // Disable EIP-1559 base fee

        // Step 8: Build final EVM instance with inspector
        let evm = ctx.build_mainnet_with_inspector(inspector);
        Ok(TraceEvm::new(evm))
    }
}



/// Creates an EVM instance using SharedBackend with no tracing
///
/// This function creates a high-performance EVM instance optimized for scenarios
/// requiring intensive state access and caching. SharedBackend provides intelligent
/// caching mechanisms that significantly improve performance for batch operations.
///
/// # Arguments
/// - `rpc_url`: RPC endpoint URL (HTTP/HTTPS or WS/WSS)
///
/// # Returns
/// - `Ok(SharedEvm)`: High-performance EVM instance with caching
/// - `Err(EvmError)`: Failed to create EVM due to network or configuration issues
///
/// # Configuration
///
/// The EVM is configured with:
/// - **SharedBackend**: High-performance caching with background thread
/// - **NoOpInspector**: No transaction tracing overhead
/// - **Latest Block**: Uses the most recent block from the blockchain
/// - **Mainnet Configuration**: Standard Ethereum mainnet settings
///
/// # Performance Characteristics
///
/// - **Intelligent Caching**: Automatic state caching reduces RPC calls
/// - **Background Processing**: Dedicated thread for RPC operations
/// - **Memory Optimization**: Shared state management reduces memory usage
/// - **Batch Efficiency**: Optimized for processing multiple transactions
///
/// # Use Cases
///
/// - High-throughput transaction processing
/// - MEV analysis and bot operations
/// - Large-scale blockchain analysis
/// - Performance-critical applications
/// - Scenarios with repeated state access
///
/// # Example
///
/// ```rust
/// let evm = create_shared_evm("https://eth.llamarpc.com").await?;
///
/// // Process multiple transactions efficiently
/// for tx in transactions {
///     let result = evm.execute_transaction(tx)?;
///     process_result(result);
/// }
/// ```
pub async fn create_shared_evm(rpc_url: &str) -> Result<SharedEvm, EvmError> {
    let evm_builder = EvmBuilder::<SharedBackend, NoOpInspector>::new_shared(rpc_url);
    evm_builder.build().await
}

/// Creates an EVM instance using SharedBackend with custom inspector
///
/// This function combines the high-performance caching capabilities of SharedBackend
/// with custom transaction tracing. This provides the best of both worlds: efficient
/// state management and comprehensive transaction monitoring.
///
/// # Type Parameters
/// - `INSP`: Inspector type implementing TraceInspector for SharedBackend contexts
///
/// # Arguments
/// - `rpc_url`: RPC endpoint URL (HTTP/HTTPS or WS/WSS)
/// - `tracer`: Custom inspector instance for transaction monitoring
///
/// # Returns
/// - `Ok(InspectorSharedEvm<INSP>)`: High-performance EVM with tracing
/// - `Err(EvmError)`: Failed to create EVM due to network or configuration issues
///
/// # Configuration
///
/// The EVM is configured with:
/// - **SharedBackend**: High-performance caching with background thread
/// - **Custom Inspector**: User-provided tracing and monitoring
/// - **Latest Block**: Uses the most recent block from the blockchain
/// - **Mainnet Configuration**: Standard Ethereum mainnet settings
///
/// # Performance Trade-offs
///
/// While the inspector adds some overhead, the SharedBackend's caching benefits
/// often outweigh the tracing costs, especially for:
/// - Repeated execution of similar transactions
/// - Operations requiring multiple state lookups
/// - Analysis of transaction patterns and behaviors
///
/// # Use Cases
///
/// - High-performance transaction analysis
/// - MEV detection with detailed tracing
/// - Optimization of gas usage patterns
/// - Large-scale security analysis
/// - Performance profiling of smart contracts
///
/// # Example
///
/// ```rust
/// let tracer = TxInspector::new();
/// let evm = create_shared_evm_with_tracer("https://eth.llamarpc.com", tracer).await?;
///
/// // Efficiently process and trace multiple transactions
/// for tx in transactions {
///     let result = evm.execute_transaction(tx)?;
///     let traces = result.inspector().get_traces();
///     analyze_traces(traces);
/// }
/// ```
pub async fn create_shared_evm_with_tracer<INSP>(
    rpc_url: &str,
    tracer: INSP,
) -> Result<InspectorSharedEvm<INSP>, EvmError>
where
    INSP: TraceInspector<MainnetContext<CacheDB<SharedBackend>>>,
{
    // Use the internal builder pattern to create EVM with custom tracer
    let evm_builder =
        EvmBuilder::<SharedBackend, NoOpInspector>::new_shared(rpc_url).with_tracer(tracer);
    evm_builder.build().await
}

// ========================= Multi-Threading Support with SharedBackend =========================

/// Creates a SharedBackend that can be safely shared across multiple threads
///
/// This function creates a SharedBackend instance that can be cloned and used
/// across multiple threads. Each thread can then use the shared backend to
/// create its own EVM instance using `create_evm_from_shared_backend`.
///
/// # Multi-Threading Strategy
///
/// While `TraceEvm` itself is not `Send + Sync`, `SharedBackend` is thread-safe
/// and can be shared across threads. This enables the following pattern:
///
/// 1. Create a SharedBackend on the main thread
/// 2. Clone the SharedBackend for each worker thread  
/// 3. Each thread creates its own TraceEvm using the shared backend
/// 4. All threads benefit from the shared cache and RPC connection pool
///
/// # Arguments
/// - `rpc_url`: RPC endpoint URL (HTTP/HTTPS or WS/WSS)
/// - `block_number`: Optional specific block number (uses latest if None)
///
/// # Returns
/// - `Ok(SharedBackend)`: Thread-safe backend ready for multi-threading
/// - `Err(EvmError)`: Failed to create backend due to network or configuration issues
///
/// # Example
///
/// ```rust
/// // Create shared backend on main thread
/// let shared_backend = create_shared_backend("https://eth.llamarpc.com", None).await?;
///
/// // Clone for multiple threads
/// let handles: Vec<_> = (0..4).map(|i| {
///     let backend = shared_backend.clone();
///     let tracer = TxInspector::new();
///     
///     tokio::spawn(async move {
///         // Each thread creates its own EVM with the shared backend
///         let evm = create_evm_from_shared_backend(backend, tracer).await?;
///         
///         // Process transactions on this thread
///         process_transactions(evm, thread_id).await
///     })
/// }).collect();
///
/// // Wait for all threads to complete
/// for handle in handles {
///     handle.await??;
/// }
/// ```
pub async fn create_shared_backend(
    rpc_url: &str,
    block_number: Option<u64>,
) -> Result<SharedBackend, EvmError> {
    // Step 1: Create provider with automatic protocol detection
    let provider = get_provider(rpc_url).await?;

    // Step 2: Fetch essential blockchain data
    let (_, block_number, timestamp) = get_block(&provider, block_number).await?;
    let block_id = BlockId::Number(BlockNumberOrTag::Number(block_number));

    // Step 3: Create block environment for metadata
    let block_env = BlockEnv {
        number: block_number,
        timestamp,
        ..BlockEnv::default()
    };

    // Step 4: Initialize blockchain database with metadata
    let meta = BlockchainDbMeta::new(block_env, rpc_url.to_string());
    let blockchain_db = BlockchainDb::new(meta, None); // None = use in-memory cache

    // Step 5: Create SharedBackend with background thread
    // The Arc<provider> allows shared access across threads
    // The pinned block ensures consistent state for all operations
    let shared_backend = SharedBackend::spawn_backend_thread(
        Arc::new(provider),
        blockchain_db,
        Some(block_id), // Pin to the preset block for consistency
    );

    Ok(shared_backend)
}

/// Creates an EVM instance from an existing SharedBackend
///
/// This function allows creating EVM instances from a pre-existing SharedBackend,
/// enabling efficient multi-threading where multiple threads share the same
/// backend but each has its own EVM instance.
///
/// # Multi-Threading Benefits
///
/// - **Shared Cache**: All EVMs benefit from the same cached state
/// - **Shared RPC Pool**: All EVMs use the same RPC connection pool
/// - **Memory Efficiency**: Only one background thread and cache per backend
/// - **Thread Isolation**: Each thread gets its own EVM instance
///
/// # Arguments
/// - `shared_backend`: Pre-created SharedBackend instance
/// - `inspector`: Inspector instance for this EVM
///
/// # Returns
/// - `Ok(TraceEvm)`: EVM instance ready for execution on current thread
/// - `Err(EvmError)`: Failed to create EVM from the shared backend
///
/// # Thread Safety
///
/// - The `SharedBackend` parameter is `Send + Sync` and can be safely passed between threads
/// - The resulting `TraceEvm` is NOT `Send + Sync` and must stay on the creating thread
/// - Each thread should call this function to create its own EVM instance
///
/// # Example
///
/// ```rust
/// // In a worker thread
/// async fn worker_thread(shared_backend: SharedBackend, thread_id: usize) -> Result<(), EvmError> {
///     let tracer = TxInspector::new();
///     let evm = create_evm_from_shared_backend(shared_backend, tracer).await?;
///     
///     // Process transactions on this thread
///     for tx in get_transactions_for_thread(thread_id) {
///         let result = evm.execute_transaction(tx)?;
///         process_result(result, thread_id).await;
///     }
///     
///     Ok(())
/// }
/// ```
pub async fn create_evm_from_shared_backend<INSP, P>(
    shared_backend: SharedBackend,
    provider: &P,
    inspector: INSP,
) -> Result<TraceEvm<CacheDB<SharedBackend>, INSP>, EvmError>
where
    P: Provider<AnyNetwork>,
    INSP: TraceInspector<MainnetContext<CacheDB<SharedBackend>>>,
{
    // Extract chain ID and block information from the SharedBackend
    let (chain_id, block_number, timestamp) = get_block(&provider, None).await?;
    // Create cache layer on top of SharedBackend
    let cache_db: CacheDB<SharedBackend> = CacheDB::new(shared_backend);

    // Create and configure EVM context
    let mut ctx = Context::mainnet().with_db(cache_db);
    let cfg = &mut ctx.cfg;

    // Network configuration
    cfg.chain_id = chain_id;

    // Disable restrictions for simulation environment
    cfg.disable_eip3607 = true; // Allow zero-address transactions
    cfg.limit_contract_code_size = None; // Remove contract size limits
    cfg.disable_block_gas_limit = true; // Remove gas limit restrictions
    cfg.disable_base_fee = true; // Disable EIP-1559 base fee

    // Set block environment from SharedBackend metadata
    ctx.block.number = block_number;
    ctx.block.timestamp = timestamp;

    // Build final EVM instance with inspector
    let evm = ctx.build_mainnet_with_inspector(inspector);
    Ok(TraceEvm::new(evm))
}

/// Creates an EVM instance with no inspector from an existing SharedBackend
///
/// Convenience function for creating an EVM without tracing from a shared backend.
/// This is useful when you want maximum performance and don't need transaction tracing.
///
/// # Arguments
/// - `shared_backend`: Pre-created SharedBackend instance
///
/// # Returns
/// - `Ok(TraceEvm)`: EVM instance with NoOpInspector
/// - `Err(EvmError)`: Failed to create EVM from the shared backend
///
/// # Example
///
/// ```rust
/// // In a worker thread for high-performance processing
/// async fn high_perf_worker(shared_backend: SharedBackend) -> Result<(), EvmError> {
///     let evm = create_evm_from_shared_backend_no_trace(shared_backend).await?;
///     
///     // Process many transactions quickly without tracing overhead
///     for tx in high_volume_transactions {
///         let result = evm.execute_transaction(tx)?;
///         // Process result without detailed tracing
///     }
///     
///     Ok(())
/// }
/// ```
pub async fn create_evm_from_shared_backend_no_trace<P>(
    shared_backend: SharedBackend,
    provider: &P,
) -> Result<TraceEvm<CacheDB<SharedBackend>, NoOpInspector>, EvmError>
where
    P: Provider<AnyNetwork>,
{
    create_evm_from_shared_backend(shared_backend, provider, NoOpInspector).await
}
