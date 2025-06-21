//! Core EVM wrapper and execution engine
//!
//! This module provides the `TraceEvm` wrapper around revm's `MainnetEvm` with enhanced
//! tracing capabilities and convenient type aliases for different database configurations.
//!
//! ## Key Components
//!
//! - **`TraceEvm`**: Main wrapper struct that adds tracing capabilities to revm's EVM
//! - **Type Aliases**: Convenient types for different database and inspector configurations
//! - **Database Reset**: Utilities for clearing cache state between executions
//!
//! ## Usage Examples
//!
//! ```rust,no_run
//! use revm_trace::evm::{TraceEvm, builder::EvmBuilder};
//! use revm::inspector::NoOpInspector;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a default EVM instance
//! let evm = EvmBuilder::default_inspector(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string()
//! )
//! .with_current_runtime()?
//! .build()
//! .await?;
//!
//! // Or create a multi-threaded EVM
//! #[cfg(feature = "multi-threading")]
//! let shared_evm = EvmBuilder::default_inspector(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string()
//! )
//! .build_shared()
//! .await?;
//! # Ok(())
//! # }
//! ```


use revm::database::{CacheDB,DatabaseRef};
pub use revm::{
    inspector::{NoOpInspector, Inspector},
    handler::MainnetContext,
    MainnetEvm,
    context_interface::ContextTr,
    database::Database
};
use std::ops::{Deref, DerefMut};
use crate::ResetDB;
// Sub-modules for EVM functionality
pub mod builder;
pub mod processor;
pub mod inspector;

/// Enhanced EVM wrapper with tracing capabilities
///
/// `TraceEvm` is a wrapper around revm's `MainnetEvm` that provides:
/// - Transparent access to all EVM functionality via `Deref`/`DerefMut`
/// - Enhanced tracing and inspection capabilities
/// - Database state management utilities
/// - Type-safe database and inspector configuration
///
/// # Type Parameters
/// - `DB`: Database backend implementing the `Database` trait
/// - `INSP`: Inspector for transaction tracing and analysis
///
/// # Examples
///
/// ```rust,no_run
/// use revm_trace::evm::{TraceEvm, builder::EvmBuilder};
/// use revm::inspector::NoOpInspector;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let evm = EvmBuilder::default_inspector(
///     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string()
/// )
/// .build()
/// .await?;
///
/// // Use evm for transaction execution...
/// # Ok(())
/// # }
/// ```
pub struct TraceEvm<DB:Database, INSP>
(
    MainnetEvm<
        MainnetContext<DB>,
        INSP
    >
);

 
impl <DB,INSP> TraceEvm<DB, INSP>
where
    DB: Database,
{
    /// Create a new TraceEvm instance from a MainnetEvm
    ///
    /// This constructor wraps an existing `MainnetEvm` instance to provide
    /// enhanced tracing capabilities while maintaining full compatibility
    /// with the underlying EVM interface.
    ///
    /// # Arguments
    /// - `evm`: A configured `MainnetEvm` instance
    ///
    /// # Returns
    /// A new `TraceEvm` wrapper instance
    ///
    /// # Example
    /// ```no_run
    /// use revm_trace::{TraceEvm, EvmBuilder, TxInspector};
    /// use revm::{Context, MainnetEvm, inspector::NoOpInspector};
    /// use revm::handler::MainnetContext;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Method 1: Using EvmBuilder (recommended)
    /// let inspector = TxInspector::new();
    /// let builder = EvmBuilder::new("https://eth.llamarpc.com".to_string(), inspector);
    /// let trace_evm = builder.build().await?; // Already returns TraceEvm
    ///
    /// // Method 2: Wrapping an existing MainnetEvm
    /// // let ctx = Context::build_mainnet().with_db(...);
    /// // let evm = ctx.build_mainnet_with_inspector(NoOpInspector);
    /// // let trace_evm = TraceEvm::new(evm);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(evm: MainnetEvm<MainnetContext<DB>, INSP>) -> Self {
        Self(evm)
    }
}

/// Transparent access to the underlying MainnetEvm
///
/// This implementation allows `TraceEvm` to be used as a drop-in replacement
/// for `MainnetEvm` by providing direct access to all its methods and fields.
impl <DB,INSP> Deref for TraceEvm<DB, INSP>
where
    DB: Database,
{
    type Target = MainnetEvm<MainnetContext<DB>, INSP>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}   

/// Mutable access to the underlying MainnetEvm
///
/// This implementation allows modification of the underlying EVM state
/// and configuration through the `TraceEvm` wrapper.
impl <DB,INSP> DerefMut for TraceEvm<DB, INSP>
where
    DB: Database,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}



// ========================= Database Management =========================

/// Implementation for TraceEvm instances with CacheDB
///
/// Provides database cache management utilities specifically for EVM instances
/// that use `CacheDB` as their database layer.
impl<DB, INSP> ResetDB for TraceEvm<CacheDB<DB>, INSP> 
where
    DB: DatabaseRef,
{
    /// Reset the database cache to clear all cached state
    ///
    /// This method clears all cached data from the `CacheDB` layer, including:
    /// - Account states and balances
    /// - Contract bytecode and storage
    /// - Event logs
    /// - Block hashes
    ///
    /// # Use Cases
    /// - Resetting state between independent transaction simulations
    /// - Clearing cache when switching to a different block context
    /// - Memory management in long-running applications
    /// - Testing scenarios requiring clean state
    ///
    /// # Performance Impact
    /// After calling this method, subsequent database queries will need to
    /// fetch data from the underlying database layer, which may be slower
    /// until the cache is repopulated.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use revm_trace::evm::AlloyTraceEvm;
    /// # let mut evm: AlloyTraceEvm = todo!();
    /// // Clear cache before processing a new batch of transactions
    /// evm.reset_db();
    /// ```
    fn reset_db(&mut self){
        let cached_db = &mut self.0.ctx.db().cache;
        cached_db.accounts.clear();
        cached_db.contracts.clear();
        cached_db.logs = Vec::new();
        cached_db.block_hashes.clear();
    }
}