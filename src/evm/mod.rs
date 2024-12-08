//! Enhanced EVM implementation with tracing capabilities
//!
//! This module provides a wrapper around revm's EVM implementation with additional
//! features for transaction tracing, token transfer tracking, and call analysis.
//!
//! # Features
//! - HTTP and WebSocket provider support
//! - Transaction simulation and tracing
//! - Token transfer tracking
//! - Execution state management
//! - Customizable chain configuration
//!
//! # Examples
//!
//! ## Basic Usage with HTTP Provider
//! ```no_run
//! use revm_trace::evm::create_evm;
//! # use anyhow::Result;
//!
//! # async fn example() -> Result<()> {
//! // Create EVM instance with default inspector
//! let mut evm = create_evm(
//!     "https://eth-mainnet.g.alchemy.com/v2/your-api-key"
//! ).await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Using Custom Inspector with WebSocket
//! ```no_run
//! use revm_trace::evm::create_evm_ws;
//! use revm_trace::inspectors::TxInspector;
//! # use anyhow::Result;
//!
//! # async fn example() -> Result<()> {
//! // Create EVM instance with transaction tracing
//! let mut evm = create_evm_ws(
//!     "wss://eth-mainnet.g.alchemy.com/v2/your-api-key",
//!     TxInspector::new(),
//! ).await?;
//!
//! # Ok(())
//! # }
//! ```

use std::ops::{Deref, DerefMut};
use crate::types::*;
use alloy::{
    providers::Provider,
    transports::Transport,
};

pub mod inspector;
pub mod execution;
pub mod builder;
pub mod config;
pub use builder::*;

/// Enhanced EVM implementation with tracing capabilities
///
/// A newtype wrapper around REVM's EVM implementation that adds:
/// - Transaction tracing support
/// - Asset transfer tracking
/// - Execution state management
/// - Custom inspector integration
///
/// # Type Parameters
/// * `'a` - Inspector lifetime
/// * `T` - Transport implementation (HTTP/WebSocket)
/// * `P` - Provider implementation
/// * `I` - Inspector implementation
///
/// # Implementation Note
/// Uses newtype pattern to wrap `InspectorEvm` without runtime overhead
/// while providing a more focused API surface.
pub struct TraceEvm<'a, T, P, I>(InspectorEvm<'a, T, P, I>)
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a;


impl<'a, T, P, I> Deref for TraceEvm<'a, T, P, I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a,
{
    type Target = InspectorEvm<'a, T, P, I>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T, P, I> DerefMut for TraceEvm<'a, T, P, I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
