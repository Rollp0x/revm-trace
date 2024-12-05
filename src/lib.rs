//! # REVM Transaction Simulator and Analyzer
//!
//! A library for simulating EVM transactions and analyzing their execution results.
//!
//! ## Core Features
//!
//! - **Transaction Simulation**
//!   - Detailed execution traces
//!   - Asset transfer tracking
//!   - Call hierarchy analysis
//!   - Error origin detection
//!
//! - **Asset Tracking**
//!   - Native token transfers
//!   - ERC20 token transfers
//!   - Token metadata collection
//!
//! - **Execution Analysis**
//!   - Complete call traces
//!   - Event logs collection
//!   - State change tracking
//!   - Comprehensive status reporting
//!   - Error propagation analysis
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use revm_trace::{
//!     create_evm,
//!     BlockEnv,
//!     SimulationTx,
//!     SimulationBatch,
//!     Tracer,
//!     TransactionStatus,
//! };
//! use alloy::primitives::{address, U256, TxKind};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Initialize EVM
//! let mut evm = create_evm(
//!     "https://rpc.ankr.com/eth",
//!     Some(1),  // Ethereum mainnet
//!     None,     // No custom configs
//! )?;
//!
//! // Create simulation transaction
//! let tx = SimulationTx {
//!     caller: address!("dead00000000000000000000000000000000beef"),
//!     transact_to: TxKind::Call(address!("dac17f958d2ee523a2206206994597c13d831ec7")),
//!     value: U256::from(1000000000000000000u64), // 1 ETH
//!     data: vec![].into(),
//! };
//!
//! // Execute transaction
//! let result = evm.trace_tx(
//!     tx,
//!     BlockEnv {
//!         number: 18000000,
//!         timestamp: 1700000000,
//!     },
//! )?;
//!
//! // Process results
//! match result.execution_status() {
//!     TransactionStatus::Success => {
//!         println!("Transaction succeeded!");
//!         for transfer in result.asset_transfers {
//!             println!(
//!                 "Transfer: {} from {} to {}",
//!                 transfer.value, transfer.from, transfer.to
//!             );
//!         }
//!     }
//!     TransactionStatus::PartialSuccess => {
//!         println!("Transaction succeeded with some internal errors");
//!     }
//!     TransactionStatus::Failed { error, origin_error } => {
//!         println!("Transaction failed: {}", error);
//!         if let Some(origin) = origin_error {
//!             println!("Original error: {}", origin);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Module Structure
//!
//! - `evm`: EVM initialization and configuration
//! - `trace`: Transaction tracing implementation
//! - `inspector`: EVM execution inspector
//! - `types`: Core data structures
//! - `utils`: Helper functions

pub mod types;
pub mod evm;
pub mod trace;
pub mod utils;
mod inspector;

// Re-export only the essential types and functions
pub use evm::{create_evm,TraceEvm};
pub use trace::Tracer;
pub use types::{
    BlockEnv,
    SimulationTx,
    SimulationBatch,
    TraceResult,
    TokenTransfer,
    TokenConfig,
    ExecutionStatus,
    FailureKind,
    TransactionStatus
};