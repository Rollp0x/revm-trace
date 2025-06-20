use anyhow::Result;
use alloy::{primitives::{Address,U256}};
use revm::inspector::NoOpInspector;
use revm_trace::{
    inspectors::test_inspector::TestInspector,
    evm::builder::EvmBuilder,
    utils::erc20_utils::query_erc20_balance,
};
use alloy::primitives::utils::{format_units};

const RPC_URL: &str = "https://eth.llamarpc.com";

// USDC 合约地址 (以太坊主网)
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";  
// Binance 的地址 (已知持有大量代币)
const BINANCE_ADDRESS: &str = "0x28C6c06298d514Db089934071355E5743bf21d60";

#[tokio::test(flavor = "multi_thread")]
async fn test_query_erc20_balance_single_thread() -> Result<()> {
    println!("🔍 Testing ERC20 balance query - Single Thread Mode");
    
    let inspector = TestInspector;
    let mut evm = EvmBuilder::new(RPC_URL.to_string(), inspector)
        .with_current_runtime()?
        .build()
        .await?;

    let token_address: Address = USDC_ADDRESS.parse()?;
    let owner_address: Address = BINANCE_ADDRESS.parse()?;

    let balance = query_erc20_balance(&mut evm, token_address, owner_address,None)?;
    
    println!("✅ USDC Balance of {}: {}", BINANCE_ADDRESS, balance);
    println!("📊 Balance in human readable (6 decimals): {}", 
            format_units(balance, 6).unwrap());

    // 验证余额不为零 (Binance 应该持有一些 USDC)
    assert!(balance >U256::ZERO, "Expected non-zero USDC balance");

    Ok(())
}

#[cfg(feature = "multi-threading")]
#[tokio::test(flavor = "multi_thread")]
async fn test_query_erc20_balance_multi_thread() -> Result<()> {
    println!("🔍 Testing ERC20 balance query - Multi Thread Mode");
    
    let inspector = NoOpInspector::default();
    let mut evm = EvmBuilder::new(RPC_URL.to_string(), inspector)
        .build_shared()
        .await?;

    let token_address: Address = USDC_ADDRESS.parse()?;
    let owner_address: Address = BINANCE_ADDRESS.parse()?;

    let balance = query_erc20_balance(&mut evm, token_address, owner_address,None)?;
    
    println!("✅ USDC Balance of {}: {}", BINANCE_ADDRESS, balance);
    println!("📊 Balance in human readable (6 decimals): {}", 
            format_units(balance, 6).unwrap());

    // 验证余额不为零
    assert!(balance > U256::ZERO, "Expected non-zero USDC balance");

    Ok(())
}

#[cfg(feature = "multi-threading")]
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_erc20_queries() -> Result<()> {
    println!("🔍 Testing Concurrent ERC20 balance queries");
    
    // 使用多个不同的地址进行并发查询
    let addresses = vec![
        BINANCE_ADDRESS,
        "0x3f5CE5FBFe3E9af3971dD833D26bA9b5C936f0bE", // Binance 热钱包
        "0x28C6c06298d514Db089934071355E5743bf21d60", // Binance 冷钱包
    ];

    let token_address: Address = USDC_ADDRESS.parse()?;
    
    let mut handles = vec![];
    
    for (i, addr_str) in addresses.iter().enumerate() {
        let addr_str = addr_str.to_string();
        let rpc_url = RPC_URL.to_string();
        
        let handle = tokio::spawn(async move {
            let inspector = NoOpInspector::default();
            let mut evm = EvmBuilder::new(rpc_url, inspector)
                .build_shared()
                .await?;

            let owner_address: Address = addr_str.parse()?;
            let balance = query_erc20_balance(&mut evm, token_address, owner_address,None)?;
            
            println!("🏦 Thread {}: USDC Balance of {}: {}", i, addr_str, balance);
            
            Ok::<_, anyhow::Error>((addr_str, balance))
        });
        
        handles.push(handle);
    }

    // 等待所有查询完成
    let mut results = vec![];
    for handle in handles {
        let result = handle.await??;
        results.push(result);
    }

    // 验证所有查询都成功了
    assert_eq!(results.len(), addresses.len());
    println!("✅ All {} concurrent queries completed successfully!", results.len());

    // 显示结果
    for (addr, balance) in results {
        println!("📈 Final: {} -> {:.6} USDC", addr, format_units(balance, 6).unwrap());
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_erc20_token_info() -> Result<()> {
    println!("🔍 Testing ERC20 token information queries");
    
    // 这个测试展示如何查询其他 ERC20 信息
    // TODO: 实现 name(), symbol(), decimals() 查询函数
    
    println!("📝 Note: This test is a placeholder for token info queries");
    println!("📝 Future implementation should include:");
    println!("   - query_erc20_name()");
    println!("   - query_erc20_symbol()");  
    println!("   - query_erc20_decimals()");
    println!("   - query_erc20_total_supply()");

    Ok(())
}
