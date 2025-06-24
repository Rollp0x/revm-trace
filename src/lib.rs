//! # REVM Transaction Simulator and Analyzer
//!
//! A high-performance library for simulating EVM transactions with comprehensive tracing capabilities.
//!
//! ## 🎯 Three Usage Modes
//!
//! REVM-Trace provides three distinct usage patterns to suit different needs:
//!
//! ### 1. 🚀 Simple Execution Mode (No Tracing)
//! - **Use case**: Gas estimation, basic simulation, high-throughput scenarios
//! - **Inspector**: `NoOpInspector` (output: `()`)
//! - **Performance**: Fastest - zero tracing overhead  
//! - **API**: `create_evm()` + `execute_batch()`
//!
//! ### 2. 🔧 Manual Inspector Control (Advanced)
//! - **Use case**: Custom tracing logic, debugging, research, fine-grained control
//! - **Inspector**: Any custom inspector (e.g., `TxInspector`)
//! - **Performance**: Full control over data collection and state management
//! - **API**: `create_evm_with_tracer()` + manual `inspect_replay_commit()`
//!
//! ### 3. 🎯 Automatic Batch Processing (Convenience)
//! - **Use case**: Standard trace analysis, automated workflows
//! - **Inspector**: Must implement `TraceOutput` trait (e.g., `TxInspector`)
//! - **Performance**: Automatic state management with predictable overhead
//! - **API**: `create_evm_with_tracer()` + `trace_transactions()`
//!
//! ## ⚠️ Critical REVM API Changes
//!
//! **Modern REVM Requirement**: Inspector execution must be explicitly activated!
//!
//! - ❌ **Old REVM**: `evm.transact(tx)` would automatically trigger Inspector hooks
//! - ✅ **New REVM**: `evm.transact(tx)` does **NOT** execute Inspector - you get raw execution only
//! - ✅ **New REVM**: Must call `evm.inspect_replay_commit()` to activate Inspector
//!
//! This change enables:
//! - Better performance control (skip tracing when not needed)
//! - Explicit separation between execution and analysis
//! - Flexible inspector activation patterns
//!
//! ## Core Features
//!
//! - **Multi-Protocol Support**: HTTP and WebSocket RPC with automatic detection
//! - **Multi-Threading Ready**: Thread-safe design for concurrent processing
//! - **Flexible Inspector System**: From NoOp to custom tracers
//! - **Comprehensive Tracing**: Asset transfers, call trees, event logs, error analysis
//! - **State Management**: Stateful and stateless execution modes
//! - **Error Handling**: Detailed error propagation and analysis
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! revm-trace = "3.1.1"
//!
//! # TLS Backend Selection (choose one):
//! # Default: native-tls (OpenSSL) for maximum compatibility
//! # Alternative: Pure Rust TLS for system-dependency-free builds
//! # revm-trace = { version = "3.1.1", default-features = false, features = ["rustls-tls"] }
//! ```
//!
//! ## Usage Examples
//!
//! ### Mode 1: Simple Execution (Fastest)
//!
//! Perfect for gas estimation and basic simulation without tracing overhead.
//!
//! ```no_run
//! use revm_trace::{create_evm, types::{SimulationTx, SimulationBatch}};
//! use alloy::primitives::{address, U256, TxKind};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create EVM with NoOpInspector (fastest mode)
//! let mut evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-api-key").await?;
//!
//! let batch = SimulationBatch {
//!     block_env: None,
//!     transactions: vec![SimulationTx {
//!         caller: address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61"),
//!         transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
//!         value: U256::from(120000000000000000u64), // 0.12 ETH
//!         data: vec![].into(),
//!     }],
//!     is_stateful: false,
//! };
//!
//! // Execute without tracing (simple execution only)
//! let results = evm.execute_batch(batch);
//! for result in results {
//!     match result {
//!         Ok(execution_result) => {
//!             println!("✅ Success! Gas: {}", execution_result.gas_used());
//!         }
//!         Err(e) => println!("❌ Failed: {:?}", e),
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Mode 2: Manual Inspector Control (Advanced)
//!
//! For users who need fine-grained control over tracing and inspector state.
//!
//! ```no_run
//! use revm_trace::{create_evm_with_tracer, TxInspector};
//! use alloy::primitives::{address, U256, TxKind};
//! use revm::context::TxEnv;
//! use revm::{ExecuteEvm, InspectCommitEvm};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let inspector = TxInspector::new();
//! let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", inspector).await?;
//!
//! // Manual workflow: Full control over inspector
//! let tx = TxEnv::builder()
//!     .caller(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
//!     .kind(TxKind::Call(address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")))
//!     .value(U256::ZERO)
//!     .chain_id(Some(evm.cfg.chain_id))
//!     .build_fill();
//!
//! // Step 1: Set transaction
//! evm.set_tx(tx);
//!
//! // Step 2: CRITICAL - Explicit Inspector activation (modern REVM requirement!)
//! // Note: evm.transact() would NOT execute the inspector
//! let result = evm.inspect_replay_commit()?;
//!
//! // Step 3: Access TxInspector-specific methods anytime
//! let inspector = evm.get_inspector();
//! let transfers = inspector.get_transfers();
//! let traces = inspector.get_traces();
//! let logs = inspector.get_logs();
//! let error_location = inspector.get_error_trace_address();
//!
//! println!("🔍 Transfers: {}, Traces: {}, Logs: {}", 
//!          transfers.len(), traces.len(), logs.len());
//!
//! if let Some(error_addr) = error_location {
//!     println!("❌ Error at: {:?}", error_addr);
//! }
//!
//! // Step 4: Manual state management (optional)
//! evm.reset_inspector();  // Clear for next transaction
//! # Ok(())
//! # }
//! ```
//!
//! ### Mode 3: Automatic Batch Processing (Convenience)
//!
//! High-level API with automatic state management - perfect for standard workflows.
//!
//! ```no_run
//! use revm_trace::{
//!     create_evm_with_tracer, TxInspector,
//!     types::{SimulationTx, SimulationBatch},
//!     traits::TransactionTrace,
//! };
//! use alloy::primitives::{address, U256, TxKind};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create EVM with tracer (required for Mode 3)
//! let tracer = TxInspector::new();
//! let mut evm = create_evm_with_tracer(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-api-key", 
//!     tracer
//! ).await?;
//!
//! let batch = SimulationBatch {
//!     block_env: None,
//!     transactions: vec![SimulationTx {
//!         caller: address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61"),
//!         transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
//!         value: U256::from(120000000000000000u64),
//!         data: vec![].into(),
//!     }],
//!     is_stateful: false,
//! };
//!
//! // Automatic batch processing with tracing
//! // Internally handles inspect_replay_commit() for each transaction
//! let results = evm.trace_transactions(batch);
//!
//! for result in results {
//!     match result {
//!         Ok((execution_result, inspector_output)) => {
//!             println!("✅ Success! Gas: {}", execution_result.gas_used());
//!             
//!             // Automatic TraceOutput collection
//!             println!("📊 Transfers: {}", inspector_output.asset_transfers.len());
//!             println!("🌲 Call tree: {:?}", inspector_output.call_trace.is_some());
//!             println!("📝 Logs: {}", inspector_output.logs.len());
//!             
//!             if let Some(error_addr) = inspector_output.error_trace_address {
//!                 println!("❌ Error at: {:?}", error_addr);
//!             }
//!         }
//!         Err(e) => println!("❌ Failed: {:?}", e),
//!     }
//! }
//! // Inspector state automatically reset between transactions
//! # Ok(())
//! # }
//! ```
//!
//! ## 🤔 Which Mode Should I Use?
//!
//! | Scenario | Mode | Reason |
//! |----------|------|---------|
//! | Gas estimation, basic simulation | **Mode 1** | Fastest, zero overhead |
//! | Custom tracing, debugging, research | **Mode 2** | Full control, all inspector methods |
//! | Standard trace analysis, automation | **Mode 3** | Clean API, automatic management |
//! | High-throughput processing | **Mode 1** or **Mode 2** | Avoid TraceOutput overhead |
//! | Inspector development | **Mode 2** | Direct access to internals |
//!
//! ## Connection Support
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use revm_trace::create_evm;
//!
//! // HTTP providers (auto-detected)
//! let evm_http = create_evm("https://eth.llamarpc.com").await?;
//! let evm_alchemy = create_evm("https://eth-mainnet.g.alchemy.com/v2/key").await?;
//!
//! // WebSocket providers (auto-detected)
//! let evm_ws = create_evm("wss://mainnet.gateway.tenderly.co").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Module Structure
//!
//! - `evm`: Core EVM implementation with tracing capabilities
//! - `inspectors`: EVM execution inspectors for different analysis needs
//! - `types`: Core data structures and type definitions
//! - `traits`: Trait definitions for extensibility
//! - `errors`: Error types and handling
//! - `utils`: Helper functions and utilities

pub mod types;
pub mod evm;
pub mod utils;
pub mod traits;
pub mod inspectors;
pub mod errors;
mod wrap_db;

// Re-export core types for easier access
pub use inspectors::tx_inspector::TxInspector;
pub use evm::{TraceEvm, builder::{
    create_evm, create_evm_with_tracer,EvmBuilder
}};
pub use types::{BlockEnv, SimulationTx, SimulationBatch};
pub use traits::*;
pub use wrap_db::MyWrapDatabaseAsync;

// Re-export core libraries for convenience
pub use revm;
pub use alloy;

#[cfg(feature = "foundry-fork")]
pub use foundry_fork_db;

#[cfg(feature = "foundry-fork")]
pub use evm::builder::fork_db::*;
