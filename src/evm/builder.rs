//! EVM Builder for creating TraceEvm instances with different configurations
//!
//! This module provides the `EvmBuilder` struct which offers a fluent API for constructing
//! EVM instances with various database backends and inspector configurations.
//!
//! ## Supported Configurations
//!
//! - **Single-threaded with AlloyDB**: Standard configuration for most use cases
//! - **Multi-threaded with SharedBackend**: Advanced configuration for concurrent operations
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use revm_trace_multi_thread::evm::builder::EvmBuilder;
//! use revm::inspector::NoOpInspector;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a standard EVM instance
//! let evm = EvmBuilder::new(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
//!     NoOpInspector
//! )
//! .with_current_runtime()?
//! .build()
//! .await?;
//!
//! // Or create a multi-threaded EVM (requires "multi-threading" feature)
//! #[cfg(feature = "multi-threading")]
//! let shared_evm = EvmBuilder::new(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
//!     NoOpInspector
//! )
//! .build_shared()
//! .await?;
//! # Ok(())
//! # }
//! ```

use revm::{
    context::Context, 
    database::{CacheDB, DatabaseRef, WrapDatabaseAsync}, 
    handler::{MainBuilder,MainContext, MainnetContext}, 
    inspector::{Inspector, NoOpInspector}
};
use alloy::{
    network::AnyNetwork,
    eips::BlockId,
    providers::{
        Provider, 
        ProviderBuilder,
    },
};
use tokio::runtime::Handle;
use crate::{
    types::{AlloyDBType,HttpProvider,CacheAlloyDB},
    errors::{EvmError,RuntimeError,InitError},
    evm::{TraceEvm,AlloyTraceEvm}
};

#[cfg(feature = "multi-threading")]
use super::{SharedTraceEvm,SharedCacheDB};

#[cfg(feature = "multi-threading")]
use crate::types::BlockEnv;

/// Builder for creating EVM instances with different database backends
///
/// The `EvmBuilder` provides a fluent API for constructing EVM instances with:
/// - Different database backends (AlloyDB, SharedBackend)
/// - Custom inspectors for transaction tracing
/// - Runtime configuration options
/// - Chain-specific settings
///
/// # Type Parameters
/// - `I`: Inspector type that implements the `Inspector` trait
///
/// # Examples
///
/// ```rust,no_run
/// use revm_trace_multi_thread::evm::builder::EvmBuilder;
/// use revm::inspector::NoOpInspector;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let builder = EvmBuilder::new(
///     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
///     NoOpInspector
/// );
///
/// let evm = builder
///     .with_current_runtime()?
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct EvmBuilder<I = NoOpInspector> 
{
    /// RPC endpoint URL for blockchain data
    rpc_url: String,
    /// Optional Tokio runtime handle for async operations
    runtime_handle: Option<Handle>,
    /// Inspector instance for transaction tracing
    inspector: I,
}

