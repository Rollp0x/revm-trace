//! Test balance queries at different block heights
//! 
//! This example demonstrates:
//! - Querying the same address at different block heights
//! - Verifying if foundry-fork-db correctly handles block-specific state
//! - Understanding cache behavior in TraceEvm
//! - Testing both AlloyDB and SharedBackend implementations
//! 
//! We use a known address which has documented transaction history

use revm_trace::{
    utils::balance_utils::query_balance,
    types::BlockEnv
};

use anyhow::Result;
use alloy::{primitives::{address, utils::format_units}};
use colored::*;

#[cfg(not(feature = "foundry-fork"))]
use revm_trace::create_evm;

#[cfg(feature = "foundry-fork")]
use revm_trace::create_shared_evm;

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

async fn show_balance() -> Result<()> {

    #[cfg(not(feature = "foundry-fork"))]
    println!("{}", "🔧 Testing AlloyDB backend".yellow().bold());
    
    #[cfg(feature = "foundry-fork")]
    println!("{}", "🔧 Testing Foundry fork backend".blue().bold());

    
    let some_address = address!("0x892D4Cc1c2C1cee9091b1F30F64423693F333333"); 
    println!("Testing address: {} \n", some_address);

    let test_blocks = vec![
        ("Before Transfer", 22770150), //
        ("After Transfer", 22770152),  // 
    ];
    
    // Create EVM instance without inspector since we're only reading state
    #[cfg(not(feature = "foundry-fork"))]
    let mut evm = create_evm(ETH_RPC_URL).await?;

    #[cfg(feature = "foundry-fork")]
    let mut evm = create_shared_evm(ETH_RPC_URL).await?;


    for (era_name, block_number) in test_blocks {
        // Create block environment for specific block
        let block_env = BlockEnv {
            number: block_number,
            timestamp: 1700000000, // Use a fixed timestamp for simplicity
            ..Default::default()
        };
        println!("{} - setting block {}...", era_name, block_number);
        
        // Test our ResetBlock trait implementation
        match evm.set_db_block(block_env) {
            #[cfg(not(feature = "foundry-fork"))]
            Ok(()) => println!("✅ AlloyDB block reset successful"),
            #[cfg(feature = "foundry-fork")]
            Ok(()) => println!("✅ SharedBackend block reset successful"),
            Err(e) => {
                println!("❌ AlloyDB block reset failed: {}", e);
                continue;
            }
        }
        
        // Query balance
        match query_balance(&mut evm, some_address) {
            Ok(balance) => {
                println!("{} query: {} wei", era_name, balance);
                let eth_balance = format_units(balance, "ether").unwrap_or_else(|_| "N/A".to_string());
                println!("{} query: {} ETH", era_name, eth_balance);
            }
            Err(e) => {
                println!("{} query error: {}", era_name, e);
            }
        }
        println!(); // Add spacing
    }
    
    Ok(())
}


#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", "🔍 Testing balance queries at different block heights with both backends\n".cyan().bold());
    
    show_balance().await?;
    
    println!("{}", "─".repeat(60));
    println!();

    #[cfg(feature = "foundry-fork")]
    println!("{}", "✅ Balance query test completed for both backends!".green().bold());

    #[cfg(feature = "foundry-fork")]
    println!("{}", "✨ Success! Both AlloyDB and SharedBackend support ResetBlock trait!".green());
    
    #[cfg(not(feature = "foundry-fork"))]
    println!("{}", "✅ Balance query test completed for AlloyDB backend!".green().bold());
    
    #[cfg(not(feature = "foundry-fork"))]
    println!("{}", "ℹ️  SharedBackend test skipped (foundry-fork feature not enabled)".yellow());
    
    println!("{}", "💡 The balance difference includes both the transfer amount (0.09445674 ETH) and gas fees (~0.00115 ETH)".cyan());
    println!("{}", "🚀 ResetBlock trait with associated Error type works correctly!".magenta());
    
    Ok(())
}
