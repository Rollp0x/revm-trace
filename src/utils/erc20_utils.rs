//! ERC20 token interaction utilities
//!
//! This module provides helper functions for interacting with ERC20 tokens:
//! - Reading token metadata (symbol, decimals)
//! - Checking token balances
//! - Handling ABI encoding/decoding
//! - Error handling for token interactions

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
use crate::{TraceEvm,types::{TokenInfo,BlockEnv}};
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
// - symbol(): Returns token symbol
// - decimals(): Returns token decimal places
// - balanceOf(address): Returns token balance for an account
sol! {
    function symbol() public returns (string);
    function decimals() public returns (uint8);
}

fn query_token_info<T, P, I>(
    evm: &mut TraceEvm<'_, T, P, I>,
    token: &Address,
    symbol_encoded: Vec<u8>,
    decimals_encoded: Vec<u8>,
) -> Result<TokenInfo,TokenError>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + Reset,
{
    // 查询 symbol
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

    // 查询 decimals
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

    Ok(TokenInfo { symbol, decimals })
}

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
    let symbol_encoded = symbolCall {}.abi_encode();
    let decimals_encoded = decimalsCall {}.abi_encode();
    
    for token in tokens {
        let token_info = query_token_info(evm, token, symbol_encoded.clone(), decimals_encoded.clone())?;
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