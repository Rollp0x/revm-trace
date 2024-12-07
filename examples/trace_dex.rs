//! Uniswap V2 swap transaction tracing example
//!
//! This example demonstrates how to:
//! - Set up an EVM instance with transaction tracing
//! - Simulate a Uniswap V2 ETH -> USDC swap
//! - Track and display asset transfers
//!
//! Note: This example might fail occasionally due to DEX pool state changes.
//! For more stable results, consider:
//! - Using a specific block number
//! - Implementing retry logic
//! - Using an archive node

use alloy::{
    eips::BlockNumberOrTag, primitives::{address, U256}, providers::{Provider, ProviderBuilder}, sol, sol_types::SolCall
};
use anyhow::Result;
use colored::*;
use prettytable::{format, Cell, Row, Table};
use revm_trace::{create_evm,Tracer,SimulationTx,types::TxKind,BlockEnv,SimulationBatch};

// Uniswap V2 Router interface for ETH -> Token swaps
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

async fn get_block_env(http_url: &str,block_number:Option<u64>) -> BlockEnv {
    let provider = ProviderBuilder::new()
        .on_http(http_url.parse().unwrap());
    if let Some(block_number) = block_number {
        let block_info = provider.get_block_by_number(BlockNumberOrTag::Number(block_number),false).await.unwrap().unwrap();
        return BlockEnv { number: block_number, timestamp: block_info.header.timestamp };
    } else {
        let latest_block = provider.get_block_number().await.unwrap();
        let block_info = provider.get_block_by_number(BlockNumberOrTag::Number(latest_block),false).await.unwrap().unwrap();
        return BlockEnv { number: latest_block, timestamp: block_info.header.timestamp };
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize EVM with transaction tracer
    let mut evm = create_evm("https://rpc.ankr.com/eth",Some(1),None)?;

    println!("{}", "✅ EVM instance created successfully\n".green());

    let caller = address!("57757E3D981446D585Af0D9Ae4d7DF6D64647806");
    let router = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
    let weth = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    let usdc = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");

    println!("Swap Configuration:");
    println!("------------------");
    println!("Caller: {}", caller);
    println!("Router: {}", router);
    println!("Path: {} -> {}\n", "WETH", "USDC");

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

    println!("Executing swap of {} ETH...\n", "0.1".bold());
    let tx = SimulationTx{
        caller,
        transact_to: TxKind::Call(router),
        value: swap_amount,
        data: data.into(),
    };
    let block_env = get_block_env("https://rpc.ankr.com/eth",None).await;

    let result = evm.trace_txs(SimulationBatch {
        block_env,
        transactions: vec![tx],
        is_bound_multicall: false,
    }).unwrap()[0].clone();

    println!("\nTransaction Result:");
    println!("-----------------");
    println!("Status: {:#?}", result.status);

    // Verify results
    assert!(
        result.asset_transfers.len() > 2,
        "❌ Expected at least 2 transfers"
    );

    // Create results table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_BOX_CHARS);
    table.add_row(Row::new(vec![
        Cell::new("Token").style_spec("Fb"),
        Cell::new("From").style_spec("Fb"),
        Cell::new("To").style_spec("Fb"),
        Cell::new("Amount").style_spec("Fb"),
    ]));

    // Add transfers to table
    for transfer in result.asset_transfers {
        let token_info = result.token_infos.get(&transfer.token);
        let amount = if let Some(info) = token_info {
            format_amount(transfer.value, info.decimals)
        } else {
            format_amount(transfer.value, 18) // Default to 18 decimals for ETH
        };

        table.add_row(Row::new(vec![
            Cell::new(
                &token_info
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
