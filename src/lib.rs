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
//! - **Unified & Flexible EVM Construction**: Builder pattern for AlloyDB and Foundry-fork-db backends, with support for custom block height, inspector, and connection (HTTP/ws).
//! - **Customizable Inspector System**: Use built-in `TxInspector` or your own inspector for tracing, slot access, and security analysis.
//! - **Comprehensive Slot Access Tracking**: Every transaction and call trace node can recursively collect all storage slot reads/writes (SlotAccess), with type filtering (read/write/all), enabling full mutation history and attack forensics.
//! - **Batch & Multi-Threaded Simulation**: High-performance, concurrent simulation with shared cache (Foundry-fork-db backend).
//! - **Asset & Event Analysis**: Simulate and analyze multiple transactions, asset transfers (ETH/ERC20/NFT), and event logs in one batch.
//! - **Safe & Isolated Simulation**: All simulations are isolated—no real blockchain state is modified.
//! - **EVM-Compatible Chain Support**: Works with any EVM-compatible blockchain, not just Ethereum mainnet.
//! - **Rich Utility Functions**: Includes tools for batch querying token balances, simulating Multicall deployment and batch execution, and more.
//!
//! ### TxInspector Highlights
//!
//! - **Comprehensive Asset & Slot Access Tracking**: Tracks all ETH/ERC20/NFT transfers and every storage slot read/write (SlotAccess) globally and per call trace, with type filtering and full context.
//! - **Advanced Call Tree & Security Analysis**: Builds hierarchical call traces, pinpoints error locations, and enables step-by-step reconstruction of storage mutation history for attack forensics and Safe wallet auditing.
//! - **Event Log Collection**: Captures and parses all emitted events during simulation.
//! - **Error Investigation Tools**: Locates exact failure points in complex call chains, decodes revert reasons, and provides contract-specific error context.
//! - **Performance**: Optimized for both single transaction and batch processing scenarios.
//!
//! ## Module Structure
//!
//! - `evm`: Core EVM implementation with tracing capabilities
//! - `inspectors`: EVM execution inspectors for different analysis needs (see `TxInspector` and [TxInspector.md](../TxInspector.md) for full call trace and slot access design)
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
pub use evm::TraceEvm;

#[cfg(any(feature = "default", feature = "rustls-tls"))]
pub use evm::builder::{create_evm, create_evm_with_tracer, EvmBuilder};

pub use inspectors::tx_inspector::TxInspector;
pub use traits::*;
pub use types::{BlockEnv, SimulationBatch, SimulationTx};
pub use wrap_db::MyWrapDatabaseAsync;

// Re-export core libraries for convenience
pub use alloy;
pub use revm;

#[cfg(feature = "foundry-fork")]
pub use foundry_fork_db;

#[cfg(all(
    feature = "foundry-fork",
    any(feature = "default", feature = "rustls-tls")
))]
pub use evm::builder::fork_db::*;
