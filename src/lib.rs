//! # REVM Transaction Simulator and Analyzer
//!
//! A library for simulating EVM transactions and analyzing their execution results.
//!
//! ## Core Features
//!
//! - **Transaction Simulation**
//!   - Batch transaction processing
//!   - Stateful and stateless execution modes
//!   - Detailed execution traces with inspector support
//!   - Error handling and propagation
//!
//! - **Multi-Protocol Support**
//!   - HTTP and WebSocket RPC connections
//!   - Automatic protocol detection
//!   - Any EVM-compatible chain support
//!   - Built-in rustls TLS support for cross-platform compatibility
//!
//! - **Flexible Inspector System**
//!   - Custom transaction tracers
//!   - NoOp inspector for simple execution
//!   - Extensible inspector trait system
//!
//! ## Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! revm-trace = "3.0.0"
//! ```
//!
//! ### TLS Backend Selection
//!
//! **Important**: Choose only one TLS backend:
//!
//! ```toml
//! # Option 1: Default - uses native-tls (OpenSSL) for maximum compatibility
//! revm-trace = "3.0.0"
//!
//! # Option 2: Pure Rust TLS with rustls for system-dependency-free builds
//! revm-trace = { version = "3.0.0", default-features = false, features = ["rustls-tls"] }
//! ```
//!
//! The library uses rustls for TLS connections, providing excellent cross-platform
//! compatibility without requiring OpenSSL or other system TLS libraries.
//! 
//! ## Example Usage
//!
//! ### Basic Transaction Execution
//!
//! ```no_run
//! use revm_trace::{
//!     create_evm, 
//!     types::{SimulationTx, SimulationBatch},
//! };
//! use alloy::primitives::{address, U256, TxKind};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create EVM instance
//! let mut evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-api-key").await?;
//!
//! // Create simulation transaction
//! let tx = SimulationTx {
//!     caller: address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61"),
//!     transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
//!     value: U256::from(120000000000000000u64), // 0.12 ETH
//!     data: vec![].into(),
//! };
//!
//! // Create batch with single transaction
//! let batch = SimulationBatch {
//!     block_env: None,
//!     transactions: vec![tx],
//!     is_stateful: false,
//! };
//!
//! // Execute transaction batch (simple execution)
//! let results = evm.execute_batch(batch);
//!
//! // Process results
//! for result in results {
//!     match result {
//!         Ok(execution_result) => {
//!             println!("Transaction succeeded!");
//!             println!("Gas used: {}", execution_result.gas_used());
//!         }
//!         Err(e) => {
//!             println!("Transaction failed: {:?}", e);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Advanced Usage with Custom Tracer
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
//! // Create EVM with custom tracer
//! let tracer = TxInspector::new();
//! let mut evm = create_evm_with_tracer(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-api-key", 
//!     tracer
//! ).await?;
//!
//! // Create simulation transaction
//! let tx = SimulationTx {
//!     caller: address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61"),
//!     transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
//!     value: U256::from(120000000000000000u64), // 0.12 ETH
//!     data: vec![].into(),
//! };
//!
//! // Create batch
//! let batch = SimulationBatch {
//!     block_env: None,
//!     transactions: vec![tx],
//!     is_stateful: false,
//! };
//!
//! // Execute with tracing
//! let results = evm.trace_transactions(batch);
//!
//! // Process results with inspector output
//! for result in results {
//!     match result {
//!         Ok((execution_result, inspector_output)) => {
//!             println!("Transaction succeeded!");
//!             println!("Gas used: {}", execution_result.gas_used());
//!             
//!             // Process inspector output
//!             for transfer in inspector_output.asset_transfers {
//!                 println!(
//!                     "Transfer: {} from {} to {:?}",
//!                     transfer.value, transfer.from, transfer.to
//!                 );
//!             }
//!         }
//!         Err(e) => {
//!             println!("Transaction failed: {:?}", e);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### WebSocket Connection
//!
//! ```no_run
//! use revm_trace::create_evm;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create EVM with WebSocket connection (auto-detected from URL)
//! let mut evm = create_evm("wss://mainnet.gateway.tenderly.co").await?;
//!
//! // Use the same way as HTTP connections
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

// Re-export core types for easier access
pub use inspectors::tx_inspector::TxInspector;
pub use evm::{TraceEvm, builder::*};
pub use types::{BlockEnv, SimulationTx, SimulationBatch};
pub use traits::*;

// Re-export core libraries for convenience
pub use revm;
pub use alloy;
pub use foundry_fork_db;