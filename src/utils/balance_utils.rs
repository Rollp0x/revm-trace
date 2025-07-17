//! Balance query utilities for EVM accounts
//!
//! Provides functions to query native token (ETH) balances from blockchain state.

use crate::{errors::BalanceError, evm::TraceEvm};
use alloy::primitives::{Address, U256};
use anyhow::Result;
use revm::{context_interface::ContextTr, database::Database};

/// Query the native token balance of an address
///
/// Retrieves the ETH balance for the specified address, optionally at a specific block.
///
/// # Arguments
/// - `evm`: EVM instance for state queries
/// - `owner`: Address to query balance for
///
/// # Returns
/// - `Ok(U256)`: Account balance in wei
/// - `Err(BalanceError)`: If balance query fails
///
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use revm_trace::{create_evm, utils::balance_utils::query_balance};
/// use alloy::primitives::address;
///
/// let mut evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-key").await?;
/// let balance = query_balance(
///     &mut evm,
///     address!("DFd5293D8e347dFe59E90eFd55b2956a1343963d"),
/// )?;
/// println!("Balance: {} wei", balance);
/// # Ok(())
/// # }
/// ```
pub fn query_balance<DB, INSP>(
    evm: &mut TraceEvm<DB, INSP>,
    owner: Address,
) -> Result<U256, BalanceError>
where
    DB: Database,
{
    // Query account state from database
    let db = evm.db();
    let account = db.basic(owner).map_err(|e| BalanceError::BalanceGetError {
        holder: owner.to_string(),
        reason: e.to_string(),
    })?;

    // Return balance (default to 0 if account doesn't exist)
    let account = account.unwrap_or_default();
    Ok(account.balance)
}