impl<I> EvmBuilder<I> {
    /// Create a new EVM builder with the specified RPC URL and inspector
    ///
    /// # Arguments
    /// - `rpc_url`: HTTP RPC endpoint URL (e.g., "https://eth-mainnet.g.alchemy.com/v2/your-key")
    /// - `inspector`: Inspector instance for transaction tracing and analysis
    ///
    /// # Returns
    /// A new `EvmBuilder` instance ready for configuration
    ///
    /// # Example
    /// ```rust
    /// use revm_trace_multi_thread::evm::builder::EvmBuilder;
    /// use revm::inspector::NoOpInspector;
    ///
    /// let builder = EvmBuilder::new(
    ///     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
    ///     NoOpInspector
    /// );
    /// ```
    pub fn new(rpc_url: String, inspector: I) -> Self {
        Self {
            rpc_url,
            runtime_handle: None,
            inspector,
        }
    }
    /// Configure the builder to use the current Tokio runtime
    ///
    /// This method attempts to capture the current Tokio runtime handle,
    /// which is required for certain async database operations.
    ///
    /// # Returns
    /// - `Ok(Self)`: Builder configured with the current runtime
    /// - `Err(EvmError)`: If no current Tokio runtime is available
    ///
    /// # Errors
    /// Returns `RuntimeError::NoTokioRuntime` if called outside of a Tokio runtime context
    ///
    /// # Example
    /// ```rust,no_run
    /// # use revm_trace_multi_thread::evm::builder::EvmBuilder;
    /// # use revm::inspector::NoOpInspector;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = EvmBuilder::new("https://rpc.url".to_string(), NoOpInspector)
    ///     .with_current_runtime()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_current_runtime(mut self) -> Result<Self, EvmError> {
        self.runtime_handle = Some(
            Handle::try_current()
                .map_err(|_| RuntimeError::NoTokioRuntime("No current tokio runtime".to_string()))?
        );
        Ok(self)
    }

    /// Configure the builder to use a specific Tokio runtime handle
    ///
    /// This method allows you to specify a particular runtime handle
    /// instead of using the current one.
    ///
    /// # Arguments
    /// - `handle`: The Tokio runtime handle to use
    ///
    /// # Returns
    /// The builder configured with the specified runtime handle
    ///
    /// # Example
    /// ```rust,no_run
    /// # use revm_trace_multi_thread::evm::builder::EvmBuilder;
    /// # use revm::inspector::NoOpInspector;
    /// # use tokio::runtime::Handle;
    /// let handle = Handle::current();
    /// let builder = EvmBuilder::new("https://rpc.url".to_string(), NoOpInspector)
    ///     .with_runtime_handle(handle);
    /// ```
    pub fn with_runtime_handle(mut self, handle: Handle) -> Self {
        self.runtime_handle = Some(handle);
        self
    }

    /// Internal method for building EVM instances with a given database
    ///
    /// This is a low-level method that creates a TraceEvm instance with:
    /// - A CacheDB wrapper around the provided database
    /// - Mainnet configuration with the specified chain ID
    /// - The configured inspector
    ///
    /// # Type Parameters
    /// - `DB`: Database type that implements `DatabaseRef`
    ///
    /// # Arguments
    /// - `chain_id`: The blockchain's chain ID (e.g., 1 for Ethereum mainnet)
    /// - `db`: Database instance for state queries
    /// - `inspector`: Inspector for transaction tracing
    ///
    /// # Returns
    /// A configured `TraceEvm` instance ready for transaction execution
    pub fn build_internal<DB>(
        chain_id:u64, 
        db:DB,
        inspector:I
    ) -> TraceEvm<CacheDB<DB>, I>
    where 
        DB: DatabaseRef
    {   
        let cache_db = CacheDB::new(db);
        let mut ctx = Context::mainnet().with_db(cache_db);
        let cfg = &mut ctx.cfg;
        cfg.chain_id = chain_id;
        let evm = ctx.build_mainnet_with_inspector(inspector);

        TraceEvm::new(evm)
    }

    /// Create an HTTP provider from the configured RPC URL
    ///
    /// This method creates an alloy HTTP provider configured for AnyNetwork,
    /// which provides maximum compatibility across different blockchain networks.
    ///
    /// # Returns
    /// - `Ok(HttpProvider)`: Successfully created provider
    /// - `Err(EvmError)`: If the RPC URL is invalid
    ///
    /// # Errors
    /// Returns `InitError::InvalidRpcUrl` if the RPC URL cannot be parsed
    pub async fn get_provider(&self) -> Result<HttpProvider, EvmError> {
        let url = self.rpc_url.parse()
            .map_err(|_| InitError::InvalidRpcUrl("Failed to parse RPC URL".to_string()))?;
        let provider = ProviderBuilder::new().network::<AnyNetwork>().connect_http(url);
        Ok(provider)
    }

    /// Build a standard single-threaded EVM instance
    ///
    /// Creates an EVM instance using:
    /// - AlloyDB for blockchain state queries
    /// - WrapDatabaseAsync for async database operations
    /// - CacheDB for improved performance
    ///
    /// This is the recommended configuration for most use cases.
    ///
    /// # Returns
    /// - `Ok(AlloyTraceEvm<I>)`: Successfully created EVM instance
    /// - `Err(EvmError)`: If initialization fails
    ///
    /// # Errors
    /// - `InitError::InvalidRpcUrl`: Invalid RPC URL
    /// - `InitError::ChainIdFetchError`: Failed to fetch chain ID
    /// - `InitError::DatabaseError`: Database initialization failed
    ///
    /// # Example
    /// ```rust,no_run
    /// # use revm_trace_multi_thread::evm::builder::EvmBuilder;
    /// # use revm::inspector::NoOpInspector;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let evm = EvmBuilder::new(
    ///     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
    ///     NoOpInspector
    /// )
    /// .with_current_runtime()?
    /// .build()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> Result<AlloyTraceEvm<I>,EvmError> 
    where
        I: Inspector<MainnetContext<CacheAlloyDB>>,
    {
        let provider = self.get_provider().await?;
        let chain_id = provider.get_chain_id().await
            .map_err(|_| InitError::ChainIdFetchError("Failed to fetch chain ID".to_string()))?;
        let alloy_db = AlloyDBType::new(
            provider,
            BlockId::latest()
        );
        let wrap_db = if let Some(handle) = self.runtime_handle {
            WrapDatabaseAsync::with_handle(alloy_db, handle)
        } else {
            WrapDatabaseAsync::new(alloy_db)
                .ok_or(InitError::DatabaseError("Failed to wrap AlloyDB".to_string()))?
        };
        Ok(Self::build_internal(chain_id, wrap_db, self.inspector))
    }

    /// Build a multi-threaded EVM instance with shared backend
    ///
    /// Creates an EVM instance using foundry-fork-db's SharedBackend for:
    /// - Thread-safe state management
    /// - Concurrent transaction processing
    /// - Advanced fork and snapshot capabilities
    ///
    /// This configuration is ideal for:
    /// - High-throughput transaction simulation
    /// - Concurrent testing scenarios
    /// - Applications requiring state isolation
    ///
    /// # Feature Requirement
    /// This method is only available when the `multi-threading` feature is enabled.
    ///
    /// # Returns
    /// - `Ok(SharedTraceEvm<I>)`: Successfully created multi-threaded EVM
    /// - `Err(EvmError)`: If initialization fails
    ///
    /// # Errors
    /// - `InitError::InvalidRpcUrl`: Invalid RPC URL
    /// - `InitError::ChainIdFetchError`: Failed to fetch chain ID
    /// - Backend initialization errors from foundry-fork-db
    ///
    /// # Example
    /// ```rust,no_run
    /// # #[cfg(feature = "multi-threading")]
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use revm_trace_multi_thread::evm::builder::EvmBuilder;
    /// # use revm::inspector::NoOpInspector;
    /// let shared_evm = EvmBuilder::new(
    ///     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
    ///     NoOpInspector
    /// )
    /// .build_shared()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "multi-threading")]
    pub async fn build_shared(self) -> Result<SharedTraceEvm<I>, EvmError> 
    where
        I: Inspector<MainnetContext<SharedCacheDB>>,
    {
        use foundry_fork_db::{backend::SharedBackend, BlockchainDb, cache::BlockchainDbMeta};
        use std::sync::Arc;

        let provider = self.get_provider().await?;
        let chain_id = provider.get_chain_id().await
            .map_err(|_| InitError::ChainIdFetchError("Failed to fetch chain ID".to_string()))?;
        
        // Create blockchain database metadata with default BlockEnv
        // foundry-fork-db will handle the block environment internally
        let meta = BlockchainDbMeta::new(BlockEnv::default(), self.rpc_url.clone());
        let blockchain_db = BlockchainDb::new(meta, None); // None = use in-memory cache

        // Spawn a dedicated backend thread for handling database operations
        // This provides thread-safe access to blockchain state
        let shared_backend = SharedBackend::spawn_backend_thread(
            Arc::new(provider),
            blockchain_db,
            Some(BlockId::latest()) // Pin to the latest block
        );
        
        Ok(Self::build_internal(chain_id, shared_backend, self.inspector))
    }
}

