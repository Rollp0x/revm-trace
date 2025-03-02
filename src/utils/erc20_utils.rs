//! ERC20 token interaction utilities
//!
//! This module provides helper functions for interacting with ERC20 tokens:
//! - Reading token metadata (symbol, decimals)
//! - Parsing ERC20 Transfer events

use alloy::{
    sol,
    network::Ethereum, 
    primitives::{Address, TxKind, U256,FixedBytes,keccak256}, 
    providers::Provider, 
    sol_types::SolCall, 
    transports::Transport
};
use anyhow::Result;
use revm::{db::{AlloyDB, CacheDB, WrapDatabaseRef}, primitives::{ExecutionResult, Output}, Inspector};
use crate::evm::TraceEvm;
use crate::types::{TokenInfo,BlockEnv};
use crate::traits::Reset;
use crate::errors::{TokenError,EvmError};
use once_cell::sync::Lazy;

/// ERC20 Transfer event signature
/// keccak256("Transfer(address,address,uint256)")
static TRANSFER_EVENT_SIGNATURE: Lazy<FixedBytes<32>> =
    Lazy::new(|| keccak256(b"Transfer(address,address,uint256)"));


// ERC20 interface for common token functions
//
// Generates Rust bindings for:
// - name(): Returns token name
// - symbol(): Returns token symbol
// - decimals(): Returns token decimal places
// - balanceOf(address): Returns token balance for an address
// - totalSupply(): Returns total token supply

sol! {
    function name() public returns (string);
    function symbol() public returns (string);
    function decimals() public returns (uint8);
    function balanceOf(address owner) public returns (uint256);
    function totalSupply() public returns (uint256);
}

/// Query token balance for an address
pub fn query_erc20_balance<T, P, I>(
    evm: &mut TraceEvm<'_, T, P, I>,
    token: &Address,
    owner: &Address,
    block_env: Option<BlockEnv>,
) -> Result<U256,TokenError>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + Reset,
{   
    if let Some(block_env) = block_env {
        evm.set_block_env(block_env);
    }
    evm.reset_inspector();

    let tx = evm.tx_mut();
    tx.transact_to = TxKind::Call(*token);
    tx.value = U256::ZERO;
    tx.data = balanceOfCall { owner: *owner }.abi_encode().into();
    
    let ref_tx  = evm.transact().unwrap();
    let balance = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => {
            let balance = balanceOfCall::abi_decode_returns(&value, true)
                .map_err(|_| TokenError::BalanceDecode { address: token.to_string(),holder:owner.to_string(), reason: "Failed to decode balance".to_string() })?
                ._0;
            Ok(balance)
        },
        _ => Err(TokenError::QueryFailed { address: token.to_string(), reason: "Failed to get balance".to_string() }),
    };
    evm.reset_inspector();
    balance
}

/// Query token info (symbol, decimals,name,total_supply)
fn query_token_info<T, P, I>(
    evm: &mut TraceEvm<'_, T, P, I>,
    token: &Address,
    name_encoded: Vec<u8>,
    symbol_encoded: Vec<u8>,
    decimals_encoded: Vec<u8>,
    total_supply_encoded: Vec<u8>,
) -> Result<TokenInfo,TokenError>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + Reset,
{
    // query name and symbol
    let tx = evm.tx_mut();
    tx.transact_to = TxKind::Call(*token);
    tx.value = U256::ZERO;
    tx.data = name_encoded.into();

    let ref_tx = evm.transact().unwrap();
    let name = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => {
            nameCall::abi_decode_returns(&value, true)
                .map_err(|_| TokenError::NameDecode { address: token.to_string(), reason: "Failed to decode name".to_string() })?
                ._0
        },
        _ => return Err(TokenError::QueryFailed { address: token.to_string(), reason: "Failed to get name".to_string() }),
    };

    let tx = evm.tx_mut();
    tx.transact_to = TxKind::Call(*token);
    tx.value = U256::ZERO;
    tx.data = symbol_encoded.into();

    let ref_tx = evm.transact().unwrap();
    let symbol = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => {
            symbolCall::abi_decode_returns(&value, true)
                .map_err(|_| TokenError::SymbolDecode { address: token.to_string(), reason: "Failed to decode symbol".to_string() })?
                ._0
        },
        _ => return Err(TokenError::QueryFailed { address: token.to_string(), reason: "Failed to get symbol".to_string() }),
    };

    // query decimals
    let tx = evm.tx_mut();
    tx.data = decimals_encoded.into();

    let ref_tx = evm.transact().unwrap();
    let decimals = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => {
            decimalsCall::abi_decode_returns(&value, true)
                .map_err(|_| TokenError::DecimalsDecode { address: token.to_string(), reason: "Failed to decode decimals".to_string() })?
                ._0
        },
        _ => return Err(TokenError::QueryFailed { address: token.to_string(), reason: "Failed to get decimals".to_string() }),
    };

    // query total supply
    let tx = evm.tx_mut();
    tx.data = total_supply_encoded.into();
    let ref_tx = evm.transact().unwrap();
    let total_supply = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => {
            totalSupplyCall::abi_decode_returns(&value, true)
                .map_err(|_| TokenError::TotalSupplyDecode { address: token.to_string(), reason: "Failed to decode total supply".to_string() })?
                ._0
        },
        _ => return Err(TokenError::QueryFailed { address: token.to_string(), reason: "Failed to get total supply".to_string() }),
    };

    Ok(TokenInfo { name,symbol, decimals, total_supply })
}

/// Query token info (name, symbol, decimals, total_supply) for multiple tokens
pub fn get_token_infos<T, P, I>(
    evm: &mut TraceEvm<'_, T, P, I>,
    tokens: &[Address],
    block_env: Option<BlockEnv>,
) -> Result<Vec<TokenInfo>,EvmError>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + Reset,
{   
    if let Some(block_env) = block_env {
        evm.set_block_env(block_env);
    }
    evm.reset_inspector();
    
    let mut token_infos = Vec::with_capacity(tokens.len());
    let name_encoded = nameCall {}.abi_encode();
    let symbol_encoded = symbolCall {}.abi_encode();
    let decimals_encoded = decimalsCall {}.abi_encode();
    let total_supply_encoded = totalSupplyCall {}.abi_encode();
    
    for token in tokens {
        let token_info = query_token_info(evm, token,name_encoded.clone(), symbol_encoded.clone(), decimals_encoded.clone(),total_supply_encoded.clone())?;
        token_infos.push(token_info);
    }
    
    evm.reset_inspector();
    Ok(token_infos)
}


/// Parses ERC20 Transfer event data
/// 
/// # Arguments
/// * `topics` - Event topics containing:
///   - [0]: Transfer event signature
///   - [1]: From address (indexed)
///   - [2]: To address (indexed)
/// * `data` - ABI-encoded transfer amount
/// 
/// # Returns
/// * `Some((from, to, amount))` if valid Transfer event
/// * `None` if invalid format or zero amount
pub fn parse_transfer_log(topics: &[FixedBytes<32>], data: &[u8]) -> Option<(Address, Address, U256)> {
    if topics.len() < 3 || topics[0] != *TRANSFER_EVENT_SIGNATURE {
        return None;
    }
    let amount = U256::from_be_slice(data);
    if !amount.is_zero() {
        Some((
            Address::from_slice(&topics[1].as_slice()[12..]),
            Address::from_slice(&topics[2].as_slice()[12..]),
            amount,
        ))
    } else {
        None
    }
}