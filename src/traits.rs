//! EVM inspector and environment configuration traits
//!
//! This module provides traits for:
//! - Accessing and resetting transaction inspectors
//! - Configuring block environment parameters
//! - Managing EVM state and execution context
//!
//! # Key Traits
//! - `GetTransactionTracer`: Access and reset inspector state
//! - `Reset`: Clear inspector data between transactions
//! - `BlockEnvConfig`: Configure block environment parameters

use revm::{Database, Inspector, Evm};
use std::any::Any;
use crate::trace::inspector::TransactionTracer;
use revm::inspectors::NoOpInspector;
use alloy::{eips::{BlockId, BlockNumberOrTag}, primitives::U256};
use crate::evm::EvmDb;

/// Trait for accessing and managing transaction inspectors
///
/// Provides methods to:
/// - Access the current inspector instance
/// - Reset inspector state between transactions
pub trait GetTransactionTracer<I: Inspector<DB>, DB: Database> {
    /// Returns a mutable reference to the current inspector if available
    fn get_inspector(&mut self) -> Option<&mut I>;
    
    /// Resets the inspector state if it implements Reset trait
    fn reset_inspector(&mut self) where I: Reset {
        if let Some(inspector) = self.get_inspector() {
            inspector.reset();
        }
    }
}

impl<'a, I, DB> GetTransactionTracer<I, DB> for Evm<'a, I, DB> 
where
    I: Inspector<DB> + 'static,
    DB: Database,
{
    fn get_inspector(&mut self) -> Option<&mut I> {
        (&mut self.context.external as &mut dyn Any).downcast_mut::<I>()
    }
}

/// Trait for resetting inspector state between transactions
///
/// Implementors should clear any accumulated state:
/// - Traces
/// - Transfers
/// - Logs
/// - Call stacks
pub trait Reset {
    /// Clears all accumulated state data
    fn reset(&mut self);
}

impl Reset for TransactionTracer {
    fn reset(&mut self) {
        self.traces = Vec::new();
        self.transfers = Vec::new();
        self.call_stack = Vec::new();
        self.logs = Vec::new();
    }
}

impl Reset for NoOpInspector {
    fn reset(&mut self) {
        // NoOpInspector has no state to reset
    }
}

/// Trait for configuring block environment parameters
///
/// Provides methods to set:
/// - Block number
/// - Block timestamp
/// - Combined block environment updates
pub trait BlockEnvConfig<I: Inspector<DB>, DB: Database> {
    /// Sets the block number for both EVM and database contexts
    ///
    /// # Arguments
    /// * `number` - Block number to set
    fn set_block_number(&mut self, number: u64) -> &mut Self;
    
    /// Sets the block timestamp in the EVM environment
    ///
    /// # Arguments
    /// * `timestamp` - Unix timestamp for the block
    fn set_block_timestamp(&mut self, timestamp: u64) -> &mut Self;
    
    /// Sets both block number and timestamp
    ///
    /// # Arguments
    /// * `number` - Block number to set
    /// * `timestamp` - Unix timestamp for the block
    fn set_block_env(&mut self, number: u64, timestamp: u64) -> &mut Self {
        self.set_block_number(number)
            .set_block_timestamp(timestamp)
    }
}

impl<'a, I> BlockEnvConfig<I, EvmDb> for Evm<'a, I, EvmDb>
where
    I: Inspector<EvmDb> + 'static,
{
    fn set_block_number(&mut self, number: u64) -> &mut Self {
        // Update EVM block environment
        self.block_mut().number = U256::from(number);
        
        // Update AlloyDB block number
        self.db_mut().0.set_block_number(BlockId::Number(BlockNumberOrTag::Number(number)));
        
        self
    }
    
    fn set_block_timestamp(&mut self, timestamp: u64) -> &mut Self {
        self.block_mut().timestamp = U256::from(timestamp);
        self
    }
} 