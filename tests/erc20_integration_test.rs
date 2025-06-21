use anyhow::Result;
use alloy::primitives::{Address, U256};
use revm_trace::{
    create_evm,
    utils::erc20_utils::query_erc20_balance,
};
use alloy::primitives::utils::format_units;

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";
// USDC 合约地址 (以太坊主网)
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";  
// Binance 的地址 (已知持有大量代币)
const BINANCE_ADDRESS: &str = "0x28C6c06298d514Db089934071355E5743bf21d60";

#[tokio::test(flavor = "multi_thread")]
async fn test_query_erc20_balance() -> Result<()> {
    println!("🔍 Testing ERC20 balance query");
    let mut evm = create_evm(ETH_RPC_URL).await?;
    let token_address: Address = USDC_ADDRESS.parse()?;
    let owner_address: Address = BINANCE_ADDRESS.parse()?;
    let balance = query_erc20_balance(&mut evm, token_address, owner_address, None)?;
    println!("✅ USDC Balance of {}: {}", BINANCE_ADDRESS, balance);
    println!("📊 Balance in human readable (6 decimals): {}", 
            format_units(balance, 6).unwrap());
    // 验证余额不为零 (Binance 应该持有一些 USDC)
    assert!(balance > U256::ZERO, "Expected non-zero USDC balance");

    Ok(())
}
