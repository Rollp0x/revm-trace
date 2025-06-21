
//! Balance query utilities for EVM accounts
//!
//! Provides functions to query native token (ETH) balances from blockchain state.

use alloy::primitives::{Address, U256};
use anyhow::Result;
use revm::{
    context_interface::ContextTr,
    database::Database,
    ExecuteEvm
};
use crate::{
    evm::TraceEvm,
    errors::BalanceError,
    types::BlockEnv
};

/// Query the native token balance of an address
///
/// Retrieves the ETH balance for the specified address, optionally at a specific block.
///
/// # Arguments
/// - `evm`: EVM instance for state queries
/// - `owner`: Address to query balance for
/// - `block_params`: Optional block number/timestamp to query at
///
/// # Returns
/// - `Ok(U256)`: Account balance in wei
/// - `Err(BalanceError)`: If balance query fails
///
/// # Example
/// ```rust,no_run
/// # use revm_trace::utils::balance_utils::query_balancee;
/// # use alloy::primitives::address;
/// use revm_trace::evm::builder::EvmBuilder;
/// use revm::inspector::NoOpInspector;
///
/// let builder = EvmBuilder::new(
///     "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
///     NoOpInspector
/// );
/// let mut evm = builder.build().await.unwrap();
/// let balance = query_balancee(&mut evm, address!("DFd5293D8e347dFe59E90eFd55b2956a1343963d"), None)?;
/// println!("Balance: {} wei", balance);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn query_balancee<DB, INSP>(
    evm: &mut TraceEvm<DB, INSP>,
    owner: Address,
    block_env:Option<BlockEnv>
) -> Result<U256, BalanceError>
where
    DB: Database
{
    // Set block context if specified
    if let Some(block_env) = block_env {
        evm.set_block(block_env);
    }
    
    // Query account state from database
    let db = evm.db();
    let account = db.basic(owner).map_err(|e| BalanceError::BalanceGetError { 
        holder: owner.to_string(), 
        reason: e.to_string() 
    })?;
    
    // Return balance (default to 0 if account doesn't exist)
    let account = account.unwrap_or_default();
    Ok(account.balance)
}