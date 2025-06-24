//! Core EVM wrapper and execution engine
//!
//! This module provides the `TraceEvm` wrapper around revm's `MainnetEvm` with enhanced
//! tracing capabilities and convenient type aliases for different database configurations.
//!
//! ## Key Components
//!
//! - **`TraceEvm`**: Main wrapper struct that adds tracing capabilities to revm's EVM
//! - **Database Reset**: Utilities for clearing cache state between executions
//! - **Inspector Integration**: Support for transaction tracing and analysis
//!
//! ## Usage Examples
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use revm_trace::create_evm;
//!
//! // Create a default EVM instance (no tracing)
//! let evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-key").await?;
//!
//! // Create an EVM with custom tracer
//! use revm_trace::{create_evm_with_tracer, TxInspector};
//! let tracer = TxInspector::new();
//! let evm_with_tracer = create_evm_with_tracer(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-key", 
//!     tracer
//! ).await?;
//! # Ok(())
//! # }
//! ```


pub use revm::{
    inspector::{NoOpInspector, Inspector},
    handler::MainnetContext,
    MainnetEvm,
    context_interface::ContextTr,
    database::Database
};
use std::ops::{Deref, DerefMut};

// Sub-modules for EVM functionality
pub mod builder;
pub mod processor;
pub mod inspector;
pub mod reset;

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
/// # Usage Patterns
///
/// `TraceEvm` supports two main usage patterns:
///
/// ## 1. Convenience Functions (Recommended for most users)
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::{create_evm_with_tracer, TxInspector, types::SimulationBatch, traits::TransactionTrace};
/// 
/// let tracer = TxInspector::new();
/// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", tracer).await?;
/// 
/// // Create a sample batch (empty for demo)
/// let batch = SimulationBatch {
///     transactions: vec![],
///     is_stateful: false,
/// };
/// 
/// // High-level batch processing with automatic state management
/// let results = evm.trace_transactions(batch);
/// # Ok(())
/// # }
/// ```
///
/// ## 2. Manual Control (Advanced users)
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::{create_evm_with_tracer, TxInspector};
/// use revm::context::TxEnv;
/// use revm::{ExecuteEvm, InspectCommitEvm};
/// use alloy::primitives::{address, U256, TxKind};
/// 
/// let tracer = TxInspector::new();
/// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", tracer).await?;
/// 
/// // Create a sample transaction
/// let tx = TxEnv::builder()
///     .caller(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
///     .kind(TxKind::Call(address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")))
///     .chain_id(Some(evm.cfg.chain_id))
///     .value(U256::ZERO)
///     .build_fill();
/// 
/// // Manual transaction execution with fine-grained control
/// evm.set_tx(tx);
/// let result = evm.inspect_replay_commit()?;  // Explicit Inspector activation
/// 
/// // Access Inspector data at any time
/// let transfers = evm.get_inspector().get_transfers();
/// let traces = evm.get_inspector().get_traces();
/// 
/// // Manual state management
/// evm.reset_inspector();  // Clear state for next transaction
/// # Ok(())
/// # }
/// ```
///
/// **Important**: Modern REVM requires explicit `inspect_replay_commit()` calls to activate 
/// Inspector hooks. The convenience functions like `trace_transactions()` automate this process.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::create_evm;
///
/// // Create a basic EVM instance
/// let evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-key").await?;
///
/// // Create an EVM with custom tracer
/// use revm_trace::{create_evm_with_tracer, TxInspector};
/// let tracer = TxInspector::new();
/// let evm_with_tracer = create_evm_with_tracer(
///     "https://eth-mainnet.g.alchemy.com/v2/your-key", 
///     tracer
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct TraceEvm<DB: Database, INSP>(
    MainnetEvm<MainnetContext<DB>, INSP>
);

impl<DB, INSP> TraceEvm<DB, INSP>
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
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use revm_trace::{create_evm_with_tracer, TxInspector};
    /// use revm::{Context, MainnetEvm};
    ///
    /// // Recommended: Use the builder functions
    /// let tracer = TxInspector::new();
    /// let trace_evm = create_evm_with_tracer(
    ///     "https://eth.llamarpc.com", 
    ///     tracer
    /// ).await?; // Already returns TraceEvm
    ///
    /// // Alternative: Wrap an existing MainnetEvm (advanced usage)
    /// // let ctx = Context::mainnet().with_db(...);
    /// // let evm = ctx.build_mainnet_with_inspector(tracer);
    /// // let trace_evm = TraceEvm::new(evm);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(evm: MainnetEvm<MainnetContext<DB>, INSP>) -> Self {
        Self(evm)
    }

    /// Get direct access to the inspector instance
    ///
    /// This method provides direct access to the Inspector for cases where
    /// you need to call inspector-specific methods. The returned reference
    /// allows you to access all methods specific to your inspector type.
    ///
    /// **Note**: You can also access the inspector directly via `&self.inspector`
    /// due to the `Deref` implementation, but this method provides explicit access
    /// for better code clarity.
    ///
    /// # Examples
    /// 
    /// ## With TxInspector
    /// ```no_run
    /// use revm_trace::{create_evm_with_tracer, TxInspector};
    /// 
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let trace_inspector = TxInspector::new();
    /// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", trace_inspector).await?;
    /// 
    /// // After processing transactions, get direct access to TxInspector methods
    /// let inspector = evm.get_inspector();
    /// 
    /// // TxInspector-specific methods
    /// let transfers = inspector.get_transfers();
    /// let traces = inspector.get_traces();
    /// let logs = inspector.get_logs();
    /// 
    /// // Advanced error tracing methods
    /// let error_addr = inspector.get_error_trace_address();
    /// let error_trace = inspector.find_error_trace();
    /// 
    /// println!("Collected {} transfers", transfers.len());
    /// println!("Collected {} call traces", traces.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## With Custom Inspector
    /// ```no_run
    /// use revm_trace::create_evm_with_tracer;
    /// use revm::inspector::NoOpInspector;
    /// 
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let inspector = NoOpInspector;
    /// let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", inspector).await?;
    /// 
    /// // Access the NoOpInspector (though it has no specific methods)
    /// let inspector_ref = evm.get_inspector();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Returns
    /// A reference to the internal inspector instance with all its specific methods
    pub fn get_inspector(&self) -> &INSP {
        &self.inspector
    }
}

/// Transparent access to the underlying MainnetEvm
///
/// This implementation allows `TraceEvm` to be used as a drop-in replacement
/// for `MainnetEvm` by providing direct access to all its methods and fields.
impl<DB, INSP> Deref for TraceEvm<DB, INSP>
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
impl<DB, INSP> DerefMut for TraceEvm<DB, INSP>
where
    DB: Database,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
