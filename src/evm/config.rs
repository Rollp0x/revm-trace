//! EVM environment configuration functionality
//! 
//! Provides methods for:
//! - Database state management
//! - Block environment configuration
//! - Chain parameters adjustment

use alloy::{
    eips::{BlockId, BlockNumberOrTag},
    primitives::U256,
    providers::Provider,
    transports::Transport,
};
use crate::types::BlockEnv;

use super::TraceEvm;

/// Environment configuration implementation
impl<'a, T, P, I> TraceEvm<'a, T, P, I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a,
{
    /// Resets the database cache while preserving the underlying provider
    /// 
    /// Clears all cached state:
    /// - Account states
    /// - Contract code
    /// - Event logs
    /// - Block hashes
    /// 
    /// # Usage
    /// - Automatically called between independent transactions in batch processing
    /// - Can be manually called to reset state
    /// 
    /// # Returns
    /// * `&mut Self` - Returns self for method chaining
    pub fn reset_db(&mut self) -> &mut Self {
        // Reset CacheDB state
        let cached_db = &mut self.db_mut().0;
        cached_db.accounts.clear();
        cached_db.contracts.clear();
        cached_db.logs = Vec::new();
        cached_db.block_hashes.clear();
        self
    }

    /// Sets the block environment parameters
    /// 
    /// Updates all block-related parameters:
    /// - Block number
    /// - Block timestamp
    /// - Database block reference
    /// 
    /// # Arguments
    /// * `block_env` - Block environment configuration containing:
    ///   - Block number
    ///   - Block timestamp
    /// 
    /// # Returns
    /// * `&mut Self` - Returns self for method chaining
    pub fn set_block_env(&mut self, block_env: BlockEnv) -> &mut Self {
        self.block_mut().number = U256::from(block_env.number);
        self.block_mut().timestamp = U256::from(block_env.timestamp);
        self.db_mut().0.db.set_block_number(BlockId::Number(
            BlockNumberOrTag::Number(block_env.number)
        ));
        self
    }

    /// Sets the block number for the current environment
    /// 
    /// Updates both:
    /// - EVM block number
    /// - Database block reference
    /// 
    /// # Arguments
    /// * `block_number` - New block number to set
    /// 
    /// # Returns
    /// * `&mut Self` - Returns self for method chaining
    pub fn set_block_number(&mut self, block_number: u64) -> &mut Self {
        self.block_mut().number = U256::from(block_number);
        self.db_mut().0.db.set_block_number(BlockId::Number(
            BlockNumberOrTag::Number(block_number)
        ));
        self
    }

    /// Sets the block timestamp for the current environment
    /// 
    /// # Arguments
    /// * `timestamp` - New timestamp value in seconds
    /// 
    /// # Returns
    /// * `&mut Self` - Returns self for method chaining
    pub fn set_block_timestamp(&mut self, timestamp: u64) -> &mut Self {
        self.block_mut().timestamp = U256::from(timestamp);
        self
    }
}