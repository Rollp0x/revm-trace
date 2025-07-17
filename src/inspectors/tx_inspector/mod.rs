//! Core transaction inspector implementation
//!
//! This module provides the main TxInspector type and its core functionality.
//! The inspector is responsible for tracking and collecting detailed information
//! about transaction execution in the EVM.
//!
//! # Architecture
//!
//! The implementation is split across several modules:
//! - `types`: Output and internal data structures
//! - `trace`: Call tracing and error tracking
//! - `inspector`: REVM Inspector trait implementation
//!
//! # Features
//!
//! - Complete transaction tracing
//! - Asset transfer tracking (ETH and ERC20)
//! - Call hierarchy reconstruction
//! - Error propagation tracking
//! - Event log collection

use crate::types::*;
use serde::Serialize;
mod inspector;
mod trace;
mod traits;
use alloy::primitives::{Address, Log};

/// Core transaction tracing inspector
///
/// Provides comprehensive transaction execution tracking by implementing
/// the REVM Inspector trait. Collects detailed information about:
///
/// - Asset transfers (ETH and ERC20 tokens)
/// - Contract calls and creations
/// - Call hierarchy and execution paths
/// - Event logs and error states
///
/// # State Management
///
/// The inspector maintains several internal collections to track execution:
/// - Transfers: Chronological list of all value movements
/// - Call traces: Complete tree of contract interactions
/// - Logs: All emitted events
/// - Call stack: Current execution path
/// - Address stack: Caller context for delegate calls
///
/// # Thread Safety
///
/// This inspector is designed to be Send + Sync safe for multi-threaded usage.
/// All internal state uses thread-safe types or types that can be safely
/// moved between threads.
#[derive(Default, Clone)]
pub struct TxInspector {
    /// Chronological record of all asset transfers during execution
    transfers: Vec<TokenTransfer>,
    /// Hierarchical tree of all contract calls and creations
    call_traces: Vec<CallTrace>,
    /// Sequential list of all emitted event logs
    logs: Vec<Log>,
    /// Stack tracking current position in call hierarchy
    call_stack: Vec<usize>,
    /// Stack maintaining caller context for delegate calls
    address_stack: Vec<Address>,
    /// Stack of pending contract creation transfers
    ///
    /// Tracks (transfer_index, transfer) pairs for each level of contract creation
    /// to properly handle nested contract creations. Operates in parallel with
    /// call_stack to maintain proper creation context.
    pending_create_transfers: Vec<(usize, TokenTransfer)>,
}

// 显式实现 Send 和 Sync 以确保多线程安全
unsafe impl Send for TxInspector {}
unsafe impl Sync for TxInspector {}

/// Complete transaction execution trace output
///
/// Aggregates all collected information during transaction execution:
/// - Asset movements (both ETH and tokens)
/// - Complete call hierarchy
/// - Event logs
/// - Error location if execution failed
#[derive(Debug, Clone, Serialize)]
pub struct TxTraceOutput {
    /// All asset transfers (ETH and tokens) during execution
    pub asset_transfers: Vec<TokenTransfer>,
    /// Complete hierarchical call tree
    pub call_trace: Option<CallTrace>,
    /// All emitted event logs
    pub logs: Vec<Log>,
    /// Location of the first error in the call tree
    pub error_trace_address: Option<Vec<usize>>,
}

impl TxInspector {
    /// Creates a new inspector instance with empty state
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns all recorded asset transfers in chronological order
    ///
    /// Includes both ETH transfers and ERC20 token transfers
    pub fn get_transfers(&self) -> &Vec<TokenTransfer> {
        &self.transfers
    }

    /// Returns the complete call trace tree
    ///
    /// The trace contains all contract interactions including:
    /// - Regular calls
    /// - Delegate calls
    /// - Contract creations
    pub fn get_traces(&self) -> &[CallTrace] {
        &self.call_traces
    }

    /// Returns all event logs emitted during execution
    ///
    /// Includes both regular events and ERC20 Transfer events
    pub fn get_logs(&self) -> &Vec<Log> {
        &self.logs
    }
}
