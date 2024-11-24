//! ERC20 token interaction utilities
//!
//! This module provides helper functions for interacting with ERC20 tokens:
//! - Reading token metadata (symbol, decimals)
//! - Checking token balances
//! - Handling ABI encoding/decoding
//! - Error handling for token interactions

use alloy::{
    network::Network,
    primitives::{Address, TxKind, U256},
    providers::Provider,
    sol,
    sol_types::{SolCall, SolValue},
    transports::Transport,
};
use anyhow::{anyhow, Result};
use revm::{
    db::{AlloyDB, WrapDatabaseRef},
    primitives::{ExecutionResult, Output},
    Evm, Inspector,
};

// ERC20 interface for common token functions
//
// Generates Rust bindings for:
// - symbol(): Returns token symbol
// - decimals(): Returns token decimal places
// - balanceOf(address): Returns token balance for an account
sol! {
    function symbol() public returns (string);
    function decimals() public returns (uint8);
    function balanceOf(address account) public returns (uint256);
}

/// Retrieves the token balance for a specific account
///
/// Makes a call to the token's balanceOf function to get the current balance.
/// The balance is returned in the token's smallest unit (considering decimals).
///
/// # Arguments
/// * `evm` - Configured EVM instance for making the call
/// * `token` - Address of the ERC20 token contract
/// * `account` - Address of the account to check balance for
///
/// # Returns
/// * `Ok(U256)` - Token balance in smallest units
/// * `Err` - If the call fails or response cannot be decoded
///
/// # Example
/// ```no_run
/// # use revm_trace::utils::erc20_utils::get_token_balance;
/// # use alloy::primitives::{address, U256};
/// # async fn example() -> anyhow::Result<()> {
/// # let mut evm = todo!();
/// let token = address!("dac17f958d2ee523a2206206994597c13d831ec7"); // USDT
/// let account = address!("dead000000000000000000000000000000000000");
/// let balance = get_token_balance(&mut evm, token, account)?;
/// println!("Token balance: {}", balance);
/// # Ok(())
/// # }
/// ```
pub fn get_token_balance<T, N, P, I>(
    evm: &mut Evm<I, WrapDatabaseRef<AlloyDB<T, N, P>>>,
    token: Address,
    account: Address,
) -> Result<U256>
where
    T: Transport + Clone,
    N: Network,
    P: Provider<T, N>,
    I: Inspector<WrapDatabaseRef<AlloyDB<T, N, P>>> + Default,
{
    let encoded = balanceOfCall { account }.abi_encode();
    let tx = evm.tx_mut();
    tx.caller = account;
    tx.transact_to = TxKind::Call(token);
    tx.data = encoded.into();
    tx.value = U256::ZERO;

    let ref_tx = evm.transact().unwrap();
    let result = ref_tx.result;
    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            #[cfg(debug_assertions)]
            println!("Balance check result: {:?}", result);
            return Err(anyhow!("'balanceOf' execution failed: {result:?}"));
        }
    };

    let balance = U256::abi_decode(&value, false)?;
    Ok(balance)
}

/// Retrieves the number of decimal places for a token
///
/// Makes a call to the token's decimals function to get the decimal precision.
/// Most tokens use 18 decimals (like ETH), but some (like USDT) use 6.
///
/// # Arguments
/// * `evm` - Configured EVM instance for making the call
/// * `token` - Address of the ERC20 token contract
///
/// # Returns
/// * `Ok(u8)` - Number of decimal places (typically 18 or 6)
/// * `Err` - If the call fails or response is invalid
///
/// # Example
/// ```no_run
/// # use revm_trace::utils::erc20_utils::get_token_decimals;
/// # use alloy::primitives::address;
/// # async fn example() -> anyhow::Result<()> {
/// # let mut evm = todo!();
/// let token = address!("dac17f958d2ee523a2206206994597c13d831ec7"); // USDT
/// let decimals = get_token_decimals(&mut evm, token)?;
/// assert_eq!(decimals, 6); // USDT uses 6 decimal places
/// # Ok(())
/// # }
/// ```
pub fn get_token_decimals<T, N, P, I>(
    evm: &mut Evm<I, WrapDatabaseRef<AlloyDB<T, N, P>>>,
    token: Address,
) -> Result<u8>
where
    T: Transport + Clone,
    N: Network,
    P: Provider<T, N>,
    I: Inspector<WrapDatabaseRef<AlloyDB<T, N, P>>> + Default,
{
    let encoded = decimalsCall {}.abi_encode();
    let tx = evm.tx_mut();
    tx.transact_to = TxKind::Call(token);
    tx.data = encoded.into();
    tx.value = U256::ZERO;

    let ref_tx = evm.transact().unwrap();
    let result = ref_tx.result;
    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            #[cfg(debug_assertions)]
            println!("Decimals check result: {:?}", result);
            return Err(anyhow!("'decimals' execution failed: {result:?}"));
        }
    };

    let decimals = value.last().ok_or_else(|| anyhow!("Empty response"))?;
    Ok(*decimals)
}

/// Retrieves the symbol for a token
///
/// Makes a call to the token's symbol function to get its identifier.
/// The symbol is typically a 3-4 letter string (e.g., "USDT", "DAI").
///
/// # Arguments
/// * `evm` - Configured EVM instance for making the call
/// * `token` - Address of the ERC20 token contract
///
/// # Returns
/// * `Ok(String)` - Token symbol (e.g., "USDT", "DAI")
/// * `Err` - If the call fails or symbol cannot be decoded
///
/// # Example
/// ```no_run
/// # use revm_trace::utils::erc20_utils::get_token_symbol;
/// # use alloy::primitives::address;
/// # async fn example() -> anyhow::Result<()> {
/// # let mut evm = todo!();
/// let token = address!("6b175474e89094c44da98b954eedeac495271d0f"); // DAI
/// let symbol = get_token_symbol(&mut evm, token)?;
/// assert_eq!(symbol, "DAI");
/// # Ok(())
/// # }
/// ```
///
/// # Note
/// Some older or non-standard tokens might not implement the symbol function
/// or might implement it in a non-standard way.
pub fn get_token_symbol<T, N, P, I>(
    evm: &mut Evm<I, WrapDatabaseRef<AlloyDB<T, N, P>>>,
    token: Address,
) -> Result<String>
where
    T: Transport + Clone,
    N: Network,
    P: Provider<T, N>,
    I: Inspector<WrapDatabaseRef<AlloyDB<T, N, P>>> + Default,
{
    let encoded = symbolCall {}.abi_encode();
    let tx = evm.tx_mut();
    tx.transact_to = TxKind::Call(token);
    tx.data = encoded.into();
    tx.value = U256::ZERO;

    let ref_tx = evm.transact().unwrap();
    let result = ref_tx.result;
    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => {
            #[cfg(debug_assertions)]
            println!("Symbol check result: {:?}", result);
            return Err(anyhow!("'symbol' execution failed: {result:?}"));
        }
    };

    let symbol = String::abi_decode(&value, false)?;
    Ok(symbol)
}
