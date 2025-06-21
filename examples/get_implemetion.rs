//! Proxy Implementation Contract Lookup Example
//! 
//! This example demonstrates how to:
//! - Query the implementation address of a proxy contract
//! - Use revm without transaction simulation
//! - Work with proxy contracts like USDC
//! 
//! Note: This example doesn't use transaction simulation or inspectors,
//! it only reads state from the blockchain.

use revm_trace::{
    utils::proxy_utils::get_implementation,
    create_evm
};
use anyhow::Result;
use alloy::primitives::address;

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting proxy implementation test...");
    // Create EVM instance without inspector since we're only reading state
    let mut evm = create_evm(
        ETH_RPC_URL
    ).await?;

    // USDC uses the proxy pattern - this is the proxy contract address
    let usdc_proxy = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    println!("📝 Testing USDC proxy contract at: {}", usdc_proxy);

    // Query the implementation contract address
    let implementation_address = get_implementation(&mut evm, usdc_proxy, None).unwrap();

    match implementation_address {
        Some(impl_addr) => {
            println!("✅ Implementation contract found at: {}", impl_addr);
            println!("🔍 Verifying against known implementation...");

            // Known USDC implementation contract
            let expected_impl = address!("43506849D7C04F9138D1A2050bbF3A0c054402dd");
            assert_eq!(
                impl_addr, expected_impl,
                "Implementation address mismatch!\nExpected: {}\nFound: {}",
                expected_impl, impl_addr
            );
            println!("✅ Implementation address verified correctly");
        }
        None => {
            println!("❌ No implementation contract found - this might not be a proxy contract");
            return Err(anyhow::anyhow!("No implementation found for USDC proxy"));
        }
    }

    println!("✅ All tests passed successfully!");
    Ok(())
}