impl EvmBuilder<NoOpInspector> {
    /// Create a new EVM builder with default NoOpInspector
    ///
    /// This is a convenience constructor for cases where you don't need
    /// custom transaction tracing and just want basic EVM execution.
    ///
    /// # Arguments
    /// - `rpc_url`: HTTP RPC endpoint URL (e.g., "https://eth-mainnet.g.alchemy.com/v2/your-key")
    ///
    /// # Returns
    /// A new `EvmBuilder` instance with `NoOpInspector`
    ///
    /// # Example
    /// ```rust
    /// use revm_trace_multi_thread::evm::builder::EvmBuilder;
    ///
    /// let builder = EvmBuilder::default_inspector(
    ///     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string()
    /// );
    /// ```
    pub fn default_inspector(rpc_url: String) -> Self {
        Self::new(rpc_url, NoOpInspector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test basic EVM builder functionality
    ///
    /// This test verifies that:
    /// - EvmBuilder can be created with a valid RPC URL
    /// - Runtime configuration works properly
    /// - EVM instance can be built successfully
    #[tokio::test(flavor = "multi_thread")]
    async fn test_evm_builder() {
        let rpc_url = "https://eth.llamarpc.com".to_string();
        let inspector = NoOpInspector;
        
        let builder = EvmBuilder::new(rpc_url, inspector)
            .with_current_runtime()
            .expect("Failed to get current runtime");

        let result = builder.build().await;
        assert!(result.is_ok(), "Failed to build EVM: {:?}", result.err());
    }

    /// Test shared EVM builder functionality (multi-threading feature)
    ///
    /// This test verifies that:
    /// - SharedBackend can be initialized properly
    /// - Multi-threaded EVM instances can be created
    /// - foundry-fork-db integration works correctly
    #[cfg(feature = "multi-threading")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shared_evm_builder() {
        let rpc_url = "https://eth.llamarpc.com".to_string();
        let inspector = NoOpInspector;
        
        let builder = EvmBuilder::new(rpc_url, inspector);
        let result = builder.build_shared().await;
        
        assert!(result.is_ok(), "Failed to build shared EVM: {:?}", result.err());
    }
}