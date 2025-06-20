//! Block environment utilities for EVM simulation
//!
//! Helper functions to create and configure block environments for transaction simulation.

use crate::types::BlockEnv;
use alloy::primitives::U256;

/// Create a block environment for EVM execution
///
/// Sets up block context parameters required for transaction simulation.
/// Uses default values for unspecified parameters.
///
/// # Arguments
/// - `block_number`: Block number for simulation
/// - `block_timestamp`: Block timestamp (Unix time)
/// - `block_difficulty`: Optional difficulty (defaults if None)
/// - `block_gas_limit`: Optional gas limit (defaults if None)
///
/// # Returns
/// Configured `BlockEnv` ready for EVM execution
///
/// # Example
/// ```rust
/// # use revm_trace_multi_thread::utils::block_utils::create_block_env;
/// let block = create_block_env(18_000_000, 1672531200, None, None);
/// ```
pub fn create_block_env(
    block_number: u64,
    block_timestamp: u64,
    block_difficulty: Option<u64>,
    block_gas_limit: Option<u64>,
) -> BlockEnv {
    let mut block = BlockEnv {
        number: block_number,
        timestamp: block_timestamp,
        ..Default::default()
    };
    
    // Override defaults if specified
    if let Some(difficulty) = block_difficulty {
        block.difficulty = U256::from(difficulty);
    }
    if let Some(gas_limit) = block_gas_limit {
        block.gas_limit = gas_limit;
    }
    
    block
}
