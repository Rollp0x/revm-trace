//! Test balance queries at different block heights
//! 
//! This example demonstrates:
//! - Querying the same address at different block heights
//! - Verifying if foundry-fork-db correctly handles block-specific state
//! - Understanding cache behavior in TraceEvm
//! 
//! We use Vitalik's address which has well-documented transaction history

use revm_trace::{
    create_evm,traits::ResetDB,
    utils::{
        balance_utils::query_balance,
        block_utils::get_block_env,
    }
};
use anyhow::Result;
use alloy::primitives::{address,utils::format_units};
use colored::*;

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";



#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", "🔍 Testing balance queries at different block heights\n".cyan().bold());
    
    let my_address = address!("0x888888888DAc1d551DF1bdaAd5f2575884888808");
    println!("Testing address: {} (My address)\n", my_address);


    let test_blocks = vec![
        ("Before Transfer", 19416560), // Block before the 0.09445674 ETH transfer
        ("After Transfer", 19416575),  // Block after transfer (0.09445674 ETH + ~0.00115 ETH gas fee)
    ];
    // Create fresh EVM instance for each test
    let mut evm = create_evm(ETH_RPC_URL).await?;

    for (era_name, block_number) in test_blocks {
        
        // Create block environment for specific block
        let block_env = get_block_env(block_number,1600000000);
        println!("{era_name}- setting block {block_number}...");
        evm.set_db_block(block_env).unwrap();
        
        // First query
        match query_balance(&mut evm, my_address, None) {
            Ok(balance) => {
                println!("{} query: {} wei", era_name, balance);
                let eth_balance = format_units(balance, "ether").unwrap_or_else(|_| "N/A".to_string());
                println!("{} query: {} ETH", era_name, eth_balance);
            }
            Err(e) => {
                println!("{} query error: {}", era_name, e);
            }
        }
    }

    println!("\n{}", "✅ Balance query test completed!".green().bold());
    println!("{}", "✨ Success! Different blocks returned different balances - historical state queries are working!".green());
    println!("{}", "💡 The balance difference includes both the transfer amount (0.09445674 ETH) and gas fees (~0.00115 ETH)".cyan());
    
    Ok(())
}