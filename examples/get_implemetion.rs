// examples/proxy_contract.rs
use alloy::primitives::address;
use anyhow::Result;
use revm_trace::{create_evm_instance, utils::proxy_utils::get_implement};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting proxy implementation test...");

    let mut evm = create_evm_instance("https://rpc.ankr.com/eth",Some(1))?;
    println!("✅ EVM instance created successfully");

    // USDC proxy contract address
    let usdc_proxy = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    println!("📝 Testing USDC proxy contract at: {}", usdc_proxy);

    let implementation_address = get_implement(&mut evm, usdc_proxy).await?;

    match implementation_address {
        Some(impl_addr) => {
            println!("✅ Implementation contract found at: {}", impl_addr);
            println!("🔍 Verifying against known implementation...");

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
