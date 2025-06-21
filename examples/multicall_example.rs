//! Multicall utilities example
//! 
//! This example demonstrates how to use the multicall utilities
//! to batch multiple contract calls efficiently.

use revm_trace::{
    create_evm,
    utils::multicall_utils::{MulticallManager,MulticallCall},
};
use anyhow::Result;
use alloy::{ primitives::{address,U256}};
use alloy::sol;
use alloy::sol_types::SolCall;

sol! {
    function balanceOf(address owner) public returns (uint256);
}


const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Multicall utilities...");
    
    // Create EVM instance
    let mut evm = create_evm(
        ETH_RPC_URL
    ).await?;
    
    // Create multicall manager
    let manager = MulticallManager::new();
    
    // Test tokens and holders
    let tokens = vec![
        address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"), // USDC
        address!("dAC17F958D2ee523a2206206994597C13D831ec7"), // USDT
    ];
    
    let holders = vec![
        address!("28C6c06298d514Db089934071355E5743bf21d60"), // Binance hot wallet
        address!("21a31Ee1afC51d94C2eFcCAa2092aD1028285549"), // Binance cold wallet 
    ];
    
    println!("Creating batch calls for {} tokens and {} holders...", tokens.len(), holders.len());
    
    // Create batch calls for balance queries
    let mut batch_calls  = vec![];
   for token in &tokens {
        for holder in &holders {
            // Create a multicall for balanceOf function
            let data = balanceOfCall {
                owner: *holder
            }.abi_encode().into();
            let call = MulticallCall {
                target: *token,
                callData:data,
            };
            batch_calls.push(call);
        }
   }
    println!("Created {} batch calls", batch_calls.len());
    
    // Execute batch calls
    println!("Executing batch calls...");
    let results = manager.deploy_and_batch_call(
        &mut evm,
        batch_calls,
        true, // require success
        None,
    )?;
    
    println!("✅ Batch execution completed!");
    println!("Results:");
    
    // Display results
    let mut index = 0;
    for (token_idx, &token) in tokens.iter().enumerate() {
        for (holder_idx, &holder) in holders.iter().enumerate() {
            let result = &results[index];
            
            if result.success && result.returnData.len() == 32 {
                // Parse the 32-byte return data as U256
                let balance = U256::from_be_slice(&result.returnData[0..32]);
                println!(
                    "Token {} ({}) -> Holder {} ({}): Balance: {} (Success: {})",
                    token_idx + 1,
                    token,
                    holder_idx + 1,
                    holder,
                    balance,
                    result.success
                );
            } else {
                println!(
                    "Token {} ({}) -> Holder {} ({}): Failed or invalid data (Success: {}, Data length: {})",
                    token_idx + 1,
                    token,
                    holder_idx + 1,
                    holder,
                    result.success,
                    result.returnData.len()
                );
            }
            index += 1;
        }
    }
    
    Ok(())
}
