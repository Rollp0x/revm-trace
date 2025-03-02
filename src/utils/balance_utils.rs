use crate::evm::TraceEvm;
use alloy::{
    network::Ethereum, 
    primitives::{Address, U256}, 
    providers::Provider, 
    transports::Transport
};
use anyhow::Result;
use crate::traits::Reset;
use crate::errors::BalanceError;
use revm::{db::{AlloyDB, CacheDB, WrapDatabaseRef}, Database, Inspector};

/// Query the balance of an address
pub fn query_balance<T, P, I>(
    evm: &mut TraceEvm<'_, T, P, I>,
    owner: &Address,
) -> Result<U256,BalanceError>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + Reset,
{
    let db = evm.db_mut();
    let account = db.basic(*owner).map_err(|e| BalanceError::BalanceGetError { holder: owner.to_string(), reason: e.to_string() })?;
    let account = account.unwrap_or_default();
    Ok(account.balance)
}