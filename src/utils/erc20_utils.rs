//! ERC20 token utilities for querying token information and balances
//!
//! Provides functions to interact with ERC20 tokens including balance queries,
//! token metadata retrieval, and transfer event parsing.

use revm::{
    context::TxEnv,
    database::Database,
    ExecuteEvm,
    context_interface::result::{ExecutionResult, Output},
};
use anyhow::Result;
use crate::{
    evm::TraceEvm,
    errors::{TokenError,EvmError},
    types::{BlockEnv, TokenInfo}
};
use once_cell::sync::Lazy;
use alloy::{
    sol,
    sol_types::SolCall, 
    primitives::{Address,Bytes, U256,TxKind,FixedBytes,keccak256},
};

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

/// ERC20 Transfer event signature
/// keccak256("Transfer(address,address,uint256)")
static TRANSFER_EVENT_SIGNATURE: Lazy<FixedBytes<32>> =
    Lazy::new(|| keccak256(b"Transfer(address,address,uint256)"));

/// Query ERC20 token balance for a specific address
///
/// Executes the `balanceOf(address)` function on the specified token contract.
///
/// # Arguments
/// - `evm`: EVM instance for contract execution
/// - `token_address`: Address of the ERC20 token contract
/// - `owner`: Address to query balance for
/// - `block_params`: Optional block context for the query
///
/// # Returns
/// - `Ok(U256)`: Token balance in the token's smallest unit
/// - `Err(...)`: If the contract call fails or returns invalid data
pub fn query_erc20_balance<DB,INSP>(
    evm: &mut TraceEvm<DB, INSP>,
    token_address: Address,
    owner: Address,
    block_env:Option<BlockEnv>
) -> Result<U256>
where 
    DB: Database
{   
    if let Some(block_env) = block_env {
        evm.set_block(block_env);
    }

    let data:Bytes = balanceOfCall { owner: owner }.abi_encode().into();
    
    // Use zero address as caller for read-only calls (no nonce needed)
    let tx = TxEnv::builder()
        .caller(Address::ZERO)
        .kind(TxKind::Call(token_address))
        .data(data)
        .nonce(0)  // Read-only call, nonce doesn't matter
        .build_fill();
    let ref_tx = evm.transact(tx).map_err(|e| anyhow::anyhow!("Failed to query ERC20 balance: {}", e))?;
    let value = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        _ => return Err(anyhow::anyhow!("Failed to execute balanceOf call")),
    };
    let balance = balanceOfCall::abi_decode_returns(&value)?;

    Ok(balance)
}


