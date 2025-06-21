//! Block environment utilities for EVM simulation
//!
//! Helper functions to create and configure block environments for transaction simulation.

use crate::types::BlockEnv;

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
/// # use revm_trace::utils::block_utils::get_block_env;
/// let block = get_block_env(18_000_000, 1672531200);
/// ```
pub fn get_block_env(
    block_number: u64,
    block_timestamp: u64,
) -> BlockEnv {
    let block = BlockEnv {
        number: block_number,
        timestamp: block_timestamp,
        ..Default::default()
    };
    
    block
}
