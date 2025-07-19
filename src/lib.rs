//! # REVM Transaction Simulator and Analyzer
//!
//! A high-performance, multi-threaded Rust library for EVM transaction simulation and analysis, built on [REVM](https://github.com/bluealloy/revm).
//!
//! ## Usage Patterns
//!
//! - **Quick Start**: Use `create_evm` or `create_evm_with_tracer` for fast EVM instantiation at the latest block.
//! - **Custom Block Height**: Use `EvmBuilder` for full control, including custom block height and inspector.
//! - **Change Block Context**: After creation, use `set_db_block` to update block context (this also resets the database cache for consistency).
//! - **Multi-Threaded Simulation**: Enable the `foundry-fork` feature for high-performance, thread-safe simulation with Foundry-fork-db backend.
//!
//! ## Key Features
//!
//! - **Flexible EVM Construction**: Unified builder pattern for AlloyDB and Foundry-fork-db backends.
//! - **Customizable Inspector System**: Use built-in `TxInspector` or your own inspector for tracing and analysis.
//! - **Multi-Threaded & High-Performance**: Foundry-fork-db backend enables safe, concurrent simulation with shared cache.
//! - **Batch Processing & Asset Analysis**: Simulate and analyze multiple transactions, including asset transfers and call traces.
//! - **Safe Simulation**: All simulations are isolated—no real blockchain state is modified.
//! - **EVM-Compatible Chain Support**: Works with any EVM-compatible blockchain, not just Ethereum mainnet.
//! - **Rich Utility Functions**: Includes tools for batch querying token balances, simulating Multicall deployment and batch execution, and more.
//! - **Flexible Connection**: Supports both HTTP and WebSocket (ws/wss) endpoints for EVM construction.
//! - **NFT (ERC721 & ERC1155) Transfer Analysis**: Automatically detects and parses NFT transfers, including tokenId extraction and type distinction.
//!
//! ### TxInspector Highlights
//!
//! - **Comprehensive Asset Transfer Tracking**: Automatically tracks ETH and ERC20 transfers with full context.
//! - **Advanced Call Tree Analysis**: Builds hierarchical call traces and pinpoints error locations.
//! - **Event Log Collection**: Captures and parses all emitted events during simulation.
//! - **Error Investigation Tools**: Locates exact failure points in complex call chains, decodes revert reasons, and provides contract-specific error context.
//! - **Performance**: Optimized for both single transaction and batch processing scenarios.
//!
//! ## Module Structure
//!
//! - `evm`: Core EVM implementation with tracing capabilities
//! - `inspectors`: EVM execution inspectors for different analysis needs
//! - `types`: Core data structures and type definitions
//! - `traits`: Trait definitions for extensibility
//! - `errors`: Error types and handling
//! - `utils`: Helper functions and utilities
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! revm-trace = "4.1.0"
//!
//! # TLS Backend Selection (choose one):
//! # Default: native-tls (OpenSSL) for maximum compatibility
//! # Alternative: Pure Rust TLS for system-dependency-free builds
//! # revm-trace = { version = "4.1.0", default-features = false, features = ["rustls-tls"] }
//! ```

pub mod errors;
pub mod evm;
pub mod inspectors;
pub mod traits;
pub mod types;
pub mod utils;
mod wrap_db;

// Re-export core types for easier access
pub use evm::{
    builder::{create_evm, create_evm_with_tracer, EvmBuilder},
    TraceEvm,
};
pub use inspectors::tx_inspector::TxInspector;
pub use traits::*;
pub use types::{BlockEnv, SimulationBatch, SimulationTx};
pub use wrap_db::MyWrapDatabaseAsync;

// Re-export core libraries for convenience
pub use alloy;
pub use revm;

#[cfg(feature = "foundry-fork")]
pub use foundry_fork_db;

#[cfg(feature = "foundry-fork")]
pub use evm::builder::fork_db::*;
