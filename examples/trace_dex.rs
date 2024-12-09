//! Uniswap V2 Swap Transaction Simulation Example
//! 
//! This example demonstrates how to:
//! - Set up an EVM instance with transaction tracing
//! - Encode Uniswap V2 swap function calls
//! - Track token transfers during swap execution
//! - Format and display swap results in a readable table
//! 
//! Note: This example simulates a swap of ETH -> USDC using the Uniswap V2 router.
//! Results may vary based on pool state at different blocks.

use std::collections::HashMap;
use revm_trace::{
    TransactionProcessor,
    types::{TxKind,TokenInfo},
    utils::erc20_utils::get_token_infos,
    create_evm_with_inspector, SimulationBatch, SimulationTx, TxInspector
};
use anyhow::Result;
use alloy::{
    primitives::{address, Address, U256},  
    sol, sol_types::SolCall
};
use prettytable::{format, Cell, Row, Table};
use colored::*;
mod common;
use common::get_block_env;

// Define Uniswap V2 Router interface for swapping
sol! {
    contract UniswapV2Router {
        /// Swaps exact amount of ETH for tokens
        /// @param amountOutMin Minimum amount of tokens to receive
        /// @param path Array of token addresses defining the swap path
        /// @param to Address to receive the output tokens
        /// @param deadline Unix timestamp deadline for the swap
        function swapExactETHForTokens(
            uint256 amountOutMin,
            address[] calldata path,
            address to,
            uint256 deadline
        ) external payable returns (uint256[] memory amounts);
    }
}

/// Formats a U256 amount with proper decimal places
///
/// # Arguments
/// * `amount` - The amount to format
/// * `decimals` - Number of decimal places for the token
///
/// # Returns
/// Formatted string with proper decimal point placement
/// 
/// # Example
/// ```
/// let amount = U256::from(1234567890);
/// let formatted = format_amount(amount, 6);
/// assert_eq!(formatted, "1234.56789");
/// ```
fn format_amount(amount: U256, decimals: u8) -> String {
    let mut value = amount.to_string();
    if value.len() <= decimals as usize {
        value.insert_str(0, &"0".repeat(decimals as usize - value.len() + 1));
        value.insert(1, '.');
    } else {
        value.insert(value.len() - decimals as usize, '.');
    }
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}


const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";

#[tokio::main]
async fn main() -> Result<()> {
    // Create EVM instance with transaction tracing
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_inspector(ETH_RPC_URL,inspector).await.unwrap();
    println!("{}", "✅ EVM instance created successfully\n".green());
    // Get block environment for simulation
    let block_env = get_block_env(ETH_RPC_URL, None).await.unwrap();

    // Configure swap parameters
    let caller = address!("57757E3D981446D585Af0D9Ae4d7DF6D64647806");
    let router = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
    let weth = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    let usdc = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");

    // Display swap configuration
    println!("Swap Configuration:");
    println!("------------------");
    println!("Caller: {}", caller);
    println!("Router: {}", router);
    println!("Path: {} -> {}\n", "WETH", "USDC");

    // Prepare swap transaction
    let swap_amount = U256::from(100000000000000000u128); // 0.1 ETH
    let path = vec![weth, usdc];
    let deadline = U256::from(u64::MAX);
    let data = UniswapV2Router::swapExactETHForTokensCall {
        amountOutMin: U256::ZERO,
        path,
        to: caller,
        deadline,
    }
    .abi_encode();

    // Create and execute swap transaction
    println!("Executing swap of {} ETH...\n", "0.1".bold());
    let tx = SimulationTx{
        caller,
        transact_to: TxKind::Call(router),
        value: swap_amount,
        data: data.into(),
    };

    // Process transaction and get results
    let result = evm.process_transactions(SimulationBatch {
        block_env,
        transactions: vec![tx],
        is_stateful: true,
    }).into_iter().map(|v| v.unwrap()).collect::<Vec<_>>()[0].clone();

    // Verify transaction success
    println!("\nTransaction Result:");
    println!("-----------------");
    assert!(
        result.0.is_success(),
        "❌ Swap failed"
    );

    // Format results in a table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_BOX_CHARS);
    table.add_row(Row::new(vec![
        Cell::new("Token").style_spec("Fb"),
        Cell::new("From").style_spec("Fb"),
        Cell::new("To").style_spec("Fb"),
        Cell::new("Amount").style_spec("Fb"),
    ]));
    // Get all unique tokens
    let mut tokens = vec![];
    for transfer in &result.1.asset_transfers {
        if !tokens.contains(&transfer.token)  && transfer.token != Address::ZERO {
            tokens.push(transfer.token);
        }
    }
    // Get token infos
    let token_infos = get_token_infos(&mut evm,&tokens, None).unwrap();
    let mut token_info_map = HashMap::new();
    token_info_map.insert(Address::ZERO, TokenInfo{
        symbol: "ETH".to_string(),
        decimals: 18,
    });
    for (i,token_info) in token_infos.into_iter().enumerate() {
        token_info_map.insert(tokens[i], token_info);
    }
    // Add transfers to table
    for transfer in &result.1.asset_transfers {
        
        let amount = if let Some(info) = token_info_map.get(&transfer.token) {
            format_amount(transfer.value, info.decimals)
        } else {
            format_amount(transfer.value, 18) // Default to 18 decimals for ETH
        };

        table.add_row(Row::new(vec![
            Cell::new(
                &token_info_map
                    .get(&transfer.token)
                    .map(|i| i.symbol.clone())
                    .unwrap_or_else(|| "ETH".to_string()),
            ),
            Cell::new(&format!("{:.8}...", transfer.from)),
            Cell::new(&format!("{:.8}...", transfer.to.unwrap())),
            Cell::new(&amount),
        ]));
    }

    println!("Swap Results:");
    println!("------------");
    table.printstd();


    println!(
        "\n{}",
        "✅ All assertions passed successfully!".bold().green()
    );
    println!("Note: This example might fail occasionally due to DEX pool state changes");
    println!("Consider using a specific block number or implementing retry logic for more stable results");
    Ok(())
}
