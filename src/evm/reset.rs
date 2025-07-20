use crate::{
    errors::EvmError,
    traits::{ResetBlock, ResetDB},
    types::AllDBType,
    TraceEvm,
};
use alloy::eips::{BlockId, BlockNumberOrTag};
use revm::{
    context::BlockEnv,
    context_interface::ContextTr,
    database::{CacheDB, DatabaseRef},
    ExecuteEvm,
};
// ========================= Database Management =========================

/// Implementation for TraceEvm instances with CacheDB
///
/// Provides database cache management utilities specifically for EVM instances
/// that use `CacheDB` as their database layer.
impl<DB, INSP> ResetDB for TraceEvm<CacheDB<DB>, INSP>
where
    DB: DatabaseRef,
{
    /// Reset the database cache to clear all cached state
    ///
    /// This method clears all cached data from the `CacheDB` layer, including:
    /// - Account states and balances
    /// - Contract bytecode and storage
    /// - Event logs
    /// - Block hashes
    ///
    /// # Use Cases
    /// - Resetting state between independent transaction simulations
    /// - Clearing cache when switching to a different block context
    /// - Memory management in long-running applications
    /// - Testing scenarios requiring clean state
    ///
    /// # Performance Impact
    /// After calling this method, subsequent database queries will need to
    /// fetch data from the underlying database layer, which may be slower
    /// until the cache is repopulated.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use revm_trace::{create_evm, traits::ResetDB};
    ///
    /// let mut evm = create_evm("https://eth.llamarpc.com").await?;
    ///
    /// // Clear cache before processing a new batch of transactions
    /// evm.reset_db();
    /// # Ok(())
    /// # }
    /// ```
    fn reset_db(&mut self) {
        let cached_db = &mut self.0.ctx.db().cache;
        cached_db.accounts.clear();
        cached_db.contracts.clear();
        cached_db.logs = Vec::new();
        cached_db.block_hashes.clear();
    }
}

impl ResetBlock for AllDBType {
    type Error = EvmError;
    fn reset_block(&mut self, block_number: u64) -> Result<(), EvmError> {
        // Reset the block number in the EVM context
        let db = self.get_db_mut();
        db.set_block_number(BlockId::Number(BlockNumberOrTag::Number(block_number)));
        Ok(())
    }
}

// Generic set_db_block implementation for any database type implementing ResetBlock
impl<DB, INSP> TraceEvm<CacheDB<DB>, INSP>
where
    DB: DatabaseRef + ResetBlock,
    <DB as ResetBlock>::Error: Into<EvmError>,
{
    /// Reset the underlying database to a specific block, clear cache, and update EVM context
    ///
    /// Steps:
    /// 1. Reset the underlying database's block state
    /// 2. Clear the outer CacheDB cache
    /// 3. Update the EVM's block context
    pub fn set_db_block(&mut self, block_env: BlockEnv) -> Result<(), EvmError> {
        // Step 1: Reset the underlying database's block state
        {
            let cache_db = &mut self.0.ctx.db().db;
            cache_db.reset_block(block_env.number).map_err(Into::into)?;
        }
        // Step 2: Clear the outer CacheDB cache
        self.reset_db();

        // Step 3: Update the EVM's block context
        self.set_block(block_env);

        Ok(())
    }
}

#[cfg(feature = "foundry-fork")]
use foundry_fork_db::backend::SharedBackend;

#[cfg(feature = "foundry-fork")]
use crate::errors::InitError;

#[cfg(feature = "foundry-fork")]
impl ResetBlock for SharedBackend {
    type Error = EvmError;
    fn reset_block(&mut self, block_number: u64) -> Result<(), EvmError> {
        // Reset the block number in the EVM context
        self.set_pinned_block(BlockId::Number(BlockNumberOrTag::Number(block_number)))
            .map_err(|e| EvmError::Init(InitError::DatabaseError(e.to_string())))?;
        // Clear the cache
        let data = self.data();
        data.clear();
        Ok(())
    }
}