/// Internal helper to query all token information with pre-encoded call data
///
/// Executes name(), symbol(), decimals(), and totalSupply() calls for a token.
///
/// # Arguments
/// - `evm`: EVM instance for contract execution
/// - `token_address`: Token contract address
/// - `name_encoded`: Pre-encoded name() call data
/// - `symbol_encoded`: Pre-encoded symbol() call data  
/// - `decimals_encoded`: Pre-encoded decimals() call data
/// - `total_supply_encoded`: Pre-encoded totalSupply() call data
///
/// # Returns
/// - `Ok(TokenInfo)`: Complete token information
/// - `Err(TokenError)`: If any call fails or returns invalid data
fn query_token_info<DB,INSP>(
    evm: &mut TraceEvm<DB, INSP>,
    token_address: Address,
    name_encoded: Bytes,
    symbol_encoded: Bytes,
    decimals_encoded: Bytes,
    total_supply_encoded: Bytes,
) -> Result<TokenInfo,TokenError>
where
    DB: Database
{
    
    let tx_name = TxEnv {
        caller: Address::ZERO,
        kind: TxKind::Call(token_address),
        data: name_encoded,
        nonce: 0,
        ..Default::default()
    };
    let ref_tx  = evm.transact(tx_name).map_err(|e| anyhow::anyhow!("Failed to query token name: {}", e))?;
    let name = match ref_tx.result {
        ExecutionResult::Success { output: Output::Call(value), .. } => {
            nameCall::abi_decode_returns(&value)
            .map_err(|_| TokenError::NameDecode { address: token_address.to_string(), reason: "Failed to decode name".to_string() })?
        },
        _ => return Err(TokenError::CallReverted { address: token_address.to_string()}),
    };
    
    let tx_symbol = TxEnv {
        caller: Address::ZERO,
        kind: TxKind::Call(token_address),
        data: symbol_encoded,
        ..Default::default()
    };
    let ref_tx = evm.transact(tx_symbol).map_err(|e| anyhow::anyhow!("Failed to query token symbol: {}", e))?;
    let symbol = match ref_tx.result {
        ExecutionResult::Success { output: Output::Call(value), .. } => {
            symbolCall::abi_decode_returns(&value)
            .map_err(|_| TokenError::SymbolDecode { address: token_address.to_string(), reason: "Failed to decode symbol".to_string() })?
        },
        _ => return Err(TokenError::CallReverted { address: token_address.to_string()}),
    };
    
    let tx_decimals = TxEnv {
        kind: TxKind::Call(token_address),
        data: decimals_encoded,
        ..Default::default()
    };
    let ref_tx = evm.transact(tx_decimals).map_err(|e| anyhow::anyhow!("Failed to query token decimals: {}", e))?;
    let decimals = match ref_tx.result {
        ExecutionResult::Success { output: Output::Call(value), .. } => {
            decimalsCall::abi_decode_returns(&value)
            .map_err(|_| TokenError::DecimalsDecode { address: token_address.to_string(), reason: "Failed to decode decimals".to_string() })?
        },
        _ => return Err(TokenError::CallReverted { address: token_address.to_string()}),
    };
    let tx_total_supply = TxEnv {
        kind: TxKind::Call(token_address),
        data: total_supply_encoded,
        ..Default::default()
    };
    let ref_tx = evm.transact(tx_total_supply).map_err(|e| anyhow::anyhow!("Failed to query token total supply: {}", e))?;
    let total_supply = match ref_tx.result {
        ExecutionResult::Success { output: Output::Call(value), .. } => {
            totalSupplyCall::abi_decode_returns(&value)
            .map_err(|_| TokenError::TotalSupplyDecode { address: token_address.to_string(), reason: "Failed to decode total supply".to_string() })?
        },
        _ => return Err(TokenError::CallReverted { address: token_address.to_string()}),
    };

    Ok(TokenInfo { name,symbol, decimals, total_supply })
}

/// Query token information for multiple ERC20 tokens in batch
///
/// Efficiently retrieves name, symbol, decimals, and total supply for multiple tokens.
///
/// # Arguments
/// - `evm`: EVM instance for contract execution
/// - `tokens`: Array of token contract addresses
/// - `block_params`: Optional block context for queries
///
/// # Returns
/// - `Ok(Vec<TokenInfo>)`: Array of token information in the same order as input
/// - `Err(EvmError)`: If any contract call fails
pub fn get_token_infos<DB, INSP>(
    evm: &mut TraceEvm<DB, INSP>,
    tokens: &[Address],
    block_env:Option<BlockEnv>
) -> Result<Vec<TokenInfo>,EvmError>
where 
    DB: Database
{   
    if let Some(block_env) = block_env {
        evm.set_block(block_env);
    }

    let name_encoded: Bytes = nameCall { }.abi_encode().into();
    let symbol_encoded: Bytes = symbolCall { }.abi_encode().into();
    let decimals_encoded: Bytes = decimalsCall { }.abi_encode().into();
    let total_supply_encoded: Bytes = totalSupplyCall { }.abi_encode().into();
    let mut token_infos = Vec::with_capacity(tokens.len());
    for token in tokens {
        let token_info = query_token_info(evm, *token,name_encoded.clone(), symbol_encoded.clone(), decimals_encoded.clone(),total_supply_encoded.clone())?;
        token_infos.push(token_info);
    }

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